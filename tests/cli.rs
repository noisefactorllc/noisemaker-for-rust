use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use noisemaker_cpu::decode_png;

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

fn temp_dir(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "noisemaker-rs-cli-{label}-{}-{}",
        std::process::id(),
        NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_noisemaker-rs"))
}

fn assert_png(path: &Path, width: u32, height: u32) {
    let surface = decode_png(&fs::read(path).unwrap()).unwrap();
    assert_eq!((surface.width(), surface.height()), (width, height));
}

#[test]
fn no_args_help_version_and_effect_inventory_are_real() {
    let no_args = cli().output().unwrap();
    assert!(no_args.status.success());
    assert!(String::from_utf8_lossy(&no_args.stdout).contains("Usage:"));

    let help = cli().arg("--help").output().unwrap();
    assert!(help.status.success());
    let help = String::from_utf8_lossy(&help.stdout);
    for command in [
        "generate", "apply", "animate", "run", "render", "effect", "effects",
    ] {
        assert!(help.contains(command), "missing {command} in {help}");
    }

    let version = cli().arg("--version").output().unwrap();
    assert!(version.status.success());
    assert_eq!(
        String::from_utf8_lossy(&version.stdout).trim(),
        "noisemaker-rs 0.1.0"
    );

    let effects = cli().arg("effects").output().unwrap();
    assert!(effects.status.success());
    let lines = String::from_utf8_lossy(&effects.stdout)
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 205);
    assert!(lines.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(lines.contains(&"synth/solid\tgenerator".to_owned()));
}

#[test]
fn generate_random_run_stdin_and_apply_write_valid_atomic_pngs() {
    let root = temp_dir("render");
    let generated = root.join("nested/input.png");
    let output = cli()
        .args([
            "generate",
            "synth/solid",
            "--width",
            "3",
            "--height",
            "2",
            "--output",
        ])
        .arg(&generated)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_png(&generated, 3, 2);

    let random_a = root.join("random-a.png");
    let random_b = root.join("random-b.png");
    let first = cli()
        .args([
            "generate", "random", "--seed", "-7", "--width", "2", "--height", "2", "--output",
        ])
        .arg(&random_a)
        .output()
        .unwrap();
    let second = cli()
        .args([
            "generate", "random", "--seed", "-7", "--width", "2", "--height", "2", "--output",
        ])
        .arg(&random_b)
        .output()
        .unwrap();
    assert!(first.status.success());
    assert!(second.status.success());
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(fs::read(random_a).unwrap(), fs::read(random_b).unwrap());

    let stdin_png = root.join("stdin.png");
    let mut child = cli()
        .args(["run", "-", "--width", "2", "--height", "2", "--output"])
        .arg(&stdin_png)
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"search synth; solid(color:#369).write(o0)")
        .unwrap();
    assert!(child.wait().unwrap().success());
    assert_png(&stdin_png, 2, 2);

    let applied = root.join("applied.png");
    let apply = cli()
        .args(["apply", "filter/invert"])
        .arg(&generated)
        .args(["--output"])
        .arg(&applied)
        .output()
        .unwrap();
    assert!(
        apply.status.success(),
        "{}",
        String::from_utf8_lossy(&apply.stderr)
    );
    assert_png(&applied, 3, 2);

    let text_applied = root.join("text-applied.png");
    let text = cli()
        .args(["apply", "filter/text"])
        .arg(&generated)
        .args(["--output"])
        .arg(&text_applied)
        .output()
        .unwrap();
    assert!(
        text.status.success(),
        "{}",
        String::from_utf8_lossy(&text.stderr)
    );
    assert_png(&text_applied, 3, 2);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn apply_accepts_decimal_schema_endpoints_after_dsl_f32_coercion() {
    let root = temp_dir("decimal-endpoints");
    let input = root.join("input.png");
    let generated = cli()
        .args([
            "generate",
            "synth/solid",
            "--width",
            "2",
            "--height",
            "2",
            "--output",
        ])
        .arg(&input)
        .output()
        .unwrap();
    assert!(generated.status.success());

    for (effect, parameter) in [
        ("filter/simpleAberration", "displacement=0.1"),
        ("filter/celShading", "edgeThreshold=0.01"),
    ] {
        let output_path = root.join(effect.replace('/', "-") + ".png");
        let output = cli()
            .args(["apply", effect])
            .arg(&input)
            .args(["--param", parameter, "--output"])
            .arg(&output_path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{effect}:{parameter}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_png(&output_path, 2, 2);
    }

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn validation_and_render_failures_preserve_existing_destinations_and_temps() {
    let root = temp_dir("atomic-failure");
    let target = root.join("kept.png");
    fs::write(&target, b"old destination").unwrap();

    let invalid = cli()
        .args(["run", "-", "--output"])
        .arg(&target)
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    let mut invalid = invalid;
    invalid
        .stdin
        .take()
        .unwrap()
        .write_all(b"not valid dsl")
        .unwrap();
    let status = invalid.wait().unwrap();
    assert!(!status.success());
    assert_eq!(fs::read(&target).unwrap(), b"old destination");
    assert_eq!(fs::read_dir(&root).unwrap().count(), 1);

    for args in [
        vec!["generate", "missing/effect"],
        vec!["generate", "synth/solid", "--param", "bad"],
        vec!["generate", "synth/solid", "--width", "0"],
        vec!["apply", "synth/noise3d", "missing.png"],
    ] {
        let output = cli().args(args).output().unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("noisemaker-rs:"));
    }

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn missing_ffmpeg_keeps_requested_frames_or_fails_without_them() {
    let root = temp_dir("ffmpeg");
    let frames = root.join("frames");
    let movie = root.join("movie.mp4");
    let saved = cli()
        .env("NOISEMAKER_FFMPEG", root.join("absent-ffmpeg"))
        .args([
            "animate",
            "synth/solid",
            "--width",
            "2",
            "--height",
            "2",
            "--frame-count",
            "1",
            "--save-frames",
        ])
        .arg(&frames)
        .args(["--output"])
        .arg(&movie)
        .output()
        .unwrap();
    assert!(
        saved.status.success(),
        "{}",
        String::from_utf8_lossy(&saved.stderr)
    );
    assert_png(&frames.join("frame_0000.png"), 2, 2);
    assert!(!movie.exists());

    let failed = cli()
        .env("NOISEMAKER_FFMPEG", root.join("absent-ffmpeg"))
        .args([
            "animate",
            "synth/solid",
            "--width",
            "2",
            "--height",
            "2",
            "--frame-count",
            "1",
            "--output",
        ])
        .arg(&movie)
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("ffmpeg not found"));
    assert!(!movie.exists());

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn animate_binds_input_texture_into_saved_media_frames() {
    let root = temp_dir("animate-media");
    let input = root.join("input.png");
    let generated = cli()
        .args([
            "generate",
            "synth/solid",
            "--width",
            "2",
            "--height",
            "2",
            "--param",
            "color=#c30",
            "--output",
        ])
        .arg(&input)
        .output()
        .unwrap();
    assert!(generated.status.success());

    let frames = root.join("frames");
    let movie = root.join("movie.mp4");
    let animated = cli()
        .env("NOISEMAKER_FFMPEG", root.join("absent-ffmpeg"))
        .args(["animate", "synth/media", "--input"])
        .arg(&input)
        .args([
            "--width",
            "2",
            "--height",
            "2",
            "--frame-count",
            "1",
            "--save-frames",
        ])
        .arg(&frames)
        .arg("--output")
        .arg(&movie)
        .output()
        .unwrap();
    assert!(
        animated.status.success(),
        "{}",
        String::from_utf8_lossy(&animated.stderr)
    );
    assert_eq!(
        decode_png(&fs::read(frames.join("frame_0000.png")).unwrap()).unwrap(),
        decode_png(&fs::read(&input).unwrap()).unwrap()
    );
    assert!(!movie.exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn schema_validation_precedes_apply_input_reads_and_animate_frame_artifacts() {
    let root = temp_dir("validation-precedence");
    let missing_input = root.join("absent.png");
    let output = root.join("output.png");
    let apply = cli()
        .args(["apply", "filter/invert"])
        .arg(&missing_input)
        .args(["--param", "unknown=1", "--output"])
        .arg(&output)
        .output()
        .unwrap();
    let apply_error = String::from_utf8_lossy(&apply.stderr);
    assert!(!apply.status.success());
    assert!(apply_error.contains("Unknown parameter"), "{apply_error}");
    assert!(!apply_error.contains("absent.png"), "{apply_error}");
    assert!(!output.exists());

    let frames = root.join("frames");
    let movie = root.join("movie.mp4");
    let animate = cli()
        .env("NOISEMAKER_FFMPEG", root.join("absent-ffmpeg"))
        .args([
            "animate",
            "synth/solid",
            "--param",
            "color=not-a-color",
            "--frame-count",
            "1",
            "--save-frames",
        ])
        .arg(&frames)
        .arg("--output")
        .arg(&movie)
        .output()
        .unwrap();
    let animate_error = String::from_utf8_lossy(&animate.stderr);
    assert!(!animate.status.success());
    assert!(animate_error.contains("<cli-animate>"), "{animate_error}");
    assert!(!animate_error.contains("ffmpeg"), "{animate_error}");
    assert!(
        !frames.exists(),
        "validation created the save-frames directory"
    );
    assert!(!movie.exists());
    assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn public_mashup_effect_uses_seeded_surface_inputs_instead_of_black_fallback() {
    let root = temp_dir("mashup-surfaces");
    let output = root.join("mashup.png");
    let rendered = cli()
        .args([
            "effect",
            "mixer/mashup",
            "--width",
            "2",
            "--height",
            "2",
            "--output",
        ])
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        rendered.status.success(),
        "{}",
        String::from_utf8_lossy(&rendered.stderr)
    );
    let surface = decode_png(&fs::read(&output).unwrap()).unwrap();
    assert!(
        surface
            .data()
            .chunks_exact(4)
            .any(|pixel| pixel[..3].iter().any(|channel| *channel > 0.0)),
        "mashup output used an unbound black source"
    );
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn ffmpeg_receives_literal_mp4_format_for_extensionless_atomic_temp() {
    let root = temp_dir("ffmpeg-success");
    let fake = root.join("fake-ffmpeg");
    let args_log = root.join("args.txt");
    fs::write(
        &fake,
        format!(
            "#!/bin/sh\nprevious=\nhas_format=0\nfor arg do\n  printf '%s\\n' \"$arg\" >> '{}'\n  if [ \"$previous\" = -f ] && [ \"$arg\" = mp4 ]; then has_format=1; fi\n  previous=$arg\n  output=$arg\ndone\n[ \"$has_format\" -eq 1 ] || exit 23\nprintf 'new-mp4' > \"$output\"\n",
            args_log.display()
        ),
    )
    .unwrap();
    let mut permissions = fs::metadata(&fake).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&fake, permissions).unwrap();
    let movie = root.join("movie.mp4");
    fs::write(&movie, b"old-mp4").unwrap();

    let output = cli()
        .env("NOISEMAKER_FFMPEG", &fake)
        .args([
            "animate",
            "synth/solid",
            "--width",
            "2",
            "--height",
            "2",
            "--frame-count",
            "1",
            "--output",
        ])
        .arg(&movie)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(&movie).unwrap(), b"new-mp4");
    let args = fs::read_to_string(&args_log).unwrap();
    assert!(args.contains("-f\nmp4\n"), "{args}");
    assert!(args.contains("frame_%04d.png"), "{args}");
    assert_eq!(
        fs::read_dir(&root)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp-"))
            .count(),
        0
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn file_and_alias_commands_params_and_textures_follow_cli_contract() {
    let root = temp_dir("aliases");
    let program = root.join("program.dsl");
    fs::write(&program, "search synth; solid(color:#c30).write(o0)").unwrap();
    let file_png = root.join("file.png");
    let file = cli()
        .arg("run")
        .arg(&program)
        .args(["--width", "2", "--height", "2", "--filename"])
        .arg(&file_png)
        .output()
        .unwrap();
    assert!(
        file.status.success(),
        "{}",
        String::from_utf8_lossy(&file.stderr)
    );
    assert_png(&file_png, 2, 2);

    let render_png = root.join("render.png");
    let mut render = cli()
        .args(["render", "-", "--width", "2", "--height", "2", "--output"])
        .arg(&render_png)
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    render
        .stdin
        .take()
        .unwrap()
        .write_all(b"search synth; solid(color:#369).write(o0)")
        .unwrap();
    assert!(render.wait().unwrap().success());

    let effect_png = root.join("effect.png");
    let effect = cli()
        .args([
            "effect", "solid", "--width", "2", "--height", "2", "--output",
        ])
        .arg(&effect_png)
        .output()
        .unwrap();
    assert!(
        effect.status.success(),
        "{}",
        String::from_utf8_lossy(&effect.stderr)
    );

    for (effect_id, name) in [
        ("mixer/mashup", "mashup.png"),
        ("points/flow", "flow.png"),
        ("filter3d/palette3d", "palette3d.png"),
        ("render/loopBegin", "loop.png"),
    ] {
        let path = root.join(name);
        let output = cli()
            .args([
                "effect", effect_id, "--width", "2", "--height", "2", "--output",
            ])
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{effect_id}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_png(&path, 2, 2);
    }

    let duplicate = root.join("duplicate.png");
    let expected = root.join("expected.png");
    for (path, params) in [
        (&duplicate, vec!["color=#f00", "color=#00f"]),
        (&expected, vec!["color=#00f"]),
    ] {
        let mut command = cli();
        command.args(["generate", "synth/solid", "--width", "2", "--height", "2"]);
        for param in params {
            command.args(["--param", param]);
        }
        let output = command.arg("--output").arg(path).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    assert_eq!(fs::read(duplicate).unwrap(), fs::read(expected).unwrap());

    let red = root.join("red.png");
    let blue = root.join("blue.png");
    for (path, color) in [(&red, "#f00"), (&blue, "#00f")] {
        assert!(
            cli()
                .args([
                    "generate",
                    "synth/solid",
                    "--width",
                    "2",
                    "--height",
                    "2",
                    "--param"
                ])
                .arg(format!("color={color}"))
                .arg("--output")
                .arg(path)
                .status()
                .unwrap()
                .success()
        );
    }
    let media = root.join("media.png");
    let media_output = cli()
        .args(["generate", "synth/media", "--width", "2", "--height", "2"])
        .args(["--texture", &format!("imageTex={}", red.display())])
        .args(["--texture", &format!("imageTex={}", blue.display())])
        .arg("--output")
        .arg(&media)
        .output()
        .unwrap();
    assert!(
        media_output.status.success(),
        "{}",
        String::from_utf8_lossy(&media_output.stderr)
    );
    assert_eq!(fs::read(media).unwrap(), fs::read(blue).unwrap());

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn every_validation_branch_fails_before_output_mutation() {
    let root = temp_dir("validation");
    let missing = root.join("missing.png");
    let cases = [
        vec!["generate", "synth/solid", "--param", "unknown=1"],
        vec!["generate", "filter/invert"],
        vec![
            "generate",
            "synth/solid",
            "--width",
            "4097",
            "--height",
            "4096",
        ],
        vec!["generate", "synth/solid", "--time", "NaN"],
        vec!["generate", "synth/solid", "--texture", "imageTex="],
        vec!["generate", "synth/media"],
        vec!["animate", "synth/solid", "--frame-count", "0"],
        vec!["animate", "synth/solid", "--fps", "0"],
        vec!["apply", "synth/solid", "absent.png"],
        vec!["apply", "filter3d/palette3d", "absent.png"],
        vec!["apply", "render/loopBegin", "absent.png"],
    ];
    for args in cases {
        let output = cli()
            .args(&args)
            .arg("--output")
            .arg(&missing)
            .output()
            .unwrap();
        assert!(!output.status.success(), "unexpected success: {args:?}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("noisemaker-rs:"),
            "missing stable prefix for {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!missing.exists(), "failure created output for {args:?}");
        assert_eq!(
            fs::read_dir(&root).unwrap().count(),
            0,
            "failure left temp for {args:?}"
        );
    }

    let parse_failures = [
        vec!["wat"],
        vec!["generate"],
        vec!["effects", "extra"],
        vec!["effects", "--bogus"],
        vec!["generate", "synth/solid", "--width", "nope"],
    ];
    for args in parse_failures {
        assert!(!cli().args(args).status().unwrap().success());
    }
    fs::remove_dir_all(root).unwrap();
}
