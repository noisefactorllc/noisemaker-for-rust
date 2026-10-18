use crate::Surface;

/// Sample a top-down surface with nearest filtering in GLSL bottom-left UV coordinates.
#[must_use]
pub fn sample_nearest(surface: &Surface, u: f32, v: f32) -> [f32; 4] {
    let width = surface.width() as usize;
    let height = surface.height() as usize;
    let x = nearest_index(u, width);
    let shader_y = nearest_index(v, height);
    let storage_y = height - 1 - shader_y;
    read_pixel(surface, x, storage_y)
}

pub(crate) fn sample_nearest_texel(surface: &Surface, u: f64, v: f64) -> [f32; 4] {
    let x = nearest_texel(u, surface.width() as usize);
    let y = surface.height() as usize - 1 - nearest_texel(v, surface.height() as usize);
    read_pixel(surface, x, y)
}

fn nearest_texel(coordinate: f64, size: usize) -> usize {
    let scaled = (coordinate * size as f64) as f32;
    if scaled.is_nan() || f64::from(scaled) >= (size - 1) as f64 {
        size - 1
    } else if scaled <= 0.0 {
        0
    } else {
        scaled.floor() as usize
    }
}

/// Sample a top-down surface with half-texel-centered bilinear filtering in
/// GLSL bottom-left UV coordinates.
#[must_use]
pub fn sample_bilinear(surface: &Surface, u: f32, v: f32) -> [f32; 4] {
    let width = surface.width() as usize;
    let height = surface.height() as usize;
    let px = clamp_coordinate(f64::from(u) * width as f64 - 0.5, width);
    let py = clamp_coordinate(f64::from(v) * height as f64 - 0.5, height);
    let x0 = px.floor() as usize;
    let shader_y0 = py.floor() as usize;
    let x1 = (x0 + 1).min(width - 1);
    let shader_y1 = (shader_y0 + 1).min(height - 1);
    let tx = px - x0 as f64;
    let ty = py - shader_y0 as f64;

    let p00 = pixel_offset(width, x0, height - 1 - shader_y0);
    let p10 = pixel_offset(width, x1, height - 1 - shader_y0);
    let p01 = pixel_offset(width, x0, height - 1 - shader_y1);
    let p11 = pixel_offset(width, x1, height - 1 - shader_y1);
    let data = surface.data();
    let mut output = [0.0; 4];

    for channel in 0..4 {
        let c00 = f64::from(data[p00 + channel]);
        let c10 = f64::from(data[p10 + channel]);
        let c01 = f64::from(data[p01 + channel]);
        let c11 = f64::from(data[p11 + channel]);
        let lower = c00 + (c10 - c00) * tx;
        let upper = c01 + (c11 - c01) * tx;
        output[channel] = (lower + (upper - lower) * ty) as f32;
    }
    output
}

fn nearest_index(coordinate: f32, size: usize) -> usize {
    let scaled = (f64::from(coordinate) * size as f64).floor();
    if scaled.is_nan() || scaled <= 0.0 {
        0
    } else if scaled >= (size - 1) as f64 {
        size - 1
    } else {
        scaled as usize
    }
}

fn clamp_coordinate(coordinate: f64, size: usize) -> f64 {
    if coordinate.is_nan() || coordinate <= 0.0 {
        0.0
    } else {
        coordinate.min((size - 1) as f64)
    }
}

fn read_pixel(surface: &Surface, x: usize, y: usize) -> [f32; 4] {
    let offset = pixel_offset(surface.width() as usize, x, y);
    let data = surface.data();
    [
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]
}

const fn pixel_offset(width: usize, x: usize, y: usize) -> usize {
    (y * width + x) * 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measured_m4_nearest_boundaries_and_special_coordinates() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/cpu-sync/nearest-texel-m4.json"
        ))
        .unwrap();
        for case in fixture["boundaries"].as_array().unwrap() {
            let size = case[0].as_u64().unwrap() as usize;
            let coordinate = f64::from(f32::from_bits(case[1].as_u64().unwrap() as u32));
            assert_eq!(
                nearest_texel(coordinate, size),
                case[2].as_u64().unwrap() as usize,
                "{case}"
            );
        }
        let pixels: Vec<f32> = (0..3)
            .flat_map(|y| (0..41).flat_map(move |x| [x as f32, (2 - y) as f32, 0.0, 1.0]))
            .collect();
        let surface = Surface::from_f32(41, 3, pixels).unwrap();
        for case in fixture["special41x3"].as_array().unwrap() {
            let coordinate =
                |index: usize| f64::from(f32::from_bits(case[index].as_u64().unwrap() as u32));
            let actual = sample_nearest_texel(&surface, coordinate(0), coordinate(1));
            assert_eq!(actual[0], case[2].as_u64().unwrap() as f32, "{case}");
            assert_eq!(actual[1], case[3].as_u64().unwrap() as f32, "{case}");
        }
    }
}
