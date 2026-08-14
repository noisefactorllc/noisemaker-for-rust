use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use clap::{CommandFactory, Parser, Subcommand};

use crate::catalog::{EffectDefinition, effect_catalog};
use crate::{
    CpuRenderer, MAX_SURFACE_PIXELS, OneShot, ParamValue, RenderOptions, RenderStep, Surface,
    compile_dsl, decode_png, encode_png, render_effect,
};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Parser)]
#[command(
    name = "noisemaker-rs",
    version,
    about = "Native CPU Noisemaker renderer"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<CliCommand>,

    #[arg(long, global = true, default_value_t = 512)]
    width: u32,
    #[arg(long, global = true, default_value_t = 512)]
    height: u32,
    #[arg(long, global = true, default_value_t = 0.0)]
    time: f32,
    #[arg(long, global = true, default_value_t = 1, allow_hyphen_values = true)]
    seed: i32,
    #[arg(long, global = true, alias = "filename")]
    output: Option<PathBuf>,
    #[arg(long, global = true)]
    input: Option<PathBuf>,
    #[arg(long, global = true, action = clap::ArgAction::Append)]
    texture: Vec<String>,
    #[arg(long, global = true, action = clap::ArgAction::Append)]
    param: Vec<String>,
    #[arg(long, global = true, hide = true, default_value = "ready")]
    one_shot: String,
}

#[derive(Subcommand)]
enum CliCommand {
    Generate {
        effect: String,
    },
    Apply {
        effect: String,
        input_png: PathBuf,
    },
    Animate {
        effect: String,
        #[arg(long, default_value_t = 50)]
        frame_count: u32,
        #[arg(long, default_value_t = 30)]
        fps: u32,
        #[arg(long, default_value_t = 1.0)]
        speed: f32,
        #[arg(long)]
        save_frames: Option<PathBuf>,
    },
    Run {
        program: Option<PathBuf>,
    },
    Render {
        program: PathBuf,
    },
    Effect {
        effect: String,
    },
    Effects,
}

pub fn run() -> Result<(), String> {
    let cli = Cli::parse();
    if cli.command.is_none() {
        Cli::command()
            .print_help()
            .map_err(|error| error.to_string())?;
        println!();
        return Ok(());
    }
    validate_common(&cli)?;
    match cli.command.as_ref().expect("command checked above") {
        CliCommand::Effects => print_effects(),
        CliCommand::Generate { effect } => generate(&cli, effect),
        CliCommand::Apply { effect, input_png } => apply(&cli, effect, input_png),
        CliCommand::Animate {
            effect,
            frame_count,
            fps,
            speed,
            save_frames,
        } => animate(
            &cli,
            effect,
            *frame_count,
            *fps,
            *speed,
            save_frames.as_deref(),
        ),
        CliCommand::Run { program } => render_program(&cli, program.as_deref()),
        CliCommand::Render { program } => render_program(&cli, Some(program)),
        CliCommand::Effect { effect } => render_effect_command(&cli, effect),
    }
}

fn validate_common(cli: &Cli) -> Result<(), String> {
    if cli.width == 0 || cli.height == 0 {
        return Err("width and height must be positive".into());
    }
    let pixels = u64::from(cli.width) * u64::from(cli.height);
    if pixels > MAX_SURFACE_PIXELS {
        return Err(format!(
            "surface has {pixels} pixels; maximum is {MAX_SURFACE_PIXELS}"
        ));
    }
    if !cli.time.is_finite() {
        return Err("time must be finite".into());
    }
    if !matches!(cli.one_shot.as_str(), "initial" | "ready") {
        return Err("one-shot must be initial or ready".into());
    }
    for assignment in cli.param.iter().chain(cli.texture.iter()) {
        parse_assignment(assignment)?;
    }
    Ok(())
}

fn print_effects() -> Result<(), String> {
    for (id, effect) in &effect_catalog().map_err(|error| error.to_string())?.effects {
        println!("{id}\t{}", effect.kind);
    }
    Ok(())
}

fn generate(cli: &Cli, requested: &str) -> Result<(), String> {
    let resolved = if requested == "random" {
        deterministic_random_effect(cli.seed)?
    } else {
        resolve_effect(requested)?.0
    };
    let (_, definition) = resolve_effect(&resolved)?;
    if definition.kind != "generator" || definition.domain != "image" {
        return Err(format!("{resolved} is not an image generator"));
    }
    let source = image_program(&resolved, definition, &cli.param, false)?;
    require_external_texture(cli, definition)?;
    if requested == "random" {
        println!("{resolved}");
    }
    render_source_to_png(
        cli,
        &source,
        cli.output.as_deref().unwrap_or(Path::new("art.png")),
    )
}

fn deterministic_random_effect(seed: i32) -> Result<String, String> {
    let catalog = effect_catalog().map_err(|error| error.to_string())?;
    let eligible = catalog
        .effects
        .iter()
        .filter(|(_, effect)| {
            effect.kind == "generator"
                && effect.domain == "image"
                && !effect.iterated
                && effect.external_texture.is_none()
        })
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    let index = seed.rem_euclid(eligible.len() as i32) as usize;
    Ok(eligible[index].clone())
}

fn apply(cli: &Cli, requested: &str, input_path: &Path) -> Result<(), String> {
    let (id, definition) = resolve_effect(requested)?;
    if definition.domain != "image" || !matches!(definition.kind.as_str(), "filter" | "mixer") {
        return Err(format!("{id} is not an image filter or mixer"));
    }
    let params = compiled_params(&id, definition, &cli.param, true)?;
    let input = read_png(input_path)?;
    let mut inputs = load_external_textures(cli)?;
    inputs.insert("inputTex".into(), input.clone());
    inputs.insert("imageTex".into(), input.clone());
    inputs.insert("textTex".into(), input.clone());
    for (name, spec) in &definition.params {
        if spec.parameter_type == "surface" && !inputs.contains_key(name) {
            inputs.insert(name.clone(), input.clone());
        }
    }
    let options = options(cli, input.width(), input.height(), cli.time)?;
    let output =
        render_effect(&id, &params, &inputs, &options).map_err(|error| error.to_string())?;
    atomic_write(
        cli.output.as_deref().unwrap_or(Path::new("mangled.png")),
        &encode_png(&output).map_err(|error| error.to_string())?,
    )
}

fn animate(
    cli: &Cli,
    requested: &str,
    frame_count: u32,
    fps: u32,
    speed: f32,
    save_frames: Option<&Path>,
) -> Result<(), String> {
    if frame_count == 0 || fps == 0 {
        return Err("frame-count and fps must be positive".into());
    }
    if !speed.is_finite() {
        return Err("speed must be finite".into());
    }
    let (id, definition) = resolve_effect(requested)?;
    if definition.kind != "generator" || definition.domain != "image" {
        return Err(format!("{id} is not an image generator"));
    }
    let source = image_program(&id, definition, &cli.param, false)?;
    compile_dsl(&source, "<cli-animate>").map_err(|error| error.to_string())?;
    require_external_texture(cli, definition)?;
    let external_textures = load_external_textures(cli)?;
    let temporary_frames;
    let frame_directory = if let Some(path) = save_frames {
        fs::create_dir_all(path).map_err(|error| error.to_string())?;
        path
    } else {
        temporary_frames = unique_directory("noisemaker-rs-frames")?;
        temporary_frames.as_path()
    };
    let render_result = (|| {
        let mut renderer = CpuRenderer::new().map_err(|error| error.to_string())?;
        let mut options = options(cli, cli.width, cli.height, 0.0)?;
        options.external_textures = external_textures;
        for frame in 0..frame_count {
            options.frame = frame;
            options.time = frame as f32 / frame_count as f32 * speed;
            let surface = renderer
                .render(&source, &options)
                .map_err(|error| error.to_string())?
                .surface;
            let path = frame_directory.join(format!("frame_{frame:04}.png"));
            atomic_write(
                &path,
                &encode_png(&surface).map_err(|error| error.to_string())?,
            )?;
        }
        encode_animation(
            frame_directory,
            fps,
            cli.output.as_deref().unwrap_or(Path::new("animation.mp4")),
            save_frames.is_some(),
        )
    })();
    if save_frames.is_none() {
        let _ = fs::remove_dir_all(frame_directory);
    }
    render_result
}

fn encode_animation(
    frames: &Path,
    fps: u32,
    output: &Path,
    keep_frames: bool,
) -> Result<(), String> {
    if let Some(parent) = output.parent().filter(|path| !path.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let temporary = exclusive_sibling_temp(output)?;
    let ffmpeg = std::env::var_os("NOISEMAKER_FFMPEG").unwrap_or_else(|| OsString::from("ffmpeg"));
    let result = Command::new(ffmpeg)
        .current_dir(frames)
        .args(["-y", "-framerate", &fps.to_string(), "-i", "frame_%04d.png"])
        .args([
            "-vf",
            "pad=ceil(iw/2)*2:ceil(ih/2)*2",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-f",
            "mp4",
        ])
        .arg(&temporary)
        .stdin(Stdio::null())
        .output();
    match result {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && keep_frames => {
            let _ = fs::remove_file(&temporary);
            println!("ffmpeg not found; kept PNG frames in {}", frames.display());
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let _ = fs::remove_file(&temporary);
            Err(
                "ffmpeg not found; install it, or pass --save-frames DIR to keep the PNG frames."
                    .into(),
            )
        }
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            Err(format!("failed to start ffmpeg: {error}"))
        }
        Ok(result) if !result.status.success() => {
            let _ = fs::remove_file(&temporary);
            let stderr = String::from_utf8_lossy(&result.stderr);
            let tail = stderr
                .lines()
                .rev()
                .take(5)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("\n");
            Err(format!("ffmpeg failed:\n{tail}"))
        }
        Ok(_) => rename_temp(&temporary, output),
    }
}

fn render_program(cli: &Cli, program: Option<&Path>) -> Result<(), String> {
    let source = match program {
        None => read_stdin()?,
        Some(path) if path == Path::new("-") => read_stdin()?,
        Some(path) => fs::read_to_string(path).map_err(|error| error.to_string())?,
    };
    render_source_to_png(
        cli,
        &source,
        cli.output.as_deref().unwrap_or(Path::new("art.png")),
    )
}

fn render_effect_command(cli: &Cli, requested: &str) -> Result<(), String> {
    let (id, definition) = resolve_effect(requested)?;
    let source = effect_program(&id, definition, &cli.param)?;
    require_external_texture(cli, definition)?;
    render_source_to_png(
        cli,
        &source,
        cli.output.as_deref().unwrap_or(Path::new("art.png")),
    )
}

fn require_external_texture(cli: &Cli, definition: &EffectDefinition) -> Result<(), String> {
    let Some(required) = definition.external_texture.as_deref() else {
        return Ok(());
    };
    let supplied_by_input = cli.input.is_some() && matches!(required, "imageTex" | "textTex");
    let supplied_by_name = cli.texture.iter().any(|assignment| {
        parse_assignment(assignment)
            .map(|(name, _)| name == required)
            .unwrap_or(false)
    });
    if supplied_by_input || supplied_by_name {
        Ok(())
    } else {
        Err(format!(
            "{}/{} requires external texture {required:?}; pass --input or --texture {required}=PATH",
            definition.namespace, definition.func
        ))
    }
}

fn render_source_to_png(cli: &Cli, source: &str, output: &Path) -> Result<(), String> {
    // Compile before loading unrelated texture paths so schema/DSL failures are cheap and deterministic.
    compile_dsl(source, "<cli>").map_err(|error| error.to_string())?;
    let mut options = options(cli, cli.width, cli.height, cli.time)?;
    options.external_textures = load_external_textures(cli)?;
    let surface = CpuRenderer::new()
        .map_err(|error| error.to_string())?
        .render(source, &options)
        .map_err(|error| error.to_string())?
        .surface;
    atomic_write(
        output,
        &encode_png(&surface).map_err(|error| error.to_string())?,
    )
}

fn options(cli: &Cli, width: u32, height: u32, time: f32) -> Result<RenderOptions, String> {
    Ok(RenderOptions {
        width,
        height,
        time,
        seed: cli.seed,
        one_shot: if cli.one_shot == "initial" {
            OneShot::Initial
        } else {
            OneShot::Ready
        },
        ..RenderOptions::default()
    })
}

fn resolve_effect(requested: &str) -> Result<(String, &'static EffectDefinition), String> {
    let catalog = effect_catalog().map_err(|error| error.to_string())?;
    if let Some(effect) = catalog.effects.get(requested) {
        return Ok((requested.into(), effect));
    }
    let matches = catalog
        .effects
        .iter()
        .filter(|(_, effect)| effect.func == requested)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [(id, effect)] => Ok(((*id).clone(), *effect)),
        [] => Err(format!("Unknown effect {requested:?}")),
        _ => Err(format!(
            "Ambiguous effect {requested:?}; use one of: {}",
            matches
                .iter()
                .map(|(id, _)| id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn image_program(
    id: &str,
    definition: &EffectDefinition,
    assignments: &[String],
    seed_input: bool,
) -> Result<String, String> {
    let args = effect_arguments(definition, assignments)?;
    let call = format!("{}({args})", definition.func);
    let chain = if seed_input || definition.kind != "generator" {
        format!("solid().{call}.write(o0)")
    } else {
        format!("{call}.write(o0)")
    };
    Ok(format!(
        "search synth,{},{}; {chain}",
        definition.namespace,
        id.split('/').next().unwrap_or("synth")
    ))
}

fn effect_program(
    id: &str,
    definition: &EffectDefinition,
    assignments: &[String],
) -> Result<String, String> {
    let mut bounded = assignments.to_vec();
    let mut supplied = assignments
        .iter()
        .filter_map(|assignment| parse_assignment(assignment).ok().map(|(name, _)| name))
        .map(|name| {
            definition
                .param_aliases
                .get(name)
                .map(String::as_str)
                .unwrap_or(name)
                .to_owned()
        })
        .collect::<std::collections::BTreeSet<_>>();
    let mut inject = |name: &str, value: String| {
        if definition.params.contains_key(name) && supplied.insert(name.into()) {
            bounded.push(format!("{name}={value}"));
        }
    };
    if definition.iterated {
        inject(
            "iterationCount",
            if definition.domain == "image" && definition.namespace != "points" {
                "4"
            } else {
                "1"
            }
            .into(),
        );
    }
    inject("volumeSize", "2".into());
    inject("stateSize", "64".into());
    if definition.namespace == "synth3d" && definition.func == "flythrough3d" {
        inject("type", "1".into());
    }

    let volume_size = resolved_integer_parameter(definition, &bounded, "volumeSize", 2)?;
    let scalar_args = effect_arguments(definition, &bounded)?;
    let mut args = if scalar_args.is_empty() {
        Vec::new()
    } else {
        vec![scalar_args]
    };
    let surfaces = if definition.kind == "generator" {
        Vec::new()
    } else {
        definition
            .param_names
            .iter()
            .filter(|name| definition.params[*name].parameter_type == "surface")
            .cloned()
            .collect::<Vec<_>>()
    };
    let mut prefix = Vec::new();
    let surface_count = surfaces.len();
    if surface_count > 0 {
        prefix.push("solid(color:#58c).write(o0)".into());
    }
    for (index, name) in surfaces.into_iter().enumerate() {
        if index == 0 {
            args.push(format!("{name}:inputTex"));
        } else {
            let surface = index - 1;
            if surface > 0 {
                prefix.push(format!("solid(color:#58c).write(o{surface})"));
            }
            args.push(format!("{name}:o{surface}"));
        }
    }
    let call = format!("{}({})", definition.func, args.join(","));
    let search = format!(
        "search {},synth,synth3d,filter3d,filter,render,points,mixer,classicNoisedeck",
        definition.namespace,
    );
    let chain = match definition.domain.as_str() {
        "image"
            if definition.namespace == "points"
                || matches!(
                    id,
                    "render/pointsEmit" | "render/pointsRender" | "render/pointsBillboardRender"
                ) =>
        {
            let middle = if id == "render/pointsEmit" {
                call
            } else {
                format!("pointsEmit(stateSize:64,iterationCount:1).{call}")
            };
            let renderer = if matches!(id, "render/pointsRender" | "render/pointsBillboardRender") {
                ""
            } else {
                ".pointsRender(iterationCount:1)"
            };
            format!("solid().{middle}{renderer}.write(o0)")
        }
        "image" if definition.kind == "generator" => format!("{call}.write(o0)"),
        "image" if surface_count > 0 => {
            format!("read(o0).{call}.write(o7)")
        }
        "image" => format!("solid().{call}.write(o7)"),
        "volume-generator" => {
            format!("{call}.render3d(volumeSize:{volume_size}).write(o0)")
        }
        "volume-filter" => {
            format!(
                "noise3d(volumeSize:{volume_size}).{call}.render3d(volumeSize:{volume_size}).write(o0)"
            )
        }
        "volume-renderer" => {
            format!("noise3d(volumeSize:{volume_size}).{call}.write(o0)")
        }
        "loop-begin" => format!("solid().{call}.loopEnd().write(o0)"),
        "loop-end" => format!("solid().loopBegin(iterationCount:1).{call}.write(o0)"),
        _ => {
            return Err(format!(
                "{id} has unsupported CLI domain {:?}",
                definition.domain
            ));
        }
    };
    prefix.push(chain);
    Ok(format!("{search}; {}", prefix.join("; ")))
}

fn compiled_params(
    id: &str,
    definition: &EffectDefinition,
    assignments: &[String],
    seed_input: bool,
) -> Result<BTreeMap<String, ParamValue>, String> {
    let source = image_program(id, definition, assignments, seed_input)?;
    let plan = compile_dsl(&source, "<cli-params>").map_err(|error| error.to_string())?;
    plan.chains
        .iter()
        .flat_map(|chain| &chain.steps)
        .find_map(|step| match step {
            RenderStep::Effect {
                effect_id, params, ..
            } if effect_id == id => Some(params.clone()),
            _ => None,
        })
        .ok_or_else(|| format!("failed to compile parameters for {id}"))
}

fn effect_arguments(
    definition: &EffectDefinition,
    assignments: &[String],
) -> Result<String, String> {
    let mut values = BTreeMap::new();
    for assignment in assignments {
        let (supplied, value) = parse_assignment(assignment)?;
        let name = definition
            .param_aliases
            .get(supplied)
            .map(String::as_str)
            .unwrap_or(supplied);
        let spec = definition.params.get(name).ok_or_else(|| {
            format!(
                "Unknown parameter {supplied:?} for {}/{}",
                definition.namespace, definition.func
            )
        })?;
        if matches!(
            spec.parameter_type.as_str(),
            "surface" | "volume" | "geometry"
        ) {
            return Err(format!(
                "Parameter {name:?} must be supplied with --texture"
            ));
        }
        let literal = match spec.parameter_type.as_str() {
            "string" => serde_json::to_string(value).map_err(|error| error.to_string())?,
            "vec2" | "vec3" | "vec4" | "mat3" if !value.trim().starts_with('[') => {
                format!("[{}]", value.trim())
            }
            _ => value.trim().to_owned(),
        };
        values.insert(name.to_owned(), literal);
    }
    Ok(values
        .into_iter()
        .map(|(name, value)| format!("{name}:{value}"))
        .collect::<Vec<_>>()
        .join(","))
}

fn resolved_integer_parameter(
    definition: &EffectDefinition,
    assignments: &[String],
    requested: &str,
    fallback: i64,
) -> Result<i64, String> {
    let mut selected = None;
    for assignment in assignments {
        let (supplied, value) = parse_assignment(assignment)?;
        let name = definition
            .param_aliases
            .get(supplied)
            .map(String::as_str)
            .unwrap_or(supplied);
        if name == requested {
            selected = Some(value.trim());
        }
    }
    let Some(value) = selected else {
        return Ok(fallback);
    };
    if let Ok(value) = value.parse::<i64>() {
        return Ok(value);
    }
    let spec = definition
        .params
        .get(requested)
        .ok_or_else(|| format!("Unknown parameter {requested:?}"))?;
    spec.metadata
        .get("choices")
        .and_then(serde_json::Value::as_object)
        .and_then(|choices| choices.get(value))
        .and_then(serde_json::Value::as_i64)
        .ok_or_else(|| format!("Unknown integer choice {value:?} for {requested}"))
}

fn parse_assignment(assignment: &str) -> Result<(&str, &str), String> {
    let Some((name, value)) = assignment.split_once('=') else {
        return Err(format!("Expected NAME=VALUE, received {assignment:?}"));
    };
    if name.is_empty() || value.is_empty() {
        return Err(format!("Expected NAME=VALUE, received {assignment:?}"));
    }
    Ok((name, value))
}

fn load_external_textures(cli: &Cli) -> Result<BTreeMap<String, Surface>, String> {
    let mut textures = BTreeMap::new();
    if let Some(path) = &cli.input {
        let surface = read_png(path)?;
        textures.insert("imageTex".into(), surface.clone());
        textures.insert("textTex".into(), surface);
    }
    for assignment in &cli.texture {
        let (name, path) = parse_assignment(assignment)?;
        textures.insert(name.into(), read_png(Path::new(path))?);
    }
    Ok(textures)
}

fn read_png(path: &Path) -> Result<Surface, String> {
    decode_png(&fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?)
        .map_err(|error| format!("{}: {error}", path.display()))
}

fn read_stdin() -> Result<String, String> {
    let mut source = String::new();
    std::io::stdin()
        .read_to_string(&mut source)
        .map_err(|error| error.to_string())?;
    Ok(source)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent().filter(|path| !path.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let temporary = exclusive_sibling_temp(path)?;
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&temporary)
            .map_err(|error| error.to_string())?;
        file.write_all(bytes).map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        drop(file);
        rename_temp(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn exclusive_sibling_temp(path: &Path) -> Result<PathBuf, String> {
    let parent = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let basename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("output");
    for _ in 0..128 {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_nanos();
        let count = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let candidate = parent.join(format!(
            "{basename}.tmp-{}-{nonce:x}-{count:x}",
            std::process::id()
        ));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(_) => return Ok(candidate),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.to_string()),
        }
    }
    Err("could not create an exclusive sibling temporary file".into())
}

fn rename_temp(temporary: &Path, output: &Path) -> Result<(), String> {
    fs::rename(temporary, output).map_err(|error| error.to_string())
}

fn unique_directory(prefix: &str) -> Result<PathBuf, String> {
    for _ in 0..128 {
        let count = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("{prefix}-{}-{count}", std::process::id()));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.to_string()),
        }
    }
    Err("could not create temporary frame directory".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flock_effect_program_uses_a_real_bounded_particle_owner_chain() {
        let catalog = effect_catalog().unwrap();
        let definition = &catalog.effects["points/flock"];
        let source = effect_program("points/flock", definition, &[]).unwrap();

        assert!(source.contains("pointsEmit(stateSize:64,iterationCount:1)"));
        assert!(source.contains(".flock(iterationCount:1)"));
        assert!(source.contains(".pointsRender(iterationCount:1)"));
        compile_dsl(&source, "<cli-particle-test>").unwrap();
    }

    #[test]
    fn volume_effect_program_uses_one_resolved_size_for_owner_effect_and_renderer() {
        let catalog = effect_catalog().unwrap();

        let noise = effect_program(
            "synth3d/noise3d",
            &catalog.effects["synth3d/noise3d"],
            &["volumeSize=3".into()],
        )
        .unwrap();
        assert!(
            noise.contains("noise3d(volumeSize:3).render3d(volumeSize:3)"),
            "{noise}"
        );
        compile_dsl(&noise, "<cli-volume-generator-test>").unwrap();

        let palette = effect_program(
            "filter3d/palette3d",
            &catalog.effects["filter3d/palette3d"],
            &["volumeSize=3".into()],
        )
        .unwrap();
        assert!(
            palette
                .contains("noise3d(volumeSize:3).palette3d(volumeSize:3).render3d(volumeSize:3)"),
            "{palette}"
        );
        compile_dsl(&palette, "<cli-volume-filter-test>").unwrap();

        let choice = effect_program(
            "synth3d/noise3d",
            &catalog.effects["synth3d/noise3d"],
            &["volumeSize=x16".into()],
        )
        .unwrap();
        assert!(choice.contains("noise3d(volumeSize:x16).render3d(volumeSize:16)"));
        compile_dsl(&choice, "<cli-volume-choice-test>").unwrap();
    }

    #[test]
    fn mashup_effect_program_seeds_and_uniquely_wires_all_nine_surfaces() {
        let catalog = effect_catalog().unwrap();
        let definition = &catalog.effects["mixer/mashup"];
        let source = effect_program("mixer/mashup", definition, &[]).unwrap();
        let surfaces = definition
            .param_names
            .iter()
            .filter(|name| definition.params[*name].parameter_type == "surface")
            .collect::<Vec<_>>();
        assert_eq!(surfaces.len(), 9);
        assert!(source.contains("solid(color:#58c).write(o0)"), "{source}");
        assert!(source.contains("read(o0).mashup("), "{source}");
        assert!(
            source.contains(&format!("{}:inputTex", surfaces[0])),
            "{source}"
        );
        for (index, name) in surfaces.into_iter().enumerate().skip(1) {
            assert!(
                source.contains(&format!("{name}:o{}", index - 1)),
                "{source}"
            );
        }
        compile_dsl(&source, "<cli-mashup-test>").unwrap();
    }
}
