use noisemaker_cpu::{CpuRenderer, OneShot, RenderOptions, render_overlay};
use sha2::{Digest, Sha256};

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[test]
fn ready_overlays_match_exact_oracle_hashes_and_are_deterministic() {
    for (effect, base, ready_hash, initial_hash, overlay_hash) in [
        (
            "fibers",
            "#000",
            "f848376d1f36bb8b2d8fe0234ecef34c096230eddec8be4cd8ef5986695ec803",
            "9503245a0161a939de15c2414db2d336e761822fa6cff8136e4148f58f1f782e",
            "72f7299bc6cb648a061ed420260abd8e9a74be8de249bb370e949f1e3eb39684",
        ),
        (
            "scratches",
            "#000",
            "419a366e89b56701e6b74134342c30b499ddf33f599f9c5cc9613fa8e8cb540c",
            "9503245a0161a939de15c2414db2d336e761822fa6cff8136e4148f58f1f782e",
            "f7bfc643afb5347f29dd673c90169d221ef32c1ecae4364fe847c93f18ffbf72",
        ),
        (
            "strayHair",
            "#fff",
            "f63e9d512329ac09335f25ba08d1895db43d30511430071fc9ef41618b6a82f0",
            "5f4ecdb7b71c3e403983fe405cddcdc2f2576b655fdb3e80d94a6f7c32e58bc2",
            "f7441a58f490ec34254403a60ddc0b34c3e7cb756f40428d889126019c61f90e",
        ),
    ] {
        let effect_id = format!("filter/{effect}");
        let first = render_overlay(&effect_id, 16, 16, 7, 1.0)
            .unwrap()
            .to_rgba8();
        let second = render_overlay(&effect_id, 16, 16, 7, 1.0)
            .unwrap()
            .to_rgba8();
        assert_eq!(hash(&first), overlay_hash, "raw {effect}");
        assert_eq!(first, second, "raw {effect}");
        assert!(first.chunks_exact(4).any(|pixel| pixel[3] != 0), "{effect}");
        for (one_shot, expected) in [
            (OneShot::Ready, ready_hash),
            (OneShot::Initial, initial_hash),
        ] {
            let mut renderer = CpuRenderer::new().unwrap();
            let output = renderer.render(
                &format!("search synth,filter; solid(color:{base}).{effect}(density:1,seed:7,alpha:1).write(o0)"),
                &RenderOptions { width: 16, height: 16, time: 0.25, seed: 1, one_shot, ..RenderOptions::default() },
            ).unwrap().surface.to_rgba8();
            assert_eq!(hash(&output), expected, "{effect} {one_shot:?}");
        }
    }
}

fn options(one_shot: OneShot) -> RenderOptions {
    RenderOptions {
        width: 8,
        height: 8,
        time: 0.25,
        seed: 1,
        one_shot,
        ..RenderOptions::default()
    }
}

#[test]
fn initial_mode_is_clear_passthrough_and_does_not_touch_cache() {
    let mut renderer = CpuRenderer::with_cpu_texture_cache_byte_limit(1024).unwrap();
    let result = renderer
        .render(
            "search synth,filter; solid(color:#2b2b2b).scratches(density:1,seed:7,alpha:1).write(o0)",
            &options(OneShot::Initial),
        )
        .unwrap();
    assert_eq!(&result.surface.to_rgba8()[..4], &[43, 43, 43, 255]);
    assert_eq!(renderer.cpu_texture_cache_stats().entries, 0);
    assert_eq!(renderer.cpu_texture_cache_stats().bytes, 0);
}

#[test]
fn cache_key_hits_misses_evicts_refreshes_and_clears() {
    let mut renderer = CpuRenderer::with_cpu_texture_cache_byte_limit(1024).unwrap();
    let render = |renderer: &mut CpuRenderer, effect: &str, seed: i32, density: f32| {
        renderer.render(
            &format!("search synth,filter; solid(color:#000).{effect}(density:{density},seed:{seed},alpha:1).write(o0)"),
            &options(OneShot::Ready),
        ).unwrap().surface.to_rgba8()
    };
    let fibers = render(&mut renderer, "fibers", 7, 1.0);
    assert_eq!(renderer.cpu_texture_cache_stats().entries, 1);
    assert_eq!(renderer.cpu_texture_cache_stats().bytes, 1024);
    assert_eq!(render(&mut renderer, "fibers", 7, 1.0), fibers);
    assert_eq!(renderer.cpu_texture_cache_stats().entries, 1);
    render(&mut renderer, "scratches", 7, 1.0);
    assert_eq!(renderer.cpu_texture_cache_stats().entries, 1);
    render(&mut renderer, "fibers", 8, 1.0);
    assert_eq!(renderer.cpu_texture_cache_stats().entries, 1);
    render(&mut renderer, "fibers", 8, 0.5);
    assert_eq!(renderer.cpu_texture_cache_stats().entries, 1);
    renderer.clear_cpu_texture_cache();
    assert_eq!(renderer.cpu_texture_cache_stats().entries, 0);
    assert_eq!(renderer.cpu_texture_cache_stats().bytes, 0);
    assert_eq!(render(&mut renderer, "fibers", 7, 1.0), fibers);
    assert_eq!(renderer.cpu_texture_cache_stats().entries, 1);
    renderer.dispose();
    assert_eq!(renderer.cpu_texture_cache_stats().entries, 0);
}

#[test]
fn zero_and_oversized_limits_leave_entries_uncached() {
    for limit in [0, 1023] {
        let mut renderer = CpuRenderer::with_cpu_texture_cache_byte_limit(limit).unwrap();
        renderer
            .render(
                "search synth,filter; solid(color:#000).fibers(density:1,seed:7,alpha:1).write(o0)",
                &options(OneShot::Ready),
            )
            .unwrap();
        assert_eq!(renderer.cpu_texture_cache_stats().entries, 0);
        assert_eq!(renderer.cpu_texture_cache_stats().bytes, 0);
        assert_eq!(renderer.cpu_texture_cache_stats().byte_limit, limit);
    }
}
