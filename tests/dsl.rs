use noisemaker_cpu::{
    DslValue, ParamValue, RenderStep, SurfaceBinding, TokenKind, compile_dsl, parse_dsl,
    tokenize_dsl,
};

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
    let color = &ast.chains[0].calls[0].args[0].value;
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
        let DslValue::Array { values, .. } = &ast.chains[0].calls[0].args[0].value else {
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
        positional.chains[0].calls[0].arg_mode.as_deref(),
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
    let args = &ast.chains[0].calls[0].args;
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
    for source in [
        "search synth; let x=1; let x=2; solid().write(o0)",
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
fn compiler_uses_canonical_positional_order_and_locked_aliases() {
    let solid = effect_step("search synth; solid(#123456, .5).write(o0)");
    assert!(matches!(solid, RenderStep::Effect { ref params, .. }
        if params["color"] == ParamValue::Vector(vec![0x12 as f32/255.0,0x34 as f32/255.0,0x56 as f32/255.0])
        && params["alpha"] == ParamValue::Float(0.5)));
    let noise =
        effect_step("search synth; noise(xScale:20, yScale:30, noiseType:linear).write(o0)");
    assert!(
        matches!(noise, RenderStep::Effect { ref params, ref explicit_params, .. }
        if params["scaleX"] == ParamValue::Float(20.0)
        && params["scaleY"] == ParamValue::Float(30.0)
        && params["type"] == ParamValue::Int(1)
        && explicit_params == &["scaleX", "scaleY", "type"])
    );
    let osc = effect_step("search synth; osc2d(frequency:25).write(o0)");
    assert!(matches!(osc, RenderStep::Effect { ref params, .. }
        if params["freq"] == ParamValue::Int(25)));
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
