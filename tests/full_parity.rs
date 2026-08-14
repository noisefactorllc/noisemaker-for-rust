use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use noisemaker_cpu::{Surface, encode_png};
use serde_json::Value;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

fn temp_dir(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "noisemaker-rust-full-parity-{label}-{}-{}",
        std::process::id(),
        NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

#[cfg(unix)]
fn executable(path: &Path, source: &str) {
    fs::write(path, source).unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

fn schema_report(extra: &[&str]) -> std::process::Output {
    Command::new("python3")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/scripts/parity.py"))
        .arg("--schema-test")
        .args(extra)
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .output()
        .expect("run parity schema mode")
}

#[test]
fn parity_schema_mode_has_exact_sorted_205_record_denominator_and_metrics() {
    let output = schema_report(&[]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["catalog"], 205);
    assert_eq!(
        report["catalog"].as_u64(),
        Some(report["compared"].as_u64().unwrap() + report["unsupported"].as_u64().unwrap())
    );
    assert_eq!(report["failed"], 0);
    assert_eq!(report["tolerance"], 2);
    let results = report["results"].as_array().unwrap();
    assert_eq!(results.len(), 205);
    let ids = results
        .iter()
        .map(|record| record["id"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(ids.iter().copied().collect::<BTreeSet<_>>().len(), 205);
    for record in results {
        match record["status"].as_str().unwrap() {
            "compared" => {
                assert!(record["max_delta"].is_u64());
                assert!(record["mean_delta"].is_f64());
                assert!(record["differing_channels"].is_u64());
                assert!(record["channels_over_2"].is_u64());
                assert!(record["pass"].is_boolean());
            }
            "unsupported" => assert!(!record["reason"].as_str().unwrap().is_empty()),
            status => panic!("unexpected status {status}"),
        }
    }
}

#[test]
fn parity_schema_validation_rejects_threshold_failure_and_reasonless_unsupported() {
    for fault in ["delta", "reason"] {
        let output = schema_report(&["--schema-fault", fault]);
        assert!(
            !output.status.success(),
            "fault {fault} unexpectedly passed"
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains("parity validation failed"));
    }
}

#[test]
fn production_rust_sources_never_delegate_to_oracle_processes() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut stack = vec![root];
    while let Some(path) = stack.pop() {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
                let source = std::fs::read_to_string(&path).unwrap();
                for forbidden in [
                    "Command::new(\"node\")",
                    "Command::new(\"python",
                    "noisemaker-for-cpu",
                ] {
                    assert!(
                        !source.contains(forbidden),
                        "{} contains {forbidden}",
                        path.display()
                    );
                }
            }
        }
    }
}

#[cfg(unix)]
#[test]
fn parity_invokes_both_cli_processes_and_classifies_invalid_png_and_timeout_as_errors() {
    let root = temp_dir("fake-clis");
    let rust_ok = root.join("rust-ok");
    executable(
        &rust_ok,
        "#!/bin/sh\ninput=\noutput=\nwhile [ $# -gt 0 ]; do\n case \"$1\" in\n  --input) input=$2; shift 2;;\n  --output) output=$2; shift 2;;\n  *) shift;;\n esac\ndone\ncp \"$input\" \"$output\"\n",
    );
    let js_ok = root.join("js-ok.mjs");
    fs::write(
        &js_ok,
        "import fs from 'node:fs'; let input, output; for (let i=2;i<process.argv.length;i++) { if (process.argv[i] === '--input') input=process.argv[++i]; else if (process.argv[i] === '--output') output=process.argv[++i]; } fs.copyFileSync(input, output);",
    )
    .unwrap();

    let run = |rust: &Path, label: &str, timeout: &str| {
        let report = root.join(format!("{label}.json"));
        let output = Command::new("python3")
            .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/scripts/parity.py"))
            .args(["--rust"])
            .arg(rust)
            .args(["--js"])
            .arg(&js_ok)
            .args(["--only", "synth/solid", "--timeout", timeout, "--json"])
            .arg(&report)
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .output()
            .unwrap();
        (
            output,
            serde_json::from_slice::<Value>(&fs::read(report).unwrap()).unwrap(),
        )
    };

    let (success, report) = run(&rust_ok, "success", "2");
    assert!(success.status.success());
    assert_eq!(report["compared"], 1);
    assert_eq!(report["errors"], 0);
    assert_eq!(report["results"][0]["max_delta"], 0);

    let bad_crc = root.join("bad-crc.png");
    let mut bad_crc_bytes = encode_png(&Surface::new(8, 8).unwrap()).unwrap();
    let idat = bad_crc_bytes
        .windows(4)
        .position(|window| window == b"IDAT")
        .unwrap();
    let length = u32::from_be_bytes(bad_crc_bytes[idat - 4..idat].try_into().unwrap()) as usize;
    bad_crc_bytes[idat + 4 + length] ^= 0xff;
    fs::write(&bad_crc, bad_crc_bytes).unwrap();
    let rust_invalid = root.join("rust-invalid");
    executable(
        &rust_invalid,
        &format!(
            "#!/bin/sh\noutput=\nwhile [ $# -gt 0 ]; do if [ \"$1\" = --output ]; then output=$2; shift 2; else shift; fi; done\ncp '{}' \"$output\"\n",
            bad_crc.display()
        ),
    );
    let (invalid, report) = run(&rust_invalid, "invalid", "2");
    assert!(!invalid.status.success());
    assert_eq!(report["errors"], 1);
    assert_eq!(report["failed"], 1);
    assert_eq!(report["results"][0]["status"], "error");
    assert!(
        report["results"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("PNG CRC")
    );

    let rust_timeout = root.join("rust-timeout");
    executable(&rust_timeout, "#!/bin/sh\nwhile :; do :; done\n");
    let (timeout, report) = run(&rust_timeout, "timeout", "0.05");
    assert!(!timeout.status.success());
    assert_eq!(report["errors"], 1);
    assert!(
        report["results"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("timeout")
    );

    let one_pixel = root.join("one.png");
    fs::write(
        &one_pixel,
        encode_png(&Surface::new(1, 1).unwrap()).unwrap(),
    )
    .unwrap();
    let rust_wrong_size = root.join("rust-wrong-size");
    executable(
        &rust_wrong_size,
        &format!(
            "#!/bin/sh\noutput=\nwhile [ $# -gt 0 ]; do if [ \"$1\" = --output ]; then output=$2; shift 2; else shift; fi; done\ncp '{}' \"$output\"\n",
            one_pixel.display()
        ),
    );
    let (wrong_size, report) = run(&rust_wrong_size, "wrong-size", "2");
    assert!(!wrong_size.status.success());
    assert_eq!(report["errors"], 1);
    assert!(
        report["results"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("size mismatch")
    );

    fs::remove_dir_all(root).unwrap();
}
