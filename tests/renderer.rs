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
