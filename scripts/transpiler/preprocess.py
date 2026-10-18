"""GLSL preprocessing + light normalization (pure Python).

Reproduces the parts of the reference JS pipeline (glsl-normalize.js + prepr)
that matter for the Python codegen:
  - strip `#version`
  - object-like `#define` expansion
  - `#ifdef`/`#ifndef`/`#if`/`#elif`/`#else`/`#endif`: static conditions are
    evaluated; conditions on a *runtime define* are lowered into real GLSL
    `if/else` fed by a uniform of that name
  - capture `out vec4 X;` -> global `vec4 X;` + record X in outputs
  - capture `in vecN Y;` varyings (dropped; codegen maps them to ctx.uv)

Unlike the JS normalizer we do NOT rewrite `uint`/`uvec`->`int`/`vec`: the Python
codegen handles unsigned types natively via the bit-exact uintmath runtime.
"""

from __future__ import annotations

import re

_IDENT = re.compile(r"\b[A-Za-z_]\w*\b")
_DEFINE = re.compile(r"define\s+(\w+)(?:\(|\s|$)")


def _strip_comments(source: str) -> str:
    """Remove block and line comments before preprocessing. The tokenizer strips
    comments too, but the preprocessor runs first on raw text — a `//` comment
    trailing a `#define` value (e.g. `#define MAX_PAIRS 32  // ...`) would
    otherwise be captured into the macro and comment out every later expansion.
    GLSL has no string literals, so this is unambiguous."""
    source = re.sub(r"/\*.*?\*/", " ", source, flags=re.DOTALL)
    source = re.sub(r"//[^\n]*", "", source)
    return source


def _adapt_canonical_source(source: str, effect_id: str | None) -> str:
    def replace_once(before, after):
        nonlocal source
        if before not in source:
            raise ValueError(f"{effect_id}: canonical source changed: {before.splitlines()[0]}")
        source = source.replace(before, after, 1)

    if effect_id == "synth3d/noise3d":
        replace_once("float hash4(vec4 p) {\n    vec4 ps = p + float(seed) * 0.1;",
                     "float hash4(vec4 p, vec4 k) {\n"
                     "    float cpuSeedOffset = float(seed * 0.1);\n"
                     "    vec4 cpuLatticeOffset = k + cpuSeedOffset;\n"
                     "    vec4 ps = p + cpuLatticeOffset;")
        replace_once("vec4 grad4(vec4 p) {\n    float h1 = hash4(p);\n    float h2 = hash4(p + 127.1);\n    float h3 = hash4(p + 269.5);\n    float h4 = hash4(p + 419.2);",
                     "vec4 grad4(vec4 p, vec4 k) {\n    float h1 = hash4(p, k);\n    float h2 = hash4(p, k + 127.1);\n    float h3 = hash4(p, k + 269.5);\n    float h4 = hash4(p, k + 419.2);")
        source, count = re.subn(r"grad4\(vec4\(i\.xyz, (iw[01])\) \+ (vec4\([01],[01],[01],0\))\)", r"grad4(vec4(i.xyz, \1), \2)", source)
        if count != 16:
            raise ValueError(f"{effect_id}: expected 16 lattice corners, found {count}")
    elif effect_id == "synth3d/cell3d":
        replace_once("vec3 hash3(vec3 p) {\n    p = p + float(seed) * 0.1;",
                     "vec3 hash3(vec3 base, vec3 neighbor) {\n"
                     "    float cpuSeedOffset = float(seed * 0.1);\n"
                     "    vec3 cpuCellPos = base + neighbor;\n"
                     "    vec3 p = vec3(base.x + float(neighbor.x + cpuSeedOffset), cpuCellPos.y + cpuSeedOffset, cpuCellPos.z + cpuSeedOffset);")
        replace_once("vec3 randomOffset = hash3(cellPos);", "vec3 randomOffset = hash3(i, neighbor);")
    elif effect_id == "synth/perlin":
        replace_once("float hash3(vec3 p) {\n    // Add seed to input to vary the noise pattern\n    p = p + float(seed) * 0.1;",
                     "float hash3(vec3 p, vec3 k) {\n"
                     "    float cpuSeedOffset = float(seed * 0.1);\n"
                     "    vec2 cpuLatticeOffset = k.xy + cpuSeedOffset;\n"
                     "    p = vec3(p.x + cpuLatticeOffset.x, p.y + cpuLatticeOffset.y, float(p.z + cpuSeedOffset) + k.z);")
        replace_once("vec3 grad3(vec3 p) {\n    // Generate 3 independent random values\n    float h1 = hash3(p);\n    float h2 = hash3(p + 127.1);\n    float h3 = hash3(p + 269.5);",
                     "vec3 grad3(vec3 p, vec3 k) {\n    float h1 = hash3(p, k);\n    float h2 = hash3(p, k + 127.1);\n    float h3 = hash3(p, k + 269.5);")
        source, count = re.subn(r"grad3\(vec3\(i\.xy, (iz[01])\) \+ (vec3\([01],[01],0\))\)", r"grad3(vec3(i.xy, \1), \2)", source)
        if count != 8:
            raise ValueError(f"{effect_id}: expected 8 lattice corners, found {count}")
    elif effect_id in {"synth3d/reactionDiffusion3d", "synth3d/cellularAutomata3d"}:
        replace_once("float hash3(vec3 p) {\n    p = p + float(seed) * 0.1;\n    p = fract(p * vec3(0.1031, 0.1030, 0.0973));\n    p += dot(p, p.yxz + 33.33);\n    return fract((p.x + p.y) * p.z);\n}",
                     "float hash3(vec3 p) {\n    p = p + float(seed * 0.1);\n    p = fract(p * vec3(0.1031, 0.1030, 0.0973));\n    float cpuHashDot = dot(p, p.yxz + 33.33);\n    return fract(float(float(float(p.x + p.y) + float(cpuHashDot + cpuHashDot)) * float(p.z + cpuHashDot)));\n}")

    source = re.sub(r"\bmod\(\s*([A-Za-z_]\w*(?:\.[xyzw]{1,4})?)\s*\+\s*1\.0\s*,\s*2\.0\s*\)\s*-\s*1\.0\b", r"(\1 - 2.0 * floor((\1 + 1.0) * 0.5))", source)
    source = source.replace("return fract((p3.x + p3.y) * p3.z);", "return fract(float(float(p3.x + p3.y) * p3.z));")
    source = source.replace("return fract((p3.xx + p3.yz) * p3.zy);", "return fract(vec2(float(float(p3.x + p3.y) * p3.z), float(float(p3.x + p3.z) * p3.y)));")
    if effect_id == "filter/extrude":
        replace_once("float s = 1.0 + h * (depth / 100.0) * 0.4;", "float s = float(1.0 + h * (depth / 100.0) * 0.4);")
        replace_once("bool topHit = all(lessThanEqual(abs(P - faceCenter), faceHalf));", "bool topHit = all(lessThanEqual(abs((P - imgCenter) - (cellC - imgCenter) * s), faceHalf));")
    if effect_id == "filter/scatter" and "vec2 rnd = hash22(" in source:
        replace_once("hash22(hashCoord + float(seed) * 101.7)", "hash22(hashCoord + float(seed * 101.7))")
    if effect_id == "synth/navierStokes" and "float hash11(float x) {" in source:
        replace_once("return fract(sin(x * 12.9898) * 43758.5453);", "return fract(float(float(sin(float(float(x) * 12.9898))) * 43758.5453));")
    return source


def normalize(source: str, runtime_defines: dict | None = None, *, effect_id: str | None = None) -> dict:
    runtime_defines = runtime_defines or {}
    source = _adapt_canonical_source(source, effect_id)
    body = _preprocess(_strip_comments(source), runtime_defines)

    out_lines = []
    outputs = []
    varyings = []
    varying_types = {}
    for line in body.split("\n"):
        m = re.match(r"\s*(?:layout\s*\([^)]*\)\s*)?out\s+(\w+)\s+(\w+)\s*;\s*$", line)
        if m:
            outputs.append(m.group(2))
            out_lines.append(f"{m.group(1)} {m.group(2)};")
            continue
        m = re.match(r"\s*(?:flat\s+)?in\s+(\w+)\s+(\w+)\s*;\s*$", line)
        if m:
            varyings.append(m.group(2))
            varying_types[m.group(2)] = m.group(1)
            continue  # codegen maps varyings to ctx.uv
        out_lines.append(line)

    # Declare runtime-define uniforms (they were lowered to runtime branches).
    decls = "".join(f"uniform {'float' if t == 'float' else 'int'} {name};\n" for name, t in runtime_defines.items())
    return {
        "source": decls + "\n".join(out_lines),
        "outputs": outputs or ["fragColor"],
        "varyings": varyings,
        "varyingTypes": varying_types,
    }


def _preprocess(source: str, runtime_defines: dict) -> list:
    out = []
    defines: dict[str, str] = {}
    stack = []  # frames: {"kind": "static"|"runtime"|"include_all", "active", "taken", "outer"}
    depth = [0]  # brace nesting of emitted content; list for closure mutation

    def emitting():
        return all(f["active"] for f in stack)

    def emit(line):
        out.append(line)
        depth[0] += line.count("{") - line.count("}")

    for raw in source.split("\n"):
        s = raw.strip()
        if s.startswith("#"):
            d = s[1:].strip()
            head = d.split()[0] if d else ""
            if head == "version" or head in ("extension", "pragma", "line"):
                continue
            if head == "define":
                if emitting() and not re.match(r"define\s+\w+\(", d):  # object-like only
                    m = re.match(r"define\s+(\w+)(?:\s+(.*))?$", d)
                    if m:
                        defines[m.group(1)] = (m.group(2) or "").strip()
                continue
            if head == "undef":
                if emitting():
                    defines.pop(d.split()[1], None)
                continue
            if head in ("ifdef", "ifndef", "if"):
                outer = emitting()
                if outer and _cond_runtime(d, head, runtime_defines):
                    if depth[0] == 0:
                        # A runtime #if at global scope gates whole declarations
                        # (e.g. conditionally-compiled functions), which can't be
                        # a runtime `if`. Include ALL branches — the transpiled
                        # functions are uniquely named and dispatched at runtime
                        # by a separate statement-scope #if.
                        stack.append({"kind": "include_all", "active": True, "taken": True, "outer": outer})
                    else:
                        emit(f"if ({_glsl_cond(d, head, defines)}) {{ __nm_preprocessor_branch__;")
                        stack.append({"kind": "runtime", "active": True, "taken": True, "outer": outer})
                else:
                    val = _eval(d, head, defines, runtime_defines) if outer else False
                    stack.append({"kind": "static", "active": outer and val, "taken": val, "outer": outer})
                continue
            if head == "elif":
                fr = stack[-1]
                if fr["kind"] == "include_all":
                    pass  # every branch is emitted
                elif fr["kind"] == "runtime":
                    emit(f"}} else if ({_glsl_cond(d, 'if', defines)}) {{ __nm_preprocessor_branch__;")
                    fr["active"] = True
                else:
                    if fr["taken"]:
                        fr["active"] = False
                    else:
                        val = _eval(d, "if", defines, runtime_defines) if fr["outer"] else False
                        fr["active"] = fr["outer"] and val
                        fr["taken"] = fr["taken"] or val
                continue
            if head == "else":
                fr = stack[-1]
                if fr["kind"] == "include_all":
                    pass
                elif fr["kind"] == "runtime":
                    emit("} else { __nm_preprocessor_branch__;")
                    fr["active"] = True
                else:
                    fr["active"] = fr["outer"] and (not fr["taken"])
                    fr["taken"] = True
                continue
            if head == "endif":
                fr = stack.pop()
                if fr["kind"] == "runtime":
                    emit("}")
                continue
            continue  # unknown directive
        if emitting():
            emit(_expand(raw, defines))
    return "\n".join(out)


def _expand(line: str, defines: dict) -> str:
    if not defines:
        return line
    for _ in range(16):
        changed = False

        def repl(m):
            nonlocal changed
            name = m.group(0)
            if name in defines:
                changed = True
                return defines[name]
            return name

        new = _IDENT.sub(repl, line)
        line = new
        if not changed:
            break
    return line


def _cond_runtime(directive: str, head: str, runtime_defines: dict) -> bool:
    # #ifdef/#ifndef are about DEFINEDNESS: a runtime define is always "defined"
    # (bound as a uniform), so those resolve statically. Only `#if <expr on the
    # value>` needs runtime lowering.
    if not runtime_defines or head in ("ifdef", "ifndef"):
        return False
    return any(rd in set(_IDENT.findall(directive)) for rd in runtime_defines)


def _strip_kw(directive: str) -> str:
    return re.sub(r"^(elif|ifdef|ifndef|if)\b\s*", "", directive).strip()


def _glsl_cond(directive: str, head: str, defines: dict) -> str:
    if head == "ifdef":
        return "true"
    if head == "ifndef":
        return "false"
    return _expand(_strip_kw(directive), defines)


def _eval(directive: str, head: str, defines: dict, runtime_defines: dict | None = None) -> bool:
    runtime_defines = runtime_defines or {}
    if head == "ifdef":
        n = directive.split()[1]
        return n in defines or n in runtime_defines
    if head == "ifndef":
        n = directive.split()[1]
        return n not in defines and n not in runtime_defines
    expr = _strip_kw(directive)
    expr = re.sub(r"defined\s*\(\s*(\w+)\s*\)", lambda m: "1" if m.group(1) in defines else "0", expr)
    expr = re.sub(r"defined\s+(\w+)", lambda m: "1" if m.group(1) in defines else "0", expr)
    expr = _expand(expr, defines)
    # undefined identifiers evaluate to 0 in C/GLSL #if
    expr = _IDENT.sub(lambda m: m.group(0) if m.group(0) in ("true", "false") else "0", expr)
    py = (
        expr.replace("&&", " and ")
        .replace("||", " or ")
        .replace("!", " not ")
        .replace(" not =", " !=")
        .replace("true", "1")
        .replace("false", "0")
    )
    py = re.sub(r"(?<![<>=!])=(?!=)", "==", py)  # stray single = (rare) -> ==
    try:
        return bool(eval(py, {"__builtins__": {}}, {}))
    except Exception:
        return False
