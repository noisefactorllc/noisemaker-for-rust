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
fn fragment_registry_is_exactly_the_four_replacement_keys() {
    assert_eq!(
        fragment_adapter_keys(),
        [
            "classicNoisedeck/fractal:fractal",
            "filter/historicPalette:historicPalette",
            "filter/palette:palette",
            "synth/julia:julia",
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
            "search synth,filter; solid(color:#789abc).palette(paletteIndex:1).write(o0)",
            &options(),
        )
        .unwrap()
        .surface
        .to_rgba8();
    assert_eq!(&palette[..16], &[255, 255, 193, 255].repeat(4));
    let historic = renderer
        .render(
            "search synth,filter; solid(color:#789abc).historicPalette(paletteIndex:1).write(o0)",
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
