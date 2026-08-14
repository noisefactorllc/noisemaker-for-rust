use std::collections::BTreeMap;

use noisemaker_cpu::{ParamValue, RenderError, RenderOptions, Surface, render_effect};

fn options() -> RenderOptions {
    RenderOptions {
        width: 8,
        height: 8,
        time: 0.25,
        seed: 1,
        ..RenderOptions::default()
    }
}
fn flat(color: [f32; 4]) -> Surface {
    let mut surface = Surface::new(8, 8).unwrap();
    surface.clear(color);
    surface
}

#[test]
fn solid_and_invert_have_literal_pixels() {
    let solid = render_effect(
        "synth/solid",
        &BTreeMap::from([
            ("color".into(), ParamValue::String("#336699".into())),
            ("alpha".into(), ParamValue::Float(0.5)),
        ]),
        &BTreeMap::new(),
        &options(),
    )
    .unwrap();
    assert_eq!(&solid.to_rgba8()[..4], &[25, 51, 76, 128]);
    assert!(
        solid
            .data()
            .chunks_exact(4)
            .all(|pixel| pixel == &solid.data()[..4])
    );

    let invert = render_effect(
        "filter/invert",
        &BTreeMap::new(),
        &BTreeMap::from([("inputTex".into(), flat([0.2, 0.4, 0.6, 0.75]))]),
        &options(),
    )
    .unwrap();
    assert_eq!(&invert.to_rgba8()[..4], &[204, 153, 102, 191]);
}

#[test]
fn noise_is_deterministic_and_render_seed_changes_it() {
    let first = render_effect(
        "synth/noise",
        &BTreeMap::new(),
        &BTreeMap::new(),
        &options(),
    )
    .unwrap();
    let second = render_effect(
        "synth/noise",
        &BTreeMap::new(),
        &BTreeMap::new(),
        &options(),
    )
    .unwrap();
    assert_eq!(first, second);
    let changed = render_effect(
        "synth/noise",
        &BTreeMap::new(),
        &BTreeMap::new(),
        &RenderOptions {
            seed: 2,
            ..options()
        },
    )
    .unwrap();
    assert_ne!(first, changed);
}

#[test]
fn multi_pass_mixer_and_derivative_effects_render_finite_8x8() {
    let input = flat([0.2, 0.4, 0.6, 1.0]);
    let cases = [
        (
            "filter/bloom",
            BTreeMap::from([("inputTex".into(), input.clone())]),
        ),
        (
            "mixer/blendMode",
            BTreeMap::from([
                ("inputTex".into(), input.clone()),
                ("tex".into(), flat([0.7, 0.3, 0.1, 1.0])),
            ]),
        ),
        (
            "filter/halftone",
            BTreeMap::from([("inputTex".into(), input)]),
        ),
        (
            "filter/normalize",
            BTreeMap::from([("inputTex".into(), flat([0.2, 0.4, 0.6, 1.0]))]),
        ),
    ];
    for (effect, inputs) in cases {
        let output = render_effect(effect, &BTreeMap::new(), &inputs, &options()).unwrap();
        assert_eq!((output.width(), output.height()), (8, 8), "{effect}");
        assert!(
            output.data().iter().all(|value| value.is_finite()),
            "{effect}"
        );
    }
}

#[test]
fn custom_struct_copy_constructor_executes_real_shapes3d_program() {
    let input = flat([0.2, 0.4, 0.6, 1.0]);
    let output = render_effect(
        "classicNoisedeck/shapes3d",
        &BTreeMap::new(),
        &BTreeMap::from([("inputTex".into(), input.clone()), ("tex".into(), input)]),
        &options(),
    )
    .unwrap();
    assert_eq!((output.width(), output.height()), (8, 8));
    assert!(output.data().iter().all(|value| value.is_finite()));
}

#[test]
fn classic_palette_defaults_match_javascript_cpu_oracle_pixels() {
    let shapes = render_effect(
        "classicNoisedeck/shapes",
        &BTreeMap::new(),
        &BTreeMap::new(),
        &options(),
    )
    .unwrap();
    assert_eq!(
        &shapes.data()[..4],
        &[0.375, 0.327_880_86, 0.536_621_1, 1.0]
    );

    let input = flat([0.2, 0.4, 0.6, 1.0]);
    let shapes3d = render_effect(
        "classicNoisedeck/shapes3d",
        &BTreeMap::new(),
        &BTreeMap::from([("inputTex".into(), input.clone()), ("tex".into(), input)]),
        &options(),
    )
    .unwrap();
    let offset = ((2 * 8 + 5) * 4) as usize;
    assert_eq!(
        &shapes3d.data()[offset..offset + 4],
        &[0.663_085_94, 0.613_769_53, 0.666_503_9, 1.0]
    );
}

#[test]
fn crt_reduced_turn_sine_matches_javascript_cpu_oracle_rgba8_pixel() {
    let input = flat([
        0x55 as f32 / 255.0,
        0x88 as f32 / 255.0,
        0xcc as f32 / 255.0,
        1.0,
    ]);
    let crt = render_effect(
        "filter/crt",
        &BTreeMap::new(),
        &BTreeMap::from([("inputTex".into(), input)]),
        &options(),
    )
    .unwrap();
    let offset = ((3 * 8) * 4) as usize;
    assert_eq!(&crt.to_rgba8()[offset..offset + 4], &[96, 151, 224, 255]);
}

#[test]
fn snow_f32_semantic_adapter_matches_javascript_cpu_oracle_pixels() {
    let snow = render_effect(
        "filter/snow",
        &BTreeMap::new(),
        &BTreeMap::from([(
            "inputTex".into(),
            flat([
                0x55 as f32 / 255.0,
                0x88 as f32 / 255.0,
                0xcc as f32 / 255.0,
                1.0,
            ]),
        )]),
        &options(),
    )
    .unwrap();
    assert_eq!(
        &snow.to_rgba8()[..16],
        &[
            83, 116, 159, 255, 50, 76, 110, 255, 65, 96, 137, 255, 44, 69, 104, 255
        ]
    );
}

#[test]
fn temporal_aberration_matches_javascript_factory_true_branch_assignment_bug() {
    let temporal = render_effect(
        "filter/temporalAberration",
        &BTreeMap::from([("iterationCount".into(), ParamValue::Int(4))]),
        &BTreeMap::from([(
            "inputTex".into(),
            flat([
                0x55 as f32 / 255.0,
                0x88 as f32 / 255.0,
                0xcc as f32 / 255.0,
                1.0,
            ]),
        )]),
        &options(),
    )
    .unwrap();
    assert_eq!(&temporal.to_rgba8()[..4], &[85, 0, 0, 255]);
}

#[test]
fn remap_computed_uniform_array_and_shape_global_name_collision_are_supported() {
    for effect_id in ["synth/remap", "synth/shape"] {
        let output =
            render_effect(effect_id, &BTreeMap::new(), &BTreeMap::new(), &options()).unwrap();
        assert_eq!((output.width(), output.height()), (8, 8), "{effect_id}");
        assert!(
            output.data().iter().all(|value| value.is_finite()),
            "{effect_id}"
        );
    }
}

#[test]
fn unknown_effect_is_typed() {
    let error = render_effect(
        "synth/notReal",
        &BTreeMap::new(),
        &BTreeMap::new(),
        &options(),
    )
    .unwrap_err();
    assert!(matches!(error, RenderError::UnknownEffect { .. }));
}

fn assert_invalid_direct_parameter(
    effect_id: &str,
    name: &str,
    value: ParamValue,
    inputs: &BTreeMap<String, Surface>,
) {
    let error = render_effect(
        effect_id,
        &BTreeMap::from([(name.to_owned(), value)]),
        inputs,
        &options(),
    )
    .expect_err("schema-invalid direct parameter unexpectedly rendered");
    match error {
        RenderError::InvalidParameter { message } => {
            assert!(message.contains(name), "{effect_id}:{name}: {message}");
        }
        other => panic!("{effect_id}:{name} returned {other:?}, expected InvalidParameter"),
    }
}

#[test]
fn direct_parameter_api_rejects_malformed_scalars_and_catalog_bound_violations() {
    let no_inputs = BTreeMap::new();
    for (effect_id, name, value) in [
        (
            "synth/noise",
            "ridges",
            ParamValue::String("definitely-not-a-bool".into()),
        ),
        ("synth/noise", "ridges", ParamValue::Int(2)),
        ("synth/noise", "octaves", ParamValue::Float(2.5)),
        (
            "synth/noise",
            "octaves",
            ParamValue::String("2147483648".into()),
        ),
        ("synth/solid", "alpha", ParamValue::Float(f32::NAN)),
        ("synth/solid", "alpha", ParamValue::Float(-0.01)),
        ("synth/solid", "alpha", ParamValue::Float(1.01)),
    ] {
        assert_invalid_direct_parameter(effect_id, name, value, &no_inputs);
    }

    assert_invalid_direct_parameter(
        "filter/palette",
        "index",
        ParamValue::String("not-a-palette".into()),
        &BTreeMap::from([("inputTex".into(), flat([0.2, 0.4, 0.6, 1.0]))]),
    );
}

#[test]
fn direct_parameter_api_rejects_wrong_width_and_non_finite_vectors() {
    let no_inputs = BTreeMap::new();
    for value in [
        ParamValue::Vector(vec![0.1, 0.2]),
        ParamValue::Vector(vec![0.1, f32::INFINITY, 0.2]),
    ] {
        assert_invalid_direct_parameter("synth/solid", "color", value, &no_inputs);
    }
}

#[test]
fn direct_parameter_api_accepts_documented_lossless_coercions() {
    let false_bool = render_effect(
        "synth/noise",
        &BTreeMap::from([
            ("ridges".into(), ParamValue::Bool(false)),
            ("octaves".into(), ParamValue::Int(2)),
        ]),
        &BTreeMap::new(),
        &options(),
    )
    .unwrap();
    let coerced = render_effect(
        "synth/noise",
        &BTreeMap::from([
            ("ridges".into(), ParamValue::String("off".into())),
            ("octaves".into(), ParamValue::Float(2.0)),
        ]),
        &BTreeMap::new(),
        &options(),
    )
    .unwrap();
    assert_eq!(coerced, false_bool);

    let shorthand = render_effect(
        "synth/solid",
        &BTreeMap::from([("color".into(), ParamValue::String("#369c".into()))]),
        &BTreeMap::new(),
        &options(),
    )
    .unwrap();
    assert_eq!(&shorthand.to_rgba8()[..4], &[51, 102, 153, 255]);
}

#[test]
fn direct_parameter_api_accepts_decimal_schema_endpoints_at_f32_precision() {
    let inputs = BTreeMap::from([("inputTex".into(), flat([0.2, 0.4, 0.6, 1.0]))]);
    for (effect_id, name, value) in [
        ("filter/simpleAberration", "displacement", 0.1_f32),
        ("filter/celShading", "edgeThreshold", 0.01_f32),
    ] {
        let output = render_effect(
            effect_id,
            &BTreeMap::from([(name.into(), ParamValue::Float(value))]),
            &inputs,
            &options(),
        )
        .unwrap_or_else(|error| panic!("{effect_id}:{name} endpoint was rejected: {error}"));
        assert_eq!((output.width(), output.height()), (8, 8));
        assert!(output.data().iter().all(|value| value.is_finite()));
    }

    assert_invalid_direct_parameter(
        "filter/simpleAberration",
        "displacement",
        ParamValue::Float(f32::from_bits(0.1_f32.to_bits() + 1)),
        &inputs,
    );
    assert_invalid_direct_parameter(
        "filter/celShading",
        "edgeThreshold",
        ParamValue::Float(f32::from_bits(0.01_f32.to_bits() - 1)),
        &inputs,
    );
}
