use std::collections::BTreeMap;

use noisemaker_cpu::{
    FilterMode, ParamValue, RenderError, Surface, TextureFormat, Value,
    catalog::{ProgramIr, RenderPass, effect_catalog, shader_bundle},
    pass_runner::{
        apply_pass_uniform_aliases, canonical_uniforms, execute_pass, normalize_parameters,
        pass_enabled, repeat_count, texture_dimensions,
    },
};
use serde_json::json;

fn literal(value_type: &str, value: serde_json::Value) -> serde_json::Value {
    json!({"kind":"literal","type":value_type,"value":value,"source":null})
}
fn id(value_type: &str, name: &str, storage: &str) -> serde_json::Value {
    json!({"kind":"identifier","type":value_type,"name":name,"storage":storage})
}
fn vec_construct(values: &[f64]) -> serde_json::Value {
    json!({"kind":"construct","type":format!("vec{}", values.len()),"arguments":values.iter().map(|value| literal("float", json!(value))).collect::<Vec<_>>()})
}
fn binary(
    value_type: &str,
    operator: &str,
    left: serde_json::Value,
    right: serde_json::Value,
) -> serde_json::Value {
    json!({"kind":"binary","type":value_type,"operator":operator,"left":left,"right":right})
}
fn output_program(
    outputs: &[&str],
    uniforms: Vec<serde_json::Value>,
    assignments: Vec<(serde_json::Value, serde_json::Value)>,
) -> ProgramIr {
    let globals = outputs
        .iter()
        .map(|name| json!({"name":name,"type":"vec4","initializer":null,"arraySize":null}))
        .collect::<Vec<_>>();
    let body = assignments.into_iter().map(|(lvalue, value): (serde_json::Value, serde_json::Value)| json!({"kind":"expression","expression":{"kind":"assignment","type":"vec4","operator":"=","lvalue":lvalue,"value":value}})).collect::<Vec<_>>();
    serde_json::from_value(json!({
        "outputs":outputs,"varyings":["v_texCoord"],"structs":[],"uniforms":uniforms,"globals":globals,
        "functions":[{"name":"main","mangledName":"main__void","returnType":"void","parameters":[],"body":body}]
    })).unwrap()
}
fn pass(
    inputs: BTreeMap<String, String>,
    outputs: BTreeMap<String, String>,
    viewport: Option<serde_json::Value>,
) -> RenderPass {
    let mut execution = BTreeMap::new();
    if let Some(viewport) = viewport {
        execution.insert("viewport".into(), viewport);
    }
    RenderPass {
        name: "synthetic".into(),
        program: "synthetic".into(),
        key: None,
        inputs,
        outputs,
        execution,
    }
}

#[test]
fn parameters_use_defaults_coerce_values_and_render_seed_inherits() {
    let catalog = effect_catalog().unwrap();
    let solid = &catalog.effects["synth/solid"];
    let defaults = normalize_parameters(solid, &BTreeMap::new(), 19).unwrap();
    assert_eq!(defaults.uniforms["alpha"], Value::Float(1.0));
    assert_eq!(defaults.uniforms["color"], Value::Vec(vec![0.5; 3]));

    let noise = &catalog.effects["synth/noise"];
    let inherited = normalize_parameters(noise, &BTreeMap::new(), 19).unwrap();
    assert_eq!(inherited.uniforms["seed"], Value::Int(19));
    let explicit = normalize_parameters(
        noise,
        &BTreeMap::from([
            ("seed".into(), ParamValue::Int(7)),
            ("ridges".into(), ParamValue::String("yes".into())),
            ("type".into(), ParamValue::String("linear".into())),
        ]),
        19,
    )
    .unwrap();
    assert_eq!(explicit.uniforms["seed"], Value::Int(7));
    assert_eq!(explicit.uniforms["ridges"], Value::Bool(true));
    assert_eq!(explicit.uniforms["NOISE_TYPE"], Value::Int(1));

    let error = normalize_parameters(
        solid,
        &BTreeMap::from([("notAParameter".into(), ParamValue::Int(1))]),
        1,
    )
    .unwrap_err();
    assert!(error.to_string().contains("unknown parameter"));
}

#[test]
fn classic_palette_indices_override_hidden_uniforms_but_zero_keeps_raw_defaults() {
    let catalog = effect_catalog().unwrap();
    let shapes = &catalog.effects["classicNoisedeck/shapes"];

    let selected = normalize_parameters(shapes, &BTreeMap::new(), 1).unwrap();
    assert_eq!(
        selected.uniforms["paletteAmp"],
        Value::Vec(vec![0.73, 0.36, 0.52])
    );
    assert_eq!(selected.uniforms["paletteFreq"], Value::Vec(vec![1.0; 3]));
    assert_eq!(
        selected.uniforms["paletteOffset"],
        Value::Vec(vec![0.78, 0.68, 0.15])
    );
    assert_eq!(
        selected.uniforms["palettePhase"],
        Value::Vec(vec![0.74, 0.93, 0.28])
    );
    assert_eq!(selected.uniforms["paletteMode"], Value::Int(3));

    let raw = normalize_parameters(
        shapes,
        &BTreeMap::from([("palette".into(), ParamValue::Int(0))]),
        1,
    )
    .unwrap();
    assert_eq!(raw.uniforms["paletteAmp"], Value::Vec(vec![0.5; 3]));
    assert_eq!(raw.uniforms["paletteFreq"], Value::Vec(vec![1.0; 3]));
    assert_eq!(
        raw.uniforms["paletteOffset"],
        Value::Vec(vec![0.83, 0.6, 0.63])
    );
    assert_eq!(
        raw.uniforms["palettePhase"],
        Value::Vec(vec![0.3, 0.1, 0.0])
    );
    assert_eq!(raw.uniforms["paletteMode"], Value::Int(0));
}

#[test]
fn canonical_uniforms_override_effect_aliases() {
    let effect = BTreeMap::from([
        ("resolution".into(), Value::Vec(vec![999.0, 999.0])),
        ("seed".into(), Value::Int(73)),
        ("time".into(), Value::Float(99.0)),
    ]);
    let uniforms = canonical_uniforms(8, 4, 0.25, 3, 7, 1.0 / 600.0, &effect);
    assert_eq!(uniforms["resolution"], Value::Vec(vec![8.0, 4.0]));
    assert_eq!(uniforms["fullResolution"], Value::Vec(vec![8.0, 4.0]));
    assert_eq!(uniforms["tileOffset"], Value::Vec(vec![0.0, 0.0]));
    assert_eq!(uniforms["aspect"], Value::Float(2.0));
    assert_eq!(uniforms["time"], Value::Float(0.25));
    assert_eq!(uniforms["globalTime"], Value::Float(0.25));
    assert_eq!(uniforms["frame"], Value::Int(7));
    assert_eq!(uniforms["seed"], Value::Int(3));
}

#[test]
fn height_grid_agent_scales_grid_depth_by_the_frame_aspect() {
    // The agent program sizes the grid plane's depth as gridScale *
    // aspectInv. On a non-square frame the depth extent follows the frame's
    // proportions instead of staying at the full gridScale. With a 8x4
    // frame the aspect is 2 and the f32-rounded aspectInv is 0.5, so a
    // gridScale of 100 puts the two state rows' depth at +12.5 and -12.5;
    // a program that ignores the aspect would put them at +25 and -25.
    let effect = &effect_catalog().unwrap().effects["points/heightGrid"];
    let pass = effect
        .passes
        .iter()
        .find(|pass| pass.name == "agent")
        .unwrap();
    let ir = &shader_bundle().unwrap().programs[pass.key.as_ref().unwrap()].ir;
    let blank = || Surface::from_f32(4, 2, vec![0.0; 4 * 2 * 4]).unwrap();
    let mut resources = BTreeMap::from([
        ("heightTex".to_string(), blank()),
        ("diffuseTex".to_string(), blank()),
        ("global_vel".to_string(), blank()),
        ("global_xyz".to_string(), blank()),
    ]);
    let mut uniforms = BTreeMap::from([
        ("gridScale".to_string(), Value::Float(100.0)),
        ("heightScale".to_string(), Value::Float(1.0)),
        ("heightOffset".to_string(), Value::Float(0.0)),
    ]);
    for (name, value) in canonical_uniforms(8, 4, 0.25, 1, 0, 0.1, &BTreeMap::new()) {
        uniforms.insert(name, value);
    }
    execute_pass(
        ir,
        pass,
        &uniforms,
        &mut resources,
        4,
        2,
        &BTreeMap::new(),
        None,
        false,
    )
    .unwrap();
    let xyz = resources["global_xyz"].data();
    let mut depths = xyz
        .iter()
        .skip(2)
        .step_by(4)
        .map(|v| v.to_bits() as i64)
        .collect::<Vec<_>>();
    depths.sort();
    let mut expected = vec![12.5f32.to_bits() as i64; 4];
    expected.extend(vec![(-12.5f32).to_bits() as i64; 4]);
    expected.sort();
    assert_eq!(depths, expected, "xyz depth components {xyz:?}");
    for pixel in xyz.chunks_exact(4) {
        assert_eq!(pixel[3].to_bits(), 1.0f32.to_bits());
        assert_eq!(pixel[1], 0.0);
    }
    let mut widths = xyz
        .iter()
        .step_by(4)
        .map(|v| v.to_bits() as i64)
        .collect::<Vec<_>>();
    widths.sort();
    let mut expected_widths = [37.5f32, -12.5, 12.5, 37.5]
        .iter()
        .flat_map(|v| [v.to_bits() as i64, (-*v).to_bits() as i64])
        .collect::<Vec<_>>();
    expected_widths.sort();
    assert_eq!(widths, expected_widths, "xyz width components {xyz:?}");
}

#[test]
fn heightmap3d_precompute_keeps_the_frame_proportions() {
    // The precompute program sizes the terrain span from the frame's aspect
    // and aspectInv and keeps the columns outside the span at zero height.
    // On an 8x4 frame the aspect is 2 and the f32-rounded aspectInv is 0.5,
    // so a volumeSize of 8 spans the full width but only the four depth
    // columns 2 to 5; a program that ignores the aspect would fill every
    // depth column of the volume.
    let effect = &effect_catalog().unwrap().effects["synth3d/heightmap3d"];
    let pass = effect
        .passes
        .iter()
        .find(|pass| pass.name == "precompute")
        .unwrap();
    let ir = &shader_bundle().unwrap().programs[pass.key.as_ref().unwrap()].ir;
    let white = Surface::from_f32(8, 8, vec![1.0; 8 * 8 * 4]).unwrap();
    let mut resources = BTreeMap::from([
        ("heightTex".to_string(), white.clone()),
        ("tex".to_string(), white.clone()),
    ]);
    let mut uniforms = BTreeMap::from([
        ("volumeSize".to_string(), Value::Int(8)),
        ("heightScale".to_string(), Value::Float(1.0)),
        ("baseHeight".to_string(), Value::Float(0.0)),
    ]);
    for (name, value) in canonical_uniforms(8, 4, 0.25, 1, 0, 0.1, &BTreeMap::new()) {
        uniforms.insert(name, value);
    }
    execute_pass(
        ir,
        pass,
        &uniforms,
        &mut resources,
        8,
        4,
        &BTreeMap::new(),
        None,
        false,
    )
    .unwrap();
    let volume = resources["volumeCache"].data();
    assert_eq!(volume.len(), 8 * 64 * 4);
    for row in 0..64usize {
        let depth = row / 8;
        let expected: f32 = if (2..=5).contains(&depth) { 1.0 } else { 0.0 };
        for column in 0..8usize {
            let pixel = &volume[(row * 8 + column) * 4..(row * 8 + column) * 4 + 4];
            assert_eq!(
                pixel[3].to_bits(),
                expected.to_bits(),
                "occupancy at column {column}, atlas row {row} (depth {depth}): {pixel:?}"
            );
            for channel in 0..3 {
                assert_eq!(
                    pixel[channel].to_bits(),
                    expected.to_bits(),
                    "color channel {channel} at column {column}, atlas row {row}: {pixel:?}"
                );
            }
        }
    }
}

#[test]
fn conditions_repeats_and_resource_dimensions_are_deterministic() {
    let uniforms = BTreeMap::from([
        ("mode".into(), Value::Int(1)),
        ("iterations".into(), Value::Int(3)),
    ]);
    assert!(pass_enabled(
        &serde_json::json!({"runIf": [{"uniform": "mode", "equals": 1}]}),
        &uniforms
    ));
    assert!(!pass_enabled(
        &serde_json::json!({"skipIf": [{"uniform": "mode", "equals": 1}]}),
        &uniforms
    ));
    assert_eq!(repeat_count(&serde_json::json!(2), &uniforms), 2);
    assert_eq!(repeat_count(&serde_json::json!("iterations"), &uniforms), 3);

    let values = BTreeMap::from([
        ("stateSize".into(), Value::Int(64)),
        ("zoom".into(), Value::Float(4.0)),
    ]);
    assert_eq!(
        texture_dimensions(
            &serde_json::json!({"width": "50%", "height": "25%"}),
            &values,
            80,
            40,
            &BTreeMap::new(),
        )
        .unwrap(),
        (40, 10)
    );
    assert_eq!(
        texture_dimensions(
            &serde_json::json!({
                "width": {"param": "stateSize", "default": 256},
                "height": {"screenDivide": "zoom", "default": 8}
            }),
            &values,
            80,
            40,
            &BTreeMap::new(),
        )
        .unwrap(),
        (64, 10)
    );
    assert_eq!(
        texture_dimensions(
            &serde_json::json!({"width": "0.4%", "height": "6.25%"}),
            &BTreeMap::new(),
            8,
            8,
            &BTreeMap::new(),
        )
        .unwrap(),
        (1, 1)
    );
}

#[test]
fn pass_conditions_compare_numeric_float_and_integer_uniforms_like_cpu() {
    let uniforms = BTreeMap::from([
        ("aperture".into(), Value::Float(0.0)),
        ("enabled".into(), Value::Bool(true)),
    ]);
    assert!(!pass_enabled(
        &json!({"skipIf": [{"uniform": "aperture", "equals": 0}]}),
        &uniforms
    ));
    assert!(pass_enabled(
        &json!({"runIf": [{"uniform": "enabled", "equals": 1}]}),
        &uniforms
    ));
    assert!(!pass_enabled(
        &json!({"runIf": [{"uniform": "aperture", "equals": 1}]}),
        &uniforms
    ));
}

#[test]
fn reserved_canonical_uniforms_survive_aliases_through_pass_execution() {
    let program = output_program(
        &["fragColor"],
        vec![json!({"name":"time","type":"float","initializer":null,"arraySize":null})],
        vec![(
            id("vec4", "fragColor", "global"),
            json!({"kind":"construct","type":"vec4","arguments":[id("float","time","uniform")]}),
        )],
    );
    let render_pass = pass(
        BTreeMap::new(),
        BTreeMap::from([("fragColor".into(), "out".into())]),
        None,
    );
    let canonical = canonical_uniforms(1, 1, 0.25, 3, 7, 0.1, &BTreeMap::new());
    let aliases = BTreeMap::from([("time".into(), json!("fakeTime"))]);
    let values = BTreeMap::from([("fakeTime".into(), Value::Float(99.0))]);
    let pass_uniforms = apply_pass_uniform_aliases(&canonical, &values, &aliases);
    let mut resources = BTreeMap::new();
    execute_pass(
        &program,
        &render_pass,
        &pass_uniforms,
        &mut resources,
        1,
        1,
        &BTreeMap::new(),
        None,
        false,
    )
    .unwrap();
    assert_eq!(resources["out"].data(), &[0.25; 4]);
}

#[test]
fn pass_uniform_aliases_bind_literal_merge_lengths_and_draw_layers() {
    let canonical = canonical_uniforms(1, 1, 0.0, 3, 0, 0.0, &BTreeMap::new());
    let sources = BTreeMap::from([("viewMode".into(), Value::Int(1))]);
    let aliases = BTreeMap::from([
        ("runLength".into(), json!(4)),
        ("blurLayer".into(), json!(1)),
        ("clearValue".into(), json!(0.5)),
        ("VIEW_MODE".into(), json!("viewMode")),
    ]);
    let bound = apply_pass_uniform_aliases(&canonical, &sources, &aliases);
    assert_eq!(bound.get("runLength"), Some(&Value::Int(4)));
    assert_eq!(bound.get("blurLayer"), Some(&Value::Int(1)));
    assert_eq!(bound.get("clearValue"), Some(&Value::Float(0.5)));
    assert_eq!(bound.get("VIEW_MODE"), Some(&Value::Int(1)));
}

#[test]
fn billboard_depth_merge_adapter_matches_shader_on_ties_and_partial_runs() {
    let effect = &effect_catalog().unwrap().effects["render/pointsBillboardRender"];
    let pass = effect
        .passes
        .iter()
        .find(|pass| pass.name == "depthMerge0")
        .unwrap();
    let ir = &shader_bundle().unwrap().programs[pass.key.as_ref().unwrap()].ir;
    let shader_pass = RenderPass {
        name: pass.name.clone(),
        program: pass.program.clone(),
        key: None,
        inputs: pass.inputs.clone(),
        outputs: pass.outputs.clone(),
        execution: pass.execution.clone(),
    };
    // Storage is top-down; GLSL's linear particle indices start in the bottom row.
    let keys = [
        [1.0, 4.0, 0.0, 1.0],
        [2.0, 5.0, 0.0, 1.0],
        [3.0, 6.0, 0.0, 1.0],
        [4.0, 7.0, 0.0, 1.0],
        [-3.0, 0.0, 0.0, 1.0],
        [-1.0, 1.0, 0.0, 1.0],
        [-3.0, 2.0, 0.0, 1.0],
        [0.0, 3.0, 0.0, 1.0],
    ];
    let source = Surface::from_f32(4, 2, keys.into_iter().flatten().collect()).unwrap();
    for run_length in [1, 2, 4, 8, 16] {
        let uniforms = BTreeMap::from([("runLength".into(), Value::Int(run_length))]);
        let base = BTreeMap::from([("depthOrderA".into(), source.clone())]);
        let mut adapted = base.clone();
        let mut interpreted = base;
        execute_pass(
            ir,
            pass,
            &uniforms,
            &mut adapted,
            4,
            2,
            &BTreeMap::new(),
            None,
            false,
        )
        .unwrap();
        execute_pass(
            ir,
            &shader_pass,
            &uniforms,
            &mut interpreted,
            4,
            2,
            &BTreeMap::new(),
            None,
            false,
        )
        .unwrap();
        assert_eq!(
            adapted["depthOrderB"].data(),
            interpreted["depthOrderB"].data(),
            "runLength={run_length}"
        );
    }
}

#[test]
fn navier_splat_initial_attachment_matches_javascript_cpu_oracle() {
    let effect = &effect_catalog().unwrap().effects["synth/navierStokes"];
    let normalized = normalize_parameters(effect, &BTreeMap::new(), 1).unwrap();
    let uniforms = canonical_uniforms(8, 8, 0.25, 1, 0, 1.0 / 600.0, &normalized.uniforms);
    let pass = &effect.passes[0];
    let program = &shader_bundle().unwrap().programs[pass.key.as_ref().unwrap()].ir;
    let mut resources = BTreeMap::from([
        ("global_ns_velocity".into(), Surface::new(8, 8).unwrap()),
        ("tex".into(), Surface::new(1, 1).unwrap()),
    ]);
    execute_pass(
        program,
        pass,
        &uniforms,
        &mut resources,
        8,
        8,
        &BTreeMap::from([("global_ns_velocity".into(), TextureFormat::Rgba16f)]),
        None,
        false,
    )
    .unwrap();
    // CPU 4aeb3e1e028b2dc412401f7b7f17c014c603bf14 nsSplat with identical
    // default uniforms, zero 8x8 buffer and zero 1x1 input, then rgba16f truncation.
    let pixels = &resources["global_ns_velocity"].data()[..16];
    assert_eq!(
        pixels
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>(),
        [
            0xbe69_a000,
            0xbec4_2000,
            0x3e0d_c000,
            0x3f80_0000,
            0xbf39_2000,
            0xbf0f_0000,
            0x3ee0_e000,
            0x3f80_0000,
            0xbf78_e000,
            0x3d9e_a000,
            0x3f19_a000,
            0x3f80_0000,
            0xbf0e_0000,
            0x3dc1_c000,
            0x3ee9_0000,
            0x3f80_0000,
        ]
    );
}

#[test]
fn atlas_z_generators_match_javascript_pre_render_volume_oracles() {
    let catalog = effect_catalog().unwrap();
    let shaders = shader_bundle().unwrap();

    let shape = &catalog.effects["synth3d/shape3d"];
    let mut shape_values = normalize_parameters(shape, &BTreeMap::new(), 1)
        .unwrap()
        .uniforms;
    shape_values.insert("volumeSize".into(), Value::Int(2));
    let shape_uniforms = canonical_uniforms(2, 4, 0.25, 1, 0, 1.0 / 600.0, &shape_values);
    let shape_pass = &shape.passes[0];
    let shape_program = &shaders.programs[shape_pass.key.as_ref().unwrap()].ir;
    let mut shape_resources = BTreeMap::new();
    execute_pass(
        shape_program,
        shape_pass,
        &shape_uniforms,
        &mut shape_resources,
        2,
        4,
        &BTreeMap::from([
            ("volumeCache".into(), TextureFormat::Rgba16f),
            ("geoBuffer".into(), TextureFormat::Rgba16f),
        ]),
        None,
        false,
    )
    .unwrap();
    let shape_density = shape_resources["volumeCache"]
        .data()
        .chunks_exact(4)
        .map(|pixel| pixel[0])
        .collect::<Vec<_>>();
    assert_eq!(
        shape_density,
        [
            0.030_685_425,
            0.030_685_425,
            0.030_685_425,
            0.030_685_425,
            0.030_685_425,
            0.030_685_425,
            0.030_685_425,
            0.030_685_425,
        ]
    );

    let reaction = &catalog.effects["synth3d/reactionDiffusion3d"];
    let mut reaction_values = normalize_parameters(reaction, &BTreeMap::new(), 1)
        .unwrap()
        .uniforms;
    reaction_values.insert("volumeSize".into(), Value::Int(2));
    reaction_values.insert("iterations".into(), Value::Int(1));
    let reaction_uniforms = canonical_uniforms(2, 4, 0.25, 1, 0, 1.0 / 600.0, &reaction_values);
    let reaction_pass = &reaction.passes[0];
    let reaction_program = &shaders.programs[reaction_pass.key.as_ref().unwrap()].ir;
    let mut reaction_resources = BTreeMap::from([
        ("global_rd_state".into(), Surface::new(2, 4).unwrap()),
        ("source".into(), Surface::new(2, 4).unwrap()),
    ]);
    execute_pass(
        reaction_program,
        reaction_pass,
        &reaction_uniforms,
        &mut reaction_resources,
        2,
        4,
        &BTreeMap::from([("global_rd_state".into(), TextureFormat::Rgba16f)]),
        None,
        false,
    )
    .unwrap();
    let reaction_b = reaction_resources["global_rd_state"]
        .data()
        .chunks_exact(4)
        .map(|pixel| pixel[0])
        .collect::<Vec<_>>();
    assert_eq!(reaction_b, [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn external_and_internal_filter_contract_is_representable() {
    let mut external = Surface::new(1, 1).unwrap();
    external.set_filter_mode(FilterMode::Linear);
    let internal = Surface::new(1, 1).unwrap();
    assert_eq!(external.filter_mode(), FilterMode::Linear);
    assert_eq!(internal.filter_mode(), FilterMode::Nearest);
}

#[test]
fn mrt_rejects_mixed_destination_dimensions_before_execution() {
    let program = output_program(
        &["first", "second"],
        vec![],
        vec![
            (
                id("vec4", "first", "global"),
                vec_construct(&[1.0, 0.0, 0.0, 1.0]),
            ),
            (
                id("vec4", "second", "global"),
                vec_construct(&[0.0, 1.0, 0.0, 1.0]),
            ),
        ],
    );
    let render_pass = pass(
        BTreeMap::new(),
        BTreeMap::from([
            ("first".into(), "global_xyz".into()),
            ("second".into(), "global_rgba".into()),
        ]),
        None,
    );
    let mut resources = BTreeMap::from([
        ("global_xyz".into(), Surface::new(4, 4).unwrap()),
        ("global_rgba".into(), Surface::new(3, 4).unwrap()),
    ]);

    let error = execute_pass(
        &program,
        &render_pass,
        &BTreeMap::new(),
        &mut resources,
        8,
        8,
        &BTreeMap::new(),
        None,
        false,
    )
    .unwrap_err();

    assert!(matches!(
        &error,
        RenderError::MrtDestinationDimensions { pass, destinations, .. }
            if pass == "synthetic"
                && destinations == &vec![("global_xyz".into(), 4, 4), ("global_rgba".into(), 3, 4)]
    ));
    assert_eq!(
        error.to_string(),
        "pass \"synthetic\" MRT destinations must share dimensions: global_xyz (4x4), global_rgba (3x4)"
    );
}

#[test]
fn single_attachment_without_viewport_preserves_declared_resource_dimensions() {
    let program = output_program(
        &["fragColor"],
        vec![],
        vec![(
            id("vec4", "fragColor", "global"),
            vec_construct(&[0.25, 0.5, 0.75, 1.0]),
        )],
    );
    let render_pass = pass(
        BTreeMap::new(),
        BTreeMap::from([("fragColor".into(), "scratch".into())]),
        None,
    );
    let mut resources = BTreeMap::from([("scratch".into(), Surface::new(2, 1).unwrap())]);

    execute_pass(
        &program,
        &render_pass,
        &BTreeMap::new(),
        &mut resources,
        8,
        8,
        &BTreeMap::new(),
        None,
        false,
    )
    .unwrap();

    let scratch = &resources["scratch"];
    assert_eq!((scratch.width(), scratch.height()), (2, 1));
    assert_eq!(scratch.to_rgba8(), [64, 128, 191, 255].repeat(2));
}

#[test]
fn execution_updates_named_attachments_uses_viewport_and_binds_black_missing_sampler() {
    let sampler = json!({"name":"inputTex","type":"sampler2D","initializer":null,"arraySize":null});
    let texture = json!({"kind":"call","type":"vec4","name":"texture","target":"builtin:texture","arguments":[id("sampler2D","inputTex","uniform"), id("vec2","v_texCoord","varying")]});
    let program = output_program(
        &["fragColor"],
        vec![sampler],
        vec![(id("vec4", "fragColor", "global"), texture)],
    );
    let render_pass = pass(
        BTreeMap::from([("inputTex".into(), "absent".into())]),
        BTreeMap::from([("fragColor".into(), "namedOutput".into())]),
        Some(json!({"width": 2, "height": 1})),
    );
    let mut resources = BTreeMap::new();
    execute_pass(
        &program,
        &render_pass,
        &BTreeMap::new(),
        &mut resources,
        8,
        8,
        &BTreeMap::new(),
        None,
        false,
    )
    .unwrap();
    let output = &resources["namedOutput"];
    assert_eq!((output.width(), output.height()), (2, 1));
    assert_eq!(output.to_rgba8(), vec![0; 8]);
}

#[test]
fn execution_samples_external_linear_and_internal_nearest() {
    let sampler = json!({"name":"inputTex","type":"sampler2D","initializer":null,"arraySize":null});
    let texture = json!({"kind":"call","type":"vec4","name":"texture","target":"builtin:texture","arguments":[id("sampler2D","inputTex","uniform"), vec_construct(&[0.5,0.5])]});
    let program = output_program(
        &["fragColor"],
        vec![sampler],
        vec![(id("vec4", "fragColor", "global"), texture)],
    );
    let render_pass = pass(
        BTreeMap::from([("inputTex".into(), "source".into())]),
        BTreeMap::from([("fragColor".into(), "out".into())]),
        None,
    );
    let source = Surface::from_f32(2, 1, vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0]).unwrap();
    let mut internal = BTreeMap::from([("source".into(), source.clone())]);
    execute_pass(
        &program,
        &render_pass,
        &BTreeMap::new(),
        &mut internal,
        1,
        1,
        &BTreeMap::new(),
        None,
        false,
    )
    .unwrap();
    assert_eq!(internal["out"].data()[0], 1.0);
    let mut external = BTreeMap::from([("source".into(), source)]);
    execute_pass(
        &program,
        &render_pass,
        &BTreeMap::new(),
        &mut external,
        1,
        1,
        &BTreeMap::new(),
        Some("inputTex"),
        false,
    )
    .unwrap();
    assert_eq!(external["out"].data()[0], 0.5);
}

#[test]
fn execution_supports_mrt_and_quantizes_each_output_format() {
    let program = output_program(
        &["out8", "out16", "out32"],
        vec![],
        vec![
            (
                id("vec4", "out8", "global"),
                vec_construct(&[0.1, 0.2, 0.3, 1.0]),
            ),
            (
                id("vec4", "out16", "global"),
                vec_construct(&[0.1, 0.2, 0.3, 1.0]),
            ),
            (
                id("vec4", "out32", "global"),
                vec_construct(&[0.1, 0.2, 0.3, 1.0]),
            ),
        ],
    );
    let render_pass = pass(
        BTreeMap::new(),
        BTreeMap::from([
            ("out8".into(), "a".into()),
            ("out16".into(), "b".into()),
            ("out32".into(), "c".into()),
        ]),
        None,
    );
    let formats = BTreeMap::from([
        ("a".into(), TextureFormat::Rgba8),
        ("b".into(), TextureFormat::Rgba16f),
        ("c".into(), TextureFormat::Rgba32f),
    ]);
    let mut resources = BTreeMap::new();
    execute_pass(
        &program,
        &render_pass,
        &BTreeMap::new(),
        &mut resources,
        1,
        1,
        &formats,
        None,
        false,
    )
    .unwrap();
    assert_eq!(resources["a"].data()[0], 26.0 / 255.0);
    assert_eq!(
        resources["b"].data()[0].to_bits(),
        0.099975586_f32.to_bits()
    );
    assert_eq!(resources["c"].data()[0].to_bits(), 0.1_f32.to_bits());
    assert_ne!(resources["a"].data()[0], resources["b"].data()[0]);
}

#[test]
fn attachment_writes_preserve_non_finite_shader_lanes_for_format_conversion() {
    let zero_over_zero = binary(
        "float",
        "/",
        literal("float", json!(0.0)),
        literal("float", json!(0.0)),
    );
    let program = output_program(
        &["fragColor"],
        vec![],
        vec![(
            id("vec4", "fragColor", "global"),
            json!({"kind":"construct","type":"vec4","arguments":[zero_over_zero]}),
        )],
    );
    let render_pass = pass(
        BTreeMap::new(),
        BTreeMap::from([("fragColor".into(), "out".into())]),
        None,
    );
    let mut resources = BTreeMap::new();
    execute_pass(
        &program,
        &render_pass,
        &BTreeMap::new(),
        &mut resources,
        1,
        1,
        &BTreeMap::new(),
        None,
        false,
    )
    .unwrap();
    assert!(resources["out"].data().iter().all(|value| value.is_nan()));
}

#[test]
fn fullscreen_blend_uses_each_attachment_before_storage_conversion() {
    let program = output_program(
        &["z", "a"],
        vec![],
        ["z", "a"]
            .into_iter()
            .map(|name| {
                (
                    id("vec4", name, "global"),
                    vec_construct(&[0.5, 0.25, 0.75, 0.5]),
                )
            })
            .collect(),
    );
    let mut pass = pass(
        BTreeMap::new(),
        BTreeMap::from([("z".into(), "left".into()), ("a".into(), "right".into())]),
        None,
    );
    pass.execution
        .insert("blend".into(), json!(["DST_COLOR", 0]));
    let mut resources = BTreeMap::from([
        (
            "left".into(),
            Surface::from_f32(1, 1, vec![0.25, 0.5, 1.0, 0.75]).unwrap(),
        ),
        (
            "right".into(),
            Surface::from_f32(1, 1, vec![0.0, 1.0, 0.25, 0.5]).unwrap(),
        ),
    ]);
    execute_pass(
        &program,
        &pass,
        &BTreeMap::new(),
        &mut resources,
        1,
        1,
        &BTreeMap::new(),
        None,
        false,
    )
    .unwrap();
    assert_eq!(resources["left"].data(), &[0.125, 0.125, 0.75, 0.375]);
    assert_eq!(resources["right"].data(), &[0.0, 0.25, 0.1875, 0.25]);
}

#[test]
fn single_fragment_output_uses_first_declared_binding_including_storage() {
    let program = output_program(
        &["fragColor"],
        vec![],
        vec![(id("vec4", "fragColor", "global"), vec_construct(&[0.5; 4]))],
    );
    for (bindings, order) in [
        ("outputs", "outputOrder"),
        ("storageTextures", "storageTextureOrder"),
    ] {
        let pass: RenderPass = serde_json::from_value(json!({
            "name":"select","program":"select",
            bindings: {"z":"first","a":"second"}, order:["z","a"],
        }))
        .unwrap();
        let mut resources = BTreeMap::new();
        execute_pass(
            &program,
            &pass,
            &BTreeMap::new(),
            &mut resources,
            1,
            1,
            &BTreeMap::new(),
            None,
            false,
        )
        .unwrap();
        assert_eq!(resources["first"].data(), &[0.5; 4]);
        assert!(!resources.contains_key("second"));
    }
}
