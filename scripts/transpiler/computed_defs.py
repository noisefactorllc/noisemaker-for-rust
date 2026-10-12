"""Hand-ported definitions for effects whose CDN bundle builds `globals`/`passes`
with real JavaScript (loops, spreads) rather than literals, so the static
extractor can't read them. The GLSL programs are still transpiled from the CDN;
only the definition (params/passes) is reproduced here, faithful to the JS.

- mixer/mashup: `d()` generates layer0_tex..layer7_tex surface params (max 8),
  each with a `layerN_active` colorModeUniform; `c` wires them plus `source`
  into the single render pass's inputs.
- synth/remap: `f()` generates zone0_tex..zone7_tex surface params (max 8), each
  with a `zoneN_active` colorModeUniform; the std140 `data` block is packed from
  the polygon geometry, bounds, alpha and host-wired active flags are packed
  from these params at render time by renderer.remap_uniform_data.
- render/pointsBillboardRender: the reference definition builds depth-sort and
  deposit pass clones with JavaScript loops and `.flatMap()`. This definition
  reproduces those passes; the draw adapter consumes their runtime uniforms.
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
        params[f"zone{z}_count"] = {"type": "int", "default": 0, "min": 0, "max": 64, "uniform": f"zone{z}_count"}
        params[f"zone{z}_alpha"] = {"type": "float", "default": 1, "min": 0, "max": 1, "uniform": f"zone{z}_alpha"}
        params[f"zone{z}_bounds"] = {"type": "vec4", "default": [0, 0, 1, 1], "uniform": f"zone{z}_bounds"}
        for pair in range(32):
            name = f"zone{z}_v{pair}"
            params[name] = {"type": "vec4", "default": [0, 0, 0, 0], "uniform": name}
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
    def condition(name, value):
        return {"uniform": name, "equals": value}

    passes = []
    for view_mode in (1, 2):
        passes.append({
            "name": f"depthKeys_{view_mode}", "program": "depthKeys",
            "key": "render/pointsBillboardRender:depthKeys",
            "inputs": {"xyzTex": "global_xyz"}, "outputs": {"fragColor": "depthOrderA"},
            "uniforms": {**{name: name for name in ("viewMode", "rotateX", "rotateY", "posZ")},
                         "VIEW_MODE": "viewMode"},
            "type": "compute", "defines": {"VIEW_MODE": view_mode},
            "conditions": {
                "runIf": [condition("blendMode", 1), condition("viewMode", view_mode)],
                "skipIf": [condition("viewMode", 0)],
            },
        })
    for stage in range(22):
        source = "depthOrderA" if stage % 2 == 0 else "depthOrderB"
        target = "depthOrderB" if stage % 2 == 0 else "depthOrderA"
        passes.append({
            "name": f"depthMerge{stage}", "program": "depthMerge",
            "key": "render/pointsBillboardRender:depthMerge",
            "inputs": {"orderTex": source}, "outputs": {"fragColor": target},
            "uniforms": {"runLength": 1 << stage}, "type": "compute",
            "conditions": {
                "runIf": [condition("blendMode", 1)],
                "skipIf": [condition("viewMode", 0)],
            },
        })
    passes.extend([
        {
            "name": "spriteMeanTiles", "program": "spriteMeanTiles",
            "key": "render/pointsBillboardRender:spriteMeanTiles",
            "inputs": {"spriteTex": "tex"}, "outputs": {"fragColor": "spriteMeanTiles"},
            "uniforms": {name: name for name in ("shapeMode", "aperture", "viewMode")},
            "type": "compute",
            "conditions": {"runIf": [condition("shapeMode", 0)],
                           "skipIf": [condition("aperture", 0), condition("viewMode", 0)]},
        },
        {
            "name": "spriteMean", "program": "spriteMean",
            "key": "render/pointsBillboardRender:spriteMean",
            "inputs": {"tilesTex": "spriteMeanTiles"}, "outputs": {"fragColor": "spriteMean"},
            "uniforms": {name: name for name in ("shapeMode", "aperture", "viewMode")},
            "type": "compute",
            "conditions": {"skipIf": [condition("aperture", 0), condition("viewMode", 0)]},
        },
        {
            "name": "clearDefocus", "program": "clearDefocus",
            "key": "render/pointsBillboardRender:clearDefocus",
            "inputs": {}, "outputs": {"fragColor": "defocus"},
            "uniforms": {"clearValue": 0},
            "conditions": {"runIf": [condition("blendMode", 0)],
                           "skipIf": [condition("aperture", 0), condition("viewMode", 0)]},
        },
    ])
    deposit_inputs = {
        "xyzTex": "global_xyz", "rgbaTex": "global_rgba", "orderTex": "depthOrderA",
        "spriteTex": "tex", "spriteMeanTex": "spriteMean",
    }
    deposit_uniforms = {name: name for name in (
        "shapeMode", "blendMode", "depositOpacity", "density", "pointSize",
        "sizeVariation", "rotationVar", "seed", "viewMode", "rotateX", "rotateY",
        "rotateZ", "viewScale", "posX", "posY", "posZ", "fieldOfView",
        "sizeDistance", "brightnessDistance", "aperture", "focalDistance",
    )}
    def deposit_pass(name, target, blend_mode, view_mode, blur_layer):
        conditions = {"runIf": [condition("blendMode", blend_mode), condition("viewMode", view_mode)]}
        if blur_layer:
            conditions["skipIf"] = [condition("aperture", 0), condition("viewMode", 0)]
        return {
            "name": name, "program": "deposit",
            "key": "render/pointsBillboardRender:deposit" if name == "deposit_0" else None,
            "inputs": deposit_inputs, "outputs": {"fragColor": target},
            "uniforms": {**deposit_uniforms, "blurLayer": blur_layer},
            "blend": True if blend_mode == 0 else ["ONE", "ONE_MINUS_SRC_ALPHA"],
            "drawMode": "billboards", "count": "input", "conditions": conditions,
            "defines": {"VIEW_MODE": view_mode, "BLEND_MODE": blend_mode, "BLUR_LAYER": blur_layer},
        }

    for view_mode in (1, 2):
        passes.append(deposit_pass(f"depositDefocus_{view_mode}", "defocus", 0, view_mode, 1))
    passes.extend([
        {
            "name": "diffuse", "program": "diffuse", "key": "render/pointsBillboardRender:diffuse",
            "inputs": {"trailTex": "global_billboard_trail", "defocusTex": "defocus"},
            "outputs": {"fragColor": "global_billboard_trail"},
            "uniforms": {name: name for name in ("intensity", "aperture", "viewMode", "blendMode")},
        },
        {
            "name": "copy", "program": "copy", "key": "render/pointsBillboardRender:copy",
            "inputs": {"sourceTex": "global_billboard_trail"},
            "outputs": {"fragColor": "global_billboard_trail"},
        },
    ])
    for blend_mode in (0, 1):
        prefix = "deposit_alpha" if blend_mode else "deposit"
        for view_mode in (0, 1, 2):
            passes.append(deposit_pass(
                f"{prefix}_{view_mode}", "global_billboard_trail", blend_mode, view_mode, 0,
            ))
    passes.append({
        "name": "blend", "program": "blend", "key": "render/pointsBillboardRender:blend",
        "inputs": {"inputTex": "inputTex", "trailTex": "global_billboard_trail"},
        "outputs": {"fragColor": "outputTex"},
        "uniforms": {"inputIntensity": "inputIntensity", "blendMode": "blendMode"},
    })
    return {
        "namespace": "render",
        "func": "pointsBillboardRender",
        "params": params,
        "passes": passes,
        "textures": {
            "depthOrderA": {"width": {"param": "stateSize", "default": 256}, "height": {"param": "stateSize", "default": 256}, "format": "rgba32f"},
            "depthOrderB": {"width": {"param": "stateSize", "default": 256}, "height": {"param": "stateSize", "default": 256}, "format": "rgba32f"},
            "spriteMeanTiles": {"width": 160, "height": 160, "format": "rgba32f"},
            "spriteMean": {"width": 5, "height": 5, "format": "rgba32f"},
            "defocus": {"width": "25%", "height": "25%", "format": "rgba16f"},
            "global_billboard_trail": {"format": "rgba16f", "width": "100%", "height": "100%"},
        },
        "externalTexture": None,
    }


COMPUTED_DEFS = {
    "mixer/mashup": _mashup(),
    "synth/remap": _remap(),
    "render/pointsBillboardRender": _points_billboard_render(),
}
