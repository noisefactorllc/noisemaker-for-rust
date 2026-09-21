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

#[test]
fn export_kit_config_is_valid_and_matches_catalog() {
    let root = repository_root();
    let config_path = root.join("export-kit/kit.config.json");
    assert!(config_path.is_file(), "missing export-kit/kit.config.json");
    let config: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&config_path).expect("read kit.config.json"))
            .expect("valid json in kit.config.json");
    assert_eq!(config["id"], "rust");
    assert_eq!(config["compat"]["mode"], "list");
    let metadata_rel = config["compat"]["fromBundleMetadata"]
        .as_str()
        .expect("compat.fromBundleMetadata string");
    let metadata_path = root.join(metadata_rel);
    assert!(metadata_path.is_file(), "missing {metadata_rel}");
    let metadata: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&metadata_path).expect("read catalog.json"))
            .expect("valid json in catalog.json");
    let effects = metadata["effects"]
        .as_object()
        .expect("effects map in catalog.json");
    assert_eq!(effects.len(), 205, "expected 205 catalog effects");
    for effect_id in [
        "points/heightGrid",
        "render/renderLandscape3d",
        "synth3d/heightmap3d",
    ] {
        assert!(
            effects.contains_key(effect_id),
            "expected {effect_id} in catalog"
        );
    }
    for (id, effect) in effects {
        assert!(
            !effect["func"].as_str().unwrap_or("").is_empty(),
            "effect {id} should declare a non-empty func"
        );
        assert!(
            !effect["domain"].as_str().unwrap_or("").is_empty(),
            "effect {id} should declare a non-empty domain"
        );
    }
}
