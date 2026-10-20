use std::collections::BTreeMap;

use noisemaker_cpu::{
    AutomationField, DslChainElement, DslValue, ParamValue, RenderStep, SurfaceBinding, TokenKind,
    catalog::effect_catalog, compile_dsl, parse_dsl, tokenize_dsl,
};

fn first_call(ast: &noisemaker_cpu::DslProgram) -> &noisemaker_cpu::DslCall {
    match &ast.chains[0].calls[0] {
        DslChainElement::Call(call) => call,
        DslChainElement::Subchain(_) => panic!("expected a top-level call"),
    }
}

#[test]
fn tokenizer_tracks_locations_comments_strings_and_numbers() {
    let tokens = tokenize_dsl(
        "  // first\n/* block */ let x = \"a\\nb\\t!\"; let y=.5e+2",
        "sample.dsl",
    )
    .unwrap();
    assert_eq!(tokens[0].lexeme, "let");
    assert_eq!(
        (tokens[0].line, tokens[0].column, tokens[0].index),
        (2, 13, 23)
    );
    let string = tokens
        .iter()
        .find(|token| token.kind == TokenKind::String)
        .unwrap();
    assert_eq!(string.lexeme, "\"a\\nb\\t!\"");
    assert_eq!(string.string_value.as_deref(), Some("a\nb\t!"));
    let number = tokens.iter().find(|token| token.lexeme == ".5e+2").unwrap();
    assert_eq!(number.number_value, Some(50.0));
    assert_eq!(tokens.last().unwrap().kind, TokenKind::Eof);
}

#[test]
fn tokenizer_failures_are_located_and_fail_closed() {
    for (source, message) in [
        ("@", "Unexpected character"),
        ("\"open", "Unterminated string"),
        ("/* open", "Unterminated block comment"),
        ("#12", "Colors must use"),
        ("1e+", "Invalid number"),
    ] {
        let error = tokenize_dsl(source, "bad.dsl").unwrap_err();
        assert_eq!(
            (
                error.source_name.as_str(),
                error.line,
                error.column,
                error.index
            ),
            ("bad.dsl", 1, 1, 0)
        );
        assert!(error.message.contains(message), "{source}: {error}");
        assert!(error.to_string().starts_with("bad.dsl:1:1:"));
    }
    assert!(
        tokenize_dsl("@", "bad.dsl")
            .unwrap_err()
            .message
            .contains("\"@\"")
    );
}

#[test]
fn parser_decodes_colors_and_composite_values() {
    let ast = parse_dsl(
        "search synth; solid(color:#f80).write(o0); render(o0)",
        "color.dsl",
    )
    .unwrap();
    let color = &first_call(&ast).args[0].value;
    let DslValue::Array { values, .. } = color else {
        panic!("expected color array")
    };
    assert_eq!(
        values,
        &[
            DslValue::Number(1.0),
            DslValue::Number(136.0 / 255.0),
            DslValue::Number(0.0),
        ]
    );
    for (literal, expected) in [
        (
            "#112233",
            vec![
                0x11 as f64 / 255.0,
                0x22 as f64 / 255.0,
                0x33 as f64 / 255.0,
            ],
        ),
        (
            "#11223344",
            vec![
                0x11 as f64 / 255.0,
                0x22 as f64 / 255.0,
                0x33 as f64 / 255.0,
                0x44 as f64 / 255.0,
            ],
        ),
    ] {
        let ast = parse_dsl(
            &format!("search synth solid(color:{literal}).write(o0)"),
            "color.dsl",
        )
        .unwrap();
        let DslValue::Array { values, .. } = &first_call(&ast).args[0].value else {
            panic!("expected color array")
        };
        assert_eq!(
            values,
            &expected
                .into_iter()
                .map(DslValue::Number)
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn compiler_normalizes_catalog_hex_color_defaults_like_supplied_colors() {
    let plan = compile_dsl(
        "search synth,filter; solid().text().write(o0)",
        "default-color.dsl",
    )
    .unwrap();
    let RenderStep::Effect { params, .. } = &plan.chains[0].steps[1] else {
        panic!("expected text effect")
    };
    assert_eq!(params["color"], ParamValue::Vector(vec![1.0, 1.0, 1.0]));
}

#[test]
fn parser_preserves_search_order_render_namespace_and_optional_semicolons() {
    let ast = parse_dsl(
        "search mixer, render, synth\nsolid().write(o0)\nrender(o0)",
        "order.dsl",
    )
    .unwrap();
    assert_eq!(ast.search, ["mixer", "render", "synth"]);
    assert_eq!(ast.render.as_ref().unwrap().name, "o0");
    let error = parse_dsl(
        "search synth; solid().write(o0); render(o0); render(o1)",
        "dup.dsl",
    )
    .unwrap_err();
    assert!(error.message.contains("only declare one render"));
    let late_search = parse_dsl("solid().write(o0); search synth", "late-search.dsl").unwrap_err();
    assert_eq!((late_search.line, late_search.column), (1, 20));
    assert!(
        late_search
            .message
            .contains("Expected effect or IO function name")
    );
}

#[test]
fn parser_rejects_mixed_arguments_at_the_conflicting_token() {
    let named_then_positional =
        parse_dsl("search synth; solid(color:#fff, .5).write(o0)", "mixed.dsl").unwrap_err();
    assert!(named_then_positional.message.contains("Cannot mix"));
    assert_eq!(named_then_positional.source_name, "mixed.dsl");
    let positional = parse_dsl("search synth; solid(#fff, .5).write(o0)", "ok.dsl").unwrap();
    assert_eq!(
        first_call(&positional).arg_mode.as_deref(),
        Some("positional")
    );
}

#[test]
fn parser_builds_precedence_unary_arrays_vectors_booleans_and_enum_paths() {
    let ast = parse_dsl(
        "search synth; let x = -1 + 2 * (3 + 4); test([true, \"x\"], vec3(1,2,3), Mode.multiply).write(o0)",
        "values.dsl",
    )
    .unwrap();
    assert!(
        matches!(ast.bindings[0].value, DslValue::Binary { ref operator, .. } if operator == "+")
    );
    let args = &first_call(&ast).args;
    assert!(matches!(args[0].value, DslValue::Array { .. }));
    assert!(matches!(args[1].value, DslValue::Vector { width: 3, .. }));
    assert!(
        matches!(args[2].value, DslValue::Identifier { ref name, .. } if name == "Mode.multiply")
    );
}

#[test]
fn read_value_forms_reduce_to_surface_and_bad_name_is_rejected() {
    for expression in ["o3", "(read(o3))", "(read(surface:o3))", "(read(tex:o3))"] {
        let ast = parse_dsl(
            &format!("search mixer; let x={expression}; read(o0).blendMode(tex:x).write(o1)"),
            "read.dsl",
        )
        .unwrap();
        assert!(
            matches!(ast.bindings[0].value, DslValue::Surface(ref surface) if surface.name == "o3")
        );
    }
    let error = parse_dsl(
        "search mixer; let x=(read(image:o3)); read(o0).blendMode(tex:x).write(o1)",
        "read.dsl",
    )
    .unwrap_err();
    assert!(error.message.contains("must be named"));
}

#[test]
fn surfaces_are_exactly_o0_through_o7() {
    for surface in ["o0", "o7"] {
        parse_dsl(
            &format!("search synth; solid().write({surface})"),
            "surface.dsl",
        )
        .unwrap();
    }
    for surface in ["o8", "o9", "o01", "o123"] {
        let error = parse_dsl(
            &format!("search synth; solid().write({surface})"),
            "surface.dsl",
        )
        .unwrap_err();
        assert!(
            error.message.contains("o0 through o7"),
            "{surface}: {error}"
        );
    }
}

fn effect_step(source: &str) -> RenderStep {
    compile_dsl(source, "compile.dsl")
        .unwrap()
        .chains
        .into_iter()
        .flat_map(|chain| chain.steps)
        .find(|step| matches!(step, RenderStep::Effect { .. }))
        .unwrap()
}

#[test]
fn compiler_requires_search_resolves_in_order_and_reports_unknown_effects() {
    let missing = compile_dsl("solid().write(o0)", "missing.dsl").unwrap_err();
    assert!(missing.message.contains("Missing required search"));
    let step = effect_step("search synth, classicNoisedeck; noise().write(o0)");
    assert!(matches!(step, RenderStep::Effect { ref effect_id, .. } if effect_id == "synth/noise"));
    let unknown =
        compile_dsl("search synth, filter; notReal().write(o0)", "unknown.dsl").unwrap_err();
    assert!(unknown.message.contains("notReal") && unknown.message.contains("synth, filter"));
}

#[test]
fn compiler_evaluates_values_and_partial_bindings_in_declaration_order() {
    let expanded = compile_dsl(
        "search synth; solid(color:#369, alpha:.5).write(o0)",
        "expanded.dsl",
    )
    .unwrap();
    let partial = compile_dsl(
        "search synth; let a=.25*2; let base=solid(color:#369); base(alpha:a).write(o0)",
        "partial.dsl",
    )
    .unwrap();
    let [
        RenderStep::Effect {
            params: expanded_params,
            ..
        },
        _,
    ] = &expanded.chains[0].steps[..]
    else {
        panic!("expanded shape")
    };
    let [
        RenderStep::Effect {
            params: partial_params,
            ..
        },
        _,
    ] = &partial.chains[0].steps[..]
    else {
        panic!("partial shape")
    };
    assert_eq!(expanded_params, partial_params);
    // Statement order (upstream 9cc041d0): a later `let` of the same name
    // reassigns it for later statements, so a duplicate name is not an error.
    let reassigned = compile_dsl(
        "search synth; let x=1; let x=2; solid(color:vec3(x,0,0)).write(o0)",
        "reassign.dsl",
    )
    .unwrap();
    let RenderStep::Effect { params, .. } = &reassigned.chains[0].steps[0] else {
        panic!("expected an effect step");
    };
    assert_eq!(params["color"], ParamValue::Vector(vec![2.0, 0.0, 0.0]));
    for source in [
        "search synth; let x=1; x().write(o0)",
        "search synth; let p=solid(); solid(color:p).write(o0)",
    ] {
        assert!(compile_dsl(source, "binding.dsl").is_err(), "{source}");
    }
    let forward = compile_dsl(
        "search synth; let first=later; let later=1; solid(color:first).write(o0)",
        "forward.dsl",
    );
    assert!(forward.is_err());
}

#[test]
fn compiler_merges_named_and_positional_partials_and_rejects_mode_changes() {
    let named =
        effect_step("search synth; let p=solid(color:#f00, alpha:.25); p(alpha:.75).write(o0)");
    assert!(
        matches!(named, RenderStep::Effect { ref params, ref explicit_params, .. }
        if params["alpha"] == ParamValue::Float(0.75)
        && explicit_params == &["color", "alpha"])
    );
    let positional = effect_step("search synth; let p=solid(#f00); p(.5).write(o0)");
    assert!(matches!(positional, RenderStep::Effect { ref params, .. }
        if params["color"] == ParamValue::Vector(vec![1.0,0.0,0.0])
        && params["alpha"] == ParamValue::Float(0.5)));
    assert!(
        compile_dsl(
            "search synth; let p=solid(color:#f00); p(.5).write(o0)",
            "mode.dsl"
        )
        .is_err()
    );
    let empty_stored =
        effect_step("search synth; let p=solid(); p(color:#0f0, alpha:.5).write(o0)");
    assert!(matches!(empty_stored, RenderStep::Effect { ref params, .. }
        if params["color"] == ParamValue::Vector(vec![0.0,1.0,0.0])
        && params["alpha"] == ParamValue::Float(0.5)));
    let empty_invocation =
        effect_step("search synth; let p=solid(color:#00f, alpha:.25); p().write(o0)");
    assert!(
        matches!(empty_invocation, RenderStep::Effect { ref params, .. }
        if params["color"] == ParamValue::Vector(vec![0.0,0.0,1.0])
        && params["alpha"] == ParamValue::Float(0.25))
    );
}

#[test]
fn compiler_uses_canonical_positional_order_and_rejects_aliases() {
    let solid = effect_step("search synth; solid(#123456, .5).write(o0)");
    assert!(matches!(solid, RenderStep::Effect { ref params, .. }
        if params["color"] == ParamValue::Vector(vec![0x12 as f32/255.0,0x34 as f32/255.0,0x56 as f32/255.0])
        && params["alpha"] == ParamValue::Float(0.5)));
    let noise = effect_step("search synth; noise(scaleX:20, scaleY:30, type:linear).write(o0)");
    assert!(
        matches!(noise, RenderStep::Effect { ref params, ref explicit_params, .. }
        if params["scaleX"] == ParamValue::Float(20.0)
        && params["scaleY"] == ParamValue::Float(30.0)
        && params["type"] == ParamValue::Int(1)
        && explicit_params == &["scaleX", "scaleY", "type"])
    );
    let osc = effect_step("search synth; osc2d(freq:25).write(o0)");
    assert!(matches!(osc, RenderStep::Effect { ref params, .. }
        if params["freq"] == ParamValue::Int(25)));
    let error = compile_dsl(
        "search synth; noise(noiseType:linear).write(o0)",
        "retired-alias.dsl",
    )
    .unwrap_err();
    assert!(
        error.message.contains("Unknown parameter \"noiseType\""),
        "unexpected error: {error}"
    );
}

#[test]
fn compiler_strictly_coerces_enums_bools_colors_vectors_ranges_and_finiteness() {
    let valid =
        effect_step("search synth; noise(type:Noise.linear, ridges:true, scaleX:25).write(o0)");
    assert!(matches!(valid, RenderStep::Effect { ref params, .. }
        if params["type"] == ParamValue::Int(1)
        && params["ridges"] == ParamValue::Bool(true)));
    for source in [
        "search synth; noise(scaleX:0).write(o0)",
        "search synth; noise(octaves:1.5).write(o0)",
        "search synth; solid(color:vec2(1,0)).write(o0)",
        "search synth; noise(scaleX:1/0).write(o0)",
        "search synth; solid(alpha:2).write(o0)",
        "search synth; solid(nope:1).write(o0)",
        "search synth; solid(#fff,1,2).write(o0)",
    ] {
        assert!(compile_dsl(source, "strict.dsl").is_err(), "{source}");
    }
}

#[test]
fn compiler_rejects_finite_f64_that_overflows_the_f32_parameter_boundary() {
    let error = compile_dsl(
        "search filter; read(o0).palette(rotation:1e100).write(o1)",
        "f32-overflow.dsl",
    )
    .unwrap_err();
    assert!(error.message.contains("representable as a finite f32"));
}

#[test]
fn compiler_separates_surface_edges_and_accepts_surface_value_forms() {
    for value in ["o1", "read(o1)"] {
        let step = effect_step(&format!(
            "search mixer; read(o0).blendMode(tex:{value}, mode:multiply).write(o2)"
        ));
        assert!(
            matches!(step, RenderStep::Effect { ref surface_params, ref params, .. }
            if surface_params["tex"] == SurfaceBinding::Surface("o1".into())
            && !params.contains_key("tex"))
        );
    }
    let none = effect_step("search mixer; read(o0).blendMode(tex:none).write(o2)");
    assert!(
        matches!(none, RenderStep::Effect { ref surface_params, .. } if surface_params.is_empty())
    );
    let current = effect_step("search mixer; read(o0).blendMode(tex:inputTex).write(o2)");
    assert!(
        matches!(current, RenderStep::Effect { ref surface_params, .. }
        if surface_params["tex"] == SurfaceBinding::Current)
    );
}

#[test]
fn compiler_enforces_image_chain_and_render_target_rules() {
    compile_dsl(
        "search synth,filter; solid().invert().write(o0)",
        "image.dsl",
    )
    .unwrap();
    for source in [
        "search synth; solid()",
        "search filter; invert().write(o0)",
        "search synth; read(o0).solid().write(o1)",
        "search synth; write(o0)",
        "search synth; solid(); render(o0)",
    ] {
        assert!(compile_dsl(source, "chain.dsl").is_err(), "{source}");
    }
    let plan = compile_dsl(
        "search synth; solid(color:#f00).write(o0); solid(color:#0f0).write(o1)",
        "render.dsl",
    )
    .unwrap();
    assert_eq!(plan.render_surface, "o1");
    let explicit = compile_dsl(
        "search synth; solid().write(o0); solid().write(o1); render(o0)",
        "render.dsl",
    )
    .unwrap();
    assert_eq!(explicit.render_surface, "o0");
    assert!(compile_dsl("search synth", "empty.dsl").is_err());
    let unwritten = compile_dsl("search synth; render(o7)", "unwritten.dsl").unwrap();
    assert_eq!(unwritten.render_surface, "o7");
}

#[test]
fn compiler_allows_surface_only_mixer_to_begin_a_chain() {
    let plan = compile_dsl(
        "search mixer; channelCombine(rTex:o0, gTex:o1, bTex:o2).write(o3)",
        "surface-only.dsl",
    )
    .unwrap();
    let RenderStep::Effect { surface_params, .. } = &plan.chains[0].steps[0] else {
        panic!("expected surface-only mixer")
    };
    assert_eq!(surface_params.len(), 3);
    assert_eq!(surface_params["rTex"], SurfaceBinding::Surface("o0".into()));
    let unbound = compile_dsl(
        "search mixer; channelCombine().write(o3)",
        "surface-only.dsl",
    )
    .unwrap_err();
    assert!(
        unbound
            .message
            .contains("requires at least one surface input")
    );
    // External-input mesh effects begin a chain with no surface binding: their
    // pass inputs are host-fed data textures, not surface parameters.
    compile_dsl("search render; meshLoader().write(o0)", "mesh-loader.dsl").unwrap();
}

#[test]
fn compiler_validates_volume_and_loop_domains_without_executing_them() {
    compile_dsl(
        "search synth3d,filter3d,render; noise3d().flow3d().render3d().write(o0)",
        "volume.dsl",
    )
    .unwrap();
    compile_dsl(
        "search synth,render; solid().loopBegin().loopEnd().write(o0)",
        "loop.dsl",
    )
    .unwrap();
    for source in [
        "search filter3d,render; flow3d().render3d().write(o0)",
        "search synth,render; solid().render3d().write(o0)",
        "search synth3d; noise3d().write(o0)",
        "search synth,render; solid().loopEnd().write(o0)",
        "search synth,render; solid().loopBegin().loopBegin().loopEnd().write(o0)",
        "search synth,render; solid().loopBegin().write(o0)",
    ] {
        assert!(compile_dsl(source, "domain.dsl").is_err(), "{source}");
    }
}

// --- GAP-038: float parameter choice names -----------------------------------

// Each program sets `rotation` on one of the five effects with inline float
// choices (none: 0, fwd: 1, back: -1). `%` is replaced with the rotation value.
const FLOAT_CHOICE_PROGRAMS: [(&str, &str); 5] = [
    (
        "filter/palette",
        "search filter, synth\nnoise(seed: 1, ridges: true).palette(rotation: %).write(o0)\nrender(o0)\n",
    ),
    (
        "filter/historicPalette",
        "search filter, synth\nnoise(seed: 1, ridges: true, colorMode: mono).historicPalette(rotation: %).write(o0)\nrender(o0)\n",
    ),
    (
        "filter/tetraColorArray",
        "search filter, synth\nnoise(seed: 1).tetraColorArray(smoothness: 0, rotation: %).write(o0)\nrender(o0)\n",
    ),
    (
        "filter/tetraCosine",
        "search filter, synth\nnoise(seed: 1).tetraCosine(rotation: %).write(o0)\nrender(o0)\n",
    ),
    (
        "filter3d/palette3d",
        "search synth3d, filter3d, render\nnoise3d(seed: 3, volumeSize: 8).palette3d(rotation: %).render3d().write(o0)\nrender(o0)\n",
    ),
];

#[test]
fn float_choice_names_compile_to_their_numeric_choice_values() {
    let catalog = effect_catalog().unwrap();
    let mut float_choice_params = Vec::new();
    for (effect_id, definition) in &catalog.effects {
        for (name, spec) in &definition.params {
            if spec.parameter_type == "float" && spec.metadata.contains_key("choices") {
                float_choice_params.push((effect_id.clone(), name.clone(), spec));
            }
        }
    }
    assert_eq!(float_choice_params.len(), 5);
    for (effect_id, name, spec) in &float_choice_params {
        assert_eq!(name, "rotation");
        let choices = spec.metadata["choices"].as_object().unwrap();
        assert_eq!(
            choices
                .iter()
                .map(|(key, value)| (key.as_str(), value.as_f64()))
                .collect::<BTreeMap<_, _>>(),
            BTreeMap::from([
                ("none", Some(0.0)),
                ("fwd", Some(1.0)),
                ("back", Some(-1.0))
            ])
        );
        let program = FLOAT_CHOICE_PROGRAMS
            .iter()
            .find(|(id, _)| id == effect_id)
            .unwrap()
            .1;
        for (choice, value) in choices {
            if value.is_null() {
                continue;
            }
            let plan = compile_dsl(&program.replace('%', choice), "choices.dsl").unwrap();
            let RenderStep::Effect { params, .. } = &plan.chains[0].steps[1] else {
                panic!("expected the {} effect step", effect_id);
            };
            let ParamValue::Float(resolved) = params[name] else {
                panic!("expected a float param for {effect_id}");
            };
            assert_eq!(
                f64::from(resolved),
                value.as_f64().unwrap(),
                "{effect_id}: {choice}"
            );
        }
    }
}

#[test]
fn a_name_that_is_not_a_float_choice_fails_and_names_the_parameter() {
    for (_, program) in FLOAT_CHOICE_PROGRAMS {
        let error = compile_dsl(&program.replace('%', "wobble"), "choices.dsl").unwrap_err();
        assert!(error.message.contains("rotation"), "{}", error.message);
    }
}

// --- GAP-044: midi() and audio() real-time input -------------------------------

// MIDI and audio input are unsupported in CPU ports. Their value calls warn and
// resolve to nothing: the consumer parameter keeps its declared offline default,
// and no live automation value reaches the renderer.
#[test]
fn midi_and_audio_calls_compile_but_leave_the_offline_parameter_default() {
    let baseline = compile_dsl(
        "search synth\nnoise(seed: 1).write(o0)\nrender(o0)\n",
        "baseline.dsl",
    )
    .unwrap();
    let RenderStep::Effect {
        params: baseline_params,
        ..
    } = &baseline.chains[0].steps[0]
    else {
        panic!("expected an effect step");
    };
    for call in [
        "midi(channel: 1, min: 0.8)",
        "audio(band: audioBand.low, min: 0.8)",
    ] {
        for source in [
            format!("search synth\nnoise(seed: 1, scaleX: {call}).write(o0)\nrender(o0)\n"),
            format!("search synth\nlet rate = {call}\nnoise(scaleX: rate).write(o0)\nrender(o0)\n"),
        ] {
            let plan = compile_dsl(&source, "midi.dsl").unwrap();
            let RenderStep::Effect { params, .. } = &plan.chains[0].steps[0] else {
                panic!("expected an effect step for {call}");
            };
            assert_eq!(params["scaleX"], baseline_params["scaleX"], "{call}");
            assert!(
                !matches!(params["scaleX"], ParamValue::Automation(_)),
                "{call} must not create live automation"
            );
        }
    }
}

// An audio() field of an osc() warns and takes the field's fallback, so the
// oscillator itself still compiles to a live automation value.
#[test]
fn unsupported_input_inside_oscillator_fields_takes_the_field_fallback() {
    let plan = compile_dsl(
        "search synth\nnoise(scaleX: osc(type: sine, speed: audio(band: low))).write(o0)\nrender(o0)\n",
        "osc-field.dsl",
    )
    .unwrap();
    let RenderStep::Effect { params, .. } = &plan.chains[0].steps[0] else {
        panic!("expected an effect step");
    };
    let ParamValue::Automation(noisemaker_cpu::AutomationValue::Oscillator {
        osc_type, speed, ..
    }) = &params["scaleX"]
    else {
        panic!("expected an oscillator automation value");
    };
    assert_eq!(*osc_type, 0);
    assert_eq!(speed, &AutomationField::Number(1.0));
}

// --- upstream 9cc041d0: statement-order lets ----------------------------------

// A chain sees only the lets declared at or before its own statement index, a
// later let of the same name reassigns it for later statements, and reading a
// pending name reports S003. Lets may still refer to each other in any order.
#[test]
fn a_chain_uses_the_latest_let_declared_before_it_and_reassignment_is_allowed() {
    let plan = compile_dsl(
        "search synth\nlet x = 10\nnoise(scaleX: x).write(o0)\nlet x = 90\nnoise(scaleX: x).write(o1)\n",
        "let-order.dsl",
    )
    .unwrap();
    let RenderStep::Effect { params: first, .. } = &plan.chains[0].steps[0] else {
        panic!("expected an effect step");
    };
    let RenderStep::Effect { params: second, .. } = &plan.chains[1].steps[0] else {
        panic!("expected an effect step");
    };
    assert_eq!(first["scaleX"], ParamValue::Float(10.0));
    assert_eq!(second["scaleX"], ParamValue::Float(90.0));
}

#[test]
fn a_chain_reading_a_let_declared_by_a_later_statement_reports_s003() {
    let error = compile_dsl(
        "search synth\nnoise(scaleX: x).write(o0)\nlet x = 20\n",
        "let-order.dsl",
    )
    .unwrap_err();
    assert!(
        error
            .message
            .contains("Variable used before assignment: 'x'")
    );
}

#[test]
fn a_let_may_reference_a_later_let_inside_its_value_expression() {
    let plan = compile_dsl(
        "search synth\nlet a = b + 1\nlet b = 5\nnoise(scaleX: a).write(o0)\n",
        "let-order.dsl",
    )
    .unwrap();
    let RenderStep::Effect { params, .. } = &plan.chains[0].steps[0] else {
        panic!("expected an effect step");
    };
    assert_eq!(params["scaleX"], ParamValue::Float(6.0));
}

#[test]
fn a_bare_identifier_naming_a_later_let_reports_s003_at_declaration() {
    let error = compile_dsl(
        "search synth\nlet a = b\nlet b = 5\nnoise(scaleX: a).write(o0)\n",
        "let-order.dsl",
    )
    .unwrap_err();
    assert!(
        error
            .message
            .contains("Variable used before assignment: 'b'")
    );
}

#[test]
fn a_value_call_may_reference_a_later_let_in_its_arguments() {
    let plan = compile_dsl(
        "search synth\nlet outer = osc(type: oscKind.sine, speed: inner)\nlet inner = osc(type: oscKind.tri, speed: 2)\nnoise(scaleX: outer).write(o0)\n",
        "let-order.dsl",
    )
    .unwrap();
    let RenderStep::Effect { params, .. } = &plan.chains[0].steps[0] else {
        panic!("expected an effect step");
    };
    let ParamValue::Automation(noisemaker_cpu::AutomationValue::Oscillator {
        osc_type, speed, ..
    }) = &params["scaleX"]
    else {
        panic!("expected an oscillator automation value");
    };
    assert_eq!(*osc_type, 0);
    let AutomationField::Ref(nested) = speed else {
        panic!("expected a nested oscillator in the speed field");
    };
    let noisemaker_cpu::AutomationValue::Oscillator {
        osc_type: nested_type,
        speed: nested_speed,
        ..
    } = nested.as_ref();
    assert_eq!(*nested_type, 1);
    assert_eq!(nested_speed, &AutomationField::Number(2.0));
}

#[test]
fn a_cyclic_let_set_fails_with_a_cycle_report() {
    let error = compile_dsl(
        "search synth\nlet a = b + 1\nlet b = a + 1\nnoise(scaleX: a).write(o0)\n",
        "let-order.dsl",
    )
    .unwrap_err();
    assert!(
        error
            .message
            .contains("Automation cycle detected: a -> b -> a")
    );
}

#[test]
fn a_let_whose_bare_identifier_names_nothing_resolvable_reports_s003_at_declaration() {
    let error = compile_dsl(
        "search synth\nlet y = x\nnoise().write(o0)\n",
        "let-order.dsl",
    )
    .unwrap_err();
    assert!(
        error
            .message
            .contains("Variable used before assignment: 'x'")
    );
}

// --- GAP-045: subchain blocks -------------------------------------------------

fn step_ids(plan: &noisemaker_cpu::RenderPlan) -> Vec<String> {
    plan.chains
        .iter()
        .flat_map(|chain| &chain.steps)
        .map(|step| match step {
            RenderStep::Read { .. } => "read".into(),
            RenderStep::Write { .. } => "write".into(),
            RenderStep::Read3d { .. } => "read3d".into(),
            RenderStep::Write3d { .. } => "write3d".into(),
            RenderStep::Effect { effect_id, .. } => effect_id.clone(),
        })
        .collect()
}

#[test]
fn a_subchain_block_compiles_to_the_steps_of_its_flattened_chain() {
    let flat = "search synth, filter\nnoise(seed: 1).invert().posterize(levels: 4).write(o0)\nrender(o0)\n";
    // Each accepted program renders the same bytes as FLAT (renderer test).
    let accepted = [
        "search synth, filter\nnoise(seed: 1).subchain(name: \"look\") { .invert() .posterize(levels: 4) }.write(o0)\nrender(o0)\n",
        "search synth, filter\nnoise(seed: 1).subchain(\"look\") { .invert() .posterize(levels: 4) }.write(o0)\nrender(o0)\n",
        "search synth, filter\nnoise(seed: 1).subchain(id: \"sc1\") { .invert() .posterize(levels: 4) }.write(o0)\nrender(o0)\n",
        "search synth, filter\nnoise(seed: 1).subchain(name: \"look\", id: \"sc1\") { .invert() .posterize(levels: 4) }.write(o0)\nrender(o0)\n",
        "search synth, filter\nnoise(seed: 1).subchain() { .invert() .posterize(levels: 4) }.write(o0)\nrender(o0)\n",
        // Two blocks in one chain, and a block between ordinary elements.
        "search synth, filter\nnoise(seed: 1).subchain(name: \"a\") { .invert() }.subchain(name: \"b\") { .posterize(levels: 4) }.write(o0)\nrender(o0)\n",
        "search synth, filter\nnoise(seed: 1).invert().subchain() { .posterize(levels: 4) }.write(o0)\nrender(o0)\n",
        // The text that the pinned authority unparser writes for the first program.
        "search synth, filter\n\nnoise()\n  .subchain(name: \"look\") {\n    .invert()\n    .posterize(levels: 4)\n  }\n  .write(o0)\n\nrender(o0)",
        // Comments between body elements.
        "search synth, filter\nnoise(seed: 1)\n  .subchain(name: \"look\") {\n    // grade\n    .invert()\n    /* steps */ .posterize(levels: 4)\n  }\n  .write(o0)\nrender(o0)\n",
        // Upstream reports these subchain arguments as warnings (P008, P009, P010)
        // and accepts the program: an unknown key, a duplicate key, a missing
        // comma and a trailing comma.
        "search synth, filter\nnoise(seed: 1).subchain(name: \"look\", color: \"red\") { .invert() .posterize(levels: 4) }.write(o0)\nrender(o0)\n",
        "search synth, filter\nnoise(seed: 1).subchain(name: \"a\", name: \"b\") { .invert() .posterize(levels: 4) }.write(o0)\nrender(o0)\n",
        "search synth, filter\nnoise(seed: 1).subchain(name: \"a\" id: \"b\") { .invert() .posterize(levels: 4) }.write(o0)\nrender(o0)\n",
        "search synth, filter\nnoise(seed: 1).subchain(name: \"a\",) { .invert() .posterize(levels: 4) }.write(o0)\nrender(o0)\n",
    ];
    let expected = step_ids(&compile_dsl(flat, "flat.dsl").unwrap());
    for source in accepted {
        assert_eq!(
            step_ids(&compile_dsl(source, "subchain.dsl").unwrap()),
            expected,
            "{source}"
        );
    }
}

#[test]
fn a_subchain_that_the_authority_rejects_still_fails() {
    for (source, expected) in [
        (
            "search synth, filter\nnoise(seed: 1).subchain(name: \"x\") { }.write(o0)\nrender(o0)\n",
            "Expected \".\"",
        ),
        (
            "search synth, filter\nsubchain(name: \"x\") { .invert() }.write(o0)\nrender(o0)\n",
            "Expected effect or IO function name",
        ),
        (
            "search synth, filter\nnoise(seed: 1).subchain() { .noise() }.write(o0)\nrender(o0)\n",
            "Generator synth/noise must begin a chain",
        ),
        (
            "search synth, filter\nnoise(seed: 1).write(o1)\nread(o1).subchain() { .read(o1) }.write(o0)\nrender(o0)\n",
            "read(surface) must begin a chain",
        ),
        (
            "search synth, filter\nnoise(seed: 1).subchain() { .invert().write(o1) }.write(o0)\nrender(o0)\n",
            "Expected effect or IO function name",
        ),
        (
            "search synth, filter\nnoise(seed: 1).subchain() { .write(o1) }.write(o0)\nrender(o0)\n",
            "Expected effect or IO function name",
        ),
        (
            "search synth, filter\nnoise(seed: 1).subchain() { .subchain() { .invert() } }.write(o0)\nrender(o0)\n",
            "Expected effect or IO function name",
        ),
        (
            "search synth, filter\nnoise(seed: 1).subchain() { invert() }.write(o0)\nrender(o0)\n",
            "Expected \".\"",
        ),
        (
            "search synth, filter\nnoise(seed: 1).subchain(name: 5) { .invert() }.write(o0)\nrender(o0)\n",
            "Expected \")\"",
        ),
        (
            "search synth, filter\nnoise(seed: 1).subchain(\"look\", id: \"sc1\") { .invert() }.write(o0)\nrender(o0)\n",
            "Expected \")\"",
        ),
        (
            "search synth, filter\nnoise(seed: 1).subchain(look) { .invert() }.write(o0)\nrender(o0)\n",
            "Expected \")\"",
        ),
        (
            "search synth, filter\nnoise(seed: 1).subchain() .invert().write(o0)\nrender(o0)\n",
            "Expected \"{\"",
        ),
        (
            "search synth, filter\nnoise(seed: 1).subchain() { .invert()\n",
            "Expected \".\"",
        ),
    ] {
        let error = compile_dsl(source, "subchain.dsl").unwrap_err();
        assert!(
            error.message.contains(expected),
            "{source}: {}",
            error.message
        );
    }
}

// --- GAP-047: grammar forms ---------------------------------------------------

#[test]
fn write3d_and_read3d_store_and_restore_the_volume_and_geometry_surfaces() {
    let source = "search synth3d, render\nnoise3d(seed: 1, volumeSize: 8).write3d(vol0, geo0)\nread3d(vol0, geo0).render3d().write(o0)\nrender(o0)\n";
    let plan = compile_dsl(source, "volume-io.dsl").unwrap();
    assert_eq!(
        step_ids(&plan),
        vec![
            "synth3d/noise3d".to_string(),
            "write3d".to_string(),
            "read3d".to_string(),
            "render/render3d".to_string(),
            "write".to_string(),
        ]
    );
    let RenderStep::Write3d {
        volume, geometry, ..
    } = &plan.chains[0].steps[1]
    else {
        panic!("expected a write3d step");
    };
    assert_eq!((volume.as_str(), geometry.as_str()), ("vol0", "geo0"));
    // A one-argument read3d(vol) names a volume (or geometry) for a parameter.
    let named = |source: &str| {
        let plan = compile_dsl(source, "read3d.dsl").unwrap();
        let RenderStep::Effect { params, .. } = &plan.chains[0].steps[0] else {
            panic!("expected an effect step");
        };
        params.clone()
    };
    let value_form = named(
        "search synth3d, render\ncellularAutomata3d(source: read3d(vol1), geoSource: read3d(geo1), volumeSize: x16).render3d().write(o0)\nrender(o0)\n",
    );
    let plain = named(
        "search synth3d, render\ncellularAutomata3d(source: vol1, geoSource: geo1, volumeSize: x16).render3d().write(o0)\nrender(o0)\n",
    );
    assert_eq!(value_form["source"], plain["source"]);
    assert_eq!(value_form["geoSource"], plain["geoSource"]);
    assert_eq!(value_form["source"], ParamValue::String("vol1".into()));
    assert_eq!(value_form["geoSource"], ParamValue::String("geo1".into()));
}

#[test]
fn a_triple_quoted_string_keeps_its_raw_content_newlines_and_quotes_included() {
    let text = |literal: &str| {
        let source = format!(
            "search synth, filter\nsolid(color: #000).text(text: {literal}).write(o0)\nrender(o0)\n"
        );
        let plan = compile_dsl(&source, "text.dsl").unwrap();
        let RenderStep::Effect { params, .. } = &plan.chains[0].steps[1] else {
            panic!("expected the text effect step");
        };
        let ParamValue::String(value) = &params["text"] else {
            panic!("expected a string param");
        };
        value.clone()
    };
    assert_eq!(text("\"\"\"hi\"\"\""), "hi");
    assert_eq!(text("\"\"\"Hello\nWorld\"\"\""), "Hello\nWorld");
    assert_eq!(text("\"\"\"say \"hi\" now\"\"\""), "say \"hi\" now");
    assert_eq!(text("\"\"\"\"\"\""), "");
    // Upstream's lexer keeps a triple-quoted string's content raw.
    assert_eq!(text("\"\"\"a\\tb\"\"\""), "a\\tb");
    // A string whose content is punctuation is a string, in the first position too.
    let punctuation = |literal: &str| {
        let source = format!(
            "search synth, filter\nsolid(color: #000).text({literal}).write(o0)\nrender(o0)\n"
        );
        let plan = compile_dsl(&source, "text.dsl").unwrap();
        let RenderStep::Effect { params, .. } = &plan.chains[0].steps[1] else {
            panic!("expected the text effect step");
        };
        let ParamValue::String(value) = &params["text"] else {
            panic!("expected a string param");
        };
        value.clone()
    };
    assert_eq!(punctuation("\")\""), ")");
    assert_eq!(punctuation("\"\"\")\"\"\""), ")");
}

#[test]
fn forms_that_the_authority_rejects_still_fail() {
    for (source, expected) in [
        (
            "search synth\nnoise(seed: 1, scaleX: Math.E).write(o0)\nrender(o0)\n",
            "Parameter \"scaleX\" must be a finite number",
        ),
        (
            "search synth\nnoise(seed: 1, scaleX: Math.PI()).write(o0)\nrender(o0)\n",
            "Expected \")\"",
        ),
        (
            "search synth, filter\nsolid(color: #000).text(text: \"\"\"open\n",
            "Unterminated string",
        ),
        (
            "search synth\nlet pattern = wobble\npattern().write(o0)\nrender(o0)\n",
            "Variable used before assignment: 'wobble'",
        ),
        (
            "search synth\nlet pattern = noise\nnoise(seed: 1, scaleX: pattern).write(o0)\nrender(o0)\n",
            "Parameter \"scaleX\" must be a finite number",
        ),
        (
            "search synth3d, render\nnoise3d(seed: 1, volumeSize: x16).write3d(vol0)\nrender(o0)\n",
            "Expected \",\"",
        ),
        (
            "search synth3d, render\nnoise3d(seed: 1, volumeSize: x16).write3d(tex3d: vol0, geo: geo0)\nrender(o0)\n",
            "Expected \",\"",
        ),
        (
            "search synth3d, render\nnoise3d(seed: 1, volumeSize: x16).write3d(5, geo0)\nrender(o0)\n",
            "Expected surface reference",
        ),
        (
            "search synth3d, render\nnoise3d(seed: 1, volumeSize: x16).write3d(vol0, geo0)\nnoise3d(seed: 2).read3d(vol0, geo0).render3d().write(o0)\nrender(o0)\n",
            "Unknown effect \"read3d\" in search namespaces synth3d, render",
        ),
        (
            "search synth3d, render\nwrite3d(vol0, geo0)\nrender(o0)\n",
            "Expected effect or IO function name",
        ),
        (
            "search synth3d, render\nnoise3d(seed: 1, volumeSize: x16).write3d(vol0, geo0)\nread3d(vol0).render3d().write(o0)\nrender(o0)\n",
            "Unknown effect \"read3d\"",
        ),
        (
            "search synth, filter\nfrom(ns: classicNoisedeck, call: noise(seed: 3)).write(o0)\nrender(o0)\n",
            "Unknown effect \"from\" in search namespaces synth, filter",
        ),
        (
            "search synth, filter\nfrom(classicNoisedeck).write(o0)\nrender(o0)\n",
            "Unknown effect \"from\" in search namespaces synth, filter",
        ),
        (
            "search synth, filter\nfrom(\"classicNoisedeck\", noise(seed: 3)).write(o0)\nrender(o0)\n",
            "Unknown effect \"from\" in search namespaces synth, filter",
        ),
        (
            "search synth, filter\nfrom(classicNoisedeck, noise).write(o0)\nrender(o0)\n",
            "Unknown effect \"from\" in search namespaces synth, filter",
        ),
        (
            "search synth, filter\nfrom(classicNoisedeck, noise(seed: 3).invert()).write(o0)\nrender(o0)\n",
            "Unknown effect \"from\" in search namespaces synth, filter",
        ),
        (
            "search synth, filter\nfrom(filter, noise(seed: 3)).write(o0)\nrender(o0)\n",
            "Unknown effect \"noise\" in search namespaces filter",
        ),
    ] {
        let error = compile_dsl(source, "forms.dsl").unwrap_err();
        assert!(
            error.message.contains(expected),
            "{source}: {}",
            error.message
        );
    }
}

// --- GAP-046: chain arguments -------------------------------------------------

#[test]
fn a_chain_argument_that_the_authority_rejects_still_fails() {
    for (source, expected) in [
        // write() is only allowed in statement context.
        (
            "search synth, mixer\nnoise(seed: 1).blendMode(tex: solid(color: #ff0000).write(o1), mode: screen).write(o0)\nrender(o0)\n",
            "Expected effect or IO function name",
        ),
        // A chain argument must start with a generator or read().
        (
            "search filter, mixer, synth\nnoise(seed: 1).blendMode(tex: invert(), mode: screen).write(o0)\nrender(o0)\n",
            "filter/invert requires an input",
        ),
        // An effect that the search order does not resolve.
        (
            "search synth, mixer\nnoise(seed: 1).blendMode(tex: wobble(), mode: screen).write(o0)\nrender(o0)\n",
            "Unknown effect \"wobble\"",
        ),
    ] {
        let error = compile_dsl(source, "inline.dsl").unwrap_err();
        assert!(
            error.message.contains(expected),
            "{source}: {}",
            error.message
        );
    }
}

#[test]
fn a_chain_argument_compiles_to_a_hidden_inline_chain_on_the_consuming_step() {
    let source = "search synth, mixer\nnoise(seed: 1).blendMode(tex: solid(color: #ff0000), mode: screen).write(o0)\nrender(o0)\n";
    let plan = compile_dsl(source, "inline.dsl").unwrap();
    let RenderStep::Effect { inline_chains, .. } = &plan.chains[0].steps[1] else {
        panic!("expected an effect step");
    };
    assert_eq!(inline_chains.len(), 1);
    assert_eq!(inline_chains[0].surface, "inline:0");
    assert_eq!(
        step_ids(&noisemaker_cpu::RenderPlan {
            search: Vec::new(),
            chains: vec![inline_chains[0].chain.clone()],
            render_surface: "inline:0".into(),
            ast: plan.ast.clone(),
        }),
        vec!["synth/solid".to_string()]
    );
}

#[test]
fn math_pi_evaluates_to_the_same_number_as_the_authority_parser_folds() {
    let plan = compile_dsl(
        "search synth\nnoise(seed: 1, scaleX: Math.PI * 4).write(o0)\nrender(o0)\n",
        "pi.dsl",
    )
    .unwrap();
    let RenderStep::Effect { params, .. } = &plan.chains[0].steps[0] else {
        panic!("expected an effect step");
    };
    let ParamValue::Float(scale_x) = params["scaleX"] else {
        panic!("expected a float param");
    };
    // Step params are stored as f32 uniform values in this port; the authority
    // folds Math.PI * 4 in f64 and the uniform upload rounds it to the same f32.
    assert_eq!(
        f64::from(scale_x),
        f64::from((std::f64::consts::PI * 4.0) as f32)
    );
}
