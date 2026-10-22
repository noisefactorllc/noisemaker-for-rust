#!/usr/bin/env python3
"""Compare every Rust CPU effect with the JavaScript CPU oracle through both CLIs."""

from __future__ import annotations

import argparse
import binascii
import json
import math
import os
import re
import struct
import subprocess
import sys
import tempfile
import time
import zlib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CATALOG_PATH = ROOT / "src/generated/catalog.json"
TOLERANCE = 0
OVERLAY_READY_IDS = {
    "filter/fibers",
    "filter/scratches",
    "filter/strayHair",
}
# These three canonical worm-overlay effects only ever expose one-shot overlay
# generation. The JavaScript CLI renders them in Ready mode and has no Initial
# one-shot flag, while the Rust parity CLI path uses Initial semantics
# everywhere else; for exactly these cases the harness compares Ready-mode
# output from both CLIs instead of skipping them.

# Reactive (MIDI/audio) and mesh (OBJ) effects consume host-fed external-input
# fixtures. Neither CLI can render them from catalog defaults alone: the Rust
# CLI binds the fixture through --external-input, while the JavaScript CPU CLI
# has no external-input interface at all (its `--effect random` pool excludes
# them), so the CPU side renders through the CPU checkout's own library with the
# checkout's scripts/parity/reactive-fixtures.js fixtures -- the exact fixture
# module the CPU parity gate itself consumes. The programs mirror the CPU
# checkout's parity/upstream-defaults manifests, and the fixture constants are
# byte-identical on both sides by construction.
EXTERNAL_INPUT_IDS = {
    "synth/roll": "midi",
    "synth/scope": "audio",
    "synth/spectrum": "audio",
    "render/meshLoader": "mesh",
    "render/meshRender": "mesh",
}
EXTERNAL_INPUT_PROGRAMS = {
    "synth/roll": "search synth\n\nroll()\n.write(o0)\n\nrender(o0)",
    "synth/scope": "search synth\n\nscope()\n.write(o0)\n\nrender(o0)",
    "synth/spectrum": "search synth\n\nspectrum()\n.write(o0)\n\nrender(o0)",
    "render/meshLoader": "search render\n\nmeshLoader().write(o0)\n\nrender(o0)",
    "render/meshRender": "search render\n\nmeshLoader()\n  .meshRender()\n  .write(o0)\n\nrender(o0)",
}

# Bounded non-default parameter sweep and stateful multi-frame sequences. Each
# effect's default render is joined by at most SWEEP_VARIANTS one-parameter
# non-default variants (deterministically derived from the catalog metadata)
# and, for stateful chains, SWEEP_FRAMES renders of the default program at the
# SWEEP_TIMES sequence. The bounds keep the sweep's denominator a fixed
# multiple of the catalog instead of an open-ended product.
SWEEP_VARIANTS = 2
SWEEP_FRAMES = 3
SWEEP_TIMES = (0.1, 0.4, 0.7)
# Resource-valued parameters cannot take a non-default DSL literal, and
# `define`-bearing parameters are compile-time choices rather than uniforms.
SWEEP_EXCLUDED_TYPES = {"surface", "geometry", "volume", "mat3", "vec2", "palette"}
# Structural arguments the harness binds itself; the sweep never overrides them.
SWEEP_STRUCTURAL_PARAMS = {"iterationCount", "volumeSize", "stateSize", "searchRadius"}
# Integer parameters whose value multiplies per-render statement execution
# (iteration counts, octaves) are capped just above their catalog default
# (twice the default, or default + 1 when the default is zero), so a bounded
# sweep variant cannot push a render past the runtime's statement budget: the
# catalog maxima of these parameters are unexecutable bounds, not render
# settings (synth/mandelbrot iterations aborts its render at 2000).
SWEEP_COST_SCALING_TOKENS = ("iter", "octave")

EXTERNAL_INPUT_NODE_SCRIPT = """import { CpuRenderer, Surface, createDefaultRegistry, kernelFactories } from '%CPU%/src/index.js'
import { externalInputsForCase } from '%CPU%/scripts/parity/reactive-fixtures.js'
import { writePng } from '%CPU%/src/node/png.js'
const [caseId, out, programOverride] = process.argv.slice(2)
const programs = %PROGRAMS%
const program = programOverride ?? programs[caseId]
const renderer = new CpuRenderer({ registry: createDefaultRegistry(), kernelFactories })
const blank = new Surface(Number(process.env.PARITY_WIDTH), Number(process.env.PARITY_HEIGHT))
blank.format = 'rgba16f'
const rendered = renderer.render(program, {
  width: Number(process.env.PARITY_WIDTH),
  height: Number(process.env.PARITY_HEIGHT),
  time: Number(process.env.PARITY_TIME),
  seed: Number(process.env.PARITY_SEED),
  externalTextures: { imageTex: blank, textTex: blank },
  externalInputs: externalInputsForCase(caseId),
  oneShot: 'initial',
})
await writePng(out, rendered)
"""

FRAME_SEQUENCE_NODE_SCRIPT = """import { CpuRenderer, Surface, createDefaultRegistry, kernelFactories } from '%CPU%/src/index.js'
import { externalInputsForCase } from '%CPU%/scripts/parity/reactive-fixtures.js'
import { writePng } from '%CPU%/src/node/png.js'
import { performance } from 'node:perf_hooks'
const [caseId, out, sizeText, seedText, program, ...timeTexts] = process.argv.slice(2)
const size = Number(sizeText)
const renderer = new CpuRenderer({ registry: createDefaultRegistry(), kernelFactories })
const fixture = new Uint8Array(size * size * 4)
for (let index = 0; index < fixture.length; index += 4) fixture.set([51, 102, 153, 255], index)
let previous = Surface.fromRgba8(size, size, fixture)
const seconds = []
for (const [index, timeText] of timeTexts.entries()) {
  const started = performance.now()
  const rendered = renderer.render(program, {
    width: size, height: size, time: Number(timeText), frame: index, seed: Number(seedText),
    externalTextures: { imageTex: previous, textTex: previous },
    externalInputs: externalInputsForCase(caseId), oneShot: 'initial',
  })
  seconds.push((performance.now() - started) / 1000)
  previous = rendered.surface
  await writePng(`${out}/frame_${index}.png`, previous)
}
console.log(JSON.stringify(seconds))
"""


def _external_input_script(cpu_root: Path) -> str:
    return EXTERNAL_INPUT_NODE_SCRIPT.replace("%CPU%", cpu_root.as_uri()).replace(
        "%PROGRAMS%", json.dumps(EXTERNAL_INPUT_PROGRAMS)
    )


def _frame_sequence_script(cpu_root: Path) -> str:
    return FRAME_SEQUENCE_NODE_SCRIPT.replace("%CPU%", cpu_root.as_uri())


def _feedback_program(program: str) -> str:
    """Make each frame consume the previous frame's imageTex via media and blendMode."""
    search = re.match(r"search [^;\n]+", program)
    target = re.search(r"render\((o\d+)\)\s*$", program)
    if not search or not target:
        raise ValueError("frame program needs a search directive and final render target")
    if re.search(r"\bo6\b", program):
        raise ValueError("frame program already uses feedback surface o6")
    output = target.group(1)
    body = program[search.end():target.start()].strip(" \n;")
    namespaces = search.group()[len("search "):].split(",")
    for namespace in ("synth", "mixer"):
        if namespace not in namespaces:
            namespaces.append(namespace)
    return (
        f"search {','.join(namespaces)}; media().write(o6); {body}; "
        f"read({output}).blendMode(tex:o6,mode:8,mix:50).write({output}); render({output})"
    )


def _external_program(effect_id: str, effect: dict, overrides: dict[str, object]) -> str:
    """Bind sweep overrides onto an external-input effect's fixed program."""
    program = EXTERNAL_INPUT_PROGRAMS[effect_id]
    if not overrides:
        return program
    call = f"{effect['func']}()"
    if program.count(call) != 1:
        raise ValueError(f"external program for {effect_id} lacks a unique {call} call")
    rendered = ",".join(f"{name}:{_literal(value)}" for name, value in sorted(overrides.items()))
    return program.replace(call, f"{effect['func']}({rendered})", 1)


def _sweep_value(spec: dict) -> object | None:
    """A deterministic non-default DSL value for one catalog parameter, or None."""
    kind = spec.get("type")
    default = spec.get("default")
    if kind in SWEEP_EXCLUDED_TYPES or "define" in spec:
        return None
    if kind in ("float", "int"):
        for candidate in (spec.get("max"), spec.get("min")):
            if candidate is not None and candidate != default:
                return candidate
        step = spec.get("step")
        if step and default + step != default:
            return default + step
        return None
    if kind == "boolean":
        return not default
    if kind == "color" and isinstance(default, list):
        candidate = [0, 1, 1, 0.5][: len(default)]
        return candidate if candidate != default else None
    if kind == "vec3":
        low, high = spec.get("min"), spec.get("max")

        def clamp(index: int, value: float) -> float:
            if isinstance(high, list):
                value = min(value, high[min(index, len(high) - 1)])
            elif high is not None:
                value = min(value, high)
            if isinstance(low, list):
                value = max(value, low[min(index, len(low) - 1)])
            elif low is not None:
                value = max(value, low)
            return value

        candidate = [clamp(index, value) for index, value in enumerate((1, 0, 0.5))]
        return candidate if candidate != default else None
    if kind in ("string", "member"):
        for candidate in (spec.get("choices") or {}).values():
            if not isinstance(candidate, str) or not isinstance(default, str):
                continue
            if candidate != default and not any(character.isspace() for character in candidate):
                return candidate
        return None
    return None


def _variant_candidates(effect_id: str, effect: dict) -> list[tuple[str, object]]:
    """Up to SWEEP_VARIANTS deterministic non-default single-parameter overrides."""
    base = _arguments(effect)
    unconditional: list[tuple[str, object]] = []
    conditional: list[tuple[str, object]] = []
    for name in effect.get("paramNames", []):
        if name in SWEEP_STRUCTURAL_PARAMS or name in base:
            continue
        spec = effect.get("params", {}).get(name, {})
        value = _sweep_value(spec)
        default = spec.get("default")
        if (
            spec.get("type") == "int"
            and isinstance(default, int)
            and isinstance(value, int)
            and any(token in name.lower() for token in SWEEP_COST_SCALING_TOKENS)
            and value > max(2 * default, default + 1)
        ):
            value = max(2 * default, default + 1)
        if value is None or _literal(value) == _literal(default):
            continue
        (conditional if spec.get("ui", {}).get("enabledBy") else unconditional).append((name, value))
    return (unconditional + conditional)[:SWEEP_VARIANTS]


def _is_stateful(effect_id: str, effect: dict) -> bool:
    """Whether the effect needs feedback frames (iterated, particle, loop, reactive, mesh)."""
    return (
        bool(effect.get("iterated"))
        or "stateSize" in effect.get("params", {})
        or effect["domain"] in ("loop-begin", "loop-end")
        or effect["namespace"] == "points"
        or effect_id in {"render/pointsEmit", "render/pointsRender", "render/pointsBillboardRender"}
        or effect_id in EXTERNAL_INPUT_IDS
    )


def _sweep_plan(catalog: dict) -> dict[str, list[dict]]:
    """The exact bounded case list per effect: default, variants, frames."""
    plan: dict[str, list[dict]] = {}
    for effect_id in sorted(catalog):
        effect = catalog[effect_id]
        cases = [{"kind": "default", "id": effect_id}]
        for name, value in _variant_candidates(effect_id, effect):
            cases.append({
                "kind": "param", "param": name, "value": value,
                "id": f"{effect_id}#param:{name}",
            })
        if _is_stateful(effect_id, effect):
            for index, frame_time in enumerate(SWEEP_TIMES):
                cases.append({
                    "kind": "frame", "time": frame_time,
                    "id": f"{effect_id}#frame:{index}",
                })
        cases.sort(key=lambda case: case["id"])
        plan[effect_id] = cases
    return plan


def _catalog() -> dict:
    return json.loads(CATALOG_PATH.read_text(encoding="utf-8"))["effects"]


def _literal(value: object) -> str:
    if value is None:
        return "none"
    if value is True:
        return "true"
    if value is False:
        return "false"
    if isinstance(value, str):
        return value
    if isinstance(value, list):
        return "[" + ",".join(_literal(item) for item in value) + "]"
    return str(value)


def _arguments(effect: dict) -> dict[str, str]:
    args: dict[str, str] = {}
    params = effect.get("params", {})
    if effect.get("iterated"):
        args["iterationCount"] = "4" if effect["domain"] == "image" else "1"
    if "volumeSize" in params:
        args["volumeSize"] = "2"
    if "stateSize" in params:
        # Catalog schema minimum; this keeps the shared DSL valid on both CLIs.
        args["stateSize"] = "64"
    if "searchRadius" in params and "min" in params["searchRadius"]:
        args["searchRadius"] = _literal(params["searchRadius"]["min"])
    if effect["namespace"] == "synth3d" and effect["func"] == "flythrough3d":
        args["type"] = "1"
    return args


_UNKNOWN_PARAMETER = re.compile(
    r'Unknown parameter "([A-Za-z_]\w*)" for [\w/]+; accepted: (.+)$', re.MULTILINE
)


def _unknown_parameter_omit(message: str, omit: set[str]) -> bool:
    """Grow `omit` from an oracle "Unknown parameter" DSL rejection.

    Returns True when the message names a parameter the oracle does not
    accept and that has not been omitted yet. The caller then re-renders the
    case without those surface arguments: every argument the oracle accepts
    stays bound, and only the arguments a pre-parameter oracle revision
    rejects are dropped, so the comparison keeps all coverage the oracle can
    render.
    """
    match = _UNKNOWN_PARAMETER.search(message)
    if not match:
        return False
    accepted = {name.strip() for name in match.group(2).split(",")}
    name = match.group(1)
    if name in accepted or name in omit:
        return False
    omit.add(name)
    return True


def _program(
    effect_id: str, effect: dict, overrides: dict[str, object] | None = None,
    omit_surface: set[str] | None = None,
) -> str:
    args = _arguments(effect)
    for name, value in (overrides or {}).items():
        args[name] = _literal(value)
    skipped = omit_surface or set()
    surface_names = [] if effect["kind"] == "generator" else [
        name for name in effect.get("paramNames", [])
        if effect.get("params", {}).get(name, {}).get("type") == "surface"
    ]
    prefix = []
    params = effect.get("params", {})
    for index, name in enumerate(surface_names):
        if name in skipped:
            continue
        value = "inputTex" if index == 0 else f"o{index - 1}"
        if value == params.get(name, {}).get("default"):
            # Binding a surface parameter to its catalog default is a no-op on
            # both CLIs. Omit the argument so the shared DSL also stays valid
            # against published oracle revisions that predate the parameter.
            continue
        if index != 0:
            prefix.append(f"solid(color:#58c).write({value})")
        args[name] = value
    rendered_args = ",".join(f"{name}:{value}" for name, value in sorted(args.items()))
    call = f"{effect['func']}({rendered_args})"
    search = (
        f"search {effect['namespace']},synth,synth3d,filter3d,filter,render,points,mixer,"
        "classicNoisedeck"
    )
    domain = effect["domain"]
    if domain == "loop-begin":
        chain, target = f"solid(color:#58c).{call}.loopEnd().write(o0)", "o0"
    elif domain == "loop-end":
        chain, target = f"solid(color:#58c).loopBegin(iterationCount:1).{call}.write(o0)", "o0"
    elif domain == "volume-generator":
        chain, target = f"{call}.render3d(volumeSize:2).write(o0)", "o0"
    elif domain == "volume-filter":
        chain, target = f"noise3d(volumeSize:2).{call}.render3d(volumeSize:2).write(o0)", "o0"
    elif domain == "volume-renderer":
        chain, target = f"noise3d(volumeSize:2).{call}.write(o0)", "o0"
    elif domain == "image" and (
        effect["namespace"] == "points"
        or effect_id in {"render/pointsEmit", "render/pointsRender", "render/pointsBillboardRender"}
    ):
        middle = call if effect_id == "render/pointsEmit" else f"pointsEmit(stateSize:64,iterationCount:4).{call}"
        suffix = "" if effect_id in {"render/pointsRender", "render/pointsBillboardRender"} else ".pointsRender(iterationCount:4)"
        chain, target = f"solid(color:#58c).{middle}{suffix}.write(o0)", "o0"
    elif domain == "image" and effect["kind"] == "generator":
        chain, target = f"{call}.write(o0)", "o0"
    elif domain == "image":
        chain, target = f"solid(color:#58c).{call}.write(o7)", "o7"
    else:
        raise ValueError(f"unhandled domain {domain!r} for {effect_id}")
    return f"{search}; {'; '.join(prefix + [chain])}; render({target})"


def _chunk(kind: bytes, data: bytes) -> bytes:
    return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", binascii.crc32(kind + data) & 0xFFFFFFFF)


def _write_fixture(path: Path, width: int, height: int) -> None:
    row = bytes([51, 102, 153, 255]) * width
    raw = b"".join(b"\0" + row for _ in range(height))
    png = b"\x89PNG\r\n\x1a\n" + _chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
    png += _chunk(b"IDAT", zlib.compress(raw)) + _chunk(b"IEND", b"")
    path.write_bytes(png)


def _paeth(a: int, b: int, c: int) -> int:
    p = a + b - c
    pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
    return a if pa <= pb and pa <= pc else b if pb <= pc else c


def _decode_png(path: Path) -> tuple[int, int, bytes]:
    data = path.read_bytes()
    if not data.startswith(b"\x89PNG\r\n\x1a\n"):
        raise ValueError("invalid PNG signature")
    offset, compressed = 8, bytearray()
    width = height = color_type = bit_depth = None
    saw_ihdr = False
    saw_iend = False
    while offset < len(data):
        if len(data) - offset < 12:
            raise ValueError("truncated PNG chunk")
        length = struct.unpack(">I", data[offset:offset + 4])[0]
        chunk_end = offset + 12 + length
        if chunk_end > len(data):
            raise ValueError("truncated PNG chunk payload")
        kind = data[offset + 4:offset + 8]
        payload = data[offset + 8:offset + 8 + length]
        expected_crc = struct.unpack(">I", data[offset + 8 + length:chunk_end])[0]
        actual_crc = binascii.crc32(kind + payload) & 0xFFFFFFFF
        if expected_crc != actual_crc:
            raise ValueError(f"invalid PNG CRC for {kind.decode('ascii', 'replace')}")
        offset = chunk_end
        if kind == b"IHDR":
            if saw_ihdr or length != 13 or compressed:
                raise ValueError("invalid PNG IHDR ordering")
            width, height, bit_depth, color_type, compression, filtering, interlace = struct.unpack(">IIBBBBB", payload)
            saw_ihdr = True
            if width == 0 or height == 0:
                raise ValueError("zero-sized PNG")
            if bit_depth != 8 or compression or filtering or interlace:
                raise ValueError("unsupported PNG encoding")
        elif kind == b"IDAT":
            if not saw_ihdr or saw_iend:
                raise ValueError("invalid PNG IDAT ordering")
            compressed.extend(payload)
        elif kind == b"IEND":
            if length != 0 or not saw_ihdr:
                raise ValueError("invalid PNG IEND")
            saw_iend = True
            break
    if not saw_iend or offset != len(data):
        raise ValueError("PNG is missing a terminal IEND chunk")
    channels = {0: 1, 2: 3, 4: 2, 6: 4}.get(color_type)
    if width is None or channels is None or not compressed:
        raise ValueError("unsupported PNG color type")
    raw = zlib.decompress(compressed)
    stride = width * channels
    expected_raw = (stride + 1) * height
    if len(raw) != expected_raw:
        raise ValueError(f"invalid decompressed PNG length: expected {expected_raw}, received {len(raw)}")
    previous = bytearray(stride)
    rgba = bytearray()
    position = 0
    for _ in range(height):
        filter_kind = raw[position]
        row = bytearray(raw[position + 1:position + 1 + stride])
        position += stride + 1
        for index in range(stride):
            left = row[index - channels] if index >= channels else 0
            up = previous[index]
            upper_left = previous[index - channels] if index >= channels else 0
            if filter_kind == 1:
                row[index] = (row[index] + left) & 255
            elif filter_kind == 2:
                row[index] = (row[index] + up) & 255
            elif filter_kind == 3:
                row[index] = (row[index] + ((left + up) // 2)) & 255
            elif filter_kind == 4:
                row[index] = (row[index] + _paeth(left, up, upper_left)) & 255
            elif filter_kind != 0:
                raise ValueError(f"unsupported PNG filter {filter_kind}")
        for pixel in range(width):
            source = row[pixel * channels:(pixel + 1) * channels]
            if color_type == 0:
                rgba.extend((source[0], source[0], source[0], 255))
            elif color_type == 2:
                rgba.extend((*source, 255))
            elif color_type == 4:
                rgba.extend((source[0], source[0], source[0], source[1]))
            else:
                rgba.extend(source)
        previous = row
    return width, height, bytes(rgba)


def _run(command: list[str], program: str, timeout: float, cwd: Path) -> tuple[str, float]:
    started = time.monotonic()
    try:
        completed = subprocess.run(
            command,
            input=program,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd=cwd,
            timeout=timeout,
            check=False,
        )
    except subprocess.TimeoutExpired as error:
        raise RuntimeError(f"timeout after {timeout:g}s") from error
    elapsed = time.monotonic() - started
    if completed.returncode != 0:
        diagnostic = completed.stderr.strip() or completed.stdout.strip() or f"exit {completed.returncode}"
        raise RuntimeError(diagnostic)
    return completed.stdout, elapsed


def _metrics(rust: bytes, js: bytes) -> dict:
    if len(rust) != len(js):
        raise ValueError(f"shape mismatch: Rust has {len(rust)} channels, JS has {len(js)}")
    deltas = [abs(left - right) for left, right in zip(rust, js)]
    maximum = max(deltas, default=0)
    return {
        "max_delta": maximum,
        "mean_delta": sum(deltas) / len(deltas) if deltas else 0.0,
        "differing_channels": sum(delta != 0 for delta in deltas),
        "channels_over_2": sum(delta > 2 for delta in deltas),
        "pass": maximum <= TOLERANCE,
    }


def _summarize(results: list[dict], size: int, render_time: float, seed: int) -> dict:
    compared = [record for record in results if record["status"] == "compared"]
    unsupported = [record for record in results if record["status"] == "unsupported"]
    errors = [record for record in results if record["status"] == "error"]
    return {
        "catalog": len(results),
        "expected": len(results),
        "executed": len(compared),
        "compared": len(compared),
        "unsupported": len(unsupported),
        "errors": len(errors),
        "passed": sum(record["pass"] for record in compared),
        "failed": sum(not record["pass"] for record in compared)
        + len(errors),
        "byte_exact": sum(record["max_delta"] == 0 for record in compared),
        "tolerance": TOLERANCE,
        "size": size,
        "time": render_time,
        "seed": seed,
        "results": results,
    }


def _validate_case(record: dict, channel_count: int) -> None:
    """Shared strictness for one result record in either report shape."""
    status = record.get("status")
    if status == "unsupported":
        if record.get("side") not in {"rust", "js", "both"} or not record.get("reason"):
            raise ValueError(f"unsupported record lacks side/reason: {record.get('id')}")
    elif status == "compared":
        required = {"max_delta", "mean_delta", "differing_channels", "channels_over_2", "pass"}
        if not required.issubset(record):
            raise ValueError(f"compared record lacks metrics: {record.get('id')}")
        if (
            isinstance(record["max_delta"], bool)
            or not isinstance(record["max_delta"], int)
            or record["max_delta"] < 0
            or not isinstance(record["mean_delta"], (int, float))
            or not math.isfinite(record["mean_delta"])
            or not 0 <= record["mean_delta"] <= record["max_delta"]
            or not isinstance(record["differing_channels"], int)
            or not 0 <= record["channels_over_2"] <= record["differing_channels"] <= channel_count
            or not isinstance(record["pass"], bool)
            or record["pass"] != (record["max_delta"] <= TOLERANCE)
        ):
            raise ValueError(f"inconsistent metrics: {record.get('id')}")
        if record["max_delta"] > TOLERANCE or not record["pass"]:
            raise ValueError(f"unapproved delta above {TOLERANCE}: {record.get('id')}")
    elif status == "error":
        if not record.get("reason"):
            raise ValueError(f"error record lacks reason: {record.get('id')}")
    else:
        raise ValueError(f"invalid status for {record.get('id')}: {status!r}")


def _validate(report: dict, expected_ids: list[str]) -> None:
    results = report.get("results")
    if not isinstance(results, list) or len(results) != len(expected_ids):
        raise ValueError("results do not match catalog denominator")
    ids = [record.get("id") for record in results]
    if ids != expected_ids or len(set(ids)) != len(ids):
        raise ValueError("result IDs must be unique and sorted")
    unsupported = 0
    compared = 0
    errors = 0
    passed = 0
    byte_exact = 0
    channel_count = report.get("size", 0) ** 2 * 4
    for record in results:
        _validate_case(record, channel_count)
        status = record.get("status")
        if status == "unsupported":
            unsupported += 1
        elif status == "compared":
            compared += 1
            passed += int(record["pass"])
            byte_exact += int(record["max_delta"] == 0)
        else:
            errors += 1
    if (
        report.get("catalog") != len(expected_ids)
        or report.get("expected") != len(expected_ids)
        or report.get("executed") != compared
        or report.get("compared") != compared
        or report.get("unsupported") != unsupported
        or report.get("errors") != errors
    ):
        raise ValueError("summary denominator mismatch")
    if len(expected_ids) != compared + unsupported + errors:
        raise ValueError("catalog != compared + unsupported + errors")
    if (
        report.get("passed") != passed
        or report.get("failed") != compared - passed + errors
        or report.get("byte_exact") != byte_exact
        or report.get("tolerance") != TOLERANCE
        or not isinstance(report.get("size"), int)
        or report["size"] <= 0
        or not isinstance(report.get("time"), (int, float))
        or not math.isfinite(report["time"])
        or not isinstance(report.get("seed"), int)
    ):
        raise ValueError("summary metrics are inconsistent")
    if report.get("failed"):
        raise ValueError(f"report contains {report['failed']} failure(s)")


def _sweep_case_dict(case: dict) -> dict:
    """The report's compact per-record case description."""
    if case["kind"] == "param":
        return {"kind": "param", "param": case["param"], "value": case["value"]}
    if case["kind"] == "frame":
        return {"kind": "frame", "time": case["time"], "feedback": "previous-output-surface"}
    return {"kind": "default"}


def _summarize_sweep(results: list[dict], plan: dict[str, list[dict]], size: int, render_time: float, seed: int) -> dict:
    compared = [record for record in results if record["status"] == "compared"]
    unsupported = [record for record in results if record["status"] == "unsupported"]
    errors = [record for record in results if record["status"] == "error"]
    expected = [case["id"] for effect_id in sorted(plan) for case in plan[effect_id]]
    return {
        "expected": len(expected),
        "executed": len(compared),
        "compared": len(compared),
        "unsupported": len(unsupported),
        "errors": len(errors),
        "passed": sum(record["pass"] for record in compared),
        "failed": sum(not record["pass"] for record in compared) + len(errors),
        "byte_exact": sum(record["max_delta"] == 0 for record in compared),
        "tolerance": TOLERANCE,
        "sweep": {
            "variants": SWEEP_VARIANTS,
            "frames": SWEEP_FRAMES,
            "times": list(SWEEP_TIMES),
            "feedback": "previous-output-surface",
            "stateful_effects": sum(
                1 for cases in plan.values() if any(case["kind"] == "frame" for case in cases)
            ),
        },
        "size": size,
        "time": render_time,
        "seed": seed,
        "results": results,
    }


def _validate_sweep(report: dict, plan: dict[str, list[dict]]) -> None:
    expected = [case["id"] for effect_id in sorted(plan) for case in plan[effect_id]]
    results = report.get("results")
    if not isinstance(results, list) or len(results) != len(expected):
        raise ValueError("results do not match sweep denominator")
    ids = [record.get("id") for record in results]
    if ids != sorted(expected) or len(set(ids)) != len(ids):
        raise ValueError("sweep result IDs must be unique and sorted")
    by_id = {
        case["id"]: (effect_id, case)
        for effect_id, cases in plan.items()
        for case in cases
    }
    unsupported = 0
    compared = 0
    errors = 0
    passed = 0
    byte_exact = 0
    channel_count = report.get("size", 0) ** 2 * 4
    for record in results:
        _validate_case(record, channel_count)
        effect_id, case = by_id[record["id"]]
        if record.get("effect") != effect_id or record.get("case") != _sweep_case_dict(case):
            raise ValueError(f"record does not match its planned case: {record['id']}")
        status = record.get("status")
        if status == "unsupported":
            unsupported += 1
        elif status == "compared":
            compared += 1
            passed += int(record["pass"])
            byte_exact += int(record["max_delta"] == 0)
        else:
            errors += 1
    if (
        report.get("expected") != len(expected)
        or report.get("executed") != compared
        or report.get("compared") != compared
        or report.get("unsupported") != unsupported
        or report.get("errors") != errors
        or len(expected) != compared + unsupported + errors
    ):
        raise ValueError("sweep summary denominator mismatch")
    if (
        report.get("passed") != passed
        or report.get("failed") != compared - passed + errors
        or report.get("byte_exact") != byte_exact
        or report.get("tolerance") != TOLERANCE
        or not isinstance(report.get("size"), int)
        or report["size"] <= 0
        or not isinstance(report.get("time"), (int, float))
        or not math.isfinite(report["time"])
        or not isinstance(report.get("seed"), int)
    ):
        raise ValueError("sweep summary metrics are inconsistent")
    sweep = report.get("sweep")
    if (
        not isinstance(sweep, dict)
        or sweep.get("variants") != SWEEP_VARIANTS
        or sweep.get("frames") != SWEEP_FRAMES
        or sweep.get("times") != list(SWEEP_TIMES)
        or sweep.get("feedback") != "previous-output-surface"
        or not isinstance(sweep.get("stateful_effects"), int)
        or sweep["stateful_effects"] < 0
    ):
        raise ValueError("sweep bounds are inconsistent")
    if report.get("failed"):
        raise ValueError(f"sweep contains {report['failed']} failure(s)")


def _schema_report(fault: str | None) -> dict:
    ids = sorted(_catalog())
    first = {
        "id": ids[0], "status": "compared", "max_delta": 0, "mean_delta": 0.0,
        "differing_channels": 0, "channels_over_2": 0, "pass": True,
    }
    if fault == "delta":
        first.update({
            "max_delta": 3,
            "mean_delta": 0.25,
            "differing_channels": 1,
            "channels_over_2": 1,
            "pass": False,
        })
    results = [first]
    for effect_id in ids[1:]:
        results.append({
            "id": effect_id, "status": "unsupported", "side": "both",
            "reason": "schema-only deterministic fixture",
        })
    if fault == "reason":
        results[1]["reason"] = ""
    return _summarize(results, 8, 0.25, 1)


def _sequence_timings(stdout: str, count: int, side: str) -> list[float]:
    try:
        timings = json.loads(stdout)
    except json.JSONDecodeError as error:
        raise ValueError(f"{side} sequence returned invalid timings") from error
    if (
        not isinstance(timings, list)
        or len(timings) != count
        or any(not isinstance(value, (int, float)) or not math.isfinite(value) or value < 0 for value in timings)
    ):
        raise ValueError(f"{side} sequence returned invalid timings")
    return timings


def _compare_frame_sequence(
    args: argparse.Namespace, effect_id: str, effect: dict, cases: list[dict],
    rust_example: Path, node_harness: Path, js_cwd: Path,
) -> list[dict]:
    """Render the complete feedback sequence once per side and compare every frame."""
    def error_records(reason: str) -> list[dict]:
        return [
            {"id": case["id"], "effect": effect_id, "case": _sweep_case_dict(case),
             "status": "error", "reason": reason}
            for case in cases
        ]

    try:
        base = (
            EXTERNAL_INPUT_PROGRAMS[effect_id]
            if effect_id in EXTERNAL_INPUT_IDS else _program(effect_id, effect)
        )
        program = _feedback_program(base)
        with tempfile.TemporaryDirectory(prefix="noisemaker-rust-parity-sequence-") as temporary:
            directory = Path(temporary)
            rust_dir, js_dir = directory / "rust", directory / "js"
            rust_dir.mkdir()
            js_dir.mkdir()
            times = [str(case["time"]) for case in cases]
            rust_stdout, _ = _run(
                [str(rust_example), str(rust_dir), str(args.size), str(args.seed), effect_id, *times],
                program, args.timeout * len(cases), ROOT,
            )
            js_stdout, _ = _run(
                ["node", str(node_harness), effect_id, str(js_dir), str(args.size),
                 str(args.seed), program, *times],
                "", args.timeout * len(cases), js_cwd,
            )
            rust_times = _sequence_timings(rust_stdout, len(cases), "Rust")
            js_times = _sequence_timings(js_stdout, len(cases), "JS")
            records = []
            for index, case in enumerate(cases):
                rust_width, rust_height, rust_bytes = _decode_png(rust_dir / f"frame_{index}.png")
                js_width, js_height, js_bytes = _decode_png(js_dir / f"frame_{index}.png")
                if (rust_width, rust_height) != (args.size, args.size):
                    raise ValueError(f"Rust frame {index} size mismatch: {rust_width}x{rust_height}")
                if (js_width, js_height) != (args.size, args.size):
                    raise ValueError(f"JS frame {index} size mismatch: {js_width}x{js_height}")
                records.append({
                    "id": case["id"], "effect": effect_id, "case": _sweep_case_dict(case),
                    "status": "compared", **_metrics(rust_bytes, js_bytes),
                    "rust_seconds": rust_times[index], "js_seconds": js_times[index],
                })
            return records
    except (OSError, RuntimeError, ValueError, zlib.error, struct.error, IndexError) as error:
        return error_records(str(error))


def _sweep_main(args: argparse.Namespace, catalog: dict, selected: list[str]) -> int:
    """Run the bounded parameter sweep and stateful multi-frame sequences."""
    wanted = set(selected)
    plan = {effect_id: cases for effect_id, cases in _sweep_plan(catalog).items() if effect_id in wanted}

    node_harness = None
    if any(effect_id in EXTERNAL_INPUT_IDS for effect_id in plan):
        cpu_root = args.js.resolve().parents[1]
        harness_fd, harness_path = tempfile.mkstemp(prefix="noisemaker-rust-parity-ext-", suffix=".mjs")
        os.close(harness_fd)
        node_harness = Path(harness_path)
        node_harness.write_text(_external_input_script(cpu_root), encoding="utf-8")
    js_cwd = args.js.resolve().parents[1]
    frame_harness = None
    frame_cases_present = any(any(case["kind"] == "frame" for case in cases) for cases in plan.values())
    rust_example = args.rust_sequence
    if frame_cases_present:
        harness_fd, harness_path = tempfile.mkstemp(prefix="noisemaker-rust-parity-frames-", suffix=".mjs")
        os.close(harness_fd)
        frame_harness = Path(harness_path)
        frame_harness.write_text(_frame_sequence_script(js_cwd), encoding="utf-8")

    results: list[dict] = []
    for index, effect_id in enumerate(sorted(plan), 1):
        effect = catalog[effect_id]
        frame_records = None
        for case in plan[effect_id]:
            if case["kind"] == "frame":
                if frame_records is None:
                    frame_cases = [item for item in plan[effect_id] if item["kind"] == "frame"]
                    frame_records = _compare_frame_sequence(
                        args, effect_id, effect, frame_cases, rust_example, frame_harness,
                        js_cwd,
                    )
                frame_index = int(case["id"].rsplit(":", 1)[1])
                record = frame_records[frame_index]
                results.append(record)
                if record["status"] == "compared":
                    print(
                        f"[{index}/{len(plan)}] {case['id']}: max={record['max_delta']} "
                        f"mean={record['mean_delta']:.4f} over2={record['channels_over_2']} "
                        f"rust={record['rust_seconds']:.3f}s js={record['js_seconds']:.3f}s",
                        flush=True,
                    )
                else:
                    print(f"[{index}/{len(plan)}] {case['id']}: ERROR: {record['reason']}", flush=True)
                continue
            case_dict = _sweep_case_dict(case)
            render_time = args.time
            with tempfile.TemporaryDirectory(prefix=f"noisemaker-rust-parity-{index:03d}-") as temporary:
                directory = Path(temporary)
                fixture = directory / "input.png"
                _write_fixture(fixture, args.size, args.size)
                overlay_ready = effect_id in OVERLAY_READY_IDS
                external_fixture = EXTERNAL_INPUT_IDS.get(effect_id)
                overrides = {case["param"]: case["value"]} if case["kind"] == "param" else None
                rust_png = directory / "rust.png"
                js_png = directory / "js.png"
                try:
                    if external_fixture:
                        program = _external_program(effect_id, effect, overrides or {})
                        node_env = {
                            **os.environ,
                            "PARITY_WIDTH": str(args.size),
                            "PARITY_HEIGHT": str(args.size),
                            "PARITY_TIME": str(render_time),
                            "PARITY_SEED": str(args.seed),
                        }
                    else:
                        program = _program(effect_id, effect, overrides)
                    common = [
                        "render", "-", "--width", str(args.size), "--height", str(args.size),
                        "--time", str(render_time), "--seed", str(args.seed), "--input", str(fixture),
                    ]
                    if external_fixture:
                        common += ["--external-input", external_fixture]
                    _, rust_elapsed = _run(
                        [str(args.rust), *common, "--one-shot", "ready" if overlay_ready else "initial",
                         "--output", str(rust_png)],
                        program, args.timeout, ROOT,
                    )
                    if external_fixture:
                        js_started = time.monotonic()
                        try:
                            harness = subprocess.run(
                                ["node", str(node_harness), effect_id, str(js_png), program],
                                cwd=js_cwd,
                                env=node_env,
                                stdout=subprocess.PIPE,
                                stderr=subprocess.PIPE,
                                timeout=args.timeout,
                                check=False,
                            )
                        except subprocess.TimeoutExpired as error:
                            raise RuntimeError(f"timeout after {args.timeout:g}s") from error
                        js_elapsed = time.monotonic() - js_started
                        if harness.returncode != 0:
                            diagnostic = harness.stderr.strip() or harness.stdout.strip() or f"exit {harness.returncode}"
                            raise RuntimeError(f"external-input CPU harness: {diagnostic}")
                    else:
                        _, js_elapsed = _run(
                            ["node", str(args.js), *common, "--output", str(js_png)],
                            program, args.timeout, js_cwd,
                        )
                    rust_width, rust_height, rust_bytes = _decode_png(rust_png)
                    js_width, js_height, js_bytes = _decode_png(js_png)
                    if (rust_width, rust_height) != (args.size, args.size):
                        raise ValueError(
                            f"Rust output size mismatch: expected {args.size}x{args.size}, "
                            f"received {rust_width}x{rust_height}"
                        )
                    if (js_width, js_height) != (args.size, args.size):
                        raise ValueError(
                            f"JS output size mismatch: expected {args.size}x{args.size}, "
                            f"received {js_width}x{js_height}"
                        )
                    if (rust_width, rust_height) != (js_width, js_height):
                        raise ValueError(
                            f"shape mismatch: Rust {rust_width}x{rust_height}, JS {js_width}x{js_height}"
                        )
                    record = {
                        "id": case["id"], "effect": effect_id, "case": case_dict,
                        "status": "compared", **_metrics(rust_bytes, js_bytes),
                    }
                    if overlay_ready:
                        record["overlay_one_shot"] = "ready"
                    if external_fixture:
                        record["external_inputs"] = external_fixture
                    record["rust_seconds"] = rust_elapsed
                    record["js_seconds"] = js_elapsed
                    results.append(record)
                    print(
                        f"[{index}/{len(plan)}] {case['id']}: max={record['max_delta']} "
                        f"mean={record['mean_delta']:.4f} over2={record['channels_over_2']} "
                        f"rust={rust_elapsed:.3f}s js={js_elapsed:.3f}s",
                        flush=True,
                    )
                except (OSError, RuntimeError, ValueError, zlib.error, struct.error, IndexError) as error:
                    results.append({
                        "id": case["id"], "effect": effect_id, "case": case_dict,
                        "status": "error", "reason": str(error),
                    })
                    print(f"[{index}/{len(plan)}] {case['id']}: ERROR: {error}", flush=True)

    try:
        report = _summarize_sweep(results, plan, args.size, args.time, args.seed)
        if args.json:
            args.json.parent.mkdir(parents=True, exist_ok=True)
            args.json.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        print(
            f"sweep denominator: expected={report['expected']} executed={report['executed']} "
            f"compared={report['compared']} unsupported={report['unsupported']} errors={report['errors']} "
            f"passed={report['passed']} failed={report['failed']} byte_exact={report['byte_exact']} "
            f"tolerance={report['tolerance']}",
            flush=True,
        )
        try:
            _validate_sweep(report, plan)
        except ValueError as error:
            print(f"parity validation failed: {error}", file=sys.stderr)
            return 1
    finally:
        if node_harness is not None:
            node_harness.unlink(missing_ok=True)
        if frame_harness is not None:
            frame_harness.unlink(missing_ok=True)
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rust", type=Path)
    parser.add_argument("--rust-sequence", type=Path,
                        help="compiled parity_sequence example for frame cases in --sweep")
    parser.add_argument("--js", type=Path)
    parser.add_argument("--only", action="append", default=[])
    parser.add_argument("--json", type=Path)
    parser.add_argument("--timeout", type=float, default=30.0)
    parser.add_argument("--size", type=int, default=8)
    parser.add_argument("--time", type=float, default=0.25)
    parser.add_argument("--seed", type=int, default=1)
    parser.add_argument(
        "--sweep", action="store_true",
        help="also run the bounded non-default parameter sweep and stateful multi-frame sequences",
    )
    parser.add_argument("--schema-test", action="store_true", help=argparse.SUPPRESS)
    parser.add_argument("--schema-fault", choices=["delta", "reason"], help=argparse.SUPPRESS)
    args = parser.parse_args(argv)
    catalog = _catalog()

    if args.schema_test:
        report = _schema_report(args.schema_fault)
        try:
            _validate(report, sorted(catalog))
        except ValueError as error:
            print(f"parity validation failed: {error}", file=sys.stderr)
            return 1
        print(json.dumps(report, sort_keys=True))
        return 0

    if args.rust is None or args.js is None:
        parser.error("--rust and --js are required")
    if args.timeout <= 0 or args.size <= 0 or not math.isfinite(args.time):
        parser.error("timeout/size must be positive and time must be finite")
    selected = sorted(catalog)
    if args.only:
        requested = set(args.only)
        missing = requested.difference(catalog)
        if missing:
            parser.error(f"unknown --only IDs: {', '.join(sorted(missing))}")
        selected = [effect_id for effect_id in selected if effect_id in requested]
    if args.sweep:
        if any(_is_stateful(effect_id, catalog[effect_id]) for effect_id in selected):
            if args.rust_sequence is None:
                parser.error("--rust-sequence is required for --sweep frame cases")
            args.rust_sequence = args.rust_sequence.absolute()
        return _sweep_main(args, catalog, selected)

    # One shared node harness for every external-input case: the JavaScript CPU
    # CLI cannot bind external inputs, so the CPU side renders through the CPU
    # checkout's library (see EXTERNAL_INPUT_IDS above).
    node_harness = None
    if any(effect_id in EXTERNAL_INPUT_IDS for effect_id in selected):
        if args.js is None:
            parser.error("--js is required")
        cpu_root = args.js.resolve().parents[1]
        harness_fd, harness_path = tempfile.mkstemp(prefix="noisemaker-rust-parity-ext-", suffix=".mjs")
        os.close(harness_fd)
        node_harness = Path(harness_path)
        node_harness.write_text(_external_input_script(cpu_root), encoding="utf-8")

    results: list[dict] = []
    for index, effect_id in enumerate(selected, 1):
        with tempfile.TemporaryDirectory(prefix=f"noisemaker-rust-parity-{index:03d}-") as temporary:
            directory = Path(temporary)
            fixture = directory / "input.png"
            _write_fixture(fixture, args.size, args.size)
            overlay_ready = effect_id in OVERLAY_READY_IDS
            external_fixture = EXTERNAL_INPUT_IDS.get(effect_id)
            rust_png = directory / f"rust-{index}.png"
            js_png = directory / f"js-{index}.png"
            common = [
                "render", "-", "--width", str(args.size), "--height", str(args.size),
                "--time", str(args.time), "--seed", str(args.seed), "--input", str(fixture),
            ]
            if external_fixture:
                common += ["--external-input", external_fixture]
            try:
                omit: set[str] = set()
                # Render the JavaScript side first: it is the side that can
                # reject a surface parameter this oracle revision predates.
                # Retry only the JS render until the program is accepted, then
                # render the Rust side once against the same final program.
                js_cwd = args.js.resolve().parents[1]
                while True:
                    program = (
                        EXTERNAL_INPUT_PROGRAMS[effect_id]
                        if external_fixture
                        else _program(effect_id, catalog[effect_id], omit_surface=omit)
                    )
                    js_started = time.monotonic()
                    try:
                        if external_fixture:
                            try:
                                harness = subprocess.run(
                                    ["node", str(node_harness), effect_id, str(js_png)],
                                    cwd=js_cwd,
                                    env={
                                        **os.environ,
                                        "PARITY_WIDTH": str(args.size),
                                        "PARITY_HEIGHT": str(args.size),
                                        "PARITY_TIME": str(args.time),
                                        "PARITY_SEED": str(args.seed),
                                    },
                                    stdout=subprocess.PIPE,
                                    stderr=subprocess.PIPE,
                                    timeout=args.timeout,
                                    check=False,
                                )
                            except subprocess.TimeoutExpired as error:
                                raise RuntimeError(f"timeout after {args.timeout:g}s") from error
                            js_elapsed = time.monotonic() - js_started
                            if harness.returncode != 0:
                                diagnostic = (
                                    harness.stderr.strip() or harness.stdout.strip()
                                    or f"exit {harness.returncode}"
                                )
                                raise RuntimeError(f"external-input CPU harness: {diagnostic}")
                        else:
                            _run(
                                ["node", str(args.js), *common, "--output", str(js_png)],
                                program, args.timeout, js_cwd,
                            )
                            js_elapsed = time.monotonic() - js_started
                        break
                    except RuntimeError as error:
                        if len(omit) < 6 and _unknown_parameter_omit(str(error), omit):
                            continue
                        raise
                _, rust_elapsed = _run(
                    [str(args.rust), *common, "--one-shot",
                     "ready" if overlay_ready else "initial",
                     "--output", str(rust_png)],
                    program, args.timeout, ROOT,
                )
                rust_width, rust_height, rust_bytes = _decode_png(rust_png)
                js_width, js_height, js_bytes = _decode_png(js_png)
                if (rust_width, rust_height) != (args.size, args.size):
                    raise ValueError(
                        f"Rust output size mismatch: expected {args.size}x{args.size}, "
                        f"received {rust_width}x{rust_height}"
                    )
                if (js_width, js_height) != (args.size, args.size):
                    raise ValueError(
                        f"JS output size mismatch: expected {args.size}x{args.size}, "
                        f"received {js_width}x{js_height}"
                    )
                if (rust_width, rust_height) != (js_width, js_height):
                    raise ValueError(
                        f"shape mismatch: Rust {rust_width}x{rust_height}, "
                        f"JS {js_width}x{js_height}"
                    )
                record = {"id": effect_id, "status": "compared", **_metrics(rust_bytes, js_bytes)}
                if overlay_ready:
                    record["overlay_one_shot"] = "ready"
                if external_fixture:
                    record["external_inputs"] = external_fixture
                if omit:
                    record["omitted_surface_args"] = sorted(omit)
                record["rust_seconds"] = rust_elapsed
                record["js_seconds"] = js_elapsed
                results.append(record)
                print(
                    f"[{index}/{len(selected)}] {effect_id}: max={record['max_delta']} "
                    f"mean={record['mean_delta']:.4f} over2={record['channels_over_2']} "
                    f"rust={rust_elapsed:.3f}s js={js_elapsed:.3f}s",
                    flush=True,
                )
            except (OSError, RuntimeError, ValueError, zlib.error, struct.error, IndexError) as error:
                results.append({"id": effect_id, "status": "error", "reason": str(error)})
                print(f"[{index}/{len(selected)}] {effect_id}: ERROR: {error}", flush=True)

    report = _summarize(results, args.size, args.time, args.seed)
    if args.json:
        args.json.parent.mkdir(parents=True, exist_ok=True)
        args.json.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(
        f"denominator: catalog={report['catalog']} compared={report['compared']} "
        f"unsupported={report['unsupported']} errors={report['errors']} "
        f"passed={report['passed']} failed={report['failed']} "
        f"byte_exact={report['byte_exact']}",
        flush=True,
    )
    try:
        _validate(report, selected)
    except ValueError as error:
        print(f"parity validation failed: {error}", file=sys.stderr)
        return 1
    finally:
        if node_harness is not None:
            node_harness.unlink(missing_ok=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
