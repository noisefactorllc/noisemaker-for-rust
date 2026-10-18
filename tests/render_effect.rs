use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

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
fn billboard_pipeline_depth_orders_overlap_and_spreads_defocus() {
    let state = |pixels: &[[f32; 4]]| {
        Surface::from_f32(
            pixels.len() as u32,
            1,
            pixels.iter().flatten().copied().collect(),
        )
        .unwrap()
    };
    let alpha_inputs = BTreeMap::from([
        ("inputTex".into(), flat([0.0; 4])),
        ("tex".into(), state(&[[1.0; 4]])),
        (
            "global_xyz".into(),
            state(&[[0.0, 0.0, 2.0, 1.0], [0.0, 0.0, 1.0, 1.0]]),
        ),
        (
            "global_rgba".into(),
            state(&[[0.5, 0.0, 0.0, 0.5], [0.0, 0.0, 0.5, 0.5]]),
        ),
        ("depthOrderA".into(), state(&[[0.0; 4]; 2])),
        ("depthOrderB".into(), state(&[[0.0; 4]; 2])),
    ]);
    let alpha_params = BTreeMap::from([
        ("blendMode".into(), ParamValue::Int(1)),
        ("viewMode".into(), ParamValue::Int(1)),
        ("shapeMode".into(), ParamValue::Int(3)),
        ("density".into(), ParamValue::Float(100.0)),
        ("depositOpacity".into(), ParamValue::Float(100.0)),
        ("pointSize".into(), ParamValue::Float(3.0)),
        ("rotateX".into(), ParamValue::Float(0.0)),
        ("intensity".into(), ParamValue::Float(0.0)),
        ("inputIntensity".into(), ParamValue::Float(0.0)),
    ]);
    let alpha = render_effect(
        "render/pointsBillboardRender",
        &alpha_params,
        &alpha_inputs,
        &options(),
    )
    .unwrap();
    let center = ((4 * 8 + 4) * 4) as usize;
    assert!(
        alpha.data()[center] > alpha.data()[center + 2],
        "far blue must deposit before near red: center={:?}",
        &alpha.data()[center..center + 4]
    );

    let blur_inputs = BTreeMap::from([
        ("inputTex".into(), flat([0.0; 4])),
        ("tex".into(), state(&[[1.0; 4]])),
        ("global_xyz".into(), state(&[[0.0, 0.0, 1.0, 1.0]])),
        ("global_rgba".into(), state(&[[1.0, 0.0, 0.0, 1.0]])),
    ]);
    let mut blur_params = BTreeMap::from([
        ("blendMode".into(), ParamValue::Int(0)),
        ("viewMode".into(), ParamValue::Int(1)),
        ("shapeMode".into(), ParamValue::Int(3)),
        ("density".into(), ParamValue::Float(100.0)),
        ("depositOpacity".into(), ParamValue::Float(100.0)),
        ("pointSize".into(), ParamValue::Float(2.0)),
        ("rotateX".into(), ParamValue::Float(0.0)),
        ("intensity".into(), ParamValue::Float(0.0)),
        ("inputIntensity".into(), ParamValue::Float(0.0)),
        ("focalDistance".into(), ParamValue::Float(40.0)),
        ("aperture".into(), ParamValue::Float(0.0)),
    ]);
    let sharp = render_effect(
        "render/pointsBillboardRender",
        &blur_params,
        &blur_inputs,
        &options(),
    )
    .unwrap();
    blur_params.insert("aperture".into(), ParamValue::Float(20.0));
    let blurred = render_effect(
        "render/pointsBillboardRender",
        &blur_params,
        &blur_inputs,
        &options(),
    )
    .unwrap();
    let lit = |surface: &Surface| {
        surface
            .data()
            .chunks_exact(4)
            .filter(|pixel| pixel[0] > 0.0)
            .count()
    };
    assert!(
        lit(&blurred) > lit(&sharp),
        "defocus must spread the sprite across pixels"
    );
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
    // Reference 0ed489ec (this round): invert un-premultiplies on read, inverts
    // straight RGB, then re-premultiplies on write. Input [0.2,0.4,0.6,0.75] ->
    // unpremultiplied [0.2667,0.5333,0.8] -> inverted [0.7333,0.4667,0.2] ->
    // re-premultiplied by 0.75 -> [0.55,0.35,0.15,0.75] -> [140,89,38,191].
    // Was [204,153,102,191] (a straight, non-premultiplied invert) before this
    // round's fix.
    assert_eq!(&invert.to_rgba8()[..4], &[140, 89, 38, 191]);
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
fn crt_native_adapter_matches_javascript_cpu_oracle_rgba8_pixel() {
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
    // CPU 4aeb3e1e crtFactory at fragCoord [0.5,4.5], alpha 0.5, speed 1;
    // raw [0.3803921937942505,0.5960784554481506,0.8843137621879578,1], then rgba16f.
    assert_eq!(&crt.to_rgba8()[offset..offset + 4], &[97, 152, 225, 255]);
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
    // CPU 4aeb3e1e028b2dc412401f7b7f17c014c603bf14 snowFactory, identical
    // 8x8 Float32 input/options, default alpha 0.5/density 75/pause false, then rgba16f.
    let rgba8 = snow.to_rgba8();
    assert_eq!(
        &rgba8[..16],
        &[
            82, 114, 158, 255, 51, 77, 111, 255, 65, 95, 136, 255, 44, 69, 104, 255
        ]
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(&rgba8)),
        "1945c3cec2d4b35d46259a2660f33fdcc5fe78683e197e089166f80fbae49b9f"
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
fn remap_bound_polygon_uses_zone_texture_and_background() {
    let output = render_effect(
        "synth/remap",
        &BTreeMap::from([
            ("zoneCount".into(), ParamValue::Int(1)),
            ("bgColor".into(), ParamValue::Vector(vec![0.0, 0.0, 1.0])),
            ("smoothEdge".into(), ParamValue::Float(0.0)),
            ("zone0_count".into(), ParamValue::Int(3)),
            (
                "zone0_v0".into(),
                ParamValue::Vector(vec![0.0, 0.0, 1.0, 0.0]),
            ),
            (
                "zone0_v1".into(),
                ParamValue::Vector(vec![0.0, 1.0, 0.0, 0.0]),
            ),
        ]),
        &BTreeMap::from([("zone0_tex".into(), flat([1.0, 0.0, 0.0, 1.0]))]),
        &options(),
    )
    .unwrap();
    let pixels = output.to_rgba8();
    assert!(
        pixels
            .chunks_exact(4)
            .all(|pixel| pixel == [255, 0, 0, 255] || pixel == [0, 0, 255, 255])
    );
    assert_eq!(
        pixels
            .chunks_exact(8 * 4)
            .map(|row| {
                row.chunks_exact(4)
                    .filter(|pixel| *pixel == [255, 0, 0, 255])
                    .count()
            })
            .collect::<Vec<_>>(),
        [8, 7, 6, 5, 4, 3, 2, 1]
    );
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

#[test]
fn numbered_test_pattern_truncates_the_digit_division_like_the_cpu_oracle() {
    let output = render_effect(
        "synth/testPattern",
        &BTreeMap::new(),
        &BTreeMap::new(),
        &RenderOptions {
            width: 64,
            height: 64,
            ..options()
        },
    )
    .unwrap();
    let pixels = output.to_rgba8();
    // The CPU compiler rewrites the digit accumulator's `temp /= 10` into
    // `temp = Math.trunc(temp / 10)`, so every tens digit stays integral and
    // each cell's glyph lookup succeeds. Cells 11..15 now draw their real
    // glyphs instead of the undefined-lookup zero bits.
    for y in 53..59 {
        for (x, expected) in [(6, 0), (22, 255), (38, 0), (54, 255)] {
            let index = (y * 64 + x) * 4;
            assert_eq!(
                &pixels[index..index + 4],
                &[expected, expected, expected, 255]
            );
        }
    }
    for y in 37..43 {
        let index = (y * 64 + 54) * 4;
        assert_eq!(&pixels[index..index + 4], &[0, 0, 0, 255]);
    }
}
