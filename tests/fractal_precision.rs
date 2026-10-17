use noisemaker_cpu::{CpuRenderer, RenderOptions};
use sha2::{Digest, Sha256};

// RGBA8 goldens from the CPU authority at 5e93c23c6cd8c2b3f777bedc40142798be185ba8.

#[test]
fn fractal_precision_case_1() {
    let source = "search classicNoisedeck; fractal(mode: 1).write(o0)";
    let options = RenderOptions {
        width: 64,
        height: 64,
        time: 0.25,
        seed: 1,
        ..RenderOptions::default()
    };
    let pixels = CpuRenderer::new()
        .unwrap()
        .render(source, &options)
        .unwrap()
        .surface
        .to_rgba8();
    assert_eq!(
        format!("{:x}", Sha256::digest(&pixels)),
        "24018d442f1d6b0a85c5be185ab3db36dc03248c34e395e24397fa6ea3fb6e2a",
        "{source}"
    );
}

#[test]
fn fractal_precision_case_2() {
    let source =
        "search classicNoisedeck; fractal(rotation: 30, iterations: 10, mode: 1).write(o0)";
    let options = RenderOptions {
        width: 64,
        height: 64,
        time: 0.25,
        seed: 1,
        ..RenderOptions::default()
    };
    let pixels = CpuRenderer::new()
        .unwrap()
        .render(source, &options)
        .unwrap()
        .surface
        .to_rgba8();
    assert_eq!(
        format!("{:x}", Sha256::digest(&pixels)),
        "8c9c0986069bd7859352ece912640cc9d8ec8292b559ebc3f5adde52281a5bcc",
        "{source}"
    );
}

#[test]
fn fractal_precision_case_3() {
    let source = "search classicNoisedeck; fractal(rotation: 30, iterations: 10).write(o0)";
    let options = RenderOptions {
        width: 64,
        height: 64,
        time: 0.25,
        seed: 1,
        ..RenderOptions::default()
    };
    let pixels = CpuRenderer::new()
        .unwrap()
        .render(source, &options)
        .unwrap()
        .surface
        .to_rgba8();
    assert_eq!(
        format!("{:x}", Sha256::digest(&pixels)),
        "8c7c7406e5039802b2266d64ae43f477b128e16bb4d2c95f18e1b7f1b5825f97",
        "{source}"
    );
}

#[test]
fn fractal_precision_case_4() {
    let source = "search classicNoisedeck; fractal(mode: 1, zoomAmt: 37, centerX: 21, centerY: -13, rotation: 30).write(o0)";
    let options = RenderOptions {
        width: 64,
        height: 48,
        time: 0.25,
        seed: 1,
        ..RenderOptions::default()
    };
    let pixels = CpuRenderer::new()
        .unwrap()
        .render(source, &options)
        .unwrap()
        .surface
        .to_rgba8();
    assert_eq!(
        format!("{:x}", Sha256::digest(&pixels)),
        "7f76779a52884cfd3756498f85fee20e5b5e94857b6108490d9ea208c1f66525",
        "{source}"
    );
}

#[test]
fn fractal_precision_case_5() {
    let source = "search classicNoisedeck; fractal(mode: 1, speed: 37, offsetX: 14, offsetY: -18, iterations: 37).write(o0)";
    let options = RenderOptions {
        width: 64,
        height: 48,
        time: 0.25,
        seed: 1,
        ..RenderOptions::default()
    };
    let pixels = CpuRenderer::new()
        .unwrap()
        .render(source, &options)
        .unwrap()
        .surface
        .to_rgba8();
    assert_eq!(
        format!("{:x}", Sha256::digest(&pixels)),
        "8f8747044ec4da6cb9bc6b1acc9c7e0585417e7089b1e3bb92215b0ae2fc80e6",
        "{source}"
    );
}

#[test]
fn fractal_precision_case_6() {
    let source = "search classicNoisedeck; fractal(palette: 0, mode: 1, colorMode: 4, paletteMode: 0, rotatePalette: 17, repeatPalette: 3, cyclePalette: 1).write(o0)";
    let options = RenderOptions {
        width: 64,
        height: 48,
        time: 0.25,
        seed: 1,
        ..RenderOptions::default()
    };
    let pixels = CpuRenderer::new()
        .unwrap()
        .render(source, &options)
        .unwrap()
        .surface
        .to_rgba8();
    assert_eq!(
        format!("{:x}", Sha256::digest(&pixels)),
        "a3e25f1890ba99bf2db689212ca47849bf404a3b68a336b259b181464c207212",
        "{source}"
    );
}

#[test]
fn fractal_precision_case_7() {
    let source = "search classicNoisedeck; fractal(palette: 0, mode: 1, colorMode: 4, paletteMode: 1, rotatePalette: 17, repeatPalette: 3, cyclePalette: -1).write(o0)";
    let options = RenderOptions {
        width: 64,
        height: 48,
        time: 0.25,
        seed: 1,
        ..RenderOptions::default()
    };
    let pixels = CpuRenderer::new()
        .unwrap()
        .render(source, &options)
        .unwrap()
        .surface
        .to_rgba8();
    assert_eq!(
        format!("{:x}", Sha256::digest(&pixels)),
        "9b260681eddc66d11f07d147e0901264d316091c4cb40c36b48d86e9c31ddc3c",
        "{source}"
    );
}

#[test]
fn fractal_precision_case_8() {
    let source = "search classicNoisedeck; fractal(palette: 0, mode: 1, colorMode: 4, paletteMode: 2, rotatePalette: 17, repeatPalette: 3).write(o0)";
    let options = RenderOptions {
        width: 64,
        height: 48,
        time: 0.25,
        seed: 1,
        ..RenderOptions::default()
    };
    let pixels = CpuRenderer::new()
        .unwrap()
        .render(source, &options)
        .unwrap()
        .surface
        .to_rgba8();
    assert_eq!(
        format!("{:x}", Sha256::digest(&pixels)),
        "f7218d4abacc5e3fa686ae9c51a963e22aeb9f4afe29a9fed717a65394785bd8",
        "{source}"
    );
}

#[test]
fn fractal_precision_case_9() {
    let source = "search classicNoisedeck; fractal(mode: 1, colorMode: 6, hueRange: 37, levels: 8).write(o0)";
    let options = RenderOptions {
        width: 64,
        height: 48,
        time: 0.25,
        seed: 1,
        ..RenderOptions::default()
    };
    let pixels = CpuRenderer::new()
        .unwrap()
        .render(source, &options)
        .unwrap()
        .surface
        .to_rgba8();
    assert_eq!(
        format!("{:x}", Sha256::digest(&pixels)),
        "e2dfeb5f154fa4419315e0fde072e2281a80c18601c378344ec19c9919bfe42d",
        "{source}"
    );
}

#[test]
fn fractal_precision_case_10() {
    let source = "search classicNoisedeck; fractal(mode: 1, bgAlpha: 37, cutoff: 10).write(o0)";
    let options = RenderOptions {
        width: 64,
        height: 48,
        time: 0.25,
        seed: 1,
        ..RenderOptions::default()
    };
    let pixels = CpuRenderer::new()
        .unwrap()
        .render(source, &options)
        .unwrap()
        .surface
        .to_rgba8();
    assert_eq!(
        format!("{:x}", Sha256::digest(&pixels)),
        "41814d5b3e4af2843d502c2ec4e55dee8af4db814134871e2dc58e1297220a71",
        "{source}"
    );
}

#[test]
fn fractal_precision_case_11() {
    let source =
        "search classicNoisedeck; fractal(type: 1, mode: 1, colorMode: 4, levels: 8).write(o0)";
    let options = RenderOptions {
        width: 64,
        height: 48,
        time: 0.25,
        seed: 1,
        ..RenderOptions::default()
    };
    let pixels = CpuRenderer::new()
        .unwrap()
        .render(source, &options)
        .unwrap()
        .surface
        .to_rgba8();
    assert_eq!(
        format!("{:x}", Sha256::digest(&pixels)),
        "67717df7951ff4670ebeb9a95a3b4e664e194a54d508e7d5a10a7cd7d622cbfc",
        "{source}"
    );
}

#[test]
fn fractal_precision_case_12() {
    let source =
        "search classicNoisedeck; fractal(type: 2, mode: 1, speed: 12, colorMode: 6).write(o0)";
    let options = RenderOptions {
        width: 64,
        height: 48,
        time: 0.25,
        seed: 1,
        ..RenderOptions::default()
    };
    let pixels = CpuRenderer::new()
        .unwrap()
        .render(source, &options)
        .unwrap()
        .surface
        .to_rgba8();
    assert_eq!(
        format!("{:x}", Sha256::digest(&pixels)),
        "5af302190e284252afe453372db1480e09d319e56464d321059fff845aa98336",
        "{source}"
    );
}
