"""Fetch shader source + effect metadata from the shaders.noisedeck.app CDN.

This is the CDN-backed replacement for reading a local `noisemaker` git
checkout: the CDN serves a per-effect ESM bundle at
`/<version>/effects/<id>.js` that inlines both the effect definition
(`globals` == params, `passes`, `textures`) and the raw GLSL source
(`shaders[program].glsl`).

Pure Python, no JavaScript execution. The per-effect bundles are minified
ESM modules -- we can't `import` them, so instead we extract the pieces we
need directly from the bundle *text*:

  * GLSL source appears as `<program>:{glsl:`...`,wgsl:`...`}` inside a
    `shaders` object literal. GLSL never contains a backtick, so a
    template-literal scan (respecting backslash escapes) safely finds the
    end of each program's source.
  * The effect definition (`namespace`, `func`, `globals`, `passes`,
    `textures`): CDN bundles use one of two shapes depending on how the
    effect source declares its config:

      1. `new X({namespace: "...", func: "...", globals: {...}, ...})`
         -- an object literal passed to the base effect class. Most
         effects use this.
      2. `class Y extends X { constructor() { ...; f(this, "namespace",
         "..."); f(this, "globals", {...}); ... } }` -- esbuild's
         constructor-based lowering of TS/JS public class fields, used by
         effects with custom instance methods (onInit, etc). The field
         name is a quoted string followed by a comma instead of a bareword
         followed by a colon.

    Either way, once we find where a field's *value* starts, extracting it
    is the same balanced-delimiter walk regardless of which convention
    introduced it -- so we search for both conventions and read whichever
    matches.

Pin an exact version with NM_SHADER_VERSION (default: the "1.0" rolling
minor channel -- see CDN_VERSION below). Fetches are cached to disk under
transpiler/.cdn-cache/<version>/ so repeat runs (and CI) are offline after
the first hit.

Known limitation: a handful of effects compute part of their definition
with real JS logic at module scope (e.g. building a `choices` lookup with
a `for` loop over another object, or sharing a `passes[].inputs` object
across effects) and reference the result by a bare identifier instead of
writing a literal. That can't be resolved without executing JS, so
`fetch_effect` raises a clear ValueError for those rather than silently
returning incomplete data -- as of the CDN's "1.0" channel this is a small,
known set (`synth/remap`, `mixer/mashup`, and a few legacy
`classicNoisedeck/*` palette effects); every other eligible effect parses
cleanly.
"""

from __future__ import annotations

import json
import os
import re
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any

import json5

from .computed_defs import COMPUTED_DEFS

CDN_BASE = os.environ.get("NM_SHADER_CDN", "https://shaders.noisedeck.app").rstrip("/")
# The "1.0" minor channel is the current release. It's a rolling tag, not an
# immutable snapshot -- pin an exact build with NM_SHADER_VERSION if byte-
# for-byte reproducibility matters. Immutable dot releases like "1.0.1" are
# stale; "1.0" always resolves to the latest 1.x build.
CDN_VERSION = os.environ.get("NM_SHADER_VERSION", "1.0")

_HERE = Path(__file__).resolve().parent
_CACHE_ROOT = _HERE / ".cdn-cache"
_USER_AGENT = "noisemaker-python-transpiler (+https://noisedeck.app)"

# Mesh and live audio/MIDI effects remain outside the CPU catalog. Stateful,
# particle, volume, cubemap, and render-loop effects are supported.
_RENDER_ALLOWLIST = frozenset(
    {
        "render/loopBegin",
        "render/loopEnd",
        "render/pointsBillboardRender",
        "render/pointsEmit",
        "render/pointsRender",
        "render/render3d",
        "render/renderCubemap3d",
        "render/renderCubemapSurface",
        "render/renderLit3d",
    }
)
_ID_EXCLUSIONS = frozenset(
    {
        "synth/roll",
        "synth/scope",
        "synth/spectrum",
    }
)
_ITERATED_IDS = frozenset(
    {
        "filter/convolutionFeedback",
        "filter/feedback",
        "filter/motionBlur",
        "filter/temporalAberration",
        "filter3d/flow3d",
        "points/attractor",
        "points/buddhabrot",
        "points/dla",
        "points/flock",
        "points/flow",
        "points/hydraulic",
        "points/lenia",
        "points/life",
        "points/physarum",
        "points/physical",
        "render/loopBegin",
        "render/pointsBillboardRender",
        "render/pointsEmit",
        "render/pointsRender",
        "synth/cellularAutomata",
        "synth/mnca",
        "synth/navierStokes",
        "synth/reactionDiffusion",
        "synth3d/cellularAutomata3d",
        "synth3d/reactionDiffusion3d",
    }
)


def _effect_domain(effect_id: str) -> str:
    namespace = effect_id.split("/", 1)[0]
    if effect_id == "render/loopBegin":
        return "loop-begin"
    if effect_id == "render/loopEnd":
        return "loop-end"
    if namespace == "synth3d":
        return "volume-generator"
    if namespace == "filter3d":
        return "volume-filter"
    if effect_id.startswith("render/render"):
        return "volume-renderer"
    return "image"


def _inject_cpu_iteration(effect_id: str, effect: dict) -> dict:
    if effect_id in _ITERATED_IDS:
        effect.setdefault("params", {})["iterationCount"] = {
            "type": "int",
            "default": 60,
            "min": 0,
            "max": 10000,
            "cpuOnly": True,
        }
    return effect


def _normalize_effect(effect_id: str, effect: dict) -> dict:
    namespace, func = effect_id.split("/", 1)
    effect["namespace"] = effect.get("namespace") or namespace
    effect["func"] = effect.get("func") or func
    effect["domain"] = _effect_domain(effect_id)
    if effect_id == "render/loopBegin":
        effect["loopRole"] = "begin"
    elif effect_id == "render/loopEnd":
        effect["loopRole"] = "end"
    return _inject_cpu_iteration(effect_id, effect)


class CDNError(RuntimeError):
    """The CDN request failed (network error or non-2xx response)."""


# ---------------------------------------------------------------------------
# HTTP + disk cache
# ---------------------------------------------------------------------------


def _cache_dir(version: str) -> Path:
    safe_version = re.sub(r"[^\w.-]", "_", version)
    return _CACHE_ROOT / safe_version


def _fetch_text(url: str) -> str:
    request = urllib.request.Request(url, headers={"User-Agent": _USER_AGENT})
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            charset = response.headers.get_content_charset() or "utf-8"
            return response.read().decode(charset)
    except urllib.error.HTTPError as exc:
        raise CDNError(f"CDN {exc.code} {exc.reason} for {url}") from exc
    except (urllib.error.URLError, OSError) as exc:
        raise CDNError(f"CDN request failed for {url}: {exc}") from exc


def _cached_text(version: str, rel_path: str, url: str) -> str:
    cache_file = _cache_dir(version) / rel_path
    if cache_file.exists():
        return cache_file.read_text(encoding="utf-8")
    text = _fetch_text(url)
    cache_file.parent.mkdir(parents=True, exist_ok=True)
    cache_file.write_text(text, encoding="utf-8")
    return text


def fetch_manifest(version: str = CDN_VERSION) -> dict:
    """Fetch (or read cached) `effects/manifest.json` for `version`.

    Returns the manifest verbatim: a dict keyed by effect id (e.g.
    "synth/solid") -> {description, glsl, tags, wgsl, starter, hasTex, ...}.
    """
    text = _cached_text(version, "effects/manifest.json", f"{CDN_BASE}/{version}/effects/manifest.json")
    return json.loads(text)


# ---------------------------------------------------------------------------
# Bundle text extraction (no JS execution -- see module docstring)
# ---------------------------------------------------------------------------

_GLSL_PROGRAM_RE = re.compile(r"(\w+)\s*:\s*\{\s*glsl\s*:\s*`")


def _skip_string(text: str, i: int) -> int:
    """`text[i]` is a quote character (', ", or `); return the index just
    past the matching close, honoring backslash escapes."""
    quote = text[i]
    i += 1
    n = len(text)
    while i < n:
        c = text[i]
        if c == "\\":
            i += 2
            continue
        if c == quote:
            return i + 1
        i += 1
    return i  # unterminated -- treat end-of-text as the boundary


def _extract_balanced(text: str, start: int) -> str:
    """Return the balanced `{...}` / `[...]` substring of `text` starting at
    `start` (which must index an opening brace/bracket). String and
    template literals are skipped whole so brackets inside them don't
    perturb the depth count. This is not a JS parser -- just enough to find
    the matching close for a JSON5-shaped sub-object/array."""
    if start >= len(text) or text[start] not in "{[":
        raise ValueError(f"expected '{{' or '[' at offset {start}")
    depth = 0
    i = start
    n = len(text)
    while i < n:
        c = text[i]
        if c in "\"'`":
            i = _skip_string(text, i)
            continue
        if c in "{[":
            depth += 1
        elif c in "}]":
            depth -= 1
            if depth == 0:
                return text[start : i + 1]
        i += 1
    raise ValueError("unbalanced brackets: reached end of text")


def _normalize_js_literals(text: str) -> str:
    """Rewrite minifier boolean shorthand (`!0` / `!1` -> `true` / `false`)
    so json5 can parse it. Walks string/template literals whole so text
    that happens to appear inside a quoted value is never rewritten."""
    out: list[str] = []
    i = 0
    n = len(text)
    while i < n:
        c = text[i]
        if c in "\"'`":
            start = i
            i = _skip_string(text, i)
            out.append(text[start:i])
            continue
        if c == "!" and text[i : i + 2] in ("!0", "!1"):
            prev_is_word = i > 0 and re.match(r"[\w$]", text[i - 1])
            next_is_word = i + 2 < n and re.match(r"[\w$]", text[i + 2])
            if not prev_is_word and not next_is_word:
                out.append("true" if text[i : i + 2] == "!0" else "false")
                i += 2
                continue
        out.append(c)
        i += 1
    return "".join(out)


def _sanitize_bare_identifiers(text: str) -> str:
    """Replace value-position bare identifiers with 0 so json5 can parse an
    object whose UI-metadata fields reference minified module-scope constants
    (e.g. `max: s`, `step: e`). Keys (identifier followed by `:`) and
    true/false/null are preserved; string/template contents are never touched.
    The rendering-relevant fields (type/default/uniform/define/texture/choices/
    enum) are string/number/object literals and are unaffected."""
    out: list[str] = []
    i, n = 0, len(text)
    while i < n:
        c = text[i]
        if c in "\"'`":
            start = i
            i = _skip_string(text, i)
            out.append(text[start:i])
            continue
        if c in "eE" and i > 0 and (text[i - 1].isdigit() or text[i - 1] == "."):
            exponent_digit = i + 1
            if exponent_digit < n and text[exponent_digit] in "+-":
                exponent_digit += 1
            if exponent_digit < n and text[exponent_digit].isdigit():
                out.append(c)
                i += 1
                continue
        if c.isalpha() or c in "_$":
            j = i
            while j < n and (text[j].isalnum() or text[j] in "_$"):
                j += 1
            ident = text[i:j]
            k = j
            while k < n and text[k] in " \t\n\r":
                k += 1
            is_key = k < n and text[k] == ":"
            out.append(ident if (is_key or ident in ("true", "false", "null")) else "0")
            i = j
            continue
        out.append(c)
        i += 1
    return "".join(out)


def _find_value_start(text: str, key: str) -> int | None:
    """Locate `key`'s value in `text`. Handles both bundle conventions seen
    on the CDN: the common object-literal `key: value` and the class-field
    -call convention esbuild emits for effects with custom instance methods
    (`"key", value`, e.g. `f(this,"key",value)` from lowered TS class
    fields). Returns the index of the first character of the value, or None
    if `key` isn't present."""
    pattern = rf"(?:\b{re.escape(key)}\b\s*:|[\"']{re.escape(key)}[\"']\s*,)\s*"
    m = re.search(pattern, text)
    return m.end() if m else None


def _read_literal(text: str, start: int | None, sanitize: bool = False) -> Any:
    """Read the single JS value (string, object, or array literal) at
    `start` and return it as a Python value via json5. Returns None if
    `start` is None, or doesn't point at a literal this function
    recognizes (e.g. a bare identifier referencing a module-scope
    variable -- see the module docstring's "Known limitation").

    ``sanitize`` replaces value-position bare identifiers with 0 (for the
    ``globals`` object, whose UI-metadata fields may reference minified
    constants that json5 can't parse; rendering-relevant fields are literals)."""
    if start is None or start >= len(text):
        return None
    c = text[start]
    if c in "{[":
        raw = _normalize_js_literals(_extract_balanced(text, start))
        if sanitize:
            raw = _sanitize_bare_identifiers(raw)
        return json5.loads(raw)
    if c in "\"'":
        end = _skip_string(text, start)
        return json5.loads(text[start:end])
    return None


def _definition_region(bundle: str) -> str:
    """Slice `bundle` down to the part that can hold the effect definition:
    everything before the `shaders` object, where GLSL/WGSL source and help
    markdown live. The definition always precedes `shaders` textually in
    CDN bundles, and bounding the search here keeps field lookups from
    matching prose in the help text or code inside GLSL/WGSL strings."""
    m = _GLSL_PROGRAM_RE.search(bundle)
    return bundle[: m.start()] if m else bundle


def _extract_programs(bundle: str) -> dict[str, str]:
    """Extract every program's GLSL template literal from the bundle's
    `shaders` object: `<program>:{glsl:`...`,...}`."""
    programs: dict[str, str] = {}
    for m in _GLSL_PROGRAM_RE.finditer(bundle):
        program = m.group(1)
        backtick_start = m.end() - 1
        end = _skip_string(bundle, backtick_start)
        programs[program] = bundle[backtick_start + 1 : end - 1]
    return programs


def _parse_field(region: str, effect_id: str, key: str, default: Any) -> Any:
    start = _find_value_start(region, key)
    if start is None:
        return default
    try:
        value = _read_literal(region, start, sanitize=(key == "globals"))
    except Exception as exc:  # json5 decode error, unbalanced brackets, etc.
        raise ValueError(
            f"CDN effect {effect_id!r}: could not parse {key!r} as a JSON5 "
            f"literal. This usually means the bundle references a "
            f"module-scope variable computed by real JS logic (e.g. a "
            f"palette `choices` map built with a loop) instead of writing "
            f"a literal -- see the cdn module docstring's Known Limitation."
        ) from exc
    return default if value is None else value


def fetch_effect(effect_id: str, version: str = CDN_VERSION) -> dict:
    """Fetch (or read cached) the per-effect bundle for `effect_id` and
    extract its metadata + GLSL without executing any JavaScript.

    Returns:
        {"id", "namespace", "func", "params": {name: spec}, "passes":
         [{name, program, inputs, outputs}, ...], "textures": {...},
         "programs": {program: glsl_source}}

    Raises:
        CDNError: the bundle couldn't be fetched.
        ValueError: `globals`/`passes`/`textures` couldn't be parsed --
            typically a bundle that computes that field with real JS logic
            instead of a literal (see module docstring).
    """
    # Cache the EXTRACTED data as JSON (never write the raw CDN .js to disk, so
    # no JavaScript files ever live in this Python project).
    cache_file = _cache_dir(version) / "effects" / f"{effect_id}.json"
    if cache_file.exists():
        cached = _normalize_effect(effect_id, json.loads(cache_file.read_text(encoding="utf-8")))
        if cached["domain"] in {"image", "loop-begin", "loop-end"} or cached.get("outputTex3d"):
            return cached

    bundle = _fetch_text(f"{CDN_BASE}/{version}/effects/{effect_id}.js")
    region = _definition_region(bundle)
    override = COMPUTED_DEFS.get(effect_id)
    if override is not None:
        # Definition is JS-computed (loops/spreads) and unreadable statically;
        # use the hand-ported params/passes but keep the GLSL from the CDN.
        result = {
            "id": effect_id,
            "namespace": override["namespace"],
            "func": override["func"],
            "params": override["params"],
            "passes": override["passes"],
            "textures": override.get("textures", {}),
            "externalTexture": override.get("externalTexture"),
            "outputTex": override.get("outputTex"),
            "outputTex3d": override.get("outputTex3d"),
            "outputGeo": override.get("outputGeo"),
            "programs": _extract_programs(bundle),
        }
    else:
        result = {
            "id": effect_id,
            "namespace": _parse_field(region, effect_id, "namespace", None),
            "func": _parse_field(region, effect_id, "func", None),
            "params": _parse_field(region, effect_id, "globals", {}),
            "passes": _parse_field(region, effect_id, "passes", []),
            "textures": _parse_field(region, effect_id, "textures", {}),
            "externalTexture": _parse_field(region, effect_id, "externalTexture", None),
            "outputTex": _parse_field(region, effect_id, "outputTex", None),
            "outputTex3d": _parse_field(region, effect_id, "outputTex3d", None),
            "outputGeo": _parse_field(region, effect_id, "outputGeo", None),
            "programs": _extract_programs(bundle),
        }
    _normalize_effect(effect_id, result)
    cache_file.parent.mkdir(parents=True, exist_ok=True)
    cache_file.write_text(json.dumps(result), encoding="utf-8")
    return result


def eligible_ids(version: str = CDN_VERSION) -> list[str]:
    """Manifest effect ids targeted by the complete CPU catalog."""
    manifest = fetch_manifest(version)
    result = []
    for effect_id in manifest:
        namespace = effect_id.split("/", 1)[0]
        if namespace == "render" and effect_id not in _RENDER_ALLOWLIST:
            continue
        if effect_id in _ID_EXCLUSIONS:
            continue
        result.append(effect_id)
    return result


def provenance(version: str = CDN_VERSION) -> dict:
    """Small provenance record (source/version/base) for build logs."""
    return {"source": "shaders.noisedeck.app CDN", "version": version, "base": CDN_BASE}
