use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use noisemaker_cpu::{ParamValue, RenderOptions, Surface, decode_png, encode_png, render_effect};

fn javascript_oracle_dir() -> PathBuf {
    let configured = std::env::var_os("NOISEMAKER_JS_CPU_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("crate must have a parent directory")
                .join("noisemaker-for-cpu")
        });
    let directory = configured.canonicalize().unwrap_or_else(|error| {
        panic!(
            "JavaScript parity oracle {} is unavailable ({error}); set NOISEMAKER_JS_CPU_DIR",
            configured.display()
        )
    });
    assert!(
        directory.join("bin/noisemaker-cpu.js").is_file(),
        "{} is not a noisemaker-for-cpu checkout",
        directory.display()
    );
    directory
}

fn options() -> RenderOptions {
    RenderOptions {
        width: 8,
        height: 8,
        time: 0.25,
        seed: 1,
        ..RenderOptions::default()
    }
}

fn rust_effect(effect: &str) -> Surface {
    let inputs = if effect == "filter/invert" {
        BTreeMap::from([(
            "inputTex".into(),
            Surface::from_rgba8(8, 8, &[51, 102, 153, 191].repeat(64)).unwrap(),
        )])
    } else {
        BTreeMap::new()
    };
    render_effect(
        effect,
        &BTreeMap::<String, ParamValue>::new(),
        &inputs,
        &options(),
    )
    .unwrap()
}

fn js_effect(effect: &str, javascript_oracle: &Path) -> Surface {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "noisemaker-rust-parity-{}-{nonce}.png",
        effect.replace('/', "-")
    ));
    let input_path = std::env::temp_dir().join(format!("noisemaker-rust-parity-input-{nonce}.png"));
    let mut command = Command::new("node");
    command.arg(javascript_oracle.join("bin/noisemaker-cpu.js"));
    if effect == "filter/invert" {
        let input = Surface::from_rgba8(8, 8, &[51, 102, 153, 191].repeat(64)).unwrap();
        fs::write(&input_path, encode_png(&input).unwrap()).unwrap();
        command.args(["apply", effect]).arg(&input_path);
    } else {
        command.args(["effect", effect]);
    }
    let output = command
        .args([
            "--width", "8", "--height", "8", "--time", "0.25", "--seed", "1", "--output",
        ])
        .arg(&path)
        .current_dir(javascript_oracle)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "JS oracle failed for {effect}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bytes = fs::read(&path).unwrap();
    let _ = fs::remove_file(&path);
    let _ = fs::remove_file(&input_path);
    decode_png(&bytes).unwrap()
}

#[test]
#[ignore = "requires a noisemaker-for-cpu checkout; set NOISEMAKER_JS_CPU_DIR and use --ignored"]
fn three_effect_js_oracle_parity_has_at_most_two_byte_delta() {
    let javascript_oracle = javascript_oracle_dir();
    for effect in ["synth/solid", "filter/invert", "synth/noise"] {
        let rust = rust_effect(effect).to_rgba8();
        let js = js_effect(effect, &javascript_oracle).to_rgba8();
        assert_eq!(rust.len(), js.len(), "shape mismatch for {effect}");
        let (index, delta) = rust
            .iter()
            .zip(&js)
            .enumerate()
            .map(|(index, (&rust, &js))| (index, (i16::from(rust) - i16::from(js)).unsigned_abs()))
            .max_by_key(|(_, delta)| *delta)
            .unwrap();
        assert!(
            delta <= 2,
            "{effect}: max delta {delta} at channel {index}: Rust={}, JS={}",
            rust[index],
            js[index]
        );
    }
}
