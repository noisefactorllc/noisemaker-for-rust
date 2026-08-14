use std::collections::BTreeMap;

use noisemaker_cpu::{CpuRenderer, ParamValue, RenderOptions, Surface, render_effect};

fn options() -> RenderOptions {
    RenderOptions {
        width: 2,
        height: 2,
        time: 0.25,
        seed: 1,
        ..RenderOptions::default()
    }
}

#[test]
fn volume_generator_executes_typed_ir_before_public_image_wrapper_rejects_channel() {
    let params = BTreeMap::from([("volumeSize".into(), ParamValue::Int(4))]);
    let error =
        render_effect("synth3d/noise3d", &params, &BTreeMap::new(), &options()).unwrap_err();
    assert!(error.to_string().contains("outputTex"), "{error}");
}

#[test]
fn volume_generator_filter_and_renderer_preserve_typed_flow() {
    // Break caught: collapsing a typed bundle to one last-pass image loses the
    // volume atlas before palette3d/render3d can consume it.
    let result = CpuRenderer::new()
        .unwrap()
        .render(
            "search synth3d,filter3d,render; noise3d(volumeSize:4).palette3d(volumeSize:8).render3d(volumeSize:8).write(o0)",
            &options(),
        )
        .unwrap();
    assert_eq!((result.surface.width(), result.surface.height()), (2, 2));
    assert!(result.surface.data().iter().all(|value| value.is_finite()));
}

#[test]
fn incoming_volume_atlas_must_be_exactly_n_by_n_squared() {
    // Break caught: accepting 4x8 as a volume atlas lets slice addressing read
    // unrelated rows rather than failing at the consumer boundary.
    let inputs = BTreeMap::from([("inputTex3d".into(), Surface::new(4, 8).unwrap())]);
    let params = BTreeMap::from([("volumeSize".into(), ParamValue::Int(8))]);
    let error = render_effect("render/render3d", &params, &inputs, &options()).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("input volume atlas expected 4x16, received 4x8"),
        "{error}"
    );
}

#[test]
fn incoming_atlas_overrides_downstream_authored_volume_size() {
    // Break caught: allocating the filter at its authored 8x64 instead of the
    // incoming 4x16 silently changes both addressing and shader uniforms.
    let source = |size| {
        format!(
            "search synth3d,filter3d,render; noise3d(volumeSize:4).palette3d(volumeSize:{size}).render3d(volumeSize:{size}).write(o0)"
        )
    };
    let mut renderer = CpuRenderer::new().unwrap();
    let inherited = renderer.render(&source(8), &options()).unwrap();
    let explicit = renderer.render(&source(4), &options()).unwrap();
    assert_eq!(inherited.surface, explicit.surface);
}

#[test]
fn zero_iteration_volume_generator_returns_zero_starter_and_bypasses_passes() {
    // Break caught: executing iteration zero mutates starter state; returning
    // no typed volume makes the downstream renderer fail.
    let source = "search synth3d,render; cellularAutomata3d(volumeSize:4, iterationCount:0).render3d().write(o0)";
    let mut renderer = CpuRenderer::new().unwrap();
    let first = renderer.render(source, &options()).unwrap();
    let second = renderer.render(source, &options()).unwrap();
    assert_eq!(first.surface, second.surface);
    assert!(first.surface.data().iter().all(|value| value.is_finite()));
}

#[test]
fn zero_iteration_clones_every_incoming_typed_channel() {
    // Break caught: an N=0 group that drops the incoming volume or geometry
    // differs from rendering the original generator bundle directly.
    let direct = "search synth3d,render; noise3d(volumeSize:4,seed:2).render3d().write(o0)";
    let bypassed = "search synth3d,render; noise3d(volumeSize:4,seed:2).cellularAutomata3d(volumeSize:8,iterationCount:0).render3d().write(o0)";
    let mut renderer = CpuRenderer::new().unwrap();
    assert_eq!(
        renderer.render(direct, &options()).unwrap().surface,
        renderer.render(bypassed, &options()).unwrap().surface
    );
}

#[test]
fn huge_volume_atlases_fail_before_allocation_or_integer_wrap() {
    // Break caught: unchecked N*N or N*N*N can wrap and allocate a deceptively
    // small surface instead of enforcing the shared pixel cap.
    for size in [4096_i32, i32::MAX] {
        let source =
            format!("search synth3d,render; noise3d(volumeSize:{size}).render3d().write(o0)");
        let error = CpuRenderer::new()
            .unwrap()
            .render(
                &source,
                &RenderOptions {
                    width: 1,
                    height: 1,
                    ..options()
                },
            )
            .unwrap_err();
        let text = error.to_string();
        assert!(
            text.contains("maximum is 16777216") || text.contains("invalid texture dimension"),
            "{size}: {text}"
        );
    }
}
