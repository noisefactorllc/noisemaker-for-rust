use noisemaker_cpu::{
    CpuRenderer, RenderOptions, fragment_adapter_keys, historic_palette_dimensions,
    palette_dimensions,
};

fn options() -> RenderOptions {
    RenderOptions {
        width: 4,
        height: 4,
        time: 0.25,
        seed: 1,
        ..RenderOptions::default()
    }
}

#[test]
fn fragment_registry_contains_the_canonical_replacement_keys() {
    assert_eq!(
        fragment_adapter_keys(),
        [
            "classicNoisedeck/fractal:fractal",
            "filter/crt:crt",
            "filter/historicPalette:historicPalette",
            "filter/median:median",
            "filter/palette:palette",
            "synth/julia:julia",
            "synth/remap:remap",
        ]
    );
    assert!(!fragment_adapter_keys().contains(&"filter/palette"));
    assert_eq!(palette_dimensions(), (55, 16));
    assert_eq!(historic_palette_dimensions(), (21, 15));
}

#[test]
fn fractal_and_julia_registered_adapters_match_oracle_pixels() {
    let mut renderer = CpuRenderer::new().unwrap();
    let fractal = renderer
        .render("search classicNoisedeck; fractal().write(o0)", &options())
        .unwrap()
        .surface
        .to_rgba8();
    assert_eq!(
        &fractal[..16],
        &[
            77, 77, 77, 255, 62, 62, 62, 255, 54, 54, 54, 255, 15, 15, 15, 255
        ]
    );
    let julia = renderer
        .render("search synth; julia().write(o0)", &options())
        .unwrap()
        .surface
        .to_rgba8();
    assert_eq!(
        &julia[..16],
        &[
            0, 0, 0, 255, 198, 198, 198, 255, 203, 203, 203, 255, 0, 0, 0, 255
        ]
    );
}

#[test]
fn cosine_and_historic_palette_adapters_match_oracle_pixels() {
    let mut renderer = CpuRenderer::new().unwrap();
    let palette = renderer
        .render(
            "search synth,filter; solid(color:#789abc).palette(index:1).write(o0)",
            &options(),
        )
        .unwrap()
        .surface
        .to_rgba8();
    assert_eq!(&palette[..16], &[255, 255, 193, 255].repeat(4));
    let historic = renderer
        .render(
            "search synth,filter; solid(color:#789abc).historicPalette(index:1).write(o0)",
            &options(),
        )
        .unwrap()
        .surface
        .to_rgba8();
    assert_eq!(&historic[..16], &[250, 250, 250, 255].repeat(4));
}

#[test]
fn nonregistered_fragment_program_still_uses_generic_vm() {
    let result = CpuRenderer::new()
        .unwrap()
        .render(
            "search synth,filter; solid(color:#123456).invert().write(o0)",
            &options(),
        )
        .unwrap();
    assert_eq!(&result.surface.to_rgba8()[..4], &[237, 203, 169, 255]);
}

#[test]
fn remap_matches_the_stable_non_square_reference_scene() {
    use sha2::{Digest, Sha256};
    // Upstream's remap parity case at CPU 170d7ba. It includes a concave zone,
    // overlapping translucent sources, explicit bounds, and a 64-vertex zone.
    let source = include_str!("fixtures/cpu-adapter-sync/remap.dsl");
    for (width, height, hash) in [
        (
            192,
            128,
            "d05625c6194cdf77d26201052f082eb0d543a9d5a07eaba62cf5c8d9896328f5",
        ),
        (
            19,
            13,
            "4a06fa59680862d3200e8eff674b80cfef329255e49935e405afde5582db628a",
        ),
    ] {
        let options = RenderOptions {
            width,
            height,
            ..Default::default()
        };
        let image = CpuRenderer::new()
            .unwrap()
            .render(source, &options)
            .unwrap()
            .surface;
        assert_eq!(format!("{:x}", Sha256::digest(image.to_rgba8())), hash);
    }
}

#[test]
fn remap_preserves_edges_on_either_side_of_the_pixel_center() {
    let options = RenderOptions {
        width: 7,
        height: 5,
        ..Default::default()
    };
    for (right, expected) in [("0.49999999999", 0), ("0.50000000001", 255)] {
        let source = format!(
            "search synth\nsolid(color: #ff0000).write(o1)\n\
            remap(smoothEdge: 0, zoneCount: 1, zone0_tex: read(o1), zone0_count: 4, \
            zone0_v0: vec4(0,0,{right},0), zone0_v1: vec4({right},1,0,1)).write(o0)"
        );
        let image = CpuRenderer::new()
            .unwrap()
            .render(&source, &options)
            .unwrap()
            .surface
            .to_rgba8();
        assert_eq!(image[(2 * 7 + 3) * 4], expected);
        assert_eq!(image[(2 * 7 + 2) * 4], 255);
        assert_eq!(image[(2 * 7 + 4) * 4], 0);
    }
}

#[test]
fn remap_routes_all_eight_zones() {
    let options = RenderOptions {
        width: 8,
        height: 5,
        ..Default::default()
    };
    let mut fields = vec!["smoothEdge: 0".to_owned(), "zoneCount: 8".to_owned()];
    for zone in 0..8 {
        let left = f64::from(zone) / 8.0;
        let right = f64::from(zone + 1) / 8.0;
        fields.push(format!(
            "zone{zone}_tex: read(o1), zone{zone}_count: 4, zone{zone}_alpha: {right}, \
            zone{zone}_v0: vec4({left},0,{right},0), zone{zone}_v1: vec4({right},1,{left},1)"
        ));
    }
    let source = format!(
        "search synth\nsolid(color: #ff0000).write(o1)\nremap({}).write(o0)",
        fields.join(",")
    );
    let image = CpuRenderer::new()
        .unwrap()
        .render(&source, &options)
        .unwrap()
        .surface
        .to_rgba8();
    let reds: Vec<_> = image[..32].chunks_exact(4).map(|pixel| pixel[0]).collect();
    assert_eq!(reds, [32, 64, 96, 128, 159, 191, 223, 255]);
}
