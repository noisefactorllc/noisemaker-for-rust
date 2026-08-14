use std::error::Error;
use std::path::PathBuf;

use noisemaker_cpu::{CpuRenderer, RenderOptions, encode_png};

fn output_path(default_name: &str) -> PathBuf {
    std::env::var_os("NOISEMAKER_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir().join(format!(
                "noisemaker-rust-{}-{default_name}",
                std::process::id()
            ))
        })
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut renderer = CpuRenderer::new()?;
    let options = RenderOptions {
        width: 64,
        height: 64,
        time: 0.25,
        seed: 3,
        ..RenderOptions::default()
    };
    let result = renderer.render(
        "search synth,classicNoisedeck,filter; gradient().kaleido(sides: 8).vignette().write(o0)",
        &options,
    )?;
    let path = output_path("dsl.png");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, encode_png(&result.surface)?)?;
    println!("{}", path.display());
    Ok(())
}
