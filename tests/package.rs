use std::process::Command;
use std::{fs, path::PathBuf};

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn package_binary_prints_usage_without_arguments() {
    let output = Command::new(env!("CARGO_BIN_EXE_noisemaker-rs"))
        .output()
        .expect("noisemaker-rs should start");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Usage:"));
    assert!(output.stderr.is_empty());
}

#[test]
fn package_metadata_and_user_facing_files_are_complete() {
    let root = repository_root();
    let manifest = fs::read_to_string(root.join("Cargo.toml")).expect("read Cargo.toml");
    for required in [
        "description =",
        "readme = \"README.md\"",
        "repository =",
        "include = [",
        "\"src/**\"",
        "\"scripts/transpiler/**\"",
        "\"examples/**\"",
        "\"docs/EFFECTS.md\"",
    ] {
        assert!(
            manifest.contains(required),
            "Cargo.toml is missing {required}"
        );
    }

    for relative in [
        "README.md",
        "docs/EFFECTS.md",
        "examples/render_effect.rs",
        "examples/render_dsl.rs",
    ] {
        assert!(root.join(relative).is_file(), "missing {relative}");
    }
}
