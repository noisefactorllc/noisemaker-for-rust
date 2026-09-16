"""Hand-ported definitions for effects whose CDN bundle builds `globals`/`passes`
with real JavaScript (loops, spreads) rather than literals, so the static
extractor can't read them. The GLSL programs are still transpiled from the CDN;
only the definition (params/passes) is reproduced here, faithful to the JS.

- mixer/mashup: `d()` generates layer0_tex..layer7_tex surface params (max 8),
  each with a `layerN_active` colorModeUniform; `c` wires them plus `source`
  into the single render pass's inputs.
- synth/remap: `f()` generates zone0_tex..zone7_tex surface params (max 8), each
  with a `zoneN_active` colorModeUniform; the std140 `data` block is packed from
  the params at render time by renderer.remap_uniform_data (see that function's
  own comment: this renderer has never packed zone polygon vertex data into it,
  a pre-existing gap predating this file, unrelated to any specific round).
- render/pointsBillboardRender: the reference definition's `.flatMap()` clones
  its deposit/depthKeys/depthMerge passes per compile-time viewMode/blendMode/
  blurLayer variant (reference 0ed489ec, the perspective+depth-sort+defocus
  round). This port's own draw_ops.rs dispatches those as RUNTIME uniforms
  inside one `billboard()` function (matching pointsRender's own pattern), not
  per-variant compiled kernels, so the params/passes below keep this port's
  pre-round 5-pass shape (diffuse/copy/deposit/deposit_alpha/blend) rather than
  the reference's clone structure -- reconstructed from this repo's own
  pre-round history (commit 6c118ed) plus this round's additions. aperture/
  focalDistance are declared but inert: this round's depth-sorted alpha-blend
  draw order and aperture defocus blur have no equivalent in this port's
  bounding-box scanline rasterizer (no oversized-quad multi-sample kernel, no
  merge-sort infrastructure) -- the same architectural-mismatch category as
  the noisemaker-for-touchdesigner/-ruby/-perl sibling ports for the identical
  reason. Perspective viewMode and distance-based size/brightness fade (which
  ARE pure per-agent math, no new rendering infrastructure needed) are ported
  in draw_ops.rs's compute_clip_center, shared with pointsRender.
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


def _points_billboard_render():
    params = {
        "shapeMode": {
            "type": "int", "default": 1, "randMin": 1, "uniform": "shapeMode",
            "choices": {"texture": 0, "circle": 1, "ring": 2, "square": 3, "diamond": 4, "triangle": 5, "star": 6, "soft": 7},
            "ui": {"label": "shape", "control": "dropdown", "category": "source"},
        },
        "tex": {
            "type": "surface", "default": "none",
            "ui": {"label": "sprite", "category": "source", "enabledBy": {"param": "shapeMode", "eq": 0}},
        },
        "blendMode": {
            "type": "int", "default": 0, "uniform": "blendMode",
            "choices": {"additive": 0, "alpha": 1},
            "ui": {"label": "blend", "control": "dropdown", "category": "visual"},
        },
        "depositOpacity": {
            "type": "float", "default": 20, "min": 1, "max": 100, "uniform": "depositOpacity",
            "ui": {"label": "opacity", "control": "slider", "category": "visual"},
        },
        "pointSize": {
            "type": "float", "default": 8, "min": 1, "max": 64, "uniform": "pointSize",
            "ui": {"label": "point size", "control": "slider", "category": "visual"},
        },
        "sizeVariation": {
            "type": "float", "default": 0, "min": 0, "max": 100, "uniform": "sizeVariation",
            "ui": {"label": "size variation", "control": "slider", "category": "visual"},
        },
        "rotationVar": {
            "type": "float", "default": 0, "min": 0, "max": 100, "uniform": "rotationVar",
            "ui": {"label": "rot variation", "control": "slider", "category": "visual"},
        },
        "seed": {
            "type": "int", "default": 42, "min": 0, "max": 1000, "uniform": "seed",
            "ui": {"label": "seed", "control": "slider", "category": "visual"},
        },
        "density": {
            "type": "float", "default": 50, "min": 0, "max": 100, "uniform": "density",
            "ui": {"label": "density", "control": "slider", "category": "visual"},
        },
        "intensity": {
            "type": "float", "default": 75, "min": 0, "max": 100, "uniform": "intensity",
            "ui": {"label": "trail intensity", "control": "slider", "category": "visual"},
        },
        "inputIntensity": {
            "type": "float", "default": 10.15, "min": 0, "max": 100, "randMin": 50, "uniform": "inputIntensity",
            "ui": {"label": "input mix", "control": "slider", "category": "visual"},
        },
        "viewMode": {
            "type": "int", "default": 0, "uniform": "viewMode",
            "choices": {"flat": 0, "ortho": 1, "perspective": 2},
            "ui": {"label": "view", "control": "dropdown", "category": "view"},
        },
        "rotateX": {
            "type": "float", "default": 0.3, "min": 0, "max": 6.283185, "step": 0.01, "uniform": "rotateX",
            "ui": {"label": "rotate x", "control": "slider", "category": "view", "enabledBy": "viewMode"},
        },
        "rotateY": {
            "type": "float", "default": 0, "min": 0, "max": 6.283185, "step": 0.01, "uniform": "rotateY",
            "ui": {"label": "rotate y", "control": "slider", "category": "view", "enabledBy": "viewMode"},
        },
        "rotateZ": {
            "type": "float", "default": 0, "min": 0, "max": 6.283185, "step": 0.01, "uniform": "rotateZ",
            "ui": {"label": "rotate z", "control": "slider", "category": "view", "enabledBy": "viewMode"},
        },
        "viewScale": {
            "type": "float", "default": 0.8, "min": 0.1, "max": 10, "step": 0.01, "uniform": "viewScale",
            "ui": {"label": "zoom", "control": "slider", "category": "view", "enabledBy": "viewMode"},
        },
        "posX": {
            "type": "float", "default": 0, "min": -50, "max": 50, "step": 0.1, "uniform": "posX",
            "ui": {"label": "pos x", "control": "slider", "category": "view", "enabledBy": "viewMode"},
        },
        "posY": {
            "type": "float", "default": 0, "min": -50, "max": 50, "step": 0.1, "uniform": "posY",
            "ui": {"label": "pos y", "control": "slider", "category": "view", "enabledBy": "viewMode"},
        },
        "posZ": {
            "type": "float", "default": 0, "min": -200, "max": 200, "step": 0.1, "uniform": "posZ",
            "ui": {"label": "pos z", "control": "slider", "category": "view", "enabledBy": {"param": "viewMode", "eq": 2}},
        },
        "fieldOfView": {
            "type": "float", "default": 60, "min": 10, "max": 150, "step": 1, "uniform": "fieldOfView",
            "ui": {"label": "field of view", "control": "slider", "category": "view", "enabledBy": {"param": "viewMode", "eq": 2}},
        },
        "sizeDistance": {
            "type": "float", "default": 0, "min": 0, "max": 500, "uniform": "sizeDistance",
            "ui": {"label": "size distance", "control": "slider", "category": "view", "enabledBy": "viewMode"},
        },
        "brightnessDistance": {
            "type": "float", "default": 0, "min": 0, "max": 500, "uniform": "brightnessDistance",
            "ui": {"label": "brightness distance", "control": "slider", "category": "view", "enabledBy": "viewMode"},
        },
        "aperture": {
            "type": "float", "default": 0, "min": 0, "max": 20, "uniform": "aperture",
            "ui": {"label": "aperture", "control": "slider", "category": "view", "enabledBy": "viewMode"},
        },
        "focalDistance": {
            "type": "float", "default": 80, "min": 1, "max": 500, "uniform": "focalDistance",
            "ui": {"label": "focal distance", "control": "slider", "category": "view", "enabledBy": "viewMode"},
        },
    }
    deposit_uniforms = {
        "density": "density", "depositOpacity": "depositOpacity", "pointSize": "pointSize",
        "posX": "posX", "posY": "posY", "posZ": "posZ",
        "rotateX": "rotateX", "rotateY": "rotateY", "rotateZ": "rotateZ",
        "rotationVar": "rotationVar", "seed": "seed", "shapeMode": "shapeMode",
        "sizeVariation": "sizeVariation", "viewMode": "viewMode", "viewScale": "viewScale",
        "fieldOfView": "fieldOfView", "sizeDistance": "sizeDistance",
        "brightnessDistance": "brightnessDistance",
    }
    deposit_inputs = {"rgbaTex": "global_rgba", "spriteTex": "tex", "xyzTex": "global_xyz"}
    passes = [
        {
            "name": "diffuse", "program": "diffuse", "key": "render/pointsBillboardRender:diffuse",
            "inputs": {"trailTex": "global_billboard_trail"},
            "outputs": {"fragColor": "global_billboard_trail"},
            "uniforms": {"intensity": "intensity"},
        },
        {
            "name": "copy", "program": "copy", "key": "render/pointsBillboardRender:copy",
            "inputs": {"sourceTex": "global_billboard_trail"},
            "outputs": {"fragColor": "global_billboard_trail"},
        },
        {
            "name": "deposit", "program": "deposit", "key": None,
            "drawMode": "billboards", "count": "input", "blend": True,
            "conditions": {"runIf": [{"uniform": "blendMode", "equals": 0}]},
            "inputs": deposit_inputs,
            "outputs": {"fragColor": "global_billboard_trail"},
            "uniforms": deposit_uniforms,
        },
        {
            "name": "deposit_alpha", "program": "deposit", "key": None,
            "drawMode": "billboards", "count": "input", "blend": ["ONE", "ONE_MINUS_SRC_ALPHA"],
            "conditions": {"runIf": [{"uniform": "blendMode", "equals": 1}]},
            "inputs": deposit_inputs,
            "outputs": {"fragColor": "global_billboard_trail"},
            "uniforms": deposit_uniforms,
        },
        {
            "name": "blend", "program": "blend", "key": "render/pointsBillboardRender:blend",
            "inputs": {"inputTex": "inputTex", "trailTex": "global_billboard_trail"},
            "outputs": {"fragColor": "outputTex"},
            "uniforms": {"blendMode": "blendMode", "inputIntensity": "inputIntensity"},
        },
    ]
    return {
        "namespace": "render",
        "func": "pointsBillboardRender",
        "params": params,
        "passes": passes,
        "textures": {"global_billboard_trail": {"format": "rgba16f", "width": "100%", "height": "100%"}},
        "externalTexture": None,
    }


COMPUTED_DEFS = {
    "mixer/mashup": _mashup(),
    "synth/remap": _remap(),
    "render/pointsBillboardRender": _points_billboard_render(),
}
