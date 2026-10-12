use std::error::Error;
use std::fs;
use std::io::{self, Read};
use std::path::Path;

use noisemaker_cpu::{CpuRenderer, OneShot, RenderOptions, encode_png, external_inputs_for_case};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 5 {
        return Err("usage: parity_sequence OUT_DIR SIZE SEED CASE_ID TIME...".into());
    }
    let output = Path::new(&args[0]);
    let size: u32 = args[1].parse()?;
    let seed: i32 = args[2].parse()?;
    let case_id = &args[3];
    let times = &args[4..];
    fs::create_dir_all(output)?;

    let mut program = String::new();
    io::stdin().read_to_string(&mut program)?;
    let fixture = [51, 102, 153, 255].repeat(usize::try_from(size)? * usize::try_from(size)?);
    let mut previous = noisemaker_cpu::Surface::from_rgba8(size, size, &fixture)?;
    let mut renderer = CpuRenderer::new()?;
    let mut seconds = Vec::with_capacity(times.len());
    for (index, text) in times.iter().enumerate() {
        let mut options = RenderOptions {
            width: size,
            height: size,
            time: text.parse()?,
            frame: index.try_into()?,
            seed,
            external_inputs: external_inputs_for_case(case_id),
            one_shot: OneShot::Initial,
            ..RenderOptions::default()
        };
        options
            .external_textures
            .insert("imageTex".into(), previous.clone());
        options
            .external_textures
            .insert("textTex".into(), previous.clone());
        let rendered = renderer.render(&program, &options)?;
        seconds.push(rendered.elapsed.as_secs_f64());
        previous = rendered.surface;
        fs::write(
            output.join(format!("frame_{index}.png")),
            encode_png(&previous)?,
        )?;
    }
    let timings = seconds
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");
    println!("[{timings}]");
    Ok(())
}
