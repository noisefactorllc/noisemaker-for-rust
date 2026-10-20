use noisemaker_cpu::{CpuRenderer, RenderError, RenderOptions};

fn options() -> RenderOptions {
    RenderOptions {
        width: 8,
        height: 8,
        time: 0.25,
        seed: 1,
        ..RenderOptions::default()
    }
}

#[test]
fn renderer_renders_solid_and_generator_filter_literal_pixels() {
    let mut renderer = CpuRenderer::new().unwrap();
    let solid = renderer
        .render(
            "search synth; solid(color:#336699, alpha:.5).write(o0); render(o0)",
            &options(),
        )
        .unwrap();
    assert_eq!(&solid.surface.to_rgba8()[..4], &[25, 51, 76, 128]);
    let inverted = renderer
        .render(
            "search synth,filter; solid(color:#336699).invert().write(o0)",
            &options(),
        )
        .unwrap();
    assert_eq!(&inverted.surface.to_rgba8()[..4], &[204, 153, 102, 255]);
}

#[test]
fn renderer_value_and_partial_bindings_equal_expanded_program() {
    let mut renderer = CpuRenderer::new().unwrap();
    let expanded = renderer
        .render(
            "search synth; solid(color:#369, alpha:.5).write(o0)",
            &options(),
        )
        .unwrap();
    let bound = renderer
        .render(
            "search synth; let alpha=.25*2; let base=solid(color:#369); base(alpha:alpha).write(o0)",
            &options(),
        )
        .unwrap();
    assert_eq!(expanded.surface, bound.surface);
}

#[test]
fn renderer_wires_mixer_surface_parameters() {
    let mut renderer = CpuRenderer::new().unwrap();
    let result = renderer
        .render(
            "search synth,mixer; solid(color:#f00).write(o0); solid(color:#0f0).write(o1); read(o0).blendMode(tex:o1, mode:multiply, mix:100).write(o2); render(o2)",
            &options(),
        )
        .unwrap();
    assert_eq!((result.surface.width(), result.surface.height()), (8, 8));
    assert!(result.surface.data().iter().all(|value| value.is_finite()));
    assert_ne!(&result.surface.to_rgba8()[..4], &[255, 0, 0, 255]);
}

#[test]
fn renderer_honors_default_last_write_and_explicit_earlier_target() {
    let source = "search synth; solid(color:#f00).write(o0); solid(color:#0f0).write(o1)";
    let mut renderer = CpuRenderer::new().unwrap();
    let defaulted = renderer.render(source, &options()).unwrap();
    assert_eq!(&defaulted.surface.to_rgba8()[..4], &[0, 255, 0, 255]);
    let explicit = renderer
        .render(&format!("{source}; render(o0)"), &options())
        .unwrap();
    assert_eq!(&explicit.surface.to_rgba8()[..4], &[255, 0, 0, 255]);
}

#[test]
fn renderer_returns_typed_unwritten_and_missing_mixer_surface_errors() {
    let mut renderer = CpuRenderer::new().unwrap();
    let unwritten = renderer
        .render("search synth; render(o3)", &options())
        .unwrap_err();
    assert!(matches!(unwritten, RenderError::UnwrittenSurface { ref surface } if surface == "o3"));
    let read = renderer
        .render("search filter; read(o2).invert().write(o0)", &options())
        .unwrap_err();
    assert!(matches!(read, RenderError::UnwrittenSurface { ref surface } if surface == "o2"));
    let mixer = renderer
        .render(
            "search synth,mixer; solid().write(o0); read(o0).blendMode(tex:o2).write(o1)",
            &options(),
        )
        .unwrap_err();
    assert!(
        matches!(mixer, RenderError::MissingSurfaceInput { ref surface, .. } if surface == "o2")
    );
}

#[test]
fn compile_failures_are_transparent_dsl_errors_before_execution() {
    let mut renderer = CpuRenderer::new().unwrap();
    for source in [
        "search synth; solid(); render(o0)",
        "search filter; invert().write(o0)",
    ] {
        assert!(matches!(
            renderer.render(source, &options()),
            Err(RenderError::Dsl(_))
        ));
    }
}

#[test]
fn independent_chains_do_not_share_current_images() {
    let mut renderer = CpuRenderer::new().unwrap();
    let result = renderer
        .render(
            "search synth; solid(color:#f00).write(o0); solid(color:#00f).write(o1); render(o1)",
            &options(),
        )
        .unwrap();
    assert_eq!(&result.surface.to_rgba8()[..4], &[0, 0, 255, 255]);
}

#[test]
fn repeated_render_calls_do_not_retain_named_surfaces() {
    let mut renderer = CpuRenderer::new().unwrap();
    renderer
        .render("search synth; solid().write(o0)", &options())
        .unwrap();
    let error = renderer
        .render("search synth; render(o0)", &options())
        .unwrap_err();
    assert!(matches!(error, RenderError::UnwrittenSurface { ref surface } if surface == "o0"));
}

// --- GAP-045/046/047: new grammar forms render like their plain programs ------

#[test]
fn renderer_subchain_blocks_render_like_their_flattened_chains() {
    let mut renderer = CpuRenderer::new().unwrap();
    let flat = "search synth, filter\nnoise(seed: 1).invert().posterize(levels: 4).write(o0)\nrender(o0)\n";
    let accepted = [
        "search synth, filter\nnoise(seed: 1).subchain(name: \"look\") { .invert() .posterize(levels: 4) }.write(o0)\nrender(o0)\n",
        "search synth, filter\nnoise(seed: 1).subchain(\"look\") { .invert() .posterize(levels: 4) }.write(o0)\nrender(o0)\n",
        "search synth, filter\nnoise(seed: 1).subchain(id: \"sc1\") { .invert() .posterize(levels: 4) }.write(o0)\nrender(o0)\n",
        "search synth, filter\nnoise(seed: 1).subchain(name: \"look\", id: \"sc1\") { .invert() .posterize(levels: 4) }.write(o0)\nrender(o0)\n",
        "search synth, filter\nnoise(seed: 1).subchain() { .invert() .posterize(levels: 4) }.write(o0)\nrender(o0)\n",
        // An unparser-style multiline body.
        "search synth, filter\n\nnoise()\n  .subchain(name: \"look\") {\n    .invert()\n    .posterize(levels: 4)\n  }\n  .write(o0)\n\nrender(o0)",
    ];
    let baseline = renderer
        .render(flat, &options())
        .unwrap()
        .surface
        .to_rgba8();
    for source in accepted {
        let rendered = renderer
            .render(source, &options())
            .unwrap()
            .surface
            .to_rgba8();
        assert_eq!(rendered, baseline, "{source}");
    }
}

#[test]
fn renderer_a_loop_in_a_subchain_renders_like_the_untouched_expansion() {
    let mut renderer = CpuRenderer::new().unwrap();
    let baseline = renderer.render("search synth, filter, render\nnoise(seed: 1).loopBegin(iterationCount: 2).blur().loopEnd().write(o0)\nrender(o0)\n", &options()).unwrap().surface.to_rgba8();
    let touched = renderer.render("search synth, filter, render\nnoise(seed: 1).subchain(name: \"feedback loop\", id: \"fb1\") { .loopBegin(iterationCount: 2) .blur() .loopEnd() }.write(o0)\nrender(o0)\n", &options()).unwrap().surface.to_rgba8();
    assert_eq!(touched, baseline);
}

#[test]
fn renderer_chain_arguments_render_like_their_textual_expansions() {
    let mut renderer = CpuRenderer::new().unwrap();
    let pairs = [
        // A one-call chain.
        (
            "search synth, mixer\nnoise(seed: 1).blendMode(tex: solid(color: #ff0000), mode: screen).write(o0)\nrender(o0)\n",
            "search synth, mixer\nsolid(color: #ff0000).write(o1)\nnoise(seed: 1).blendMode(tex: read(o1), mode: screen).write(o0)\nrender(o0)\n",
        ),
        // A multi-call chain.
        (
            "search synth, filter, mixer\nnoise(seed: 1).blendMode(tex: noise(seed: 2).invert(), mode: screen).write(o0)\nrender(o0)\n",
            "search synth, filter, mixer\nnoise(seed: 2).invert().write(o1)\nnoise(seed: 1).blendMode(tex: read(o1), mode: screen).write(o0)\nrender(o0)\n",
        ),
        // Two inline arguments in one call.
        (
            "search synth, mixer\nnoise(seed: 1).alphaMask(tex: solid(color: #ff0000), baseTex: solid(color: #0000ff, alpha: 0.25)).write(o0)\nrender(o0)\n",
            "search synth, mixer\nsolid(color: #ff0000).write(o1)\nsolid(color: #0000ff, alpha: 0.25).write(o2)\nnoise(seed: 1).alphaMask(tex: read(o1), baseTex: read(o2)).write(o0)\nrender(o0)\n",
        ),
        // Positional arguments.
        (
            "search synth, mixer\nnoise(seed: 1).blendMode(solid(color: #00ff00), screen).write(o0)\nrender(o0)\n",
            "search synth, mixer\nsolid(color: #00ff00).write(o1)\nnoise(seed: 1).blendMode(read(o1), screen).write(o0)\nrender(o0)\n",
        ),
        // A chain inside a chain argument.
        (
            "search synth, filter, mixer\nnoise(seed: 1).blendMode(tex: solid(color: #ff0000).blendMode(tex: noise(seed: 3), mode: multiply), mode: screen).write(o0)\nrender(o0)\n",
            "search synth, filter, mixer\nnoise(seed: 3).write(o2)\nsolid(color: #ff0000).blendMode(tex: read(o2), mode: multiply).write(o1)\nnoise(seed: 1).blendMode(tex: read(o1), mode: screen).write(o0)\nrender(o0)\n",
        ),
        // A partial binding called in a chain argument, and a partial that holds
        // a chain argument. A partial resolves its arguments where it is called.
        (
            "search synth, mixer\nlet red = solid(color: #ff0000)\nnoise(seed: 1).blendMode(tex: red(), mode: screen).write(o0)\nrender(o0)\n",
            "search synth, mixer\nsolid(color: #ff0000).write(o1)\nnoise(seed: 1).blendMode(tex: read(o1), mode: screen).write(o0)\nrender(o0)\n",
        ),
        (
            "search synth, mixer\nlet tint = blendMode(tex: solid(color: #ff0000), mode: screen)\nnoise(seed: 1).tint().write(o0)\nnoise(seed: 2).tint().write(o1)\nrender(o0)\n",
            "search synth, mixer\nsolid(color: #ff0000).write(o2)\nnoise(seed: 1).blendMode(tex: read(o2), mode: screen).write(o0)\nnoise(seed: 2).blendMode(tex: read(o2), mode: screen).write(o1)\nrender(o0)\n",
        ),
        // A partial that holds an automation value.
        (
            "search synth\nlet tuned = noise(seed: 1, scaleX: osc(type: oscKind.sine, min: 0.25, max: 0.75, speed: 0))\ntuned().write(o0)\nrender(o0)\n",
            "search synth\nnoise(seed: 1, scaleX: 25.75).write(o0)\nrender(o0)\n",
        ),
        // Chain arguments of a volume generator.
        (
            "search synth, filter, synth3d, render\nheightmap3d(heightTex: solid(color: #00ff00).invert(), tex: solid(color: #ff0000), volumeSize: x16).renderLandscape3d().write(o0)\nrender(o0)\n",
            "search synth, filter, synth3d, render\nsolid(color: #00ff00).invert().write(o1)\nsolid(color: #ff0000).write(o2)\nheightmap3d(heightTex: read(o1), tex: read(o2), volumeSize: x16).renderLandscape3d().write(o0)\nrender(o0)\n",
        ),
        // A chain that starts from read(), after a mid-chain write in the same statement.
        (
            "search synth, filter, mixer\nnoise(seed: 1).write(o1).blendMode(tex: read(o1).invert(), mode: screen).write(o0)\nrender(o0)\n",
            "search synth, filter, mixer\nnoise(seed: 1).write(o1)\nread(o1).invert().write(o2)\nread(o1).blendMode(tex: read(o2), mode: screen).write(o0)\nrender(o0)\n",
        ),
        // A subchain block inside a chain argument.
        (
            "search synth, filter, mixer\nnoise(seed: 1).blendMode(tex: noise(seed: 2).subchain(name: \"x\") { .invert() }, mode: screen).write(o0)\nrender(o0)\n",
            "search synth, filter, mixer\nnoise(seed: 2).invert().write(o1)\nnoise(seed: 1).blendMode(tex: read(o1), mode: screen).write(o0)\nrender(o0)\n",
        ),
    ];
    for (inline, expanded) in pairs {
        let left = renderer
            .render(inline, &options())
            .unwrap()
            .surface
            .to_rgba8();
        let right = renderer
            .render(expanded, &options())
            .unwrap()
            .surface
            .to_rgba8();
        assert_eq!(left, right, "{inline}");
    }
}

#[test]
fn renderer_grammar_forms_render_like_their_plain_equivalents() {
    let mut renderer = CpuRenderer::new().unwrap();
    let pi_third = ((std::f64::consts::PI + 1.0) / 2.0) - (-std::f64::consts::PI);
    let pairs = [
        // Math.PI
        ("search synth\nnoise(seed: 1, scaleX: Math.PI * 4).write(o0)\nrender(o0)\n", "search synth\nnoise(seed: 1, scaleX: 12.566370614359172).write(o0)\nrender(o0)\n".to_string()),
        ("search synth\nlet tau = Math.PI * 2\nnoise(seed: 1, scaleX: tau).write(o0)\nrender(o0)\n", "search synth\nnoise(seed: 1, scaleX: 6.283185307179586).write(o0)\nrender(o0)\n".to_string()),
        ("search synth\nnoise(seed: 1, scaleX: (Math.PI + 1) / 2 - -Math.PI).write(o0)\nrender(o0)\n", format!("search synth\nnoise(seed: 1, scaleX: {pi_third}).write(o0)\nrender(o0)\n")),
        // Triple-quoted strings keep raw content: newlines and quotes included.
        ("search synth, filter\nsolid(color: #000).text(text: \"\"\"hi\"\"\").write(o0)\nrender(o0)\n", "search synth, filter\nsolid(color: #000).text(text: \"hi\").write(o0)\nrender(o0)\n".to_string()),
        ("search synth, filter\nsolid(color: #000).text(text: \"\"\"Hello\nWorld\"\"\").write(o0)\nrender(o0)\n", "search synth, filter\nsolid(color: #000).text(text: \"Hello\\nWorld\").write(o0)\nrender(o0)\n".to_string()),
        // Aliases and partials in one program.
        ("search synth\nlet pattern = noise\npattern(seed: 3).write(o0)\nrender(o0)\n", "search synth\nnoise(seed: 3).write(o0)\nrender(o0)\n".to_string()),
        ("search synth\nlet pattern = noise\npattern(10).write(o0)\nrender(o0)\n", "search synth\nnoise(10).write(o0)\nrender(o0)\n".to_string()),
        ("search synth\nlet tuned = noise(seed: 3)\nlet again = tuned\nagain(scaleX: 20).write(o0)\nrender(o0)\n", "search synth\nnoise(seed: 3, scaleX: 20).write(o0)\nrender(o0)\n".to_string()),
        ("search synth, filter\nlet fx = invert\nnoise(seed: 1).fx().write(o0)\nrender(o0)\n", "search synth, filter\nnoise(seed: 1).invert().write(o0)\nrender(o0)\n".to_string()),
        // write3d / read3d.
        ("search synth3d, render\nnoise3d(seed: 1, volumeSize: 8).write3d(vol0, geo0)\nread3d(vol0, geo0).render3d().write(o0)\nrender(o0)\n", "search synth3d, render\nnoise3d(seed: 1, volumeSize: 8).render3d().write(o0)\nrender(o0)\n".to_string()),
        ("search synth3d, render\nnoise3d(seed: 1, volumeSize: 8).write3d(vol1, geo1).render3d().write(o0)\nrender(o0)\n", "search synth3d, render\nnoise3d(seed: 1, volumeSize: 8).render3d().write(o0)\nrender(o0)\n".to_string()),
        ("search synth3d, render\nnoise3d(seed: 1, volumeSize: 8).write3d(vol0, geo0)\nread3d(tex3d: vol0, geo: geo0).render3d().write(o0)\nrender(o0)\n", "search synth3d, render\nnoise3d(seed: 1, volumeSize: 8).render3d().write(o0)\nrender(o0)\n".to_string()),
        // from().
        ("search synth, filter\nfrom(classicNoisedeck, noise(seed: 3)).write(o0)\nrender(o0)\n", "search classicNoisedeck\nnoise(seed: 3).write(o0)\nrender(o0)\n".to_string()),
        ("search synth\nnoise(seed: 1).from(filter, invert()).write(o0)\nrender(o0)\n", "search synth, filter\nnoise(seed: 1).invert().write(o0)\nrender(o0)\n".to_string()),
        ("search synth, mixer\nnoise(seed: 1).blendMode(tex: from(synth, solid(color: #ff0000)), mode: screen).write(o0)\nrender(o0)\n", "search synth, mixer\nsolid(color: #ff0000).write(o1)\nnoise(seed: 1).blendMode(tex: read(o1), mode: screen).write(o0)\nrender(o0)\n".to_string()),
        ("search synth, filter\nlet n = from(classicNoisedeck, noise(seed: 3))\nn().write(o0)\nrender(o0)\n", "search classicNoisedeck\nnoise(seed: 3).write(o0)\nrender(o0)\n".to_string()),
    ];
    for (form, plain) in pairs {
        let left = renderer
            .render(form, &options())
            .unwrap()
            .surface
            .to_rgba8();
        let right = renderer
            .render(&plain, &options())
            .unwrap()
            .surface
            .to_rgba8();
        assert_eq!(left, right, "{form}");
    }
}

#[test]
fn renderer_text_textures_render_like_their_explicit_wiring() {
    let mut renderer = CpuRenderer::new().unwrap();
    let inline = "search synth, filter\nsolid(color: #000).text(text: \"\"\"ok\"\"\").write(o0)\nrender(o0)\n";
    let wired =
        "search synth, filter\nsolid(color: #000).text(text: \"ok\").write(o0)\nrender(o0)\n";
    let left = renderer
        .render(inline, &options())
        .unwrap()
        .surface
        .to_rgba8();
    let right = renderer
        .render(wired, &options())
        .unwrap()
        .surface
        .to_rgba8();
    assert_eq!(left, right);
}

#[test]
fn renderer_float_choice_names_render_like_their_numeric_values() {
    let mut renderer = CpuRenderer::new().unwrap();
    let pairs = [
        (
            "search filter, synth\nnoise(seed: 1, ridges: true).palette(rotation: fwd).write(o0)\nrender(o0)\n",
            "search filter, synth\nnoise(seed: 1, ridges: true).palette(rotation: 1).write(o0)\nrender(o0)\n",
        ),
        (
            "search filter, synth\nnoise(seed: 1, ridges: true).palette(rotation: back).write(o0)\nrender(o0)\n",
            "search filter, synth\nnoise(seed: 1, ridges: true).palette(rotation: -1).write(o0)\nrender(o0)\n",
        ),
        (
            "search filter, synth\nnoise(seed: 1, ridges: true, colorMode: mono).historicPalette(rotation: fwd).write(o0)\nrender(o0)\n",
            "search filter, synth\nnoise(seed: 1, ridges: true, colorMode: mono).historicPalette(rotation: 1).write(o0)\nrender(o0)\n",
        ),
        (
            "search filter, synth\nnoise(seed: 1).tetraColorArray(smoothness: 0, rotation: back).write(o0)\nrender(o0)\n",
            "search filter, synth\nnoise(seed: 1).tetraColorArray(smoothness: 0, rotation: -1).write(o0)\nrender(o0)\n",
        ),
        (
            "search filter, synth\nnoise(seed: 1).tetraCosine(rotation: back).write(o0)\nrender(o0)\n",
            "search filter, synth\nnoise(seed: 1).tetraCosine(rotation: -1).write(o0)\nrender(o0)\n",
        ),
        (
            "search synth3d, filter3d, render\nnoise3d(seed: 3, volumeSize: 8).palette3d(rotation: fwd).render3d().write(o0)\nrender(o0)\n",
            "search synth3d, filter3d, render\nnoise3d(seed: 3, volumeSize: 8).palette3d(rotation: 1).render3d().write(o0)\nrender(o0)\n",
        ),
    ];
    for (named, numeric) in pairs {
        let left = renderer
            .render(named, &options())
            .unwrap()
            .surface
            .to_rgba8();
        let right = renderer
            .render(numeric, &options())
            .unwrap()
            .surface
            .to_rgba8();
        assert_eq!(left, right, "{named}");
    }
}
