"""Hand-ported definitions for effects whose CDN bundle builds `globals`/`passes`
with real JavaScript (loops, spreads) rather than literals, so the static
extractor can't read them. The GLSL programs are still transpiled from the CDN;
only the definition (params/passes) is reproduced here, faithful to the JS.

- mixer/mashup: `d()` generates layer0_tex..layer7_tex surface params (max 8),
  each with a `layerN_active` colorModeUniform; `c` wires them plus `source`
  into the single render pass's inputs.
- synth/remap: `f()` generates zone0_tex..zone7_tex surface params (max 8), each
  with a `zoneN_active` colorModeUniform; the std140 `data` block is packed from
  the params at render time by renderer._remap_uniform_data.
"""

from __future__ import annotations

_MASHUP_LAYERS = 8
_REMAP_ZONES = 8


def _mashup():
    params = {
        "source": {"type": "surface", "default": "none"},
        "layers": {"type": "int", "default": 4, "uniform": "layers"},
        "smoothness": {"type": "float", "default": 0.1, "uniform": "smoothness"},
    }
    inputs = {"source": "source"}
    for e in range(_MASHUP_LAYERS):
        params[f"layer{e}_tex"] = {"type": "surface", "default": "none", "colorModeUniform": f"layer{e}_active"}
        inputs[f"layer{e}_tex"] = f"layer{e}_tex"
    passes = [{"name": "render", "program": "mashup", "inputs": inputs, "outputs": {"fragColor": "outputTex"}}]
    return {
        "namespace": "mixer",
        "func": "mashup",
        "params": params,
        "passes": passes,
        "textures": {},
        "externalTexture": None,
    }


def _remap():
    params = {
        "zoneCount": {"type": "int", "default": 0, "uniform": "zoneCount"},
        "bgColor": {"type": "color", "default": [0, 0, 0], "uniform": "bgColor"},
        "bgAlpha": {"type": "float", "default": 1, "uniform": "bgAlpha"},
        "smoothEdge": {"type": "float", "default": 0.04, "uniform": "smoothEdge"},
    }
    inputs = {}
    for z in range(_REMAP_ZONES):
        params[f"zone{z}_tex"] = {"type": "surface", "default": "none", "colorModeUniform": f"zone{z}_active"}
        inputs[f"zone{z}_tex"] = f"zone{z}_tex"
    passes = [{"name": "render", "program": "remap", "inputs": inputs, "outputs": {"fragColor": "outputTex"}}]
    return {
        "namespace": "synth",
        "func": "remap",
        "params": params,
        "passes": passes,
        "textures": {},
        "externalTexture": None,
    }


COMPUTED_DEFS = {
    "mixer/mashup": _mashup(),
    "synth/remap": _remap(),
}
