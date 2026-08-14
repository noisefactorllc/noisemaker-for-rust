#!/usr/bin/env python3
"""Compare every Rust CPU effect with the JavaScript CPU oracle through both CLIs."""

from __future__ import annotations

import argparse
import binascii
import json
import math
import os
import struct
import subprocess
import sys
import tempfile
import time
import zlib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CATALOG_PATH = ROOT / "src/generated/catalog.json"
TOLERANCE = 2
OVERLAY_INTERFACE_UNSUPPORTED = {
    "filter/fibers",
    "filter/scratches",
    "filter/strayHair",
}
OVERLAY_REASON = (
    "the JavaScript CLI exposes Ready one-shot overlay generation but no Initial one-shot flag; "
    "the Rust parity CLI path intentionally uses Initial semantics"
)


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


def _program(effect_id: str, effect: dict) -> str:
    args = _arguments(effect)
    surface_names = [] if effect["kind"] == "generator" else [
        name for name in effect.get("paramNames", [])
        if effect.get("params", {}).get(name, {}).get("type") == "surface"
    ]
    prefix = []
    for index, name in enumerate(surface_names):
        if index == 0:
            args[name] = "inputTex"
        else:
            surface = f"o{index - 1}"
            prefix.append(f"solid(color:#58c).write({surface})")
            args[name] = surface
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
        "channels_over_2": sum(delta > TOLERANCE for delta in deltas),
        "pass": maximum <= TOLERANCE,
    }


def _summarize(results: list[dict], size: int, render_time: float, seed: int) -> dict:
    compared = [record for record in results if record["status"] == "compared"]
    unsupported = [record for record in results if record["status"] == "unsupported"]
    errors = [record for record in results if record["status"] == "error"]
    return {
        "catalog": len(results),
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
        status = record.get("status")
        if status == "unsupported":
            unsupported += 1
            if record.get("side") not in {"rust", "js", "both"} or not record.get("reason"):
                raise ValueError(f"unsupported record lacks side/reason: {record.get('id')}")
        elif status == "compared":
            compared += 1
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
            passed += int(record["pass"])
            byte_exact += int(record["max_delta"] == 0)
            if record["max_delta"] > TOLERANCE or not record["pass"]:
                raise ValueError(f"unapproved delta above {TOLERANCE}: {record.get('id')}")
        elif status == "error":
            errors += 1
            if not record.get("reason"):
                raise ValueError(f"error record lacks reason: {record.get('id')}")
        else:
            raise ValueError(f"invalid status for {record.get('id')}: {status!r}")
    if report.get("catalog") != len(expected_ids):
        raise ValueError("catalog count mismatch")
    if (
        report.get("compared") != compared
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


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rust", type=Path)
    parser.add_argument("--js", type=Path)
    parser.add_argument("--only", action="append", default=[])
    parser.add_argument("--json", type=Path)
    parser.add_argument("--timeout", type=float, default=30.0)
    parser.add_argument("--size", type=int, default=8)
    parser.add_argument("--time", type=float, default=0.25)
    parser.add_argument("--seed", type=int, default=1)
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

    results: list[dict] = []
    for index, effect_id in enumerate(selected, 1):
        with tempfile.TemporaryDirectory(prefix=f"noisemaker-rust-parity-{index:03d}-") as temporary:
            directory = Path(temporary)
            fixture = directory / "input.png"
            _write_fixture(fixture, args.size, args.size)
            if effect_id in OVERLAY_INTERFACE_UNSUPPORTED:
                record = {"id": effect_id, "status": "unsupported", "side": "js", "reason": OVERLAY_REASON}
                results.append(record)
                print(f"[{index}/{len(selected)}] {effect_id}: unsupported: {OVERLAY_REASON}", flush=True)
                continue
            program = _program(effect_id, catalog[effect_id])
            rust_png = directory / f"rust-{index}.png"
            js_png = directory / f"js-{index}.png"
            common = [
                "render", "-", "--width", str(args.size), "--height", str(args.size),
                "--time", str(args.time), "--seed", str(args.seed), "--input", str(fixture),
            ]
            try:
                _, rust_elapsed = _run(
                    [str(args.rust), *common, "--one-shot", "initial", "--output", str(rust_png)],
                    program, args.timeout, ROOT,
                )
                js_cwd = args.js.resolve().parents[1]
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
                record = {"id": effect_id, "status": "compared", **_metrics(rust_bytes, js_bytes)}
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
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
