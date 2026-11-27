use std::collections::BTreeMap;

use super::scalar_number;
use crate::catalog::RenderPass;
use crate::sampler::{sample_bilinear_top_down, sample_nearest_texel};
use crate::{FilterMode, RenderError, Surface, Value};

// The CPU remap factory reads the semantic Number uniforms directly. The GLSL
// data[275] block rounds polygon coordinates to f32 before the edge test.
type Uniforms = BTreeMap<String, Value>;

fn number(uniforms: &Uniforms, name: &str, default: f64) -> Result<f64, RenderError> {
    if uniforms.contains_key(name) {
        scalar_number(uniforms, name)
    } else {
        Ok(default)
    }
}

fn vector<const N: usize>(
    uniforms: &Uniforms,
    name: &str,
    default: [f64; N],
) -> Result<[f64; N], RenderError> {
    let Some(value) = uniforms.get(name) else {
        return Ok(default);
    };
    let mut result = default;
    for (i, lane) in result.iter_mut().enumerate() {
        *lane = match value {
            Value::Vec(values) => values.get(i).copied().map(f64::from),
            Value::Array(values) => values.get(i).and_then(|value| match value {
                Value::Float(value) => Some(*value),
                Value::Int(value) => Some(f64::from(*value)),
                Value::Uint(value) => Some(f64::from(*value)),
                _ => None,
            }),
            _ => None,
        }
        .ok_or_else(|| RenderError::InvalidGraph {
            message: format!("remap uniform {name:?} requires {N} numeric components"),
        })?;
    }
    Ok(result)
}

struct Zone<'a> {
    texture: &'a Surface,
    vertices: Vec<[f64; 2]>,
    bounds: [f64; 4],
    alpha: f64,
}

impl Zone<'_> {
    fn coverage(&self, x: f64, y: f64, feather: f64) -> f64 {
        let mut inside = false;
        let mut nearest_squared: f64 = 1e30;
        let mut previous = self.vertices[self.vertices.len() - 1];
        for &[ax, ay] in &self.vertices {
            let [bx, by] = previous;
            let (ex, ey) = (bx - ax, by - ay);
            let (wx, wy) = (x - ax, y - ay);
            let (c1, c2, c3) = (y >= ay, y < by, ex * wy > ey * wx);
            if (c1 && c2 && c3) || !(c1 || c2 || c3) {
                inside = !inside;
            }
            if feather > 0.0 {
                let t = ((wx * ex + wy * ey) / (ex * ex + ey * ey).max(1e-6)).clamp(0.0, 1.0);
                let (dx, dy) = (wx - ex * t, wy - ey * t);
                nearest_squared = nearest_squared.min(dx * dx + dy * dy);
            }
            previous = [ax, ay];
        }
        if inside {
            1.0
        } else if feather > 0.0 {
            let t = (nearest_squared.sqrt() / feather).clamp(0.0, 1.0);
            1.0 - t * t * (3.0 - 2.0 * t)
        } else {
            0.0
        }
    }
}

pub(super) fn render(
    pass: &RenderPass,
    uniforms: &Uniforms,
    resources: &BTreeMap<String, Surface>,
    destination: &mut Surface,
) -> Result<(), RenderError> {
    let [width, height] = [destination.width(), destination.height()].map(f64::from);
    let [full_w, full_h] = vector(uniforms, "fullResolution", [width, height])?;
    let [offset_x, offset_y] = vector(uniforms, "tileOffset", [0.0; 2])?;
    let feather = number(uniforms, "smoothEdge", 0.04)?.max(0.0) * 0.05 * full_w.min(full_h);
    let bg_alpha = number(uniforms, "bgAlpha", 1.0)?;
    let [r, g, b] = vector(uniforms, "bgColor", [0.0; 3])?;
    let background = [r * bg_alpha, g * bg_alpha, b * bg_alpha, bg_alpha];

    let count = (number(uniforms, "zoneCount", 0.0)? as usize).min(8);
    let mut zones = Vec::new();
    for i in (0..count).rev() {
        let prefix = format!("zone{i}_");
        let vertices = (number(uniforms, &format!("{prefix}count"), 0.0)? as usize).min(64);
        if vertices < 3 || number(uniforms, &format!("{prefix}active"), 0.0)? < 0.5 {
            continue;
        }
        let Some(texture) = pass
            .inputs
            .get(&format!("{prefix}tex"))
            .and_then(|name| resources.get(name))
        else {
            continue;
        };
        let mut points = Vec::with_capacity(vertices);
        for pair in 0..vertices.div_ceil(2) {
            let [x0, y0, x1, y1] = vector(uniforms, &format!("{prefix}v{pair}"), [0.0; 4])?;
            points.push([x0 * full_w, y0 * full_h]);
            if points.len() < vertices {
                points.push([x1 * full_w, y1 * full_h]);
            }
        }
        let [left, top, right, bottom] =
            vector(uniforms, &format!("{prefix}bounds"), [0.0, 0.0, 1.0, 1.0])?;
        zones.push(Zone {
            texture,
            vertices: points,
            bounds: [
                left - feather / full_w,
                top - feather / full_h,
                right + feather / full_w,
                bottom + feather / full_h,
            ],
            alpha: number(uniforms, &format!("{prefix}alpha"), 1.0)?,
        });
    }

    let stride = destination.width() as usize;
    for (pixel, out) in destination.data_mut().chunks_exact_mut(4).enumerate() {
        let frag_x = (pixel % stride) as f64 + 0.5;
        let frag_y = height - (pixel / stride) as f64 - 0.5;
        let (x, y) = (frag_x + offset_x, full_h - (frag_y + offset_y));
        let (px, py) = (x / full_w, y / full_h);
        let (u, v) = (frag_x / width, frag_y / height);
        let mut result = [0.0; 4];
        for zone in &zones {
            let [left, top, right, bottom] = zone.bounds;
            if px < left || py < top || px > right || py > bottom {
                continue;
            }
            let coverage = zone.coverage(x, y, feather);
            if coverage <= 0.0 {
                continue;
            }
            let sample = match zone.texture.filter_mode() {
                FilterMode::Nearest => sample_nearest_texel(zone.texture, u, v),
                FilterMode::Linear => sample_bilinear_top_down(zone.texture, u, 1.0 - v),
            };
            let weight = coverage * zone.alpha;
            let remaining = 1.0 - result[3];
            for (channel, source) in result.iter_mut().zip(sample) {
                *channel += f64::from(source) * weight * remaining;
            }
            if result[3] >= 0.999 {
                break;
            }
        }
        let remaining = 1.0 - result[3];
        for ((lane, value), background) in out.iter_mut().zip(result).zip(background) {
            *lane = (value + background * remaining) as f32;
        }
    }
    Ok(())
}
