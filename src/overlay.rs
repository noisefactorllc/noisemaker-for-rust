#![allow(clippy::needless_range_loop)]

use crate::{Surface, SurfaceError};

const TAU: f64 = std::f64::consts::PI * 2.0;

#[derive(Clone, Copy)]
enum Behavior {
    Obedient,
    Chaotic,
}

struct Rng {
    state: u32,
}
fn js_uint(value: f64) -> u32 {
    value.trunc().rem_euclid(4_294_967_296.0) as u32
}
impl Rng {
    fn new(seed: i64) -> Self {
        Self {
            state: js_uint(f64::from(seed as u32) * 747_796_405.0 + 2_891_336_453.0),
        }
    }
    fn next(&mut self) -> u32 {
        self.state = js_uint(f64::from(self.state) * 747_796_405.0 + 2_891_336_453.0);
        let mixed = ((self.state >> ((self.state >> 28) + 4)) ^ self.state) as i32;
        let word = js_uint(f64::from(mixed) * 277_803_737.0);
        (word >> 22) ^ word
    }
    fn float(&mut self) -> f64 {
        f64::from(self.next()) / 4_294_967_295.0
    }
    fn normal(&mut self, mean: f64, deviation: f64) -> f64 {
        let u1 = self.float().max(1e-10);
        let u2 = self.float();
        mean + deviation * (-2.0 * u1.ln()).sqrt() * (TAU * u2).cos()
    }
}

fn value_noise(width: u32, height: u32, frequency: f64, rng: &mut Rng) -> Vec<f32> {
    let grid_width = frequency.ceil() as usize + 2;
    let grid_height = frequency.ceil() as usize + 2;
    let grid = (0..grid_width * grid_height)
        .map(|_| rng.float() as f32)
        .collect::<Vec<_>>();
    let mut field = vec![0.0; width as usize * height as usize];
    for y in 0..height as usize {
        for x in 0..width as usize {
            let field_x = x as f64 / f64::from(width) * frequency;
            let field_y = y as f64 / f64::from(height) * frequency;
            let integer_x = field_x.floor() as usize;
            let integer_y = field_y.floor() as usize;
            let delta_x = field_x - integer_x as f64;
            let delta_y = field_y - integer_y as f64;
            let smooth_x = delta_x * delta_x * (3.0 - 2.0 * delta_x);
            let smooth_y = delta_y * delta_y * (3.0 - 2.0 * delta_y);
            let top_left = f64::from(grid[integer_y * grid_width + integer_x]);
            let top_right = f64::from(grid[integer_y * grid_width + integer_x + 1]);
            let bottom_left = f64::from(grid[(integer_y + 1) * grid_width + integer_x]);
            let bottom_right = f64::from(grid[(integer_y + 1) * grid_width + integer_x + 1]);
            field[y * width as usize + x] = ((top_left * (1.0 - smooth_x) + top_right * smooth_x)
                * (1.0 - smooth_y)
                + (bottom_left * (1.0 - smooth_x) + bottom_right * smooth_x) * smooth_y)
                as f32;
        }
    }
    field
}

fn draw_segment(
    surface: &mut Surface,
    start: [f64; 2],
    end: [f64; 2],
    line_width: f64,
    color: [f64; 3],
    alpha: f64,
) {
    if alpha <= 0.0 {
        return;
    }
    let radius = line_width * 0.5;
    let min_x = (start[0].min(end[0]) - radius - 1.0).floor().max(0.0) as i64;
    let max_x = (start[0].max(end[0]) + radius + 1.0)
        .ceil()
        .min(f64::from(surface.width() - 1)) as i64;
    let min_y = (start[1].min(end[1]) - radius - 1.0).floor().max(0.0) as i64;
    let max_y = (start[1].max(end[1]) + radius + 1.0)
        .ceil()
        .min(f64::from(surface.height() - 1)) as i64;
    let (dx, dy) = (end[0] - start[0], end[1] - start[1]);
    let length_squared = dx * dx + dy * dy;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
            let amount = if length_squared > 0.0 {
                (((px - start[0]) * dx + (py - start[1]) * dy) / length_squared).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let distance = (px - (start[0] + dx * amount)).hypot(py - (start[1] + dy * amount));
            let source_alpha = alpha * (radius + 0.5 - distance).clamp(0.0, 1.0);
            if source_alpha <= 0.0 {
                continue;
            }
            let offset = ((y * i64::from(surface.width()) + x) * 4) as usize;
            let destination_alpha = f64::from(surface.data()[offset + 3]);
            let output_alpha = source_alpha + destination_alpha * (1.0 - source_alpha);
            for channel in 0..3 {
                let destination = f64::from(surface.data()[offset + channel]);
                surface.data_mut()[offset + channel] = if output_alpha > 0.0 {
                    ((color[channel] * source_alpha
                        + destination * destination_alpha * (1.0 - source_alpha))
                        / output_alpha) as f32
                } else {
                    0.0
                };
            }
            surface.data_mut()[offset + 3] = output_alpha as f32;
        }
    }
}

struct TraceOptions {
    seed: i64,
    density: f64,
    kink: f64,
    stride: f64,
    stride_deviation: f64,
    duration: f64,
    behavior: Behavior,
    flow_frequency: f64,
    line_width: f64,
    alpha: f64,
    color: fn(&mut Rng) -> [f64; 3],
}

fn trace(surface: &mut Surface, options: TraceOptions) {
    let mut rng = Rng::new(options.seed);
    let min_dimension = surface.width().min(surface.height());
    let max_dimension = surface.width().max(surface.height());
    let stride_scale = f64::from(max_dimension) / 1024.0;
    let flow = value_noise(
        surface.width(),
        surface.height(),
        options.flow_frequency,
        &mut Rng::new(options.seed * 31_337),
    );
    let count = (f64::from(max_dimension) * options.density)
        .floor()
        .max(1.0) as usize;
    let shared_rotation = rng.float() * TAU;
    let mut worms = Vec::with_capacity(count);
    for _ in 0..count {
        worms.push((
            rng.float() * f64::from(surface.width()),
            rng.float() * f64::from(surface.height()),
            rng.normal(options.stride, options.stride_deviation) * stride_scale,
            if matches!(options.behavior, Behavior::Obedient) {
                shared_rotation
            } else {
                rng.float() * TAU
            },
            (options.color)(&mut rng),
        ));
    }
    let iterations = (f64::from(min_dimension).sqrt() * options.duration)
        .floor()
        .max(1.0) as usize;
    for (mut x, mut y, stride, rotation, color) in worms {
        for iteration in 0..iterations {
            let lifetime = if iterations > 1 {
                iteration as f64 / (iterations - 1) as f64
            } else {
                1.0
            };
            let exposure = 1.0 - (1.0 - lifetime * 2.0).abs();
            let flow_x = x.rem_euclid(f64::from(surface.width())).floor() as usize;
            let flow_y = y.rem_euclid(f64::from(surface.height())).floor() as usize;
            let angle =
                f64::from(flow[flow_y * surface.width() as usize + flow_x]) * TAU * options.kink
                    + if matches!(options.behavior, Behavior::Obedient) {
                        shared_rotation
                    } else {
                        rotation
                    };
            let next = [x + angle.sin() * stride, y + angle.cos() * stride];
            draw_segment(
                surface,
                [x, y],
                next,
                options.line_width,
                color,
                options.alpha * exposure,
            );
            x = next[0];
            y = next[1];
        }
    }
}

fn bright(rng: &mut Rng) -> [f64; 3] {
    std::array::from_fn(|_| (rng.float() * 200.0 + 55.0).floor() / 255.0)
}
fn white(_: &mut Rng) -> [f64; 3] {
    [1.0; 3]
}
fn dark(rng: &mut Rng) -> [f64; 3] {
    std::array::from_fn(|_| (rng.float() * 30.0).floor() / 255.0)
}

pub fn render_overlay(
    effect: &str,
    width: u32,
    height: u32,
    seed: i32,
    density: f32,
) -> Result<Surface, SurfaceError> {
    let mut surface = Surface::new(width, height)?;
    let seed = if seed == 0 { 1 } else { seed } as i64;
    match effect {
        "filter/fibers" => {
            for layer in 0..4 {
                let layer_seed = seed * 1000 + layer * 137;
                trace(
                    &mut surface,
                    TraceOptions {
                        seed: layer_seed,
                        density: 0.5 + f64::from(density) * 2.0,
                        kink: 5.0 + (layer_seed % 5) as f64,
                        stride: 0.75,
                        stride_deviation: 0.125,
                        duration: 1.0,
                        behavior: Behavior::Chaotic,
                        flow_frequency: 4.0,
                        line_width: (f64::from(width) / 384.0).max(1.5),
                        alpha: 0.5,
                        color: bright,
                    },
                );
            }
        }
        "filter/scratches" => {
            for layer in 0..4 {
                let layer_seed = seed * 1000 + layer * 251;
                trace(
                    &mut surface,
                    TraceOptions {
                        seed: layer_seed,
                        density: 0.1 + f64::from(density) * 0.4,
                        kink: 0.125 + (layer_seed % 50) as f64 / 400.0,
                        stride: 0.75,
                        stride_deviation: 0.5,
                        duration: (2 + layer_seed % 3) as f64,
                        behavior: if layer_seed % 2 == 0 {
                            Behavior::Obedient
                        } else {
                            Behavior::Chaotic
                        },
                        flow_frequency: (2 + layer_seed % 3) as f64,
                        line_width: (f64::from(width) / 1024.0).max(0.5),
                        alpha: 1.0,
                        color: white,
                    },
                );
            }
        }
        "filter/strayHair" => {
            let layer_seed = seed * 1000 + 42;
            trace(
                &mut surface,
                TraceOptions {
                    seed: layer_seed,
                    density: 0.001 + f64::from(density) * 0.004,
                    kink: 5.0 + (layer_seed % 45) as f64,
                    stride: 0.5,
                    stride_deviation: 0.25,
                    duration: (8 + layer_seed % 8) as f64,
                    behavior: Behavior::Chaotic,
                    flow_frequency: 4.0,
                    line_width: (f64::from(width) / 400.0).max(1.0),
                    alpha: 0.666,
                    color: dark,
                },
            );
        }
        _ => unreachable!(),
    }
    for value in surface.data_mut() {
        *value = ((f64::from((*value).clamp(0.0, 1.0)) * 255.0).round() / 255.0) as f32;
    }
    Ok(surface)
}

#[cfg(test)]
mod tests {
    use super::Rng;

    #[test]
    fn rng_matches_javascript_number_and_bitwise_semantics() {
        let mut rng = Rng::new(7000);
        assert_eq!(rng.state, 1_901_037_629);
        assert_eq!(rng.next(), 3_409_099_260);
        assert_eq!(rng.next(), 2_174_012_102);
    }
}
