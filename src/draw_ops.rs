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

pub const DRAW_OP_KEYS: [&str; 8] = [
    "filter/wormhole:deposit",
    "filter3d/flow3d:deposit",
    "points/dla:depositGrid",
    "points/lenia:deposit",
    "points/physarum:deposit",
    "render/meshRender:render",
    "render/pointsBillboardRender:deposit",
    "render/pointsRender:deposit",
];

#[must_use]
pub const fn draw_op_keys() -> [&'static str; 8] {
    DRAW_OP_KEYS
}

#[must_use]
pub fn is_draw_pass(pass: &RenderPass) -> bool {
    matches!(
        pass.execution
            .get("drawMode")
            .and_then(serde_json::Value::as_str),
        Some("points" | "billboards" | "triangles")
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

/// Like `scalar`, but a missing uniform yields `default` instead of an
/// error -- for uniforms a caller (e.g. pointsRender's `deposit` pass) may
/// not always wire in.
fn scalar_or(
    uniforms: &BTreeMap<String, Value>,
    name: &str,
    default: f32,
) -> Result<f32, RenderError> {
    if uniforms.contains_key(name) {
        scalar(uniforms, name)
    } else {
        Ok(default)
    }
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

/// Port of deposit.wgsl's vertex-stage world->clip projection, shared by
/// pointsRender and pointsBillboardRender. Returns
/// `[clip_x, clip_y, camera_depth, camera_distance, projected_scale]`, or
/// `None` if the point is behind the near plane in perspective view
/// (viewMode 2) -- the caller must cull on `None` the same way the reference
/// culls before emitting a vertex. `camera_depth`/`camera_distance`/
/// `projected_scale` are only meaningful when viewMode != 0 (ortho/
/// perspective); flat view returns the reference's fixed camera_depth=80,
/// camera_distance=0, projected_scale=1.
///
/// Y orientation: this function has never flipped Y in any mode (unlike the
/// WGSL reference, which flips clip_y explicitly) -- some other stage of this
/// port's pipeline already compensates. Preserved into the new perspective
/// branch for consistency with flat/ortho rather than matching the WGSL
/// literally and risking a double-flip regression.
pub fn compute_clip_center(
    x: f32,
    y: f32,
    z: f32,
    uniforms: &BTreeMap<String, Value>,
    dest_width: u32,
    dest_height: u32,
) -> Result<Option<[f32; 5]>, RenderError> {
    let view_mode = integer(uniforms, "viewMode")?;
    if view_mode == 0 {
        return Ok(Some([x * 2.0 - 1.0, y * 2.0 - 1.0, 80.0, 0.0, 1.0]));
    }
    let is_2d =
        view_mode == 1 && z.abs() < 1.0 && (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y);
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
    let (x2, y2, z2) = (x1 * cos_y + z1 * sin_y, y1, -x1 * sin_y + z1 * cos_y);
    let rotate_z = scalar(uniforms, "rotateZ")?;
    let (cos_z, sin_z) = (rotate_z.cos(), rotate_z.sin());
    let fx = x2 * cos_z - y2 * sin_z + scalar(uniforms, "posX")?;
    let fy = x2 * sin_z + y2 * cos_z + scalar(uniforms, "posY")?;
    let fz = z2 + scalar_or(uniforms, "posZ", 0.0)?;
    let camera_depth = 80.0 - fz;
    let camera_distance = (fx * fx + fy * fy + camera_depth * camera_depth).sqrt();
    let scale = scalar(uniforms, "viewScale")?;
    if view_mode == 2 {
        if camera_depth <= 0.1 {
            return Ok(None);
        }
        let field_of_view = scalar_or(uniforms, "fieldOfView", 60.0)?.clamp(10.0, 150.0);
        let focal_length = 1.0 / (field_of_view * 0.00872664626).tan();
        let mut clip_x = fx * focal_length * scale / camera_depth;
        if dest_width != 0 {
            clip_x *= dest_height as f32 / dest_width as f32;
        }
        let clip_y = fy * focal_length * scale / camera_depth;
        let projected_scale = 80.0 * focal_length * scale / (1.732050808 * camera_depth);
        return Ok(Some([
            clip_x,
            clip_y,
            camera_depth,
            camera_distance,
            projected_scale,
        ]));
    }
    let [clip_x, clip_y] = if is_2d {
        [fx * 3.5 * scale, fy * 3.5 * scale]
    } else {
        [fx / 40.0 * scale, fy / 40.0 * scale]
    };
    Ok(Some([clip_x, clip_y, camera_depth, camera_distance, 1.0]))
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
    execute_draw_pass_with(
        key,
        pass,
        uniforms,
        resources,
        width,
        height,
        formats,
        &crate::external_input::ExternalInputs::default(),
    )
}

/// [`execute_draw_pass`] with the render's external inputs (reactive MIDI/audio
/// state and packed mesh data); the deposit adapters ignore them, the mesh
/// triangles adapter consumes them.
#[allow(clippy::too_many_arguments)]
pub fn execute_draw_pass_with(
    key: &str,
    pass: &RenderPass,
    uniforms: &BTreeMap<String, Value>,
    resources: &mut BTreeMap<String, Surface>,
    width: u32,
    height: u32,
    formats: &BTreeMap<String, TextureFormat>,
    external_inputs: &crate::external_input::ExternalInputs,
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
        "render/meshRender:render" => {
            mesh_render_triangles(uniforms, external_inputs, &mut destination)?
        }
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
        let Some(
            [
                clip_x,
                clip_y,
                _camera_depth,
                _camera_distance,
                _projected_scale,
            ],
        ) = compute_clip_center(
            position[0],
            position[1],
            position[2],
            uniforms,
            destination.width(),
            destination.height(),
        )?
        else {
            return Ok(());
        };
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
        let mut color = texel_fetch_agent(rgba, sx, sy);
        let Some(
            [
                center_x,
                center_y,
                _camera_depth,
                camera_distance,
                projected_scale,
            ],
        ) = compute_clip_center(
            position[0],
            position[1],
            position[2],
            uniforms,
            destination_width,
            destination_height,
        )?
        else {
            return Ok(());
        };
        let center = [center_x, center_y];
        // Distance-based size/brightness fade (viewMode ortho or perspective
        // only; flat mode's fixed camera_distance=0 makes both no-ops).
        let mut size_fade = 1.0;
        let size_distance = f64::from(scalar_or(uniforms, "sizeDistance", 0.0)?);
        if size_distance > 0.0 {
            size_fade = 1.0 - smoothstep_f64(0.0, size_distance, f64::from(camera_distance));
        }
        let brightness_distance = f64::from(scalar_or(uniforms, "brightnessDistance", 0.0)?);
        if brightness_distance > 0.0 {
            let brightness_fade =
                1.0 - smoothstep_f64(0.0, brightness_distance, f64::from(camera_distance));
            for channel in 0..4 {
                color[channel] = (f64::from(color[channel]) * brightness_fade) as f32;
            }
        }
        let size = point_size
            * f64::from(projected_scale)
            * size_fade
            * (1.0 - size_variation * (billboard_hash(vertex as f32, seed) - 0.5));
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

// ---------------------------------------------------------------------------
// render/meshRender triangle-mesh rasterizer
// ---------------------------------------------------------------------------

/// CPU triangle-mesh rasterizer for `drawMode: 'triangles'` passes
/// (`render/meshRender`).
///
/// Byte-for-byte port of the the JavaScript CPU port `src/effects/cpu/mesh-render.js`
/// adapter, which follows the upstream draw exactly (shaders/src/runtime/
/// backends/webgl2.js triangle-mesh mode):
///   - drawArrays(TRIANGLES) over one texel per vertex of the mesh positions
///     texture, consecutive texel triples forming a de-indexed triangle soup
///     (`parse_obj` packs exactly that order, so no index buffer exists on
///     either side);
///   - depth test LESS against a per-pass depth buffer cleared to 1.0, back-face
///     culling with CCW = front, blending disabled;
///   - the vertex stage is `render.vert` (mesh texture fetch, scale/offset,
///     Rz*Ry*Rx rotation in degrees, orthographic projection with viewScale and
///     aspect divide, z mapped to [0, 1] over nearZ -10 / farZ 10) and the
///     fragment stage is `render.frag` (Blinn-Phong diffuse/specular, ambient,
///     Fresnel rim, optional wireframe discard via screen-space normal
///     derivatives, gamma 1/2.2).
///
/// Floating point follows GLSL f32 semantics: every elementary operation is a
/// native f32 computation (the CPU port's `Math.fround` discipline), while the
/// transcendentals (`cos`, `sin`, `pow`, `hypot`, `sqrt`) evaluate in f64 on the
/// f32 operands exactly like JavaScript's `Math.*` and are rounded back to f32.
#[allow(clippy::too_many_lines, clippy::neg_cmp_op_on_partial_ord)]
fn mesh_render_triangles(
    uniforms: &BTreeMap<String, Value>,
    external_inputs: &crate::external_input::ExternalInputs,
    destination: &mut Surface,
) -> Result<usize, RenderError> {
    let Some(mesh_data) = &external_inputs.mesh_data else {
        return Err(RenderError::InvalidGraph {
            message: "render/meshRender requires external mesh data (external_inputs.mesh_data)"
                .into(),
        });
    };
    let positions = &mesh_data.position_data;
    let normals = &mesh_data.normal_data;
    let tex_width = if mesh_data.tex_width == 0 {
        256
    } else {
        mesh_data.tex_width
    };
    let tex_height = if mesh_data.tex_height == 0 {
        256
    } else {
        mesh_data.tex_height
    };
    let width = destination.width();
    let height = destination.height();
    let aspect = width as f32 / height as f32;
    let wireframe = scalar_or(uniforms, "wireframe", 0.0)? as i32;
    let view = MeshUniforms::from_uniforms(uniforms, aspect, wireframe)?;

    // Per-pixel depth buffer cleared to 1.0 (gl.clear(DEPTH_BUFFER_BIT) each pass).
    let mut depth = vec![1.0_f32; (width * height) as usize];
    let mut covered = 0_usize;
    let vertex_count = tex_width * tex_height;
    let triangle_count = vertex_count / 3;
    for tri in 0..triangle_count {
        let mut verts = [MeshVertex::default(); 3];
        let mut all_invalid = true;
        for (v, vert) in verts.iter_mut().enumerate() {
            let texel = tri * 3 + v;
            let x = texel % tex_width;
            let y = texel / tex_width;
            let pi = (y * tex_width + x) * 4;
            let pos_w = positions[pi + 3];
            if pos_w != 0.0 {
                all_invalid = false;
            }
            *vert = vertex_stage(
                [positions[pi], positions[pi + 1], positions[pi + 2], pos_w],
                [
                    normals[pi],
                    normals[pi + 1],
                    normals[pi + 2],
                    normals[pi + 3],
                ],
                &view,
                width,
                height,
            );
        }
        if all_invalid {
            continue;
        }
        let [v0, v1, v2] = verts;
        // Signed area in GL window space (y-up); CCW = front face.
        let area = (v1.px - v0.px) * (v2.py - v0.py) - (v2.px - v0.px) * (v1.py - v0.py);
        if !(area > 0.0) {
            continue; // back face or degenerate: culled
        }
        // Analytic screen-space derivatives of the interpolated normal (wireframe).
        let det = v0.px * (v1.py - v2.py) + v1.px * (v2.py - v0.py) + v2.px * (v0.py - v1.py);
        let mut d_fdx_normal = [0.0_f32; 3];
        let mut d_fdy_normal = [0.0_f32; 3];
        if view.wireframe == 1 && det != 0.0 {
            // dFdx(vNormal) and dFdy(vNormal): standard barycentric-gradient
            // numerators with the full determinant dividing the SUM (the det
            // division applies to the complete edge-function numerator, not just
            // its last term).
            let d_ndx = |comp: usize| {
                (v0.normal[comp] * (v1.py - v2.py)
                    + v1.normal[comp] * (v2.py - v0.py)
                    + v2.normal[comp] * (v0.py - v1.py))
                    / det
            };
            let d_ndy = |comp: usize| {
                (v0.normal[comp] * (v2.px - v1.px)
                    + v1.normal[comp] * (v0.px - v2.px)
                    + v2.normal[comp] * (v1.px - v0.px))
                    / det
            };
            d_fdx_normal = [d_ndx(0), d_ndx(1), d_ndx(2)];
            d_fdy_normal = [d_ndy(0), d_ndy(1), d_ndy(2)];
        }
        // Bounding box of the triangle, clamped to the viewport.
        let min_px = v0.px.min(v1.px).min(v2.px);
        let max_px = v0.px.max(v1.px).max(v2.px);
        let min_py = v0.py.min(v1.py).min(v2.py);
        let max_py = v0.py.max(v1.py).max(v2.py);
        let min_x = ((min_px - 0.5).floor() as i64).max(0);
        let max_x = ((max_px - 0.5).ceil() as i64)
            .min(i64::from(width) - 1)
            .max(-1);
        let min_y_gl = ((min_py - 0.5).floor() as i64).max(0);
        let max_y_gl = ((max_py - 0.5).ceil() as i64)
            .min(i64::from(height) - 1)
            .max(-1);
        let data = destination.data_mut();
        for py_gl in min_y_gl..=max_y_gl {
            // Surface rows are top-down; GL window y is bottom-up.
            let row = (i64::from(height) - 1 - py_gl) as usize;
            let cy = py_gl as f32 + 0.5;
            for px_gl in min_x..=max_x {
                let cx = px_gl as f32 + 0.5;
                // Barycentric coordinates via edge functions (CCW, positive area).
                let b0 = ((v1.px - v0.px) * (cy - v0.py) - (v1.py - v0.py) * (cx - v0.px)) / area;
                let b1 = ((v2.px - v1.px) * (cy - v1.py) - (v2.py - v1.py) * (cx - v1.px)) / area;
                let b2 = 1.0 - (b0 + b1);
                if !(b0 >= 0.0 && b1 >= 0.0 && b2 >= 0.0) {
                    continue;
                }
                let z = (b0 * v0.z + b1 * v1.z) + b2 * v2.z;
                let depth_index = row * width as usize + px_gl as usize;
                if !(z < depth[depth_index]) {
                    continue; // depthFunc LESS
                }
                depth[depth_index] = z;
                let v_normal = [
                    (b0 * v0.normal[0] + b1 * v1.normal[0]) + b2 * v2.normal[0],
                    (b0 * v0.normal[1] + b1 * v1.normal[1]) + b2 * v2.normal[1],
                    (b0 * v0.normal[2] + b1 * v1.normal[2]) + b2 * v2.normal[2],
                ];
                // The CPU port computes the interpolated position and passes it to
                // the fragment stage, which ignores it; keep the computation for
                // parity shape and let the unused-value lint stay silent.
                let _v_position = [
                    (b0 * v0.position[0] + b1 * v1.position[0]) + b2 * v2.position[0],
                    (b0 * v0.position[1] + b1 * v1.position[1]) + b2 * v2.position[1],
                    (b0 * v0.position[2] + b1 * v1.position[2]) + b2 * v2.position[2],
                ];
                let Some(color) = fragment_stage(v_normal, &view, d_fdx_normal, d_fdy_normal)
                else {
                    continue; // wireframe discard
                };
                let out_index = depth_index * 4;
                data[out_index] = color[0];
                data[out_index + 1] = color[1];
                data[out_index + 2] = color[2];
                data[out_index + 3] = 1.0;
                covered += 1;
            }
        }
    }
    Ok(covered)
}

#[derive(Clone, Copy, Default)]
struct MeshVertex {
    px: f32,
    py: f32,
    z: f32,
    normal: [f32; 3],
    position: [f32; 3],
}

/// Flattened uniform bag for the mesh vertex/fragment stages.
#[derive(Clone, Copy)]
struct MeshUniforms {
    mesh_scale: f32,
    mesh_offset_x: f32,
    mesh_offset_y: f32,
    mesh_offset_z: f32,
    rotate_x: f32,
    rotate_y: f32,
    rotate_z: f32,
    view_scale: f32,
    pos_x: f32,
    pos_y: f32,
    light_direction: [f32; 3],
    diffuse_color: [f32; 3],
    diffuse_intensity: f32,
    specular_color: [f32; 3],
    specular_intensity: f32,
    shininess: f32,
    ambient_color: [f32; 3],
    rim_intensity: f32,
    rim_power: f32,
    mesh_color: [f32; 3],
    wireframe: i32,
    aspect: f32,
}

impl MeshUniforms {
    fn from_uniforms(
        uniforms: &BTreeMap<String, Value>,
        aspect: f32,
        wireframe: i32,
    ) -> Result<Self, RenderError> {
        fn float(uniforms: &BTreeMap<String, Value>, name: &str) -> Result<f32, RenderError> {
            scalar(uniforms, name)
        }
        fn vec3(uniforms: &BTreeMap<String, Value>, name: &str) -> Result<[f32; 3], RenderError> {
            match uniforms.get(name) {
                Some(Value::Vec(values)) if values.len() == 3 => {
                    Ok([values[0], values[1], values[2]])
                }
                value => Err(RenderError::InvalidGraph {
                    message: format!("mesh uniform {name:?} is not a vec3: {value:?}"),
                }),
            }
        }
        Ok(Self {
            mesh_scale: float(uniforms, "meshScale")?,
            mesh_offset_x: float(uniforms, "meshOffsetX")?,
            mesh_offset_y: float(uniforms, "meshOffsetY")?,
            mesh_offset_z: float(uniforms, "meshOffsetZ")?,
            rotate_x: float(uniforms, "rotateX")?,
            rotate_y: float(uniforms, "rotateY")?,
            rotate_z: float(uniforms, "rotateZ")?,
            view_scale: float(uniforms, "viewScale")?,
            pos_x: float(uniforms, "posX")?,
            pos_y: float(uniforms, "posY")?,
            light_direction: vec3(uniforms, "lightDirection")?,
            diffuse_color: vec3(uniforms, "diffuseColor")?,
            diffuse_intensity: float(uniforms, "diffuseIntensity")?,
            specular_color: vec3(uniforms, "specularColor")?,
            specular_intensity: float(uniforms, "specularIntensity")?,
            shininess: float(uniforms, "shininess")?,
            ambient_color: vec3(uniforms, "ambientColor")?,
            rim_intensity: float(uniforms, "rimIntensity")?,
            rim_power: float(uniforms, "rimPower")?,
            mesh_color: vec3(uniforms, "meshColor")?,
            wireframe,
            aspect,
        })
    }
}

/// Vertex stage of render.vert for one mesh texel.
#[allow(clippy::many_single_char_names)]
fn vertex_stage(
    pos_data: [f32; 4],
    normal_data: [f32; 4],
    u: &MeshUniforms,
    width: u32,
    height: u32,
) -> MeshVertex {
    let position = vec3(pos_data[0], pos_data[1], pos_data[2]);
    let normal = vec3(normal_data[0], normal_data[1], normal_data[2]);
    let position = vec3(
        position[0] * u.mesh_scale,
        position[1] * u.mesh_scale,
        position[2] * u.mesh_scale,
    );
    let position = vec3(
        position[0] + u.mesh_offset_x,
        position[1] + u.mesh_offset_y,
        position[2] + u.mesh_offset_z,
    );
    let deg2rad: f32 = 3.141_592_65 / 180.0;
    let rx = u.rotate_x * deg2rad;
    let ry = u.rotate_y * deg2rad;
    let rz = u.rotate_z * deg2rad;
    // Math.cos / Math.sin evaluate in f64 on the f32 operand (JavaScript
    // semantics), then round back to f32.
    let cx = (f64::from(rx).cos()) as f32;
    let sx = (f64::from(rx).sin()) as f32;
    let cy = (f64::from(ry).cos()) as f32;
    let sy = (f64::from(ry).sin()) as f32;
    let cz = (f64::from(rz).cos()) as f32;
    let sz = (f64::from(rz).sin()) as f32;
    // mat3 rotationZ * rotationY * rotationX (GLSL column-major constructor
    // values inlined).
    let rot_x = [1.0, 0.0, 0.0, 0.0, cx, sx, 0.0, -sx, cx];
    let rot_y = [cy, 0.0, sy, 0.0, 1.0, 0.0, -sy, 0.0, cy];
    let rot_z = [cz, -sz, 0.0, sz, cz, 0.0, 0.0, 0.0, 1.0];
    let rotation = mul_mat3(&mul_mat3(&rot_z, &rot_y), &rot_x);
    let rotated_pos = apply_mat3(&rotation, &position);
    let rotated_normal = apply_mat3(&rotation, &normal);
    let rotated_pos = vec3(
        rotated_pos[0] + u.pos_x,
        rotated_pos[1] + u.pos_y,
        rotated_pos[2],
    );
    let mut clip_x = rotated_pos[0] * u.view_scale;
    let clip_y = rotated_pos[1] * u.view_scale;
    clip_x /= u.aspect;
    let near_z = -10.0_f32;
    let far_z = 10.0_f32;
    let ndc_z = (rotated_pos[2] - near_z) / (far_z - near_z);
    // Window coordinates, GL bottom-up: px = (ndcX + 1) / 2 * width.
    let px = ((clip_x + 1.0) * 0.5) * width as f32;
    let py = ((clip_y + 1.0) * 0.5) * height as f32;
    MeshVertex {
        px,
        py,
        z: ndc_z,
        normal: rotated_normal,
        position: rotated_pos,
    }
}

/// Fragment stage of render.frag for one covered pixel. Returns `None` for the
/// wireframe interior discard (the interpolated position argument exists for
/// parity with the CPU port's signature).
fn fragment_stage(
    v_normal: [f32; 3],
    u: &MeshUniforms,
    d_fdx_normal: [f32; 3],
    d_fdy_normal: [f32; 3],
) -> Option<[f32; 3]> {
    let normal = vec3_normalize(v_normal);
    let light_dir = vec3_normalize(u.light_direction);
    let view_dir = [0.0_f32, 0.0, 1.0];
    let mesh_color = u.mesh_color;
    let ambient = [
        u.ambient_color[0] * mesh_color[0],
        u.ambient_color[1] * mesh_color[1],
        u.ambient_color[2] * mesh_color[2],
    ];
    let diffuse_factor =
        (normal[0] * light_dir[0] + normal[1] * light_dir[1] + normal[2] * light_dir[2]).max(0.0);
    let diffuse = [
        (u.diffuse_color[0] * diffuse_factor) * mesh_color[0] * u.diffuse_intensity,
        (u.diffuse_color[1] * diffuse_factor) * mesh_color[1] * u.diffuse_intensity,
        (u.diffuse_color[2] * diffuse_factor) * mesh_color[2] * u.diffuse_intensity,
    ];
    let half_dir = vec3_normalize([
        light_dir[0] + view_dir[0],
        light_dir[1] + view_dir[1],
        light_dir[2] + view_dir[2],
    ]);
    let spec_angle =
        (half_dir[0] * normal[0] + half_dir[1] * normal[1] + half_dir[2] * normal[2]).max(0.0);
    let specular_factor = if spec_angle == 0.0 && u.shininess == 0.0 {
        1.0
    } else {
        // Math.pow evaluates in f64 on the f32 operands (JavaScript semantics).
        f64::from(spec_angle).powf(f64::from(u.shininess)) as f32
    };
    let specular = [
        (u.specular_color[0] * specular_factor) * u.specular_intensity,
        (u.specular_color[1] * specular_factor) * u.specular_intensity,
        (u.specular_color[2] * specular_factor) * u.specular_intensity,
    ];
    let rim_base = 1.0
        - (normal[0] * view_dir[0] + normal[1] * view_dir[1] + normal[2] * view_dir[2]).max(0.0);
    let rim = if rim_base == 0.0 && u.rim_power == 0.0 {
        1.0
    } else {
        f64::from(rim_base).powf(f64::from(u.rim_power)) as f32
    };
    let rim_light = [
        rim * u.rim_intensity,
        rim * u.rim_intensity,
        rim * u.rim_intensity,
    ];
    let mut color = [
        (ambient[0] + diffuse[0]) + (specular[0] + rim_light[0]),
        (ambient[1] + diffuse[1]) + (specular[1] + rim_light[1]),
        (ambient[2] + diffuse[2]) + (specular[2] + rim_light[2]),
    ];
    if u.wireframe == 1 {
        // dFdx/dFdy of the interpolated normal, evaluated analytically per
        // triangle by the caller (screen-space derivatives are
        // per-triangle-constant here up to the GPU's 2x2 helper-quad mixing at
        // edges). The caller passes them via the derivative arguments.
        // Math.hypot(a, b, c) with f32 components evaluated in f64.
        let hypot = |v: [f32; 3]| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        let normal_edge = hypot(d_fdx_normal) + hypot(d_fdy_normal);
        if normal_edge < 0.1 {
            return None; // discard: interior pixel
        }
        color = mesh_color;
    }
    // Gamma correction: pow(color, 1/2.2)
    let gamma: f32 = 1.0 / 2.2;
    Some([
        (f64::from(color[0]).powf(f64::from(gamma))) as f32,
        (f64::from(color[1]).powf(f64::from(gamma))) as f32,
        (f64::from(color[2]).powf(f64::from(gamma))) as f32,
    ])
}

fn vec3(x: f32, y: f32, z: f32) -> [f32; 3] {
    [x, y, z]
}

fn vec3_normalize(v: [f32; 3]) -> [f32; 3] {
    let len_sq = (v[0] * v[0] + v[1] * v[1]) + v[2] * v[2];
    if len_sq == 0.0 {
        return [0.0, 0.0, 0.0];
    }
    let inv_len = (1.0 / f64::from(len_sq).sqrt()) as f32;
    [v[0] * inv_len, v[1] * inv_len, v[2] * inv_len]
}

/// `a * b` with column-major mat3 layout:
/// `out[col*3+row] = sum a[k*3+row]*b[col*3+k]`, every product/sum f32-rounded.
fn mul_mat3(a: &[f32; 9], b: &[f32; 9]) -> [f32; 9] {
    let mut out = [0.0_f32; 9];
    for col in 0..3 {
        for row in 0..3 {
            out[col * 3 + row] = (a[row] * b[col * 3])
                + (a[3 + row] * b[col * 3 + 1])
                + (a[6 + row] * b[col * 3 + 2]);
        }
    }
    out
}

fn apply_mat3(m: &[f32; 9], v: &[f32; 3]) -> [f32; 3] {
    [
        (m[0] * v[0]) + (m[3] * v[1]) + (m[6] * v[2]),
        (m[1] * v[0]) + (m[4] * v[1]) + (m[7] * v[2]),
        (m[2] * v[0]) + (m[5] * v[1]) + (m[8] * v[2]),
    ]
}
