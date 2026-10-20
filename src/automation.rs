// CPU port of upstream Noisemaker automation evaluation for the `osc(...)`
// parameter values of the DSL. Upstream resolves these per frame inside
// Pipeline.resolveUniformValue with a normalized 0..1 loop time; the port
// resolves them per render in the renderer with the render options' time, which
// is the same normalized time the canonical kernels receive as their `time`
// uniform.
//
// MIDI and audio input are unsupported in CPU ports: their DSL calls warn and
// leave parameter defaults in place, so this evaluator handles only time-based
// osc(). The math is a verbatim port of the upstream evaluation, including the
// noise2d two-stage periodic noise (speed applied once, after the first
// periodic wrap, matching the osc2d shader). Every computation runs in f64,
// mirroring JavaScript Numbers.

use crate::catalog::ParameterSpec;
use serde_json::Value as JsonValue;

pub const MAX_AUTOMATION_DEPTH: usize = 8;

const TAU: f64 = std::f64::consts::TAU;

// A numeric automation field: a literal number, or a nested automation value.
#[derive(Clone, Debug, PartialEq)]
pub enum AutomationField {
    Number(f64),
    Ref(Box<AutomationValue>),
}

impl AutomationField {
    #[must_use]
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(*value),
            Self::Ref(_) => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum AutomationValue {
    Oscillator {
        osc_type: i32,
        min: AutomationField,
        max: AutomationField,
        speed: AutomationField,
        offset: AutomationField,
        seed: AutomationField,
    },
}

fn scale_automation_value(value: f64, range: Option<(f64, f64)>) -> f64 {
    match range {
        Some((min, max)) if min.is_finite() && max.is_finite() => min + value * (max - min),
        _ => value,
    }
}

fn resolve_automation_field(
    field: &AutomationField,
    normalized_time: f64,
    range: (f64, f64),
    depth: usize,
    fallback: f64,
) -> f64 {
    match field {
        AutomationField::Ref(value) => {
            evaluate_automation(value, normalized_time, Some(range), depth + 1)
        }
        AutomationField::Number(value) => {
            if value.is_finite() {
                *value
            } else {
                fallback
            }
        }
    }
}

const AUTOMATION_FIELD_UNIT: (f64, f64) = (0.0, 1.0);
const AUTOMATION_FIELD_OSCILLATOR_SPEED: (f64, f64) = (-20.0, 20.0);
const AUTOMATION_FIELD_OSCILLATOR_OFFSET: (f64, f64) = (-1.0, 1.0);
const AUTOMATION_FIELD_OSCILLATOR_SEED: (f64, f64) = (1.0, 9999.0);

fn osc_sine(t: f64) -> f64 {
    // Smooth continuous sine: 0->1->0 over t=0..1, no discontinuity at wrap
    (1.0 - (t * TAU).cos()) * 0.5
}

fn osc_tri(t: f64) -> f64 {
    // Triangle wave: 0->1->0 over t=0..1
    let tf = t - t.floor();
    1.0 - (tf * 2.0 - 1.0).abs()
}

fn osc_saw(t: f64) -> f64 {
    // Sawtooth: 0->1 over t=0..1
    t - t.floor()
}

fn osc_saw_inv(t: f64) -> f64 {
    // Inverted sawtooth: 1->0 over t=0..1
    1.0 - (t - t.floor())
}

fn osc_square(t: f64) -> f64 {
    // Square wave: 0 or 1
    if (t - t.floor()) >= 0.5 { 1.0 } else { 0.0 }
}

// Simple hash for noise
fn hash21(px: f64, py: f64, s: f64) -> f64 {
    let mut x = (px * 234.34 + s) % 1.0;
    let mut y = (py * 435.345 + s) % 1.0;
    if x < 0.0 {
        x += 1.0;
    }
    if y < 0.0 {
        y += 1.0;
    }
    let p = x + y + (x + y) * 34.23;
    (x * y * p) % 1.0
}

// Value noise 2D
fn noise2d(px: f64, py: f64, s: f64) -> f64 {
    let ix = px.floor();
    let iy = py.floor();
    let mut fx = px - ix;
    let mut fy = py - iy;
    fx = fx * fx * (3.0 - 2.0 * fx);
    fy = fy * fy * (3.0 - 2.0 * fy);

    let a = hash21(ix, iy, s);
    let b = hash21(ix + 1.0, iy, s);
    let c = hash21(ix, iy + 1.0, s);
    let d = hash21(ix + 1.0, iy + 1.0, s);

    a * (1.0 - fx) * (1.0 - fy) + b * fx * (1.0 - fy) + c * (1.0 - fx) * fy + d * fx * fy
}

// Looping noise - samples on a circle for seamless temporal loops
fn osc_noise(t: f64, seed: f64) -> f64 {
    let temporal = t % 1.0;
    let angle = temporal * TAU;
    let radius = 2.0;
    let loop_x = angle.cos() * radius;
    let loop_y = angle.sin() * radius;
    let n1 = noise2d(loop_x + seed, loop_y + seed, seed);
    let n2 = noise2d(loop_x + seed * 2.0, loop_y + seed * 2.0, seed);
    (n1 + n2) / 2.0
}

// Two-stage periodic noise (noise2d, kind 6) - mirrors the osc2d effect:
//   scaledTime = periodicValue(time, timeNoise) * speed
//   value      = periodicValue(scaledTime, valueNoise)
// `time` is the normalized loop time plus the phase offset; speed is applied
// once, after the first periodic wrap, exactly as in the osc2d shader.
// osc() has no spatial position, so both noise stages are sampled at a fixed
// position derived from the seed (the osc2d shader salts the second stage with
// +12345). periodicValue() has period 1 in time, so whole-number speeds loop
// seamlessly.
fn osc_noise2d(time: f64, speed: f64, seed: f64) -> f64 {
    let periodic_value = |x: f64, v: f64| (((x - v) * TAU).sin() + 1.0) * 0.5;
    let px = (seed.rem_euclid(16.0) + 0.5) / 16.0;
    let py = ((seed / 16.0).floor().rem_euclid(16.0) + 0.5) / 16.0;
    let time_noise = noise2d(px, py, seed + 12345.0);
    let value_noise = noise2d(px, py, seed);
    let scaled_time = periodic_value(time, time_noise) * speed;
    periodic_value(scaled_time, value_noise)
}

// 16-point Gauss-Legendre nodes and weights on [-1, 1]. Fixed quadrature keeps
// noise and deeply nested rate modulation deterministic and seekable.
const INTEGRATION_NODES: [f64; 16] = [
    -0.9894009349916499,
    -0.9445750230732326,
    -0.8656312023878318,
    -0.755404408355003,
    -0.6178762444026438,
    -0.4580167776572274,
    -0.2816035507792589,
    -0.0950125098376374,
    0.0950125098376374,
    0.2816035507792589,
    0.4580167776572274,
    0.6178762444026438,
    0.755404408355003,
    0.8656312023878318,
    0.9445750230732326,
    0.9894009349916499,
];
const INTEGRATION_WEIGHTS: [f64; 16] = [
    0.0271524594117541,
    0.0622535239386479,
    0.0951585116824928,
    0.1246289712555339,
    0.1495959888165767,
    0.1691565193950025,
    0.1826034150449236,
    0.1894506104550685,
    0.1894506104550685,
    0.1826034150449236,
    0.1691565193950025,
    0.1495959888165767,
    0.1246289712555339,
    0.0951585116824928,
    0.0622535239386479,
    0.0271524594117541,
];
const INTEGRATION_RULE_8_NODES: [f64; 8] = [
    -0.9602898564975363,
    -0.7966664774136267,
    -0.525532409916329,
    -0.1834346424956498,
    0.1834346424956498,
    0.525532409916329,
    0.7966664774136267,
    0.9602898564975363,
];
const INTEGRATION_RULE_8_WEIGHTS: [f64; 8] = [
    0.1012285362903763,
    0.2223810344533745,
    0.3137066458778873,
    0.362683783378362,
    0.362683783378362,
    0.3137066458778873,
    0.2223810344533745,
    0.1012285362903763,
];
const INTEGRATION_RULE_4_NODES: [f64; 4] = [
    -0.8611363115940526,
    -0.3399810435848563,
    0.3399810435848563,
    0.8611363115940526,
];
const INTEGRATION_RULE_4_WEIGHTS: [f64; 4] = [
    0.3478548451374538,
    0.6521451548625461,
    0.6521451548625461,
    0.3478548451374538,
];
const INTEGRATION_RULE_2_NODES: [f64; 2] = [-0.5773502691896257, 0.5773502691896257];
const INTEGRATION_RULE_2_WEIGHTS: [f64; 2] = [1.0, 1.0];

fn can_integrate_oscillator_exactly(osc: &AutomationValue) -> bool {
    let AutomationValue::Oscillator {
        osc_type,
        min,
        max,
        speed,
        offset,
        seed,
    } = osc;
    (0..=4).contains(osc_type)
        && [min, max, speed, offset, seed]
            .iter()
            .all(|field| matches!(field, AutomationField::Number(value) if value.is_finite()))
}

fn osc_primitive(osc_type: i32, x: f64) -> Option<f64> {
    let whole = x.floor();
    let fraction = x - whole;
    match osc_type {
        0 => Some(x * 0.5 - (x * TAU).sin() / (2.0 * TAU)),
        1 => {
            let partial = if fraction < 0.5 {
                fraction * fraction
            } else {
                2.0 * fraction - fraction * fraction - 0.5
            };
            Some(whole * 0.5 + partial)
        }
        2 => Some(whole * 0.5 + fraction * fraction * 0.5),
        3 => Some(x - (whole * 0.5 + fraction * fraction * 0.5)),
        4 => Some(whole * 0.5 + (fraction - 0.5).max(0.0)),
        _ => None,
    }
}

fn integrate_simple_oscillator(config: &AutomationValue, normalized_time: f64) -> f64 {
    let AutomationValue::Oscillator {
        osc_type,
        min,
        max,
        speed,
        offset,
        ..
    } = config;
    let min = min.as_number().unwrap_or(0.0);
    let max = max.as_number().unwrap_or(1.0);
    let speed = speed.as_number().unwrap_or(1.0);
    let offset = offset.as_number().unwrap_or(0.0);
    if speed == 0.0 {
        return evaluate_oscillator(config, 0.0, 0) * normalized_time;
    }
    let start = osc_primitive(*osc_type, offset);
    let end = osc_primitive(*osc_type, offset + speed * normalized_time);
    let raw_integral = match (start, end) {
        (Some(start), Some(end)) => (end - start) / speed,
        _ => 0.0,
    };
    min * normalized_time + (max - min) * raw_integral
}

fn integrate_automation(
    config: &AutomationValue,
    normalized_time: f64,
    range: Option<(f64, f64)>,
    depth: usize,
) -> f64 {
    let integral = if can_integrate_oscillator_exactly(config) {
        integrate_simple_oscillator(config, normalized_time)
    } else {
        // Decrease the quadrature order as rate modulators nest. This bounds an
        // eight-level graph to thousands, rather than millions, of evaluations
        // while retaining the highest precision at the user-visible output.
        let (nodes, weights): (&[f64], &[f64]) = match depth {
            0 => (&INTEGRATION_NODES, &INTEGRATION_WEIGHTS),
            1 => (&INTEGRATION_RULE_8_NODES, &INTEGRATION_RULE_8_WEIGHTS),
            2 => (&INTEGRATION_RULE_4_NODES, &INTEGRATION_RULE_4_WEIGHTS),
            _ => (&INTEGRATION_RULE_2_NODES, &INTEGRATION_RULE_2_WEIGHTS),
        };
        let midpoint = normalized_time * 0.5;
        let half_width = normalized_time * 0.5;
        let mut sum = 0.0;
        for (node, weight) in nodes.iter().zip(weights.iter()) {
            let sample_time = midpoint + half_width * node;
            sum += weight * evaluate_automation(config, sample_time, None, depth + 1);
        }
        half_width * sum
    };

    match range {
        Some((min, max)) if min.is_finite() && max.is_finite() => {
            min * normalized_time + integral * (max - min)
        }
        _ => integral,
    }
}

fn evaluate_oscillator(osc: &AutomationValue, normalized_time: f64, depth: usize) -> f64 {
    let AutomationValue::Oscillator {
        osc_type,
        min,
        max,
        speed,
        offset,
        seed,
    } = osc;
    let min = resolve_automation_field(min, normalized_time, AUTOMATION_FIELD_UNIT, depth, 0.0);
    let max = resolve_automation_field(max, normalized_time, AUTOMATION_FIELD_UNIT, depth, 1.0);
    let offset = resolve_automation_field(
        offset,
        normalized_time,
        AUTOMATION_FIELD_OSCILLATOR_OFFSET,
        depth,
        0.0,
    );
    let seed = resolve_automation_field(
        seed,
        normalized_time,
        AUTOMATION_FIELD_OSCILLATOR_SEED,
        depth,
        1.0,
    );

    // A modulated rate is frequency modulation, so phase is the integral of
    // rate. Literal rates keep the existing closed form exactly.
    let phase = match speed {
        AutomationField::Ref(value) => integrate_automation(
            value,
            normalized_time,
            Some(AUTOMATION_FIELD_OSCILLATOR_SPEED),
            depth,
        ),
        AutomationField::Number(value) => {
            normalized_time * if value.is_finite() { *value } else { 1.0 }
        }
    };
    let t = phase + offset;

    // Get raw oscillator value (0..1)
    let value = match *osc_type {
        0 => osc_sine(t),
        1 => osc_tri(t),
        2 => osc_saw(t),
        3 => osc_saw_inv(t),
        4 => osc_square(t),
        5 => osc_noise(t, seed),
        6 => {
            let speed = resolve_automation_field(
                speed,
                normalized_time,
                AUTOMATION_FIELD_OSCILLATOR_SPEED,
                depth,
                1.0,
            );
            osc_noise2d(
                normalized_time + offset,
                if speed.is_finite() { speed } else { 1.0 },
                seed,
            )
        }
        _ => 0.0,
    };

    // Map to min..max range
    min + value * (max - min)
}

pub fn evaluate_automation(
    config: &AutomationValue,
    normalized_time: f64,
    range: Option<(f64, f64)>,
    depth: usize,
) -> f64 {
    if depth > MAX_AUTOMATION_DEPTH {
        return scale_automation_value(0.0, range);
    }
    let value = evaluate_oscillator(config, normalized_time, depth);
    scale_automation_value(value, range)
}

// The consumer-range spec upstream's expander builds for automation scaling
// (shaders/src/runtime/expander.js uniformSpecs): a float/int parameter without
// choices scales the 0..1 automation output into its declared min..max (0..100
// when undeclared); an int parameter with choices (a conditional selector) is
// only rounded to the selected integer, scaled into its declared range when it
// declares one. Everything else gets no spec, so an automation value resolves
// unscaled.
#[derive(Clone, Copy, Debug)]
pub struct AutomationSpec {
    pub range: Option<(f64, f64)>,
    pub round: bool,
}

#[must_use]
pub fn automation_param_spec(param: &ParameterSpec) -> Option<AutomationSpec> {
    let has_choices = param
        .metadata
        .get("choices")
        .is_some_and(JsonValue::is_object);
    if (param.parameter_type == "float" || param.parameter_type == "int") && !has_choices {
        let minimum = param
            .metadata
            .get("min")
            .and_then(JsonValue::as_f64)
            .unwrap_or(0.0);
        let maximum = param
            .metadata
            .get("max")
            .and_then(JsonValue::as_f64)
            .unwrap_or(100.0);
        return Some(AutomationSpec {
            range: Some((minimum, maximum)),
            round: false,
        });
    }
    if param.parameter_type == "int" && has_choices {
        let minimum = param.metadata.get("min").and_then(JsonValue::as_f64);
        let maximum = param.metadata.get("max").and_then(JsonValue::as_f64);
        return Some(AutomationSpec {
            range: match (minimum, maximum) {
                (Some(minimum), Some(maximum)) if minimum.is_finite() && maximum.is_finite() => {
                    Some((minimum, maximum))
                }
                _ => None,
            },
            round: true,
        });
    }
    None
}

// Port of upstream Pipeline.resolveUniformValue: resolve an automation value for
// the current frame, scaled into the consumer parameter's declared range, with
// the upstream integer rounding for `type: 'int'` consumers.
#[must_use]
pub fn resolve_automation_uniform(
    value: &AutomationValue,
    normalized_time: f64,
    spec: Option<AutomationSpec>,
) -> f64 {
    let resolved = evaluate_automation(value, normalized_time, spec.and_then(|spec| spec.range), 0);
    if spec.is_some_and(|spec| spec.round) {
        resolved.round()
    } else {
        resolved
    }
}
