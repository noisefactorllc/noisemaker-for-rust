use std::collections::BTreeMap;
use std::error::Error;
use std::path::PathBuf;

use noisemaker_cpu::{ParamValue, RenderOptions, Surface, encode_png, render_effect};

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
    let options = RenderOptions {
        width: 64,
        height: 64,
        time: 0.25,
        seed: 3,
        ..RenderOptions::default()
    };
    let params = BTreeMap::<String, ParamValue>::new();
    let inputs = BTreeMap::<String, Surface>::new();
    let surface = render_effect("synth/gradient", &params, &inputs, &options)?;
    let path = output_path("effect.png");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, encode_png(&surface)?)?;
    println!("{}", path.display());
    Ok(())
}
