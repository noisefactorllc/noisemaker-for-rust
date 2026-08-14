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
    let pixels = &resources["global_ns_velocity"].data()[..16];
    assert_eq!(
        pixels
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>(),
        [
            0xbe4b_2000,
            0xbe84_4000,
            0x3db5_e000,
            0x3f80_0000,
            0xbf18_c000,
            0xbf0c_c000,
            0x3e9f_6000,
            0x3f80_0000,
            0xbf39_2000,
            0xbf4b_8000,
            0x3f21_4000,
            0x3f80_0000,
            0xbe95_e000,
            0xbfb0_0000,
            0x3f80_0000,
            0x3f80_0000,
        ]
    );
}

#[test]
fn fractional_atlas_z_generators_match_javascript_pre_render_volume_oracles() {
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
            0.831_054_7,
            0.831_054_7,
            0.030_685_425,
            0.030_685_425,
            0.246_215_82,
            0.246_215_82,
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
    assert_eq!(reaction_b, [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
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
