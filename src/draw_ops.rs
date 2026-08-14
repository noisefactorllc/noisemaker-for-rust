#![allow(
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::needless_range_loop
)]

use std::collections::BTreeMap;

use crate::catalog::RenderPass;
use crate::pass_runner::texture_dimensions;
use crate::texture_format::truncate_to_binary16;
use crate::{
    FilterMode, RenderError, Surface, TextureFormat, Value, quantize_texture, sample_bilinear,
    sample_nearest,
};

pub const DRAW_OP_KEYS: [&str; 7] = [
    "filter/wormhole:deposit",
    "filter3d/flow3d:deposit",
    "points/dla:depositGrid",
    "points/lenia:deposit",
    "points/physarum:deposit",
    "render/pointsBillboardRender:deposit",
    "render/pointsRender:deposit",
];

#[must_use]
pub const fn draw_op_keys() -> [&'static str; 7] {
    DRAW_OP_KEYS
}

#[must_use]
pub fn is_draw_pass(pass: &RenderPass) -> bool {
    matches!(
        pass.execution
            .get("drawMode")
            .and_then(serde_json::Value::as_str),
        Some("points" | "billboards")
    )
}

fn scalar(uniforms: &BTreeMap<String, Value>, name: &str) -> Result<f32, RenderError> {
    match uniforms.get(name) {
        Some(Value::Float(value)) => Ok(*value),
        Some(Value::Int(value)) => Ok(*value as f32),
        Some(Value::Uint(value)) => Ok(*value as f32),
        value => Err(RenderError::InvalidGraph {
            message: format!("draw uniform {name:?} is not scalar: {value:?}"),
        }),
    }
}

fn integer(uniforms: &BTreeMap<String, Value>, name: &str) -> Result<i32, RenderError> {
    Ok(scalar(uniforms, name)? as i32)
}

fn input<'a>(
    pass: &RenderPass,
    resources: &'a BTreeMap<String, Surface>,
    name: &str,
) -> Result<&'a Surface, RenderError> {
    let attachment = pass
        .inputs
        .get(name)
        .ok_or_else(|| RenderError::InvalidGraph {
            message: format!(
                "draw pass {:?} has no input binding for {name:?}",
                pass.name
            ),
        })?;
    resources
        .get(attachment)
        .ok_or_else(|| RenderError::InvalidGraph {
            message: format!(
                "draw pass {:?} is missing input resource {attachment:?}",
                pass.name
            ),
        })
}

#[must_use]
pub fn texel_fetch_agent(surface: &Surface, sx: i32, sy: i32) -> [f32; 4] {
    let x = sx.clamp(0, surface.width() as i32 - 1) as usize;
    let shader_y = sy.clamp(0, surface.height() as i32 - 1) as usize;
    let storage_y = surface.height() as usize - 1 - shader_y;
    let offset = (storage_y * surface.width() as usize + x) * 4;
    let data = surface.data();
    [
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]
}

#[must_use]
pub fn scatter_point_pixel(
    clip_x: f32,
    clip_y: f32,
    clip_w: f32,
    destination_width: u32,
    destination_height: u32,
) -> Option<usize> {
    if clip_w <= 0.0 {
        return None;
    }
    let ndc_x = clip_x / clip_w;
    let ndc_y = clip_y / clip_w;
    let gl_col = ((ndc_x * 0.5 + 0.5) * destination_width as f32).floor();
    let gl_row = ((ndc_y * 0.5 + 0.5) * destination_height as f32).floor();
    if !gl_col.is_finite() || !gl_row.is_finite() {
        return None;
    }
    let gl_col = gl_col as i64;
    let gl_row = gl_row as i64;
    if gl_col < 0
        || gl_row < 0
        || gl_col >= i64::from(destination_width)
        || gl_row >= i64::from(destination_height)
    {
        return None;
    }
    let storage_row = i64::from(destination_height) - 1 - gl_row;
    Some(((storage_row * i64::from(destination_width) + gl_col) * 4) as usize)
}

pub fn compute_clip_center(
    x: f32,
    y: f32,
    z: f32,
    uniforms: &BTreeMap<String, Value>,
) -> Result<[f32; 2], RenderError> {
    if integer(uniforms, "viewMode")? == 0 {
        return Ok([x * 2.0 - 1.0, y * 2.0 - 1.0]);
    }
    let is_2d = z.abs() < 1.0 && (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y);
    let (mut px, mut py, mut pz) = (x, y, z);
    if is_2d {
        px -= 0.5;
        py -= 0.5;
        pz = 0.0;
    }
    let rotate_x = scalar(uniforms, "rotateX")?;
    let (cos_x, sin_x) = (rotate_x.cos(), rotate_x.sin());
    let (x1, y1, z1) = (px, py * cos_x - pz * sin_x, py * sin_x + pz * cos_x);
    let rotate_y = scalar(uniforms, "rotateY")?;
    let (cos_y, sin_y) = (rotate_y.cos(), rotate_y.sin());
    let (x2, y2) = (x1 * cos_y + z1 * sin_y, y1);
    let rotate_z = scalar(uniforms, "rotateZ")?;
    let (cos_z, sin_z) = (rotate_z.cos(), rotate_z.sin());
    let fx = x2 * cos_z - y2 * sin_z + scalar(uniforms, "posX")?;
    let fy = x2 * sin_z + y2 * cos_z + scalar(uniforms, "posY")?;
    let scale = scalar(uniforms, "viewScale")?;
    Ok(if is_2d {
        [fx * 3.5 * scale, fy * 3.5 * scale]
    } else {
        [fx / 40.0 * scale, fy / 40.0 * scale]
    })
}

#[allow(clippy::too_many_arguments)]
pub fn execute_draw_pass(
    key: &str,
    pass: &RenderPass,
    uniforms: &BTreeMap<String, Value>,
    resources: &mut BTreeMap<String, Surface>,
    width: u32,
    height: u32,
    formats: &BTreeMap<String, TextureFormat>,
) -> Result<usize, RenderError> {
    if !DRAW_OP_KEYS.contains(&key) {
        return Err(RenderError::MissingDrawOp {
            key: key.into(),
            pass: pass.name.clone(),
        });
    }
    let viewport = pass
        .execution
        .get("viewport")
        .unwrap_or(&serde_json::Value::Null);
    let (pass_width, pass_height) =
        texture_dimensions(viewport, uniforms, width, height, resources)?;
    let attachment =
        pass.outputs
            .values()
            .next()
            .cloned()
            .ok_or_else(|| RenderError::InvalidGraph {
                message: format!("draw pass {:?} has no output", pass.name),
            })?;
    let mut destination = resources
        .get(&attachment)
        .filter(|surface| surface.width() == pass_width && surface.height() == pass_height)
        .cloned()
        .unwrap_or(Surface::new(pass_width, pass_height)?);
    let pixels = match key {
        "filter/wormhole:deposit" => wormhole(pass, uniforms, resources, &mut destination)?,
        "filter3d/flow3d:deposit" => flow3d(pass, uniforms, resources, &mut destination)?,
        "points/dla:depositGrid" => dla(pass, uniforms, resources, &mut destination)?,
        "points/lenia:deposit" => lenia(pass, uniforms, resources, &mut destination)?,
        "points/physarum:deposit" => physarum(pass, uniforms, resources, &mut destination)?,
        "render/pointsRender:deposit" => {
            points_render(pass, uniforms, resources, &mut destination)?
        }
        "render/pointsBillboardRender:deposit" => {
            billboard(pass, uniforms, resources, &mut destination)?
        }
        _ => unreachable!(),
    };
    quantize_texture(
        &mut destination,
        formats.get(&attachment).copied().unwrap_or_default(),
    );
    resources.insert(attachment, destination);
    Ok(pixels)
}

fn for_each_agent(
    state: &Surface,
    mut callback: impl FnMut(usize, i32, i32) -> Result<(), RenderError>,
) -> Result<(), RenderError> {
    let width = state.width() as usize;
    for vertex in 0..width * state.height() as usize {
        callback(vertex, (vertex % width) as i32, (vertex / width) as i32)?;
    }
    Ok(())
}

fn dla(
    pass: &RenderPass,
    uniforms: &BTreeMap<String, Value>,
    resources: &BTreeMap<String, Surface>,
    destination: &mut Surface,
) -> Result<usize, RenderError> {
    let xyz = input(pass, resources, "xyzTex")?;
    let vel = input(pass, resources, "velTex")?;
    let rgba = input(pass, resources, "rgbaTex")?;
    let energy = scalar(uniforms, "deposit")? * 0.1;
    let mut pixels = 0;
    for_each_agent(xyz, |_, sx, sy| {
        if texel_fetch_agent(vel, sx, sy)[1] < 0.5 {
            return Ok(());
        }
        let position = texel_fetch_agent(xyz, sx, sy);
        let Some(offset) = scatter_point_pixel(
            position[0] * 2.0 - 1.0,
            position[1] * 2.0 - 1.0,
            1.0,
            destination.width(),
            destination.height(),
        ) else {
            return Ok(());
        };
        let color = texel_fetch_agent(rgba, sx, sy);
        let data = destination.data_mut();
        for channel in 0..3 {
            data[offset + channel] += color[channel] * energy;
        }
        data[offset + 3] += energy;
        pixels += 1;
        Ok(())
    })?;
    Ok(pixels)
}

fn lenia(
    pass: &RenderPass,
    uniforms: &BTreeMap<String, Value>,
    resources: &BTreeMap<String, Surface>,
    destination: &mut Surface,
) -> Result<usize, RenderError> {
    let xyz = input(pass, resources, "xyzTex")?;
    let amount = scalar(uniforms, "depositAmount")?;
    let mut pixels = 0;
    for_each_agent(xyz, |_, sx, sy| {
        let position = texel_fetch_agent(xyz, sx, sy);
        if position[3] < 0.5 {
            return Ok(());
        }
        let Some(offset) = scatter_point_pixel(
            position[0] * 2.0 - 1.0,
            position[1] * 2.0 - 1.0,
            1.0,
            destination.width(),
            destination.height(),
        ) else {
            return Ok(());
        };
        let data = destination.data_mut();
        data[offset] += amount;
        data[offset + 3] += 1.0;
        pixels += 1;
        Ok(())
    })?;
    Ok(pixels)
}

fn physarum(
    pass: &RenderPass,
    uniforms: &BTreeMap<String, Value>,
    resources: &BTreeMap<String, Surface>,
    destination: &mut Surface,
) -> Result<usize, RenderError> {
    let xyz = input(pass, resources, "xyzTex")?;
    let rgba = input(pass, resources, "rgbaTex")?;
    let deposit = scalar(uniforms, "deposit")?;
    let mut pixels = 0;
    for_each_agent(xyz, |_, sx, sy| {
        let position = texel_fetch_agent(xyz, sx, sy);
        if position[3] < 0.5 {
            return Ok(());
        }
        let Some(offset) = scatter_point_pixel(
            position[0] * 2.0 - 1.0,
            position[1] * 2.0 - 1.0,
            1.0,
            destination.width(),
            destination.height(),
        ) else {
            return Ok(());
        };
        let color = texel_fetch_agent(rgba, sx, sy);
        for channel in 0..4 {
            destination.data_mut()[offset + channel] += color[channel] * deposit;
        }
        pixels += 1;
        Ok(())
    })?;
    Ok(pixels)
}

const GOLDEN_RATIO_CONJUGATE: f64 = 0.618033988749895;

fn points_render(
    pass: &RenderPass,
    uniforms: &BTreeMap<String, Value>,
    resources: &BTreeMap<String, Surface>,
    destination: &mut Surface,
) -> Result<usize, RenderError> {
    let xyz = input(pass, resources, "xyzTex")?;
    let rgba = input(pass, resources, "rgbaTex")?;
    let threshold = f64::from(scalar(uniforms, "density")?) / 100.0;
    let mut pixels = 0;
    for_each_agent(xyz, |vertex, sx, sy| {
        let random = (vertex as f64 * GOLDEN_RATIO_CONJUGATE).fract();
        if random > threshold {
            return Ok(());
        }
        let position = texel_fetch_agent(xyz, sx, sy);
        if position[3] < 0.5 {
            return Ok(());
        }
        let [clip_x, clip_y] =
            compute_clip_center(position[0], position[1], position[2], uniforms)?;
        let Some(offset) = scatter_point_pixel(
            clip_x,
            clip_y,
            1.0,
            destination.width(),
            destination.height(),
        ) else {
            return Ok(());
        };
        let color = texel_fetch_agent(rgba, sx, sy);
        for channel in 0..4 {
            destination.data_mut()[offset + channel] += color[channel];
        }
        pixels += 1;
        Ok(())
    })?;
    Ok(pixels)
}

fn flow3d(
    pass: &RenderPass,
    uniforms: &BTreeMap<String, Value>,
    resources: &BTreeMap<String, Surface>,
    destination: &mut Surface,
) -> Result<usize, RenderError> {
    let state1 = input(pass, resources, "stateTex1")?;
    let state2 = input(pass, resources, "stateTex2")?;
    let capacity = u64::from(state1.width()) * u64::from(state1.height());
    let max_agents =
        (state1.width().max(state1.height()) as f32 * scalar(uniforms, "density")? * 0.2)
            .trunc()
            .max(0.0) as u64;
    let draw_count = pass
        .execution
        .get("count")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(capacity);
    let count = draw_count.min(capacity).min(max_agents) as usize;
    let volume_size = integer(uniforms, "volumeSize")?;
    let atlas_height = volume_size * volume_size;
    let mut pixels = 0;
    let state_width = state1.width() as usize;
    for vertex in 0..count {
        let sx = (vertex % state_width) as i32;
        let sy = (vertex / state_width) as i32;
        let position = texel_fetch_agent(state1, sx, sy);
        let color = texel_fetch_agent(state2, sx, sy);
        let atlas_x = position[0];
        let atlas_y = position[1] + position[2].floor() * volume_size as f32;
        let Some(offset) = scatter_point_pixel(
            atlas_x / volume_size as f32 * 2.0 - 1.0,
            atlas_y / atlas_height as f32 * 2.0 - 1.0,
            1.0,
            destination.width(),
            destination.height(),
        ) else {
            continue;
        };
        let data = destination.data_mut();
        for channel in 0..3 {
            data[offset + channel] += color[channel];
        }
        data[offset + 3] += 1.0;
        pixels += 1;
    }
    Ok(pixels)
}

fn oklab_lightness(red: f32, green: f32, blue: f32) -> f32 {
    let (r, g, b) = (
        red.clamp(0.0, 1.0),
        green.clamp(0.0, 1.0),
        blue.clamp(0.0, 1.0),
    );
    let l = 0.4122214708_f32 * r + 0.5363325363_f32 * g + 0.0514459929_f32 * b;
    let m = 0.2119034982_f32 * r + 0.6806995451_f32 * g + 0.1073969566_f32 * b;
    let s = 0.0883024619_f32 * r + 0.2817188376_f32 * g + 0.6299787005_f32 * b;
    let exponent = 1.0_f32 / 3.0_f32;
    0.2104542553_f32 * l.max(0.0).powf(exponent) + 0.793617785_f32 * m.max(0.0).powf(exponent)
        - 0.0040720468_f32 * s.max(0.0).powf(exponent)
}

fn wrap_repeat(value: i64, size: i64) -> i64 {
    value.rem_euclid(size)
}
fn wrap_mirror(value: i64, size: i64) -> i64 {
    let mirrored = wrap_repeat(value, size * 2);
    size - 1 - (mirrored - size + 1).abs()
}

fn wormhole(
    pass: &RenderPass,
    uniforms: &BTreeMap<String, Value>,
    resources: &BTreeMap<String, Surface>,
    destination: &mut Surface,
) -> Result<usize, RenderError> {
    const TAU: f32 = 6.28318530717959;
    let source = input(pass, resources, "inputTex")?;
    if source.width() != destination.width() || source.height() != destination.height() {
        return Err(RenderError::InvalidGraph {
            message: "wormhole deposit requires matching source and destination dimensions".into(),
        });
    }
    let width = i64::from(source.width());
    let height = i64::from(source.height());
    let kink = scalar(uniforms, "kink")?;
    let stride = 1024.0 * scalar(uniforms, "stride")?;
    let rotation = scalar(uniforms, "rotation")? * std::f32::consts::PI / 180.0;
    let wrap = integer(uniforms, "wrap")?;
    for source_y in 0..height {
        for source_x in 0..width {
            let source_row = height - 1 - source_y;
            let source_offset = ((source_row * width + source_x) * 4) as usize;
            let data = source.data();
            let lightness = oklab_lightness(
                data[source_offset],
                data[source_offset + 1],
                data[source_offset + 2],
            );
            let angle = lightness * TAU * kink + rotation;
            let mut destination_x = (source_x as f32 + (angle.cos() + 1.0) * stride).floor() as i64;
            let mut destination_y = (source_y as f32 + (angle.sin() + 1.0) * stride).floor() as i64;
            if wrap == 0 {
                destination_x = wrap_mirror(destination_x, width);
                destination_y = wrap_mirror(destination_y, height);
            } else if wrap == 2 {
                destination_x = destination_x.clamp(0, width - 1);
                destination_y = destination_y.clamp(0, height - 1);
            } else {
                destination_x = wrap_repeat(destination_x, width);
                destination_y = wrap_repeat(destination_y, height);
            }
            let destination_row = height - 1 - destination_y;
            let destination_offset = ((destination_row * width + destination_x) * 4) as usize;
            let weight = lightness * lightness;
            for channel in 0..3 {
                let value = destination.data()[destination_offset + channel]
                    + data[source_offset + channel] * weight;
                destination.data_mut()[destination_offset + channel] = truncate_to_binary16(value);
            }
        }
    }
    Ok((width * height) as usize)
}

fn hash_uint32(seed_bits: u32) -> u32 {
    let state = seed_bits
        .wrapping_mul(747_796_405)
        .wrapping_add(2_891_336_453);
    let word = ((state >> ((state >> 28) + 4)) ^ state).wrapping_mul(277_803_737);
    (word >> 22) ^ word
}

#[must_use]
pub fn billboard_hash(value: f32, seed: f32) -> f64 {
    f64::from(hash_uint32((value + seed).to_bits())) / 4_294_967_295.0
}

fn smoothstep_f64(edge0: f64, edge1: f64, value: f64) -> f64 {
    let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn signed_distance(shape: i32, mut x: f64, mut y: f64) -> f64 {
    match shape {
        1 => x.hypot(y) - 0.45,
        2 => (x.hypot(y) - 0.35).abs() - 0.08,
        3 => x.abs().max(y.abs()) - 0.4,
        4 => x.abs() + y.abs() - 0.45,
        5 => {
            let radius = 0.25;
            let root_three = 1.732050808;
            x = x.abs() - radius;
            y = y - 0.04 + radius / root_three;
            if x + root_three * y > 0.0 {
                (x, y) = ((x - root_three * y) / 2.0, (-root_three * x - y) / 2.0);
            }
            x -= x.clamp(-2.0 * radius, 0.0);
            -x.hypot(y) * y.signum()
        }
        _ => {
            let radius = 0.35;
            let radius_factor = 0.4;
            let (k1x, k1y) = (0.809016994375, -0.587785252292);
            let (k2x, k2y) = (-k1x, k1y);
            x = x.abs();
            let amount = (k1x * x + k1y * y).max(0.0);
            x -= 2.0 * amount * k1x;
            y -= 2.0 * amount * k1y;
            let amount = (k2x * x + k2y * y).max(0.0);
            x -= 2.0 * amount * k2x;
            y -= 2.0 * amount * k2y;
            x = x.abs();
            y -= radius;
            let (bax, bay) = (radius_factor * -k1y, radius_factor * k1x - 1.0);
            let h = ((x * bax + y * bay) / (bax * bax + bay * bay)).clamp(0.0, radius);
            (x - bax * h).hypot(y - bay * h) * (y * bax - x * bay).signum()
        }
    }
}

#[must_use]
pub fn billboard_shape_alpha(shape: i32, u: f64, v: f64) -> f64 {
    let (x, y) = (u - 0.5, v - 0.5);
    if (1..=6).contains(&shape) {
        1.0 - smoothstep_f64(-0.02, 0.02, signed_distance(shape, x, y))
    } else {
        (-(x * x + y * y) * 8.0).exp()
    }
}

fn premultiplied_blend(pass: &RenderPass) -> bool {
    let Some(blend) = pass
        .execution
        .get("blend")
        .and_then(serde_json::Value::as_array)
    else {
        return false;
    };
    blend.len() >= 2
        && blend[0]
            .as_str()
            .is_some_and(|value| value.eq_ignore_ascii_case("ONE"))
        && blend[1]
            .as_str()
            .is_some_and(|value| value.eq_ignore_ascii_case("ONE_MINUS_SRC_ALPHA"))
}

fn billboard(
    pass: &RenderPass,
    uniforms: &BTreeMap<String, Value>,
    resources: &BTreeMap<String, Surface>,
    destination: &mut Surface,
) -> Result<usize, RenderError> {
    const TAU_APPROX: f64 = 6.283185;
    let xyz = input(pass, resources, "xyzTex")?;
    let rgba = input(pass, resources, "rgbaTex")?;
    let sprite = input(pass, resources, "spriteTex")?;
    let threshold = f64::from(scalar(uniforms, "density")?) / 100.0;
    let shape = integer(uniforms, "shapeMode")?;
    let opacity = f64::from(scalar(uniforms, "depositOpacity")?) / 100.0;
    let seed = scalar(uniforms, "seed")?;
    let size_variation = f64::from(scalar(uniforms, "sizeVariation")?) / 100.0;
    let rotation_variation = f64::from(scalar(uniforms, "rotationVar")?) / 100.0;
    let point_size = f64::from(scalar(uniforms, "pointSize")?);
    let premultiplied = premultiplied_blend(pass);
    let (destination_width, destination_height) = (destination.width(), destination.height());
    let mut pixels = 0;
    for_each_agent(xyz, |vertex, sx, sy| {
        if (vertex as f64 * GOLDEN_RATIO_CONJUGATE).fract() > threshold {
            return Ok(());
        }
        let position = texel_fetch_agent(xyz, sx, sy);
        if position[3] < 0.5 {
            return Ok(());
        }
        let color = texel_fetch_agent(rgba, sx, sy);
        let center = compute_clip_center(position[0], position[1], position[2], uniforms)?;
        let size =
            point_size * (1.0 - size_variation * (billboard_hash(vertex as f32, seed) - 0.5));
        if size <= 0.0 {
            return Ok(());
        }
        let rotation =
            rotation_variation * billboard_hash(vertex as f32 + 1234.5, seed) * TAU_APPROX;
        let (cosine, sine) = (rotation.cos(), rotation.sin());
        let half_size = size * 0.5;
        let size_clip_x = half_size * (2.0 / f64::from(destination_width));
        let size_clip_y = half_size * (2.0 / f64::from(destination_height));
        let (mut min_x, mut max_x, mut min_y, mut max_y) = (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        );
        for (ox, oy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            let rotated_x = ox * cosine - oy * sine;
            let rotated_y = ox * sine + oy * cosine;
            let pixel_x = (f64::from(center[0]) + rotated_x * size_clip_x)
                * 0.5
                * f64::from(destination_width)
                + 0.5 * f64::from(destination_width);
            let pixel_y = (f64::from(center[1]) + rotated_y * size_clip_y)
                * 0.5
                * f64::from(destination_height)
                + 0.5 * f64::from(destination_height);
            min_x = min_x.min(pixel_x);
            max_x = max_x.max(pixel_x);
            min_y = min_y.min(pixel_y);
            max_y = max_y.max(pixel_y);
        }
        let col_start = min_x.floor().max(0.0) as i64;
        let col_end = max_x.ceil().min(f64::from(destination_width - 1)) as i64;
        let row_start = min_y.floor().max(0.0) as i64;
        let row_end = max_y.ceil().min(f64::from(destination_height - 1)) as i64;
        for gl_row in row_start..=row_end {
            let sample_y = ((gl_row as f64 + 0.5) / f64::from(destination_height)) * 2.0 - 1.0;
            let b = (sample_y - f64::from(center[1])) / size_clip_y;
            let storage_row = i64::from(destination_height) - 1 - gl_row;
            for column in col_start..=col_end {
                let sample_x = ((column as f64 + 0.5) / f64::from(destination_width)) * 2.0 - 1.0;
                let a = (sample_x - f64::from(center[0])) / size_clip_x;
                let offset_x = a * cosine + b * sine;
                let offset_y = -a * sine + b * cosine;
                if !(-1.0..=1.0).contains(&offset_x) || !(-1.0..=1.0).contains(&offset_y) {
                    continue;
                }
                let (u, v) = (offset_x * 0.5 + 0.5, offset_y * 0.5 + 0.5);
                let mut source = [0.0_f32; 4];
                if shape == 0 {
                    let sample = if sprite.filter_mode() == FilterMode::Linear {
                        sample_bilinear(sprite, u as f32, v as f32)
                    } else {
                        sample_nearest(sprite, u as f32, v as f32)
                    };
                    for channel in 0..4 {
                        source[channel] = (f64::from(sample[channel])
                            * f64::from(color[channel])
                            * opacity) as f32;
                    }
                } else {
                    let alpha = billboard_shape_alpha(shape, u, v);
                    for channel in 0..3 {
                        source[channel] = (f64::from(color[channel]) * alpha * opacity) as f32;
                    }
                    source[3] = (alpha * f64::from(color[3]) * opacity) as f32;
                }
                let destination_offset =
                    ((storage_row * i64::from(destination_width) + column) * 4) as usize;
                if premultiplied {
                    let inverse_alpha = 1.0 - source[3];
                    for channel in 0..4 {
                        destination.data_mut()[destination_offset + channel] = source[channel]
                            + destination.data()[destination_offset + channel] * inverse_alpha;
                    }
                } else {
                    for channel in 0..4 {
                        destination.data_mut()[destination_offset + channel] += source[channel];
                    }
                }
                pixels += 1;
            }
        }
        Ok(())
    })?;
    Ok(pixels)
}
