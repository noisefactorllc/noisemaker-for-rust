#![allow(
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::manual_clamp
)]

use std::collections::BTreeMap;

use crate::catalog::{Expression, RenderPass, shader_bundle};
use crate::{RenderError, Surface, Value};

pub const FRAGMENT_ADAPTER_KEYS: [&str; 6] = [
    "classicNoisedeck/fractal:fractal",
    "filter/crt:crt",
    "filter/historicPalette:historicPalette",
    "filter/median:median",
    "filter/palette:palette",
    "synth/julia:julia",
];

#[must_use]
pub const fn fragment_adapter_keys() -> [&'static str; 6] {
    FRAGMENT_ADAPTER_KEYS
}

#[must_use]
pub const fn palette_dimensions() -> (usize, usize) {
    (55, 16)
}

#[must_use]
pub const fn historic_palette_dimensions() -> (usize, usize) {
    (21, 15)
}

pub(crate) fn apply_classic_palette_uniforms(
    palette_index: i32,
    uniforms: &mut BTreeMap<String, Value>,
) -> Result<(), RenderError> {
    let Ok(index) = usize::try_from(palette_index - 1) else {
        return Ok(());
    };
    let table = adapter_table("filter/palette:palette", "PALETTES")?;
    let Some(entry) = table.get(index) else {
        return Ok(());
    };
    uniforms.insert("paletteAmp".into(), Value::Vec(entry[0..3].to_vec()));
    uniforms.insert("paletteFreq".into(), Value::Vec(entry[4..7].to_vec()));
    uniforms.insert("paletteOffset".into(), Value::Vec(entry[8..11].to_vec()));
    uniforms.insert("palettePhase".into(), Value::Vec(entry[12..15].to_vec()));
    uniforms.insert(
        "paletteMode".into(),
        Value::Int(if entry[3] == 0.0 { 3 } else { entry[3] as i32 }),
    );
    Ok(())
}

pub(crate) fn execute_fragment_adapter(
    key: &str,
    pass: &RenderPass,
    uniforms: &BTreeMap<String, Value>,
    resources: &BTreeMap<String, Surface>,
    width: u32,
    height: u32,
) -> Result<Option<(String, Surface)>, RenderError> {
    if !FRAGMENT_ADAPTER_KEYS.contains(&key) {
        return Ok(None);
    }
    let output_name =
        pass.outputs
            .values()
            .next()
            .cloned()
            .ok_or_else(|| RenderError::InvalidGraph {
                message: format!("adapter pass {:?} has no output", pass.name),
            })?;
    let mut output = Surface::new(width, height)?;
    match key {
        "classicNoisedeck/fractal:fractal" => render_fractal(uniforms, &mut output)?,
        "filter/palette:palette" => render_palette(uniforms, resources, &mut output, false)?,
        "filter/historicPalette:historicPalette" => {
            render_palette(uniforms, resources, &mut output, true)?
        }
        "synth/julia:julia" => render_julia(uniforms, &mut output)?,
        "filter/crt:crt" => render_crt(
            uniforms,
            semantic_input(pass, resources, "inputTex")?,
            &mut output,
        )?,
        "filter/median:median" => render_median(
            uniforms,
            semantic_input(pass, resources, "inputTex")?,
            &mut output,
        )?,
        _ => unreachable!(),
    }
    Ok(Some((output_name, output)))
}

pub(crate) fn execute_semantic_fragment_adapter(
    key: &str,
    pass: &RenderPass,
    uniforms: &BTreeMap<String, Value>,
    resources: &BTreeMap<String, Surface>,
    width: u32,
    height: u32,
) -> Result<Option<(String, Surface)>, RenderError> {
    if !matches!(
        key,
        "filter/snow:snow"
            | "synth/navierStokes:nsSplat"
            | "render/pointsBillboardRender:depthMerge"
    ) {
        return Ok(None);
    }
    if key == "render/pointsBillboardRender:depthMerge" {
        let source = semantic_input(pass, resources, "orderTex")?;
        if (source.width(), source.height()) != (width, height) {
            return Ok(None);
        }
    }
    let output_name =
        pass.outputs
            .values()
            .next()
            .cloned()
            .ok_or_else(|| RenderError::InvalidGraph {
                message: format!("semantic adapter pass {:?} has no output", pass.name),
            })?;
    let mut output = Surface::new(width, height)?;
    match key {
        "filter/snow:snow" => {
            let input = semantic_input(pass, resources, "inputTex")?;
            render_snow(uniforms, input, &mut output)?;
        }
        "synth/navierStokes:nsSplat" => {
            let buffer = semantic_input(pass, resources, "bufTex")?;
            let black = Surface::new(1, 1)?;
            let input = pass
                .inputs
                .get("inputTex")
                .and_then(|resource| resources.get(resource))
                .unwrap_or(&black);
            render_navier_splat(uniforms, buffer, input, &mut output)?;
        }
        "render/pointsBillboardRender:depthMerge" => {
            let input = semantic_input(pass, resources, "orderTex")?;
            let run_length = match uniforms.get("runLength") {
                Some(Value::Int(value)) => *value as usize,
                _ => return Ok(None),
            };
            if run_length == 0 {
                return Ok(None);
            }
            render_billboard_depth_merge(input, run_length, &mut output);
        }
        _ => unreachable!(),
    }
    Ok(Some((output_name, output)))
}

// depthMerge.frag's bounded merge-path search. GLSL texel indices start at the
// bottom row, while Surface stores the top row first.
fn render_billboard_depth_merge(source: &Surface, run_length: usize, output: &mut Surface) {
    let width = source.width() as usize;
    let height = source.height() as usize;
    let count = width * height;
    if run_length >= count {
        output.data_mut().copy_from_slice(source.data());
        return;
    }
    let key_at = |index: usize| -> [f32; 2] {
        let x = index % width;
        let y = index / width;
        let offset = ((height - 1 - y) * width + x) * 4;
        [source.data()[offset], source.data()[offset + 1]]
    };
    let before = |a: [f32; 2], b: [f32; 2]| a[0] < b[0] || (a[0] == b[0] && a[1] <= b[1]);
    for index in 0..count {
        let start = index / (2 * run_length) * (2 * run_length);
        let length_a = run_length.min(count - start);
        let length_b = run_length.min(count - start - length_a);
        let diagonal = index - start;
        let mut low = diagonal.saturating_sub(length_b);
        let mut high = diagonal.min(length_a);
        for _ in 0..22 {
            if low >= high {
                break;
            }
            let mid = (low + high) / 2;
            let other = diagonal - mid;
            if mid < length_a
                && other > 0
                && before(key_at(start + mid), key_at(start + length_a + other - 1))
            {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        let other = diagonal - low;
        let a = if low < length_a {
            key_at(start + low)
        } else {
            [f32::MAX; 2]
        };
        let b = if other < length_b {
            key_at(start + length_a + other)
        } else {
            [f32::MAX; 2]
        };
        let selected = if before(a, b) { a } else { b };
        let x = index % width;
        let y = index / width;
        let offset = ((height - 1 - y) * width + x) * 4;
        output.data_mut()[offset..offset + 4].copy_from_slice(&[
            selected[0],
            selected[1],
            0.0,
            1.0,
        ]);
    }
}

fn semantic_input<'a>(
    pass: &RenderPass,
    resources: &'a BTreeMap<String, Surface>,
    uniform: &str,
) -> Result<&'a Surface, RenderError> {
    let resource = pass
        .inputs
        .get(uniform)
        .ok_or_else(|| RenderError::InvalidGraph {
            message: format!(
                "semantic adapter pass {:?} has no {uniform} input",
                pass.name
            ),
        })?;
    resources
        .get(resource)
        .ok_or_else(|| RenderError::InvalidGraph {
            message: format!(
                "semantic adapter pass {:?} requires resource {resource:?}",
                pass.name
            ),
        })
}

fn f32_from_js(value: f64) -> f32 {
    value as f32
}

fn fadd(left: f32, right: f32) -> f32 {
    f32_from_js(f64::from(left) + f64::from(right))
}

fn fsub(left: f32, right: f32) -> f32 {
    f32_from_js(f64::from(left) - f64::from(right))
}

fn fmul(left: f32, right: f32) -> f32 {
    f32_from_js(f64::from(left) * f64::from(right))
}

fn fdiv(left: f32, right: f32) -> f32 {
    f32_from_js(f64::from(left) / f64::from(right))
}

fn ffract(value: f32) -> f32 {
    f32_from_js(f64::from(value) - f64::from(value).floor())
}

fn snow_sine(value: f32) -> f32 {
    const TAU: f32 = std::f32::consts::TAU;
    const INV_TAU: f32 = 1.0 / std::f32::consts::TAU;
    let turns = fmul(value, INV_TAU);
    let phase = f64::from(turns) - f64::from(turns).floor();
    f32_from_js((phase * f64::from(TAU)).sin())
}

fn snow_cosine(value: f32) -> f32 {
    f32_from_js(f64::from(value).cos())
}

fn snow_periodic_value(time: f32, value: f32) -> f32 {
    fmul(
        fadd(
            snow_sine(fmul(fsub(time, value), std::f32::consts::TAU)),
            1.0,
        ),
        0.5,
    )
}

fn snow_hash(x: f32, y: f32, z: f32) -> f32 {
    let scale = f32_from_js(0.1031);
    let offset = f32_from_js(33.33);
    let sx = ffract(fmul(x, scale));
    let sy = ffract(fmul(y, scale));
    let sz = ffract(fmul(z, scale));
    let dot = sz.mul_add(
        fadd(sx, offset),
        sy.mul_add(fadd(sz, offset), fmul(sx, fadd(sy, offset))),
    );
    let shifted_xy = f32_from_js(f64::from(fadd(sx, sy)) + 2.0 * f64::from(dot));
    ffract(fmul(shifted_xy, fadd(sz, dot))).clamp(0.0, 1.0)
}

fn navier_hash11(x: f64) -> f32 {
    const SCALE: f32 = 43_758.546_875;
    let scaled = fmul(x as f32, 12.989_800_453_186_035_f32);
    let sine = f64::from(scaled).sin() as f32;
    ffract(fmul(sine, SCALE))
}

/// The CPU oracle folds a GLSL `dot` into a fused-multiply-add chain with one
/// f32 rounding per accumulated term, so the splat adapter accumulates the
/// same way (f64 products of f32 values are exact, and the f64 add keeps the
/// single final rounding of an fma).
fn navier_dot(a: [f32; 2], b: [f32; 2]) -> f32 {
    let mut sum = 0.0_f32;
    for index in 0..2 {
        sum = (f64::from(a[index]) * f64::from(b[index]) + f64::from(sum)) as f32;
    }
    sum
}

fn navier_hash22(p: [f32; 2]) -> [f32; 2] {
    const SCALE: f32 = 43_758.546_875;
    let first = navier_dot(
        [p[0], p[1]],
        [127.099_998_474_121_1_f32, 311.700_012_207_031_25_f32],
    );
    let second = navier_dot(p, [269.5_f32, 183.300_003_051_757_8_f32]);
    [first, second].map(|value| {
        let sine = f64::from(value).sin() as f32;
        let product = (f64::from(sine) * f64::from(SCALE)) as f32;
        (f64::from(product) - f64::from(product).floor()) as f32
    })
}

fn navier_luminance(color: [f32; 4]) -> f64 {
    f64::from(0.212_599_992_752_075_2_f32) * f64::from(color[0])
        + f64::from(0.715_200_006_961_822_5_f32) * f64::from(color[1])
        + f64::from(0.072_200_000_286_102_3_f32) * f64::from(color[2])
}

fn render_navier_splat(
    uniforms: &BTreeMap<String, Value>,
    buffer: &Surface,
    input: &Surface,
    output: &mut Surface,
) -> Result<(), RenderError> {
    let seed = f64::from(scalar(uniforms, "seed")?);
    let speed = f64::from(scalar(uniforms, "speed")?);
    let input_force = f64::from(scalar(uniforms, "inputForce")?);
    let input_dye = f64::from(scalar(uniforms, "inputDye")?);
    let reset_state = boolean(uniforms, "resetState")?;
    let texture_width = buffer.width() as f64;
    let texture_height = buffer.height() as f64;

    for storage_y in 0..output.height() {
        let shader_y = output.height() - 1 - storage_y;
        for x in 0..output.width() {
            let uv = [
                crate::metal_division::metal_divide(f64::from(x as f32 + 0.5), texture_width)
                    as f32,
                crate::metal_division::metal_divide(
                    f64::from(shader_y as f32 + 0.5),
                    texture_height,
                ) as f32,
            ];
            let previous = navier_texture(buffer, uv);
            let mut result = [0.0_f32; 4];

            if reset_state || previous[3] == 0.0 {
                let mut velocity = [0.0_f32; 2];
                let mut dye = 0.0_f64;
                for index in 0..9 {
                    let id = index as f64;
                    let center = navier_hash22([
                        (id * f64::from(7.309_999_942_779_541_f32) + 1.0) as f32,
                        (seed * f64::from(13.699_999_809_265_137_f32) + id) as f32,
                    ]);
                    let sign = if navier_hash11(
                        id * f64::from(4.170_000_076_293_945_f32)
                            + seed * f64::from(5.900_000_095_367_432_f32),
                    ) > 0.5
                    {
                        1.0_f64
                    } else {
                        -1.0_f64
                    };
                    let radius = f64::from(0.100_000_001_490_116_12_f32)
                        + f64::from(0.059_999_998_658_895_49_f32)
                            * f64::from(navier_hash11(
                                id * f64::from(2.109_999_895_095_825_f32) + seed,
                            ));
                    let delta = [
                        (f64::from(uv[0]) - f64::from(center[0])) as f32,
                        (f64::from(uv[1]) - f64::from(center[1])) as f32,
                    ];
                    let mut radius_squared = 0.0_f32;
                    radius_squared = (f64::from(delta[0]) * f64::from(delta[0])
                        + f64::from(radius_squared)) as f32;
                    radius_squared = (f64::from(delta[1]) * f64::from(delta[1])
                        + f64::from(radius_squared)) as f32;
                    let falloff = crate::metal_division::metal_divide(
                        -f64::from(radius_squared),
                        (2.0 * radius) * radius,
                    )
                    .exp() as f32;
                    let tangent = [-delta[1], delta[0]];
                    velocity[0] = (f64::from(velocity[0])
                        + ((f64::from(tangent[0]) * sign) * f64::from(falloff)) * 12.0)
                        as f32;
                    velocity[1] = (f64::from(velocity[1])
                        + ((f64::from(tangent[1]) * sign) * f64::from(falloff)) * 12.0)
                        as f32;
                    dye += f64::from(falloff);
                }
                result.copy_from_slice(&[
                    velocity[0],
                    velocity[1],
                    dye.clamp(0.0, 1.0) as f32,
                    1.0,
                ]);
            } else {
                let mut velocity = [previous[0], previous[1]];
                let mut dye = f64::from(previous[2]);
                let delta_time = f64::from(speed.clamp(0.0, 200.0) as f32)
                    * f64::from(0.000_099_999_997_473_787_52_f32);
                let force = f64::from(input_force.clamp(0.0, 100.0) as f32)
                    * f64::from(0.009_999_999_776_482_582_f32);
                let dye_amount = f64::from(input_dye.clamp(0.0, 100.0) as f32)
                    * f64::from(0.009_999_999_776_482_582_f32);
                if force > 0.0 || dye_amount > 0.0 {
                    let texel = [
                        crate::metal_division::metal_divide(1.0, texture_width) as f32,
                        crate::metal_division::metal_divide(1.0, texture_height) as f32,
                    ];
                    let center_luminance = navier_luminance(navier_texture(input, uv));
                    let right_uv = [(f64::from(uv[0]) + f64::from(texel[0])) as f32, uv[1]];
                    let upper_uv = [uv[0], (f64::from(uv[1]) + f64::from(texel[1])) as f32];
                    let right_luminance = navier_luminance(navier_texture(input, right_uv));
                    let upper_luminance = navier_luminance(navier_texture(input, upper_uv));
                    let gradient = [
                        (right_luminance - center_luminance) as f32,
                        (upper_luminance - center_luminance) as f32,
                    ];
                    velocity[0] =
                        (f64::from(velocity[0]) + (f64::from(gradient[0]) * force) * 50.0) as f32;
                    velocity[1] =
                        (f64::from(velocity[1]) + (f64::from(gradient[1]) * force) * 50.0) as f32;
                    dye += ((center_luminance * dye_amount) * delta_time) * 60.0;
                }
                result.copy_from_slice(&[
                    velocity[0],
                    velocity[1],
                    dye.clamp(0.0, 2.0) as f32,
                    1.0,
                ]);
            }

            let offset = ((storage_y * output.width() + x) * 4) as usize;
            output.data_mut()[offset..offset + 4].copy_from_slice(&result);
        }
    }
    Ok(())
}

fn snow_noise(x: f32, y: f32, time: f32, speed: f32, seed: [f32; 3]) -> f32 {
    const TIME_SEED_OFFSETS: [f32; 3] = [97.0, 57.0, 131.0];
    let angle = fmul(time, std::f32::consts::TAU);
    let cosine = snow_cosine(angle);
    let z_base = fmul(cosine, speed);
    let base = snow_hash(fadd(x, seed[0]), fadd(y, seed[1]), fadd(z_base, seed[2]));
    if speed == 0.0 || time == 0.0 {
        return base;
    }
    let time_value = snow_hash(
        fadd(x, fadd(seed[0], TIME_SEED_OFFSETS[0])),
        fadd(y, fadd(seed[1], TIME_SEED_OFFSETS[1])),
        fadd(1.0, fadd(seed[2], TIME_SEED_OFFSETS[2])),
    );
    let scaled_time = fmul(snow_periodic_value(time, time_value), speed);
    snow_periodic_value(scaled_time, base).clamp(0.0, 1.0)
}

fn render_snow(
    uniforms: &BTreeMap<String, Value>,
    input: &Surface,
    output: &mut Surface,
) -> Result<(), RenderError> {
    const STATIC_SEED: [f32; 3] = [37.0, 17.0, 53.0];
    const LIMITER_SEED: [f32; 3] = [113.0, 71.0, 193.0];
    let alpha = scalar_number(uniforms, "alpha")?.clamp(0.0, 1.0);
    let time = if boolean(uniforms, "pause")? {
        0.0
    } else {
        scalar(uniforms, "time")?
    };
    let density = crt_max(
        f32_from_js(scalar_number(uniforms, "density")? * f64::from(0.01_f32)),
        0.0001_f32,
    );
    let exponent = fdiv(fsub(1.0, density), density);
    for storage_y in 0..output.height() {
        let shader_y = output.height() - 1 - storage_y;
        for x in 0..output.width() {
            let source_offset = adapter_texel_offset(input, f64::from(x), f64::from(shader_y));
            let output_offset = ((storage_y * output.width() + x) * 4) as usize;
            if alpha == 0.0 {
                output.data_mut()[output_offset..output_offset + 4]
                    .copy_from_slice(&input.data()[source_offset..source_offset + 4]);
                continue;
            }
            let fx = x as f32 + 0.5;
            let fy = shader_y as f32 + 0.5;
            let static_value = snow_noise(fx, fy, time, 100.0, STATIC_SEED);
            let limiter_value = snow_noise(fx, fy, time, 100.0, LIMITER_SEED);
            let power = f32_from_js(
                f64::from(limiter_value.min(f32_from_js(0.99))).powf(f64::from(exponent)),
            );
            let limiter_mask = f32_from_js(f64::from(power) * alpha);
            let inverse_mask = fsub(1.0, limiter_mask);
            for channel in 0..3 {
                output.data_mut()[output_offset + channel] = f32_from_js(
                    f64::from(input.data()[source_offset + channel]) * f64::from(inverse_mask)
                        + f64::from(static_value) * f64::from(limiter_mask),
                );
            }
            output.data_mut()[output_offset + 3] = input.data()[source_offset + 3];
        }
    }
    Ok(())
}

fn scalar(uniforms: &BTreeMap<String, Value>, name: &str) -> Result<f32, RenderError> {
    match uniforms.get(name) {
        Some(Value::Float(value)) => Ok(*value as f32),
        Some(Value::Int(value)) => Ok(*value as f32),
        Some(Value::Uint(value)) => Ok(*value as f32),
        value => Err(RenderError::InvalidGraph {
            message: format!("adapter uniform {name:?} is not scalar: {value:?}"),
        }),
    }
}

fn integer(uniforms: &BTreeMap<String, Value>, name: &str) -> Result<i32, RenderError> {
    Ok(scalar(uniforms, name)? as i32)
}

fn boolean(uniforms: &BTreeMap<String, Value>, name: &str) -> Result<bool, RenderError> {
    match uniforms.get(name) {
        Some(Value::Bool(value)) => Ok(*value),
        Some(Value::Int(value)) => Ok(*value != 0),
        value => Err(RenderError::InvalidGraph {
            message: format!("adapter uniform {name:?} is not boolean: {value:?}"),
        }),
    }
}

fn vector<'a>(uniforms: &'a BTreeMap<String, Value>, name: &str) -> Result<&'a [f32], RenderError> {
    match uniforms.get(name) {
        Some(Value::Vec(value)) => Ok(value),
        value => Err(RenderError::InvalidGraph {
            message: format!("adapter uniform {name:?} is not a vector: {value:?}"),
        }),
    }
}

fn map(value: f64, in_min: f64, in_max: f64, out_min: f64, out_max: f64) -> f64 {
    out_min + (out_max - out_min) * (value - in_min) / (in_max - in_min)
}

fn fract(value: f64) -> f64 {
    value - value.floor()
}

fn mix(a: f64, b: f64, amount: f64) -> f64 {
    a * (1.0 - amount) + b * amount
}

fn rotate(x: f64, y: f64, rotation: f64, aspect: f64) -> (f64, f64) {
    const PI: f64 = 3.14159265359;
    let angle = map(rotation, 0.0, 360.0, 0.0, 2.0) * PI;
    let (px, py) = (x - 0.5 * aspect, y - 0.5);
    let (sn, cs) = angle.sin_cos();
    (cs * px + sn * py + 0.5 * aspect, -sn * px + cs * py + 0.5)
}

// Keep exact f64 products between the CPU authority's explicit float32 rounds.
fn fractal_f32(value: f64) -> f64 {
    f64::from(value as f32)
}

fn fractal_map(value: f64, in_min: f64, in_max: f64, out_min: f64, out_max: f64) -> f64 {
    fractal_f32(
        out_min
            + fractal_f32(out_max - out_min)
                * ((value - in_min) * fractal_f32(1.0 / fractal_f32(in_max - in_min))),
    )
}

fn fractal_rotate(x: f64, y: f64, rotation: f64, aspect: f64) -> (f64, f64) {
    let angle = fractal_f32(fractal_map(rotation, 0.0, 360.0, 0.0, 2.0) * 3.1415929794311523);
    let px = fractal_f32(x - fractal_f32(0.5 * aspect));
    let py = fractal_f32(y - 0.5);
    let cs = fractal_f32(angle.cos());
    let sn = fractal_f32(angle.sin());
    (
        fractal_f32(sn * py + fractal_f32(cs * px)),
        fractal_f32(cs * py + fractal_f32(-sn * px)),
    )
}

fn fractal_hsv(h: f64, s: f64, v: f64) -> [f32; 3] {
    let h = fractal_f32(fract(h));
    let c = fractal_f32(v * s);
    let base = fractal_f32(h * 6.0);
    let modulo = fractal_f32(base - fractal_f32(2.0 * fractal_f32(base / 2.0).floor()));
    let x = fractal_f32(c * fractal_f32(1.0 - (modulo - 1.0).abs()));
    let m = fractal_f32(v - c);
    let color = if h < 1.0 / 6.0 {
        [c, x, 0.0]
    } else if h < 2.0 / 6.0 {
        [x, c, 0.0]
    } else if h < 3.0 / 6.0 {
        [0.0, c, x]
    } else if h < 4.0 / 6.0 {
        [0.0, x, c]
    } else if h < 5.0 / 6.0 {
        [x, 0.0, c]
    } else {
        [c, 0.0, x]
    };
    color.map(|channel| (channel + m) as f32)
}

fn render_fractal(
    uniforms: &BTreeMap<String, Value>,
    output: &mut Surface,
) -> Result<(), RenderError> {
    const TAU: f64 = 6.28318530718;
    let full = vector(uniforms, "fullResolution")?;
    let tile = vector(uniforms, "tileOffset")?;
    let aspect = f64::from(full[0]) / f64::from(full[1]);
    let effect_type = integer(uniforms, "type")?;
    let iterations = integer(uniforms, "iterations")?;
    let time = f64::from(scalar(uniforms, "time")?);
    let rotation = f64::from(scalar(uniforms, "rotation")?);
    let zoom = f64::from(scalar(uniforms, "zoomAmt")?);
    let speed_value = f64::from(scalar(uniforms, "speed")?);
    let offset_x = f64::from(scalar(uniforms, "offsetX")?);
    let offset_y = f64::from(scalar(uniforms, "offsetY")?);
    let center_x = f64::from(scalar(uniforms, "centerX")?);
    let center_y = f64::from(scalar(uniforms, "centerY")?);
    let mode = integer(uniforms, "mode")?;
    let classic_palettes = adapter_table("filter/palette:palette", "PALETTES")?;
    let palette_index = integer(uniforms, "palette")?;
    let palette_entry = usize::try_from(palette_index - 1)
        .ok()
        .and_then(|index| classic_palettes.get(index));
    for storage_y in 0..output.height() {
        let shader_y = output.height() - 1 - storage_y;
        for x_index in 0..output.width() {
            let global_x = f64::from(x_index as f32 + 0.5 + tile[0]);
            let global_y = f64::from(shader_y as f32 + 0.5 + tile[1]);
            let (mut x, mut y) = (global_x / f64::from(full[1]), global_y / f64::from(full[1]));
            let mut distance;
            if effect_type == 0 {
                x = fractal_f32(x);
                y = fractal_f32(y);
                let aspect = fractal_f32(aspect);
                let time = fractal_f32(time);
                let zoom = fractal_map(zoom, 0.0, 100.0, 2.0, 0.5);
                let speedy = fractal_map(speed_value, 0.0, 100.0, 0.0, 1.0);
                let speed = fractal_f32(
                    fractal_f32(fractal_f32(speedy * 0.05) * fractal_f32(1.0 - speedy))
                        + fractal_f32(fractal_f32(speedy * 0.125) * speedy),
                );
                let phase = fractal_f32(time * fractal_f32(TAU));
                let cx = fractal_f32(
                    phase.sin() * speed + fractal_map(offset_x, -100.0, 100.0, -0.5, 0.5),
                );
                let cy = fractal_f32(
                    phase.cos() * speed + fractal_map(offset_y, -100.0, 100.0, -1.0, 1.0),
                );
                (x, y) = fractal_rotate(x, y, fractal_f32(rotation), aspect);
                x = fractal_f32(x * zoom + fractal_map(center_x, -100.0, 100.0, 1.0, -1.0));
                y = fractal_f32(y * zoom + fractal_map(center_y, -100.0, 100.0, 1.0, -1.0));
                let count = iterations * 2;
                let mut iteration = 0;
                for index in 0..count {
                    iteration = index;
                    let next_x = fractal_f32(fractal_f32(x * x + cx) - y * y);
                    let next_y = fractal_f32(y * x + x * y + cy);
                    if fractal_f32(next_x * next_x + next_y * next_y) > 4.0 {
                        break;
                    }
                    x = next_x;
                    y = next_y;
                }
                if count - iteration < scalar(uniforms, "cutoff")? as i32 {
                    distance = 1.0;
                } else if mode == 0 {
                    distance = fractal_f32(f64::from(iteration) / f64::from(count));
                } else {
                    distance = fractal_f32(fractal_f32(x * x + y * y).sqrt());
                }
            } else if effect_type == 1 {
                (x, y) = rotate(x, y, rotation + 90.0, aspect);
                x = (x - 0.5 * aspect) * map(zoom, 0.0, 130.0, 1.0, 0.01) + center_y * 0.01;
                y = (y - 0.5) * map(zoom, 0.0, 130.0, 1.0, 0.01) + center_x * 0.01;
                let speed = map(speed_value, 0.0, 100.0, 0.0, 1.0);
                let ox = map(offset_x, -100.0, 100.0, -0.25, 0.25);
                let oy = map(offset_y, -100.0, 100.0, -0.25, 0.25);
                let mut iteration = 0;
                for _ in 0..iterations {
                    let fx = x * x * x - 3.0 * x * y * y - 1.0;
                    let fy = 3.0 * x * x * y - y * y * y;
                    let fpx = 3.0 * x * x - 3.0 * y * y;
                    let fpy = 6.0 * x * y;
                    let denominator = fpx * fpx + fpy * fpy;
                    let tx =
                        (fx * fpx + fy * fpy) / denominator + (time * TAU).sin() * 0.1 * speed + ox;
                    let ty =
                        (fy * fpx - fx * fpy) / denominator + (time * TAU).cos() * 0.1 * speed + oy;
                    if tx.hypot(ty) < 0.001 {
                        break;
                    }
                    x -= tx;
                    y -= ty;
                    iteration += 1;
                }
                distance = if mode == 0 {
                    f64::from(iteration) / f64::from(iterations)
                } else {
                    x.hypot(y)
                };
            } else {
                let zoom = map(zoom, 0.0, 100.0, 2.0, 0.5);
                let speedy = map(speed_value, 0.0, 100.0, 0.0, 1.0);
                let speed = mix(speedy * 0.05, speedy * 0.125, speedy);
                (x, y) = rotate(x, y, rotation, aspect);
                y = y * 2.0 - 1.0;
                x = x * 2.0 - aspect;
                let (cx, cy) = (
                    zoom * x - (center_x + 50.0) * 0.01,
                    zoom * y - center_y * 0.01,
                );
                x = (time * TAU).sin() * speed;
                y = (time * TAU).cos() * speed;
                let mut iteration = 0;
                while iteration < iterations {
                    (x, y) = (x * x - y * y + cx, 2.0 * x * y + cy);
                    if x * x + y * y > 16.0 {
                        break;
                    }
                    iteration += 1;
                }
                distance = if iteration == iterations {
                    1.0
                } else if mode == 0 {
                    f64::from(iteration) / f64::from(iterations)
                } else {
                    x.hypot(y) / f64::from(iterations)
                };
            }
            let offset = ((storage_y * output.width() + x_index) * 4) as usize;
            if distance == 1.0 {
                let bg = vector(uniforms, "bgColor")?;
                output.data_mut()[offset..offset + 4].copy_from_slice(&[
                    bg[0],
                    bg[1],
                    bg[2],
                    scalar(uniforms, "bgAlpha")? * 0.01,
                ]);
                continue;
            }
            if integer(uniforms, "cyclePalette")? == -1 {
                distance = fractal_f32(distance - time);
            } else if integer(uniforms, "cyclePalette")? == 1 {
                distance = fractal_f32(distance + time);
            }
            distance = fractal_f32(fract(fractal_f32(
                distance * f64::from(scalar(uniforms, "repeatPalette")?)
                    + fractal_f32(
                        f64::from(scalar(uniforms, "rotatePalette")?) * fractal_f32(0.01),
                    ),
            )));
            let levels = integer(uniforms, "levels")?;
            if levels > 0 {
                let levels = fractal_f32(f64::from(levels) + 1.0);
                distance = fractal_f32(fractal_f32(distance * levels).floor() / levels);
            }
            let color = match integer(uniforms, "colorMode")? {
                0 => [fract(distance) as f32; 3],
                4 => cosine_palette(distance, uniforms, palette_entry)?,
                6 => fractal_hsv(
                    fractal_f32(
                        distance
                            * fractal_f32(
                                f64::from(scalar(uniforms, "hueRange")?) * fractal_f32(0.01),
                            ),
                    ),
                    1.0,
                    1.0,
                ),
                _ => [0.0, 0.0, 1.0],
            };
            output.data_mut()[offset..offset + 4]
                .copy_from_slice(&[color[0], color[1], color[2], 1.0]);
        }
    }
    Ok(())
}

fn cosine_palette(
    t: f64,
    uniforms: &BTreeMap<String, Value>,
    entry: Option<&Vec<f32>>,
) -> Result<[f32; 3], RenderError> {
    let uniform_offset = vector(uniforms, "paletteOffset")?;
    let uniform_amp = vector(uniforms, "paletteAmp")?;
    let uniform_freq = vector(uniforms, "paletteFreq")?;
    let uniform_phase = vector(uniforms, "palettePhase")?;
    let (amp, freq, offset, phase, mode) = if let Some(entry) = entry {
        (
            &entry[0..3],
            &entry[4..7],
            &entry[8..11],
            &entry[12..15],
            if entry[3] == 0.0 { 3 } else { entry[3] as i32 },
        )
    } else {
        (
            uniform_amp,
            uniform_freq,
            uniform_offset,
            uniform_phase,
            integer(uniforms, "paletteMode")?,
        )
    };
    let mut color = std::array::from_fn(|channel| {
        if mode == 2 {
            (f64::from(offset[channel])
                + f64::from(amp[channel])
                    * (6.28318 * (f64::from(freq[channel]) * t + f64::from(phase[channel]))).cos())
                as f32
        } else {
            let inner = fractal_f32(f64::from(freq[channel]) * t + f64::from(phase[channel]));
            let cosine = fractal_f32(fractal_f32(inner * fractal_f32(6.28318)).cos());
            (f64::from(amp[channel]) * cosine + f64::from(offset[channel])) as f32
        }
    });
    if mode == 1 {
        color = fractal_hsv(
            f64::from(color[0]),
            f64::from(color[1]),
            f64::from(color[2]),
        );
    } else if mode == 2 {
        color = fractal_oklab_to_rgb(color);
    }
    Ok(color)
}

fn hsv_to_rgb(hue: f32, saturation: f32, value: f32) -> [f32; 3] {
    let hue = hue - hue.floor();
    let chroma = value * saturation;
    let x = chroma * (1.0 - ((hue * 6.0).rem_euclid(2.0) - 1.0).abs());
    let m = value - chroma;
    let color = if hue < 1.0 / 6.0 {
        [chroma, x, 0.0]
    } else if hue < 2.0 / 6.0 {
        [x, chroma, 0.0]
    } else if hue < 3.0 / 6.0 {
        [0.0, chroma, x]
    } else if hue < 4.0 / 6.0 {
        [0.0, x, chroma]
    } else if hue < 5.0 / 6.0 {
        [x, 0.0, chroma]
    } else {
        [chroma, 0.0, x]
    };
    color.map(|channel| channel + m)
}

fn fractal_oklab_to_rgb(color: [f32; 3]) -> [f32; 3] {
    let l_value = f64::from(color[0]);
    let a = f64::from(color[1]) * -0.509 + 0.276;
    let b = f64::from(color[2]) * -0.509 + 0.198;
    let l1 = l_value + 0.3963377774 * a + 0.2158037573 * b;
    let m1 = l_value - 0.1055613458 * a - 0.0638541728 * b;
    let s1 = l_value - 0.0894841775 * a - 1.291485548 * b;
    let (l, m, s) = (l1 * l1 * l1, m1 * m1 * m1, s1 * s1 * s1);
    let linear_to_srgb = |value: f64| -> f32 {
        (if value <= 0.0031308 {
            value * 12.92
        } else {
            1.055 * value.powf(1.0 / 2.4) - 0.055
        }) as f32
    };
    [
        linear_to_srgb(4.0767245293 * l - 3.3072168827 * m + 0.2307590544 * s),
        linear_to_srgb(-1.2681437731 * l + 2.6093323231 * m - 0.341134429 * s),
        linear_to_srgb(-0.0041119885 * l - 0.7034763098 * m + 1.7068625689 * s),
    ]
}

fn adapter_table(key: &str, global: &str) -> Result<Vec<Vec<f32>>, RenderError> {
    let shaders = shader_bundle()?;
    let program = &shaders.programs[key];
    let initializer = program
        .ir
        .globals
        .iter()
        .find(|entry| entry.name == global)
        .and_then(|entry| entry.initializer.as_ref())
        .ok_or_else(|| RenderError::InvalidGraph {
            message: format!("adapter table {global} is missing"),
        })?;
    let Expression::Construct { arguments, .. } = initializer else {
        return Err(RenderError::InvalidGraph {
            message: format!("adapter table {global} is malformed"),
        });
    };
    arguments.iter().map(flatten_literals).collect()
}

fn flatten_literals(expression: &Expression) -> Result<Vec<f32>, RenderError> {
    match expression {
        Expression::Literal { value, .. } => match value {
            serde_json::Value::Number(value) => Ok(vec![value.as_f64().unwrap_or_default() as f32]),
            _ => Err(RenderError::InvalidGraph {
                message: "adapter literal is not numeric".into(),
            }),
        },
        Expression::Construct { arguments, .. } => arguments
            .iter()
            .map(flatten_literals)
            .collect::<Result<Vec<_>, _>>()
            .map(|parts| parts.into_iter().flatten().collect()),
        _ => Err(RenderError::InvalidGraph {
            message: "adapter table contains a nonliteral".into(),
        }),
    }
}

fn render_palette(
    uniforms: &BTreeMap<String, Value>,
    resources: &BTreeMap<String, Surface>,
    output: &mut Surface,
    historic: bool,
) -> Result<(), RenderError> {
    let input = resources
        .get("inputTex")
        .ok_or_else(|| RenderError::InvalidGraph {
            message: "palette adapter requires inputTex".into(),
        })?;
    let table = adapter_table(
        if historic {
            "filter/historicPalette:historicPalette"
        } else {
            "filter/palette:palette"
        },
        "PALETTES",
    )?;
    let index = integer(uniforms, "paletteIndex")?;
    for storage_y in 0..output.height() {
        let shader_y = output.height() - 1 - storage_y;
        for x in 0..output.width() {
            let uv = [
                (x as f32 + 0.5) / input.width() as f32,
                (shader_y as f32 + 0.5) / input.height() as f32,
            ];
            let source = navier_texture(input, uv);
            let lum = source[0] * 0.299 + source[1] * 0.587 + source[2] * 0.114;
            let color = if historic {
                historic_color(&table[index.clamp(0, 20) as usize], lum, uniforms)?
            } else if index <= 0 || index as usize > table.len() {
                [source[0], source[1], source[2]]
            } else {
                palette_color(&table[index as usize - 1], lum, uniforms)?
            };
            let alpha = scalar(uniforms, "alpha")?;
            let offset = ((storage_y * output.width() + x) * 4) as usize;
            output.data_mut()[offset..offset + 4].copy_from_slice(&[
                source[0] * (1.0 - alpha) + color[0] * alpha,
                source[1] * (1.0 - alpha) + color[1] * alpha,
                source[2] * (1.0 - alpha) + color[2] * alpha,
                source[3],
            ]);
        }
    }
    Ok(())
}

fn palette_color(
    entry: &[f32],
    lum: f32,
    uniforms: &BTreeMap<String, Value>,
) -> Result<[f32; 3], RenderError> {
    let mut t = lum * scalar(uniforms, "repeat")? + scalar(uniforms, "offset")? * 0.01;
    let rotation = scalar(uniforms, "rotation")?;
    if rotation == -1.0 {
        t += scalar(uniforms, "time")?;
    } else if rotation == 1.0 {
        t -= scalar(uniforms, "time")?;
    }
    let mut color = std::array::from_fn(|channel| {
        (entry[8 + channel]
            + entry[channel]
                * (std::f32::consts::TAU * (entry[4 + channel] * t + entry[12 + channel])).cos())
        .clamp(0.0, 1.0)
    });
    if entry[3] as i32 == 1 {
        color = hsv_to_rgb(color[0], color[1], color[2]);
    } else if entry[3] as i32 == 2 {
        color = oklab_to_rgb(color);
    }
    Ok(color)
}

fn oklab_to_rgb(color: [f32; 3]) -> [f32; 3] {
    let l_value = color[0];
    let a = color[1] * -0.509 + 0.276;
    let b = color[2] * -0.509 + 0.198;
    let l1 = l_value + 0.3963377774 * a + 0.2158037573 * b;
    let m1 = l_value - 0.1055613458 * a - 0.0638541728 * b;
    let s1 = l_value - 0.0894841775 * a - 1.291485548 * b;
    let (l, m, s) = (l1.powi(3), m1.powi(3), s1.powi(3));
    [
        linear_to_srgb(4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s),
        linear_to_srgb(-1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s),
        linear_to_srgb(-0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s),
    ]
    .map(|value| value.clamp(0.0, 1.0))
}

fn linear_to_srgb(value: f32) -> f32 {
    if value <= 0.0031308 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}

fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    if edge0 == edge1 {
        return if value < edge0 { 0.0 } else { 1.0 };
    }
    let amount = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    amount * amount * (3.0 - 2.0 * amount)
}

fn historic_color(
    entry: &[f32],
    lum: f32,
    uniforms: &BTreeMap<String, Value>,
) -> Result<[f32; 3], RenderError> {
    let mut t =
        lum * (1.0 - 1e-4) * scalar(uniforms, "repeat")? + scalar(uniforms, "offset")? * 0.01;
    let rotation = scalar(uniforms, "rotation")?;
    if rotation == -1.0 {
        t += scalar(uniforms, "time")?;
    } else if rotation == 1.0 {
        t -= scalar(uniforms, "time")?;
    }
    let lum = t.rem_euclid(1.0);
    let blend_width = scalar(uniforms, "smoothness")? * 0.1;
    let mut color = [entry[0], entry[1], entry[2]];
    for color_index in 1..5 {
        let threshold = color_index as f32 * 0.2;
        let amount = smoothstep(threshold - blend_width, threshold + blend_width, lum);
        for channel in 0..3 {
            color[channel] =
                color[channel] * (1.0 - amount) + entry[color_index * 3 + channel] * amount;
        }
    }
    if blend_width > 0.0 {
        let distance = if lum > 0.5 { lum - 1.0 } else { lum };
        let wrap_factor = smoothstep(-blend_width, blend_width, distance);
        let wrap_mask = 1.0 - smoothstep(0.0, blend_width, distance.abs());
        for channel in 0..3 {
            let wrap_color =
                entry[12 + channel] * (1.0 - wrap_factor) + entry[channel] * wrap_factor;
            color[channel] = color[channel] * (1.0 - wrap_mask) + wrap_color * wrap_mask;
        }
    }
    Ok(color)
}

#[derive(Clone, Copy)]
struct JuliaIteration {
    iteration: f32,
    z_magnitude2: f32,
    derivative_magnitude2: f32,
    stripe_sum: f32,
    stripe_count: f32,
    stripe_last: f32,
    trap_min: f64,
}

fn df_add(a_high: f32, a_low: f32, b_high: f32, b_low: f32) -> (f32, f32) {
    let sum = a_high + b_high;
    let virtual_value = sum - a_high;
    let error = (a_high - (sum - virtual_value)) + (b_high - virtual_value);
    (sum, error + (a_low + b_low))
}

fn df_multiply(a_high: f32, a_low: f32, b_high: f32, b_low: f32) -> (f32, f32) {
    let product = a_high * b_high;
    let a_temp = 4097.0 * a_high;
    let a_hi = a_temp - (a_temp - a_high);
    let a_lo = a_high - a_hi;
    let b_temp = 4097.0 * b_high;
    let b_hi = b_temp - (b_temp - b_high);
    let b_lo = b_high - b_hi;
    let mut error = a_hi * b_hi - product;
    error += a_hi * b_lo;
    error += a_lo * b_hi;
    error += a_lo * b_lo;
    error += a_high * b_low;
    error += a_low * b_high;
    (product, error)
}

fn julia_transform(
    x: f64,
    y: f64,
    full: &[f32],
    rotation: f32,
    center_x: f32,
    center_y: f32,
    zoom: f64,
) -> [f32; 4] {
    const TAU: f32 = 6.28318530718;
    let extent = f64::from(full[0].min(full[1]));
    let mut uv_x = ((x - 0.5 * f64::from(full[0])) / extent) as f32;
    let mut uv_y = ((y - 0.5 * f64::from(full[1])) / extent) as f32;
    let angle = -rotation * TAU / 360.0;
    let cosine = angle.cos();
    let sine = angle.sin();
    let rotated_x = cosine * uv_x + sine * uv_y;
    uv_y = -sine * uv_x + cosine * uv_y;
    uv_x = rotated_x;
    let scale = (2.5 / zoom) as f32;
    let (re_high, re_low) = df_multiply(uv_x, 0.0, scale, 0.0);
    let (re_high, re_low) = df_add(re_high, re_low, center_x, 0.0);
    let (im_high, im_low) = df_multiply(uv_y, 0.0, scale, 0.0);
    let (im_high, im_low) = df_add(im_high, im_low, center_y, 0.0);
    [re_high, re_low, im_high, im_low]
}

fn julia_iterate(
    coordinates: [f32; 4],
    c_real: f64,
    c_imag: f64,
    max_iterations: i32,
    frequency: f32,
    trap_shape: i32,
) -> JuliaIteration {
    const BAILOUT2: f32 = 256.0 * 256.0;
    let [mut re_high, mut re_low, mut im_high, mut im_low] = coordinates;
    let (mut derivative_x, mut derivative_y) = (1.0_f32, 0.0_f32);
    let mut iteration = 0.0_f32;
    let (mut stripe_sum, mut stripe_last, mut stripe_count) = (0.0_f32, 0.0_f32, 0.0_f32);
    let mut trap_min = 1e10_f64;
    let (mut slow_x, mut slow_y) = (re_high, im_high);
    let mut period = 0;
    for _ in 0..max_iterations.min(1000) {
        let next_derivative_x = 2.0 * (re_high * derivative_x - im_high * derivative_y);
        derivative_y = 2.0 * (re_high * derivative_y + im_high * derivative_x);
        derivative_x = next_derivative_x;

        let (re2_high, re2_low) = df_multiply(re_high, re_low, re_high, re_low);
        let (im2_high, im2_low) = df_multiply(im_high, im_low, im_high, im_low);
        let (product_high, product_low) = df_multiply(re_high, re_low, im_high, im_low);
        let (next_re_high, next_re_low) = df_add(re2_high, re2_low, -im2_high, -im2_low);
        let (next_re_high, next_re_low) = df_add(next_re_high, next_re_low, c_real as f32, 0.0);
        let (next_im_high, next_im_low) = df_multiply(product_high, product_low, 2.0, 0.0);
        let (next_im_high, next_im_low) = df_add(next_im_high, next_im_low, c_imag as f32, 0.0);
        re_high = next_re_high;
        re_low = next_re_low;
        im_high = next_im_high;
        im_low = next_im_low;

        let magnitude2 = re_high * re_high + im_high * im_high;
        if magnitude2 > BAILOUT2 {
            break;
        }
        iteration += 1.0;
        if frequency > 0.0 {
            stripe_last = (0.5
                * (f64::from(frequency) * f64::from(im_high).atan2(f64::from(re_high))).sin()
                + 0.5) as f32;
            stripe_sum += stripe_last;
            stripe_count += 1.0;
        }
        let re = f64::from(re_high);
        let im = f64::from(im_high);
        let trap_distance = match trap_shape {
            0 => re.hypot(im),
            1 => re.abs().min(im.abs()),
            _ => (re.hypot(im) - 1.0).abs(),
        };
        trap_min = trap_min.min(trap_distance);
        period += 1;
        if period == 20 {
            period = 0;
            slow_x = re_high;
            slow_y = im_high;
        } else if f64::from(re_high - slow_x).hypot(f64::from(im_high - slow_y)) < 1e-10 {
            iteration = max_iterations as f32;
            break;
        }
    }
    JuliaIteration {
        iteration,
        z_magnitude2: re_high * re_high + im_high * im_high,
        derivative_magnitude2: derivative_x * derivative_x + derivative_y * derivative_y,
        stripe_sum,
        stripe_count,
        stripe_last,
        trap_min,
    }
}

fn clamp01(value: f64) -> f64 {
    if value < 0.0 {
        0.0
    } else if value > 1.0 {
        1.0
    } else {
        value
    }
}

fn julia_smooth_iteration(result: JuliaIteration, max_iterations: i32) -> f64 {
    const LOG2: f64 = 0.6931471805599453;
    if result.iteration >= max_iterations as f32 {
        return 0.0;
    }
    let log_magnitude = f64::from(result.z_magnitude2).ln() * 0.5;
    let nu = (log_magnitude / LOG2).ln() / LOG2;
    clamp01((f64::from(result.iteration) + 1.0 - nu) / f64::from(max_iterations))
}

fn render_julia(
    uniforms: &BTreeMap<String, Value>,
    output: &mut Surface,
) -> Result<(), RenderError> {
    const TAU: f64 = 6.28318530718;
    const LOG2: f64 = 0.6931471805599453;
    const POI_VALUES: [[f64; 2]; 11] = [
        [-0.123, 0.745],
        [-0.123, 0.745],
        [-0.3905, 0.5868],
        [0.0, 1.0],
        [-1.0, 0.0],
        [-0.7455, 0.113],
        [-0.0986, 0.6534],
        [-0.8, 0.156],
        [-0.75, 0.0],
        [-0.5792, 0.5385],
        [0.28, 0.008],
    ];
    let full = vector(uniforms, "fullResolution")?;
    let tile = vector(uniforms, "tileOffset")?;
    let time = f64::from(scalar(uniforms, "time")?);
    let poi = integer(uniforms, "poi")?;
    let c_path = integer(uniforms, "cPath")?;
    let theta = time * f64::from(scalar(uniforms, "cSpeed")?) * TAU;
    let constant = if poi > 0 {
        POI_VALUES
            .get(poi as usize)
            .copied()
            .unwrap_or(POI_VALUES[0])
    } else {
        match c_path {
            1 => [
                theta.cos() * 0.5 - (2.0 * theta).cos() * 0.25,
                theta.sin() * 0.5 - (2.0 * theta).sin() * 0.25,
            ],
            2 => [
                theta.cos() * f64::from(scalar(uniforms, "cRadius")?),
                theta.sin() * f64::from(scalar(uniforms, "cRadius")?),
            ],
            3 => [-1.0 + theta.cos() * 0.25, theta.sin() * 0.25],
            _ => [
                f64::from(scalar(uniforms, "cReal")?),
                f64::from(scalar(uniforms, "cImag")?),
            ],
        }
    };
    let zoom_speed = f64::from(scalar(uniforms, "zoomSpeed")?);
    let zoom_depth = f64::from(scalar(uniforms, "zoomDepth")?);
    let zoom = if zoom_speed > 0.0 {
        let phase = 0.5 * (1.0 - (time * zoom_speed * TAU).cos());
        10.0_f64.powf(zoom_depth * phase)
    } else {
        10.0_f64.powf(zoom_depth)
    };
    let rotation = scalar(uniforms, "rotation")?;
    let center_x = scalar(uniforms, "centerX")?;
    let center_y = scalar(uniforms, "centerY")?;
    let max_iterations = integer(uniforms, "iterations")?;
    let output_mode = integer(uniforms, "outputMode")?;
    let stripe_frequency = scalar(uniforms, "stripeFreq")?;
    let trap_shape = integer(uniforms, "trapShape")?;
    let invert = boolean(uniforms, "invert")?;
    for storage_y in 0..output.height() {
        let shader_y = output.height() - 1 - storage_y;
        for x in 0..output.width() {
            let global_x = f64::from(x as f32 + 0.5 + tile[0]);
            let global_y = f64::from(shader_y as f32 + 0.5 + tile[1]);
            let iterate_smooth = |sample_x: f64, sample_y: f64| {
                let coordinates =
                    julia_transform(sample_x, sample_y, full, rotation, center_x, center_y, zoom);
                julia_smooth_iteration(
                    julia_iterate(
                        coordinates,
                        constant[0],
                        constant[1],
                        max_iterations,
                        0.0,
                        0,
                    ),
                    max_iterations,
                )
            };
            let mut value = if output_mode == 4 {
                let base = iterate_smooth(global_x, global_y);
                let mut normal_x = iterate_smooth(global_x + 1.0, global_y) - base;
                let mut normal_y = iterate_smooth(global_x, global_y + 1.0) - base;
                let mut normal_z = 0.05;
                let magnitude = normal_x.hypot(normal_y).hypot(normal_z);
                normal_x /= magnitude;
                normal_y /= magnitude;
                normal_z /= magnitude;
                let angle = f64::from(scalar(uniforms, "lightAngle")?) * TAU / 360.0;
                let (mut light_x, mut light_y, mut light_z) = (angle.cos(), angle.sin(), 0.7_f64);
                let magnitude = light_x.hypot(light_y).hypot(light_z);
                light_x /= magnitude;
                light_y /= magnitude;
                light_z /= magnitude;
                clamp01((normal_x * light_x + normal_y * light_y + normal_z * light_z).max(0.0))
            } else {
                let coordinates =
                    julia_transform(global_x, global_y, full, rotation, center_x, center_y, zoom);
                let result = julia_iterate(
                    coordinates,
                    constant[0],
                    constant[1],
                    max_iterations,
                    stripe_frequency,
                    trap_shape,
                );
                match output_mode {
                    0 => julia_smooth_iteration(result, max_iterations),
                    1 if result.iteration >= max_iterations as f32 => 0.0,
                    1 => {
                        let magnitude = f64::from(result.z_magnitude2).sqrt();
                        let derivative = f64::from(result.derivative_magnitude2).sqrt();
                        if derivative < 1e-10 {
                            0.0
                        } else {
                            clamp01(
                                (2.0 * magnitude * magnitude.ln() / derivative + 1.0).ln() * 2.0,
                            )
                        }
                    }
                    2 if result.iteration >= max_iterations as f32 || result.stripe_count < 1.0 => {
                        0.0
                    }
                    2 => {
                        let average = f64::from(result.stripe_sum) / f64::from(result.stripe_count);
                        let previous = if result.stripe_count > 1.0 {
                            f64::from(result.stripe_sum - result.stripe_last)
                                / f64::from(result.stripe_count - 1.0)
                        } else {
                            average
                        };
                        let log_magnitude = f64::from(result.z_magnitude2).ln() * 0.5;
                        let nu = (log_magnitude / LOG2).ln() / LOG2;
                        let amount = clamp01(1.0 - nu + nu.floor());
                        clamp01(previous * (1.0 - amount) + average * amount)
                    }
                    3 => {
                        if result.iteration >= max_iterations as f32 {
                            0.0
                        } else {
                            clamp01(1.0 - result.trap_min)
                        }
                    }
                    _ => julia_smooth_iteration(result, max_iterations),
                }
            };
            if invert {
                value = 1.0 - value;
            }
            let value = value as f32;
            let offset = ((storage_y * output.width() + x) * 4) as usize;
            output.data_mut()[offset..offset + 4].copy_from_slice(&[value, value, value, 1.0]);
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "adapters_sync_tests.rs"]
mod cpu_adapter_sync;

fn navier_texture(surface: &Surface, uv: [f32; 2]) -> [f32; 4] {
    if surface.filter_mode() == crate::FilterMode::Linear {
        crate::sample_bilinear(surface, uv[0], uv[1])
    } else {
        crate::sampler::sample_nearest_texel(surface, f64::from(uv[0]), f64::from(uv[1]))
    }
}

// GLSL texelFetch and snow's direct fetch coerce through JavaScript ToInt32.
fn adapter_int32(value: f64) -> i32 {
    if !value.is_finite() || value == 0.0 {
        return 0;
    }
    value.trunc().rem_euclid(4_294_967_296.0) as u32 as i32
}
fn adapter_texel_offset(surface: &Surface, x: f64, y: f64) -> usize {
    let x = adapter_int32(x).clamp(0, surface.width() as i32 - 1) as u32;
    let y = adapter_int32(y).clamp(0, surface.height() as i32 - 1) as u32;
    (((surface.height() - 1 - y) * surface.width() + x) * 4) as usize
}
fn adapter_fetch(surface: &Surface, x: f32, y: f32) -> [f32; 4] {
    let offset = adapter_texel_offset(surface, f64::from(x), f64::from(y));
    surface.data()[offset..offset + 4].try_into().unwrap()
}

fn median_half(value: f32) -> u32 {
    if value.is_nan() {
        return 0x7e00;
    }
    let bits = value.to_bits();
    let sign = (bits >> 16) & 0x8000;
    let mut exponent = ((bits >> 23) & 0xff) as i32 - 127 + 15;
    let mut fraction = bits & 0x7fffff;
    if exponent <= 0 {
        if exponent < -10 {
            return sign;
        }
        fraction = (fraction | 0x800000) >> (1 - exponent);
        return sign | ((fraction + 0x1000) >> 13);
    }
    if exponent >= 31 {
        return sign | 0x7c00;
    }
    fraction += 0x1000;
    if fraction & 0x800000 != 0 {
        fraction = 0;
        exponent += 1;
        if exponent >= 31 {
            return sign | 0x7c00;
        }
    }
    sign | ((exponent as u32) << 10) | (fraction >> 13)
}
fn median_unhalf(value: u32) -> f32 {
    let sign = if value & 0x8000 != 0 { -1.0_f64 } else { 1.0 };
    let exponent = (value >> 10) & 0x1f;
    let fraction = value & 0x3ff;
    if exponent == 0 {
        return (sign * 2.0_f64.powi(-14) * f64::from(fraction) / 1024.0) as f32;
    }
    if exponent == 31 {
        return if fraction != 0 {
            f32::NAN
        } else {
            (sign * f64::INFINITY) as f32
        };
    }
    (sign * 2.0_f64.powi(exponent as i32 - 15) * (1.0 + f64::from(fraction) / 1024.0)) as f32
}
fn render_median(
    uniforms: &BTreeMap<String, Value>,
    input: &Surface,
    output: &mut Surface,
) -> Result<(), RenderError> {
    let radius = adapter_int32(f64::from(scalar(uniforms, "RADIUS")?));
    let threshold = scalar_number(uniforms, "threshold")?;
    let mut records = Vec::with_capacity(49);
    for row in 0..output.height() {
        for x in 0..output.width() {
            let y = output.height() - 1 - row;
            let original = adapter_fetch(input, x as f32, y as f32);
            records.clear();
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    let pixel = adapter_fetch(input, x as f32 + dx as f32, y as f32 + dy as f32);
                    let luminance = fadd(
                        fadd(
                            f32_from_js(f64::from(pixel[0]) * 0.2126),
                            f32_from_js(f64::from(pixel[1]) * 0.7152),
                        ),
                        f32_from_js(f64::from(pixel[2]) * 0.0722),
                    );
                    records.push((
                        luminance.to_bits(),
                        (median_half(pixel[0]) << 16) | median_half(pixel[1]),
                        median_half(pixel[2]),
                    ));
                }
            }
            records.sort_unstable();
            let (_, packed, blue) = records[(records.len() - 1) >> 1];
            let median = [
                median_unhalf(packed >> 16),
                median_unhalf(packed & 0xffff),
                median_unhalf(blue),
            ];
            let difference = (0..3)
                .map(|i| (f64::from(original[i]) - f64::from(median[i])).abs())
                .fold(0.0_f64, |a, b| {
                    if a.is_nan() || b.is_nan() {
                        f64::NAN
                    } else {
                        a.max(b)
                    }
                });
            let replace = threshold <= 0.0 || difference >= threshold / 100.0;
            let offset = ((row * output.width() + x) * 4) as usize;
            output.data_mut()[offset..offset + 3].copy_from_slice(if replace {
                &median
            } else {
                &original[..3]
            });
            output.data_mut()[offset + 3] = original[3];
        }
    }
    Ok(())
}

// This adapter follows CPU crt.js's separate M2 reciprocal and sine model.
fn crt_div(a: f32, b: f32) -> f32 {
    fmul(a, (1.0 / f64::from(b)) as f32)
}
fn crt_min(a: f32, b: f32) -> f32 {
    if a.is_nan() || b.is_nan() {
        f32::NAN
    } else if a == 0.0 && b == 0.0 {
        f32::from_bits((a.to_bits() | b.to_bits()) & 0x80000000)
    } else {
        a.min(b)
    }
}
fn crt_max(a: f32, b: f32) -> f32 {
    if a.is_nan() || b.is_nan() {
        f32::NAN
    } else if a == 0.0 && b == 0.0 {
        f32::from_bits(a.to_bits() & b.to_bits() & 0x80000000)
    } else {
        a.max(b)
    }
}
fn crt_clamp(v: f32, low: f32, high: f32) -> f32 {
    crt_min(crt_max(v, low), high)
}
fn crt_clamp01(v: f32) -> f32 {
    crt_clamp(v, 0.0, 1.0)
}
fn crt_sqrt(v: f32) -> f32 {
    f64::from(v).sqrt() as f32
}
fn crt_mix(a: f32, b: f32, t: f32) -> f32 {
    t.mul_add(fsub(b, a), a)
}
fn crt_length(x: f32, y: f32) -> f32 {
    crt_sqrt(y.mul_add(y, fmul(x, x)))
}
fn crt_dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[2].mul_add(b[2], a[1].mul_add(b[1], fmul(a[0], b[0])))
}
fn crt_quadrant(r: f32) -> f32 {
    let r = f64::from(r);
    let n = r.floor();
    let f = r - n;
    let y = if (n as i32) & 1 != 0 { 1.0 - f } else { f };
    if y == 0.0 {
        return if n >= 2.0 { -0.0 } else { 0.0 };
    }
    let z = 2.0 * y * y - 1.0;
    let quotient = (y * std::f64::consts::PI / 2.0).sin() / y;
    let p = (quotient
        - 2.0_f64.powi(-23) * (0.783734 + 0.010102 * z + 0.052674 * (2.0 * z * z - 1.0)))
        as f32;
    let value = (y * f64::from(p)) as f32;
    if n >= 2.0 { -value } else { value }
}
fn crt_sin_turns(turns: f32) -> f32 {
    crt_quadrant((4.0 * (f64::from(turns) - f64::from(turns).floor())) as f32)
}
fn crt_sin(x: f32) -> f32 {
    crt_sin_turns(fmul(x, (1.0 / (2.0 * std::f64::consts::PI)) as f32))
}
fn crt_cos_turns(turns: f32) -> f32 {
    crt_sin_turns(fadd(turns, 0.25))
}
fn crt_periodic(time: f32, value: f32) -> f32 {
    crt_sin_turns(fsub(time, value)).mul_add(0.5, 0.5)
}
fn crt_mod289(x: f32) -> f32 {
    fsub(x, fmul(fmul(x, (1.0 / 289.0) as f32).floor(), 289.0))
}
fn crt_permute(x: f32) -> f32 {
    crt_mod289(fmul(x.mul_add(34.0, 1.0), x))
}
fn crt_simplex(v: [f32; 3]) -> f32 {
    let cx = (1.0 / 6.0) as f32;
    let cy = (1.0 / 3.0) as f32;
    let dv = crt_dot(v, [cy; 3]);
    let i = v.map(|x| fadd(x, dv).floor());
    let di = crt_dot(i, [cx; 3]);
    let x0 = std::array::from_fn::<_, 3, _>(|k| fadd(fsub(v[k], i[k]), di));
    let s: [f32; 3] = [
        if x0[0] < x0[1] { 0.0 } else { 1.0 },
        if x0[1] < x0[2] { 0.0 } else { 1.0 },
        if x0[2] < x0[0] { 0.0 } else { 1.0 },
    ];
    let l = [1.0 - s[2], 1.0 - s[0], 1.0 - s[1]];
    let i1 = std::array::from_fn::<_, 3, _>(|k| s[k].min(l[k]));
    let i2 = std::array::from_fn::<_, 3, _>(|k| s[k].max(l[k]));
    let corners = [
        x0,
        std::array::from_fn(|k| fadd(fsub(x0[k], i1[k]), cx)),
        std::array::from_fn(|k| fadd(fsub(x0[k], i2[k]), cy)),
        x0.map(|x| fsub(x, 0.5)),
    ];
    let i = i.map(crt_mod289);
    let offsets = [[0.0; 3], i1, i2, [1.0; 3]];
    let mut p = [0.0; 4];
    for k in 0..4 {
        let pz = crt_permute(fadd(i[2], offsets[k][2]));
        let py = crt_permute(fadd(fadd(pz, i[1]), offsets[k][1]));
        p[k] = crt_permute(fadd(fadd(py, i[0]), offsets[k][0]));
    }
    let n = (1.0 / 7.0) as f32;
    let nsx = fmul(n, 2.0);
    let nsy = fsub(fmul(n, 0.5), 1.0);
    let mut gradients = [[0.0; 3]; 4];
    for k in 0..4 {
        let j = fsub(p[k], fmul(49.0, fmul(fmul(p[k], n), n).floor()));
        let xi = fmul(j, n).floor();
        let yi = fsub(j, fmul(7.0, xi)).floor();
        let x = xi.mul_add(nsx, nsy);
        let y = yi.mul_add(nsx, nsy);
        let h = fsub(fsub(1.0, x.abs()), y.abs());
        let sh = if h > 0.0 { 0.0 } else { -1.0 };
        gradients[k] = [
            fadd(fmul(x.floor(), 2.0), 1.0).mul_add(sh, x),
            fadd(fmul(y.floor(), 2.0), 1.0).mul_add(sh, y),
            h,
        ];
    }
    let mut sum = 0.0;
    for k in 0..4 {
        let g = gradients[k];
        let norm = (-0.85373472095314_f32).mul_add(crt_dot(g, g), 1.79284291400159_f32);
        let c = corners[k];
        let m = crt_max(fsub(0.6, crt_dot(c, c)), 0.0);
        let msq = fmul(m, m);
        let term = crt_dot(g.map(|x| fmul(x, norm)), c);
        sum = if k == 0 {
            fmul(fmul(msq, msq), term)
        } else {
            fmul(msq, msq).mul_add(term, sum)
        };
    }
    fmul(42.0, sum)
}
fn crt_wrap(value: f32, limit: f32) -> f32 {
    if limit <= 0.0 {
        return 0.0;
    }
    let mut result = (-crt_div(value, limit).floor()).mul_add(limit, value);
    if result < 0.0 {
        result = fadd(result, limit);
    }
    result
}
fn crt_noise_hash(c: [f32; 3], seed: f32) -> f32 {
    ffract(fmul(
        crt_sin(seed.mul_add(0.001, crt_dot(c, [12.9898, 78.233, 37.719]))),
        43758.5453,
    ))
}
fn crt_fade(v: f32) -> f32 {
    fmul(fmul(v, v), (-2.0_f32).mul_add(v, 3.0))
}
fn crt_value_noise(v: [f32; 3], seed: f32) -> f32 {
    let c = v.map(f32::floor);
    let t = std::array::from_fn::<_, 3, _>(|k| crt_fade(fsub(v[k], c[k])));
    let h = |x, y, z| crt_noise_hash([c[0] + x, c[1] + y, c[2] + z], seed);
    let lerp = |a, b, t| fsub(b, a).mul_add(t, a);
    let x00 = lerp(h(0.0, 0.0, 0.0), h(1.0, 0.0, 0.0), t[0]);
    let x10 = lerp(h(0.0, 1.0, 0.0), h(1.0, 1.0, 0.0), t[0]);
    let x01 = lerp(h(0.0, 0.0, 1.0), h(1.0, 0.0, 1.0), t[0]);
    let x11 = lerp(h(0.0, 1.0, 1.0), h(1.0, 1.0, 1.0), t[0]);
    lerp(lerp(x00, x10, t[1]), lerp(x01, x11, t[1]), t[2])
}
fn crt_wrap_unit(v: f32) -> f32 {
    let mut w = fsub(v, v.floor());
    if w < 0.0 {
        w = fadd(w, 1.0);
    }
    w
}
fn crt_rgb_hsv(c: [f32; 3]) -> [f32; 3] {
    let max = crt_max(crt_max(c[0], c[1]), c[2]);
    let min = crt_min(crt_min(c[0], c[1]), c[2]);
    let delta = fsub(max, min);
    let mut hue = 0.0;
    if delta > 0.0 {
        hue = if max == c[0] {
            let s = crt_div(fsub(c[1], c[2]), delta);
            if s < 0.0 { fadd(s, 6.0) } else { s }
        } else if max == c[1] {
            fadd(crt_div(fsub(c[2], c[0]), delta), 2.0)
        } else {
            fadd(crt_div(fsub(c[0], c[1]), delta), 4.0)
        };
        hue = crt_wrap_unit(crt_div(hue, 6.0));
    }
    [hue, if max != 0.0 { crt_div(delta, max) } else { 0.0 }, max]
}
fn crt_hsv_rgb(h: f32, s: f32, v: f32) -> [f32; 3] {
    let dh = fmul(h, 6.0);
    let one = fsub(1.0, s);
    [
        crt_clamp01(fmul(
            s.mul_add(crt_clamp01(fsub(fsub(dh, 3.0).abs(), 1.0)), one),
            v,
        )),
        crt_clamp01(fmul(
            s.mul_add(crt_clamp01(fadd(-fsub(dh, 2.0).abs(), 2.0)), one),
            v,
        )),
        crt_clamp01(fmul(
            s.mul_add(crt_clamp01(fadd(-fsub(dh, 4.0).abs(), 2.0)), one),
            v,
        )),
    ]
}
fn crt_hue(c: [f32; 3], amount: f32) -> [f32; 3] {
    let hsv = crt_rgb_hsv(c);
    crt_hsv_rgb(
        crt_wrap_unit(fadd(hsv[0], amount)),
        crt_clamp01(hsv[1]),
        crt_clamp01(hsv[2]),
    )
}
fn crt_saturation(c: [f32; 3], amount: f32) -> [f32; 3] {
    let hsv = crt_rgb_hsv(c);
    crt_hsv_rgb(
        hsv[0],
        crt_clamp01(fmul(hsv[1], amount)),
        crt_clamp01(hsv[2]),
    )
}

struct CrtState {
    width: f32,
    height: f32,
    freq: [f32; 2],
    time: f32,
    speed: f64,
    base: [f32; 3],
    time_seed: [f32; 3],
    z: f32,
    aspect: f32,
    max_radius: f32,
    scan: [f32; 2],
}
impl CrtState {
    fn noise(&self, u: f32, v: f32) -> f32 {
        let base = crt_simplex([
            fadd(u, self.base[0]),
            fadd(v, self.base[1]),
            fadd(self.z, self.base[2]),
        ]);
        let mut value = crt_clamp01(base.mul_add(0.5, 0.5));
        if self.speed != 0.0 && self.time != 0.0 {
            let n = crt_simplex([
                fadd(u, self.time_seed[0]),
                fadd(v, self.time_seed[1]),
                self.time_seed[2],
            ]);
            let t = crt_clamp01(n.mul_add(0.5, 0.5));
            value = crt_clamp01(crt_periodic(
                f32_from_js(f64::from(crt_periodic(self.time, t)) * self.speed),
                value,
            ));
        }
        crt_clamp01(value)
    }
    fn lens(&self, x: f32, y: f32) -> [f32; 2] {
        let x = crt_wrap(x, self.width);
        let y = crt_wrap(y, self.height);
        let noise = self.noise(
            fmul(crt_div(x, self.width), self.freq[1]),
            fmul(crt_div(y, self.height), self.freq[0]),
        );
        let dx = fsub(crt_div(fadd(x, 0.5), self.width), 0.5).abs();
        let dy = fsub(crt_div(fadd(y, 0.5), self.height), 0.5).abs();
        let mut mask = 0.0;
        if self.max_radius > 0.0 {
            let normalized = crt_clamp01(crt_div(
                crt_length(fmul(dx, self.aspect), dy),
                self.max_radius,
            ));
            let masked = crt_sqrt(normalized);
            let square = fmul(masked, masked);
            mask = fmul(fmul(square, square), masked);
        }
        let distortion = fmul(noise.mul_add(2.0, -1.0), mask);
        [
            fmul(fmul(crt_cos_turns(distortion), 0.0625), self.width),
            fmul(fmul(crt_sin_turns(distortion), 0.0625), self.height),
        ]
    }
    fn scan(&self, x: f32, y: f32) -> f32 {
        let x = crt_clamp(crt_wrap(x, self.width), 0.0, fsub(self.width, 1.0));
        let y = crt_clamp(crt_wrap(y, self.height), 0.0, fsub(self.height, 1.0));
        let y0 = y.floor();
        let y1 = crt_min(fadd(y0, 1.0), fsub(self.height, 1.0));
        let tx = crt_clamp01(fsub(x, x.floor()));
        let ty = crt_clamp01(fsub(y, y0));
        let row = |r: f32| {
            self.scan[if crt_div(r, 2.5).floor().trunc() % 2.0 == 0.0 {
                0
            } else {
                1
            }]
        };
        let a = row(y0);
        let b = row(y1);
        crt_mix(crt_mix(a, a, tx), crt_mix(b, b, tx), ty)
    }
}
fn crt_blend_cosine(a: f32, b: f32, value: f32) -> f32 {
    let c = crt_sin_turns(crt_clamp01(value).mul_add(0.5, 0.25));
    let weight = fmul(fsub(1.0, c), 0.5);
    b.mul_add(weight, fmul(a, 0.5_f32.mul_add(c, 0.5)))
}
fn crt_blend_scan(v: f32, scan: f32) -> f32 {
    crt_mix(v, fmul(fadd(v, scan), scan), 0.5)
}
fn render_crt(
    uniforms: &BTreeMap<String, Value>,
    input: &Surface,
    output: &mut Surface,
) -> Result<(), RenderError> {
    let resolution = vector(uniforms, "resolution")?;
    let full = vector(uniforms, "fullResolution")?;
    let full = if full[0] > 0.0 { full } else { resolution };
    let tile = vector(uniforms, "tileOffset")?;
    let render_scale = scalar_number(uniforms, "renderScale")?;
    let rs = if render_scale.is_nan() {
        f64::NAN
    } else {
        render_scale.max(1.0)
    };
    let alpha = scalar_number(uniforms, "alpha")?.clamp(0.0, 1.0);
    let seed = scalar(uniforms, "seed")?;
    let time = scalar(uniforms, "time")?;
    let speed = scalar_number(uniforms, "speed")?;
    let width = crt_max(fmul(full[0], (1.0 / rs) as f32), 1.0);
    let height = crt_max(fmul(full[1], (1.0 / rs) as f32), 1.0);
    let freq = if fsub(width, height).abs() < 1e-5_f32 {
        [2.0, 2.0]
    } else if height < width {
        [2.0, crt_max(crt_div(fmul(2.0, width), height).floor(), 1.0)]
    } else {
        [crt_max(crt_div(fmul(2.0, height), width).floor(), 1.0), 2.0]
    };
    let s = fmul(seed, 73.0);
    let base = [fadd(17.0, s), s.mul_add(1.1, 29.0), s.mul_add(0.7, 47.0)];
    let time_seed = [
        fadd(base[0], 54.0),
        fadd(base[1], 82.0),
        fadd(base[2], 124.0),
    ];
    let aspect = crt_div(width, height);
    let scaled_time = fmul(f32_from_js(f64::from(time) * speed), 0.1);
    let noise_seed = seed.mul_add(31.0, 19.37);
    let state = CrtState {
        width,
        height,
        freq,
        time,
        speed,
        base,
        time_seed,
        z: f32_from_js(f64::from(crt_cos_turns(time)) * speed),
        aspect,
        max_radius: crt_length(fmul(aspect, 0.5), 0.5),
        scan: [
            crt_value_noise([0.0, 0.0, scaled_time], noise_seed),
            crt_value_noise([1.0, 0.0, scaled_time], noise_seed),
        ],
    };
    let hash = |offset: f32| {
        let inv = (1.0 / (2.0 * std::f64::consts::PI)) as f32;
        ffract(fmul(
            crt_sin_turns(seed.mul_add(
                (73.0 * f64::from(inv)) as f32,
                fmul(fadd(17.0, offset), inv),
            )),
            43758.5453123,
        ))
    };
    let displacement = fmul(fmul(hash(0.37).mul_add(0.00625, 0.0125), width), hash(0.73));
    let hue = hash(1.91).mul_add(0.25, -0.125);
    let vignette = fmul(hash(3.17), 0.175);
    for row in 0..output.height() {
        let gy = (output.height() - 1 - row) as f32;
        for gx in 0..output.width() {
            let original = adapter_fetch(input, gx as f32, gy);
            let offset = ((row * output.width() + gx) * 4) as usize;
            if alpha == 0.0 {
                output.data_mut()[offset..offset + 4].copy_from_slice(&original);
                continue;
            }
            let x = fmul(fadd(gx as f32, tile[0]), (1.0 / rs) as f32);
            let y = fmul(fadd(gy, tile[1]), (1.0 / rs) as f32);
            let lens = state.lens(x, y);
            let scan = state.scan(fadd(x, lens[0]), fadd(y, lens[1]));
            let green = [original[0], original[1], original[2]]
                .map(|v| crt_clamp01(crt_blend_scan(v, scan)));
            let dx = crt_div(fsub(x, fmul(width, 0.5)), width);
            let dy = crt_div(fsub(y, fmul(height, 0.5)), height);
            let singularity = crt_length(dx, dy);
            let mask = fmul(fmul(singularity, singularity), singularity);
            let gradient = crt_clamp01(crt_div(x, fsub(width, 1.0)));
            let red_x = crt_blend_cosine(
                x,
                crt_mix(
                    crt_min(fadd(x, displacement), fsub(width, 1.0)),
                    x,
                    gradient,
                ),
                mask,
            );
            let red = adapter_fetch(
                input,
                fsub(f32_from_js(f64::from(red_x) * render_scale), tile[0]).trunc(),
                gy,
            );
            let lens = state.lens(red_x, y);
            let red_scan = state.scan(fadd(red_x, lens[0]), fadd(y, lens[1]));
            let blue_x = crt_blend_cosine(
                x,
                crt_mix(x, crt_max(fsub(x, displacement), 0.0), gradient),
                mask,
            );
            let blue = adapter_fetch(
                input,
                fsub(f32_from_js(f64::from(blue_x) * render_scale), tile[0]).trunc(),
                gy,
            );
            let lens = state.lens(blue_x, y);
            let blue_scan = state.scan(fadd(blue_x, lens[0]), fadd(y, lens[1]));
            let red = crt_hue(
                [red[0], red[1], red[2]].map(|v| crt_blend_scan(v, red_scan)),
                hue,
            );
            let green = crt_hue(green, hue);
            let blue = crt_hue(
                [blue[0], blue[1], blue[2]].map(|v| crt_blend_scan(v, blue_scan)),
                hue,
            );
            let mut color = crt_saturation(crt_hue([red[0], green[1], blue[2]], -hue), 1.125);
            for c in &mut color {
                *c = crt_mix(*c, crt_mix(*c, 0.0, singularity), crt_clamp01(vignette));
            }
            let mean = fmul(fadd(fadd(color[0], color[1]), color[2]), (1.0 / 3.0) as f32);
            for c in 0..3 {
                let normalized = crt_clamp01(fsub(color[c], mean).mul_add(1.25, mean));
                let quantized = crt_div(crt_clamp01(normalized).mul_add(255.0, 0.5).floor(), 255.0);
                output.data_mut()[offset + c] = crt_fma_number(
                    alpha,
                    f64::from(fsub(quantized, original[c])),
                    f64::from(original[c]),
                );
            }
            output.data_mut()[offset + 3] = original[3];
        }
    }
    Ok(())
}

fn scalar_number(uniforms: &BTreeMap<String, Value>, name: &str) -> Result<f64, RenderError> {
    match uniforms.get(name) {
        Some(Value::Float(value)) => Ok(*value),
        Some(Value::Int(value)) => Ok(f64::from(*value)),
        Some(Value::Uint(value)) => Ok(f64::from(*value)),
        value => Err(RenderError::InvalidGraph {
            message: format!("adapter uniform {name:?} is not scalar: {value:?}"),
        }),
    }
}
// Uniform scalar Numbers can have f64 precision. Match the CPU helper's TwoSum
// for those inputs; all internal float32 chains use native f32::mul_add above.
fn crt_fma_number(a: f64, b: f64, c: f64) -> f32 {
    let product = a * b;
    let sum = product + c;
    let back = sum - product;
    let error = (product - (sum - back)) + (c - back);
    let rounded = sum as f32;
    if error == 0.0 || f64::from(rounded) == sum {
        return rounded;
    }
    let toward = if sum > f64::from(rounded) { 1_i32 } else { -1 };
    let neighbor = if rounded == 0.0 {
        toward as f32 * f32::from_bits(1)
    } else {
        let step = if (rounded > 0.0) == (toward > 0) {
            1_i32
        } else {
            -1
        };
        f32::from_bits(rounded.to_bits().wrapping_add(step as u32))
    };
    if (f64::from(rounded) + f64::from(neighbor)) / 2.0 != sum {
        return rounded;
    }
    if error > 0.0 {
        crt_max(rounded, neighbor)
    } else {
        crt_min(rounded, neighbor)
    }
}
