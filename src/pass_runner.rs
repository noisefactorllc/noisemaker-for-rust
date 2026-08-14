use std::collections::BTreeMap;

use serde_json::Value as JsonValue;

use crate::adapters::{execute_fragment_adapter, execute_semantic_fragment_adapter};
use crate::catalog::{ProgramIr, RenderPass};
use crate::{
    DerivativeMode, FilterMode, PixelContext, RenderError, Runtime, ShaderOutputs, ShaderVm,
    Surface, TextureFormat, Value, quantize_texture,
};

type DerivativeLog = Vec<(String, Value)>;
type QuadLogs = [DerivativeLog; 4];
pub(crate) const RESERVED_FEEDBACK_RESOURCE: &str = "__reserved_feedback";

fn canonical_resource_name(name: &str) -> &str {
    if matches!(name, "selfTex" | "feedback") {
        RESERVED_FEEDBACK_RESOURCE
    } else {
        name
    }
}

pub(crate) fn resource_surface<'a>(
    resources: &'a BTreeMap<String, Surface>,
    name: &str,
) -> Option<&'a Surface> {
    resources.get(canonical_resource_name(name))
}

pub use crate::param::{NormalizedParameters, normalize_parameters};

#[must_use]
pub fn canonical_uniforms(
    width: u32,
    height: u32,
    time: f32,
    seed: i32,
    frame: u32,
    delta_time: f32,
    effect_uniforms: &BTreeMap<String, Value>,
) -> BTreeMap<String, Value> {
    let mut uniforms = BTreeMap::from([
        ("renderScale".into(), Value::Float(1.0)),
        ("speed".into(), Value::Int(0)),
        ("seed".into(), Value::Int(seed)),
        ("centerLoX".into(), Value::Int(0)),
        ("centerLoY".into(), Value::Int(0)),
        ("size".into(), Value::Vec(vec![0.0; 4])),
        ("motion".into(), Value::Vec(vec![0.0; 4])),
    ]);
    uniforms.extend(effect_uniforms.clone());
    let resolution = Value::Vec(vec![width as f32, height as f32]);
    uniforms.extend([
        ("resolution".into(), resolution.clone()),
        ("fullResolution".into(), resolution),
        ("tileOffset".into(), Value::Vec(vec![0.0, 0.0])),
        (
            "aspectRatio".into(),
            Value::Float(width as f32 / height as f32),
        ),
        ("aspect".into(), Value::Float(width as f32 / height as f32)),
        ("time".into(), Value::Float(time)),
        ("globalTime".into(), Value::Float(time)),
        ("frame".into(), Value::Int(frame as i32)),
        ("deltaTime".into(), Value::Float(delta_time)),
        ("seed".into(), Value::Int(seed)),
    ]);
    uniforms
}

const RESERVED_CANONICAL_UNIFORMS: [&str; 10] = [
    "resolution",
    "fullResolution",
    "tileOffset",
    "aspect",
    "aspectRatio",
    "time",
    "globalTime",
    "frame",
    "deltaTime",
    "seed",
];

/// Apply metadata aliases without permitting them to replace renderer-owned
/// canonical bindings.
#[must_use]
pub fn apply_pass_uniform_aliases(
    canonical: &BTreeMap<String, Value>,
    sources: &BTreeMap<String, Value>,
    aliases: &BTreeMap<String, JsonValue>,
) -> BTreeMap<String, Value> {
    let mut pass_uniforms = canonical.clone();
    for (glsl_name, parameter) in aliases {
        if RESERVED_CANONICAL_UNIFORMS.contains(&glsl_name.as_str()) {
            continue;
        }
        if let Some(parameter) = parameter.as_str() {
            if let Some(value) = sources.get(parameter) {
                pass_uniforms.insert(glsl_name.clone(), value.clone());
            }
        }
    }
    pass_uniforms
}

#[must_use]
pub fn pass_enabled(conditions: &JsonValue, uniforms: &BTreeMap<String, Value>) -> bool {
    let check = |entry: &JsonValue| {
        let name = entry.get("uniform").and_then(JsonValue::as_str)?;
        let expected = json_to_value(entry.get("equals")?)?;
        Some(uniforms.get(name) == Some(&expected))
    };
    let run = conditions
        .get("runIf")
        .and_then(JsonValue::as_array)
        .is_none_or(|entries| entries.iter().all(|entry| check(entry) == Some(true)));
    let skip = conditions
        .get("skipIf")
        .and_then(JsonValue::as_array)
        .is_some_and(|entries| entries.iter().any(|entry| check(entry) == Some(true)));
    run && !skip
}

#[must_use]
pub fn repeat_count(repeat: &JsonValue, uniforms: &BTreeMap<String, Value>) -> usize {
    let count = match repeat {
        JsonValue::Number(number) => number.as_i64().unwrap_or(1),
        JsonValue::String(name) => uniforms.get(name).and_then(integer_value).unwrap_or(1),
        _ => 1,
    };
    count.max(0) as usize
}

pub fn texture_dimensions(
    spec: &JsonValue,
    params: &BTreeMap<String, Value>,
    width: u32,
    height: u32,
    resources: &BTreeMap<String, Surface>,
) -> Result<(u32, u32), RenderError> {
    Ok((
        size_component(spec.get("width"), params, width, resources, true)?,
        size_component(spec.get("height"), params, height, resources, false)?,
    ))
}

/// Execute one fragment pass and replace each named output attachment.
#[allow(clippy::too_many_arguments)]
pub fn execute_pass(
    program: &ProgramIr,
    render_pass: &RenderPass,
    uniforms: &BTreeMap<String, Value>,
    resources: &mut BTreeMap<String, Surface>,
    width: u32,
    height: u32,
    formats: &BTreeMap<String, TextureFormat>,
    external_sampler: Option<&str>,
    derivatives: bool,
) -> Result<(), RenderError> {
    if program.outputs.len() != render_pass.outputs.len() {
        return Err(RenderError::InvalidGraph {
            message: format!(
                "pass {:?} maps {} attachments from {} shader outputs",
                render_pass.name,
                render_pass.outputs.len(),
                program.outputs.len()
            ),
        });
    }
    let output_names = program.outputs.clone();
    let attachment_names =
        if output_names.len() == 1 {
            vec![render_pass.outputs.values().next().unwrap().clone()]
        } else {
            output_names
                .iter()
                .map(|name| {
                    render_pass.outputs.get(name).cloned().ok_or_else(|| {
                        RenderError::InvalidGraph {
                            message: format!(
                                "MRT pass {:?} has no attachment for shader output {name:?}",
                                render_pass.name
                            ),
                        }
                    })
                })
                .collect::<Result<Vec<_>, _>>()?
        };
    let viewport = render_pass.execution.get("viewport");
    let fallback_dimensions = texture_dimensions(
        viewport.unwrap_or(&JsonValue::Null),
        uniforms,
        width,
        height,
        resources,
    )?;
    let destination_dimensions = attachment_names
        .iter()
        .map(|name| {
            let dimensions = if viewport.is_none() {
                resources
                    .get(name)
                    .map(|surface| (surface.width(), surface.height()))
                    .unwrap_or(fallback_dimensions)
            } else {
                fallback_dimensions
            };
            (name.clone(), dimensions.0, dimensions.1)
        })
        .collect::<Vec<_>>();
    if destination_dimensions.len() > 1
        && destination_dimensions.iter().skip(1).any(|destination| {
            destination.1 != destination_dimensions[0].1
                || destination.2 != destination_dimensions[0].2
        })
    {
        let detail = destination_dimensions
            .iter()
            .map(|(name, width, height)| format!("{name} ({width}x{height})"))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(RenderError::MrtDestinationDimensions {
            pass: render_pass.name.clone(),
            destinations: destination_dimensions,
            detail,
        });
    }
    let (pass_width, pass_height) = destination_dimensions
        .first()
        .map(|destination| (destination.1, destination.2))
        .unwrap_or(fallback_dimensions);
    if let Some(key) = &render_pass.key {
        let adapted = execute_semantic_fragment_adapter(
            key,
            render_pass,
            uniforms,
            resources,
            pass_width,
            pass_height,
        )?
        .or(execute_fragment_adapter(
            key,
            render_pass,
            uniforms,
            resources,
            pass_width,
            pass_height,
        )?);
        if let Some((attachment, mut surface)) = adapted {
            quantize_texture(
                &mut surface,
                formats.get(&attachment).copied().unwrap_or_default(),
            );
            resources.insert(attachment, surface);
            return Ok(());
        }
    }
    let mut runtime = Runtime::new();
    runtime.set_reduced_turn_sine(render_pass.key.as_deref() == Some("filter/crt:crt"));
    let black = Surface::new(1, 1)?;
    let mut context = PixelContext {
        uniforms: uniforms.clone(),
        ..PixelContext::default()
    };
    let pass_resolution = Value::Vec(vec![pass_width as f32, pass_height as f32]);
    context
        .uniforms
        .insert("resolution".into(), pass_resolution);
    let pass_aspect = Value::Float(pass_width as f32 / pass_height as f32);
    context
        .uniforms
        .insert("aspect".into(), pass_aspect.clone());
    context.uniforms.insert("aspectRatio".into(), pass_aspect);
    for uniform in &program.uniforms {
        if uniform.value_type.0 != "sampler2D" {
            if let Some(value) = context.uniforms.get(&uniform.name).cloned() {
                let converted = runtime.convert(&uniform.value_type.0, &value)?;
                context.uniforms.insert(uniform.name.clone(), converted);
            }
        }
    }
    for uniform in &program.uniforms {
        if uniform.value_type.0 != "sampler2D" {
            continue;
        }
        let source_name = render_pass.inputs.get(&uniform.name);
        let mut surface = source_name
            .and_then(|name| resource_surface(resources, name))
            .cloned()
            .unwrap_or_else(|| black.clone());
        surface.set_filter_mode(if external_sampler == Some(uniform.name.as_str()) {
            FilterMode::Linear
        } else {
            FilterMode::Nearest
        });
        let sampler = runtime.add_texture(surface);
        context.uniforms.insert(uniform.name.clone(), sampler);
    }

    let mut surfaces = output_names
        .iter()
        .map(|_| Surface::new(pass_width, pass_height))
        .collect::<Result<Vec<_>, _>>()?;
    if derivatives {
        render_derivative(
            program,
            runtime,
            &context,
            &output_names,
            &mut surfaces,
            pass_width,
            pass_height,
        )?;
    } else {
        let mut vm = ShaderVm::new(program, runtime)?;
        vm.set_temporal_aberration_factory_compatibility(
            render_pass.key.as_deref() == Some("filter/temporalAberration:temporalAberration"),
        );
        let mut outputs = ShaderOutputs::new();
        for storage_y in 0..pass_height {
            let shader_y = pass_height - storage_y;
            for x in 0..pass_width {
                bind_pixel(&mut context, x, shader_y - 1, pass_width, pass_height);
                match vm.run_pixel(&context, &mut outputs) {
                    Ok(()) => write_outputs(
                        &outputs,
                        &output_names,
                        &mut surfaces,
                        x,
                        storage_y,
                        pass_width,
                    )?,
                    Err(crate::VmError::Discarded) => {}
                    Err(error) => return Err(error.into()),
                }
            }
        }
    }
    for (attachment, surface) in attachment_names.iter().zip(surfaces.iter_mut()) {
        quantize_texture(
            surface,
            formats.get(attachment).copied().unwrap_or_default(),
        );
    }
    for (attachment, surface) in attachment_names.into_iter().zip(surfaces) {
        resources.insert(attachment, surface);
    }
    Ok(())
}

fn bind_pixel(context: &mut PixelContext, x: u32, shader_y: u32, width: u32, height: u32) {
    let fx = x as f32 + 0.5;
    let fy = shader_y as f32 + 0.5;
    let uv = Value::Vec(vec![fx / width as f32, fy / height as f32]);
    context
        .builtins
        .insert("gl_FragCoord".into(), Value::Vec(vec![fx, fy, 0.0, 1.0]));
    for name in ["v_texCoord", "vTexCoord", "texCoord"] {
        context.varyings.insert(name.into(), uv.clone());
    }
}

fn write_outputs(
    outputs: &ShaderOutputs,
    output_names: &[String],
    surfaces: &mut [Surface],
    x: u32,
    storage_y: u32,
    width: u32,
) -> Result<(), RenderError> {
    let offset = ((storage_y * width + x) * 4) as usize;
    for (name, surface) in output_names.iter().zip(surfaces) {
        let value = outputs.get(name).ok_or_else(|| RenderError::InvalidGraph {
            message: format!("shader did not write declared output {name:?}"),
        })?;
        let lanes = value.float_lanes()?;
        if lanes.len() != 4 {
            return Err(RenderError::InvalidGraph {
                message: format!("shader output {name:?} is not vec4"),
            });
        }
        surface.data_mut()[offset..offset + 4].copy_from_slice(lanes);
    }
    Ok(())
}

fn render_derivative(
    program: &ProgramIr,
    runtime: Runtime,
    context: &PixelContext,
    output_names: &[String],
    surfaces: &mut [Surface],
    width: u32,
    height: u32,
) -> Result<(), RenderError> {
    let mut lane_cache: BTreeMap<(u32, u32), QuadLogs> = BTreeMap::new();
    for storage_y in 0..height {
        let pixel_y = height - 1 - storage_y;
        for x in 0..width {
            let quad = (x / 2, pixel_y / 2);
            if let std::collections::btree_map::Entry::Vacant(entry) = lane_cache.entry(quad) {
                let mut logs = [const { Vec::new() }; 4];
                for (lane, log) in logs.iter_mut().enumerate() {
                    let lane_x = quad.0 * 2 + (lane as u32 & 1);
                    let lane_y = quad.1 * 2 + (lane as u32 >> 1);
                    let mut lane_context = context.clone();
                    bind_pixel(&mut lane_context, lane_x, lane_y, width, height);
                    let mut lane_runtime = runtime.clone();
                    lane_runtime.set_derivative_mode(DerivativeMode::Record);
                    let mut lane_vm = ShaderVm::new(program, lane_runtime)?;
                    let mut ignored = ShaderOutputs::new();
                    match lane_vm.run_pixel(&lane_context, &mut ignored) {
                        Ok(()) | Err(crate::VmError::Discarded) => {}
                        Err(error) => return Err(error.into()),
                    }
                    *log = lane_vm.runtime_mut().take_derivative_log();
                }
                entry.insert(logs);
            }
            let logs = &lane_cache[&quad];
            let diffs = Runtime::fine_derivatives(logs, (x & 1) as usize, (pixel_y & 1) as usize)?;
            let mut replay_runtime = runtime.clone();
            replay_runtime.set_derivative_replay(diffs);
            let mut vm = ShaderVm::new(program, replay_runtime)?;
            let mut pixel_context = context.clone();
            bind_pixel(&mut pixel_context, x, pixel_y, width, height);
            let mut outputs = ShaderOutputs::new();
            match vm.run_pixel(&pixel_context, &mut outputs) {
                Ok(()) => write_outputs(&outputs, output_names, surfaces, x, storage_y, width)?,
                Err(crate::VmError::Discarded) => {}
                Err(error) => return Err(error.into()),
            }
        }
    }
    Ok(())
}

fn size_component(
    spec: Option<&JsonValue>,
    params: &BTreeMap<String, Value>,
    full: u32,
    resources: &BTreeMap<String, Surface>,
    width: bool,
) -> Result<u32, RenderError> {
    let Some(spec) = spec else { return Ok(full) };
    if let Some(number) = spec.as_f64() {
        return checked_dimension(number.round());
    }
    if let Some(text) = spec.as_str() {
        if matches!(text, "input" | "screen" | "resolution" | "100%") {
            return Ok(full);
        }
        if let Some(percent) = text.strip_suffix('%') {
            let percent = percent
                .parse::<f64>()
                .map_err(|_| RenderError::InvalidGraph {
                    message: format!("invalid percentage dimension {text:?}"),
                })?;
            return checked_dimension((f64::from(full) * percent / 100.0).round());
        }
        return Ok(full);
    }
    if let Some(object) = spec.as_object() {
        if let Some(name) = object.get("inputOverride").and_then(JsonValue::as_str) {
            if let Some(surface) = resource_surface(resources, name) {
                return Ok(if width {
                    surface.width()
                } else {
                    surface.height()
                });
            }
        }
        if let Some(name) = object.get("param").and_then(JsonValue::as_str) {
            let mut value = params
                .get(name)
                .and_then(numeric_value)
                .or_else(|| object.get("paramDefault").and_then(JsonValue::as_f64))
                .or_else(|| object.get("default").and_then(JsonValue::as_f64))
                .unwrap_or(f64::from(full));
            if let Some(power) = object.get("power").and_then(JsonValue::as_f64) {
                value = value.powf(power);
            }
            return checked_dimension(value.round());
        }
        if let Some(name) = object.get("screenDivide").and_then(JsonValue::as_str) {
            let divisor = params
                .get(name)
                .and_then(numeric_value)
                .or_else(|| object.get("default").and_then(JsonValue::as_f64))
                .unwrap_or(1.0)
                .max(1.0);
            return checked_dimension((f64::from(full) / divisor).ceil());
        }
    }
    Ok(full)
}

fn checked_dimension(value: f64) -> Result<u32, RenderError> {
    if !value.is_finite() || value > f64::from(u32::MAX) {
        Err(RenderError::InvalidGraph {
            message: format!("invalid texture dimension {value}"),
        })
    } else {
        Ok(value.max(1.0) as u32)
    }
}

fn numeric_value(value: &Value) -> Option<f64> {
    match value {
        Value::Int(value) => Some(f64::from(*value)),
        Value::Uint(value) => Some(f64::from(*value)),
        Value::Float(value) => Some(f64::from(*value)),
        _ => None,
    }
}
fn integer_value(value: &Value) -> Option<i64> {
    numeric_value(value).map(|value| value as i64)
}
fn json_to_value(value: &JsonValue) -> Option<Value> {
    match value {
        JsonValue::Bool(value) => Some(Value::Bool(*value)),
        JsonValue::Number(value) if value.is_i64() => Some(Value::Int(value.as_i64()? as i32)),
        JsonValue::Number(value) => Some(Value::Float(value.as_f64()? as f32)),
        _ => None,
    }
}
