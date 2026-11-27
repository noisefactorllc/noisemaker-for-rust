use std::collections::BTreeMap;

use serde_json::Value as JsonValue;

use crate::adapters::apply_classic_palette_uniforms;
use crate::automation::{AutomationValue, automation_param_spec, resolve_automation_uniform};
use crate::catalog::{EffectDefinition, ParameterSpec};
use crate::{RenderError, Value};

/// User-facing effect parameter value.
#[derive(Clone, Debug, PartialEq)]
pub enum ParamValue {
    Bool(bool),
    Int(i32),
    Float(f32),
    String(String),
    Vector(Vec<f32>),
    /// Authored values retained by CPU adapters that operate in double precision.
    Float64(f64),
    Vector64(Vec<f64>),
    // An `osc(...)` automation value (numeric params only, matching upstream's
    // uniformSpecs contract). Kept as-is at compile time and resolved to a
    // concrete number per render before normalization, as upstream's
    // CpuRenderer.effectParams does.
    Automation(AutomationValue),
}

pub(crate) fn uses_authored_precision(effect: &EffectDefinition) -> bool {
    effect.namespace == "synth" && effect.func == "remap"
}

impl ParamValue {
    pub(crate) fn number(value: f64, keep_precision: bool) -> Self {
        if keep_precision {
            Self::Float64(value)
        } else {
            Self::Float(value as f32)
        }
    }

    pub(crate) fn vector(values: Vec<f64>, keep_precision: bool) -> Self {
        if keep_precision {
            Self::Vector64(values)
        } else {
            Self::Vector(values.into_iter().map(|x| x as f32).collect())
        }
    }
}

impl From<bool> for ParamValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}
impl From<i32> for ParamValue {
    fn from(value: i32) -> Self {
        Self::Int(value)
    }
}
impl From<f32> for ParamValue {
    fn from(value: f32) -> Self {
        Self::Float(value)
    }
}
impl From<&str> for ParamValue {
    fn from(value: &str) -> Self {
        Self::String(value.into())
    }
}
impl From<String> for ParamValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

#[derive(Clone, Debug, Default)]
pub struct NormalizedParameters {
    pub values: BTreeMap<String, Value>,
    pub uniforms: BTreeMap<String, Value>,
}

// Resolves `osc(...)` automation values in a step's params to concrete numbers
// for this render, using the normalized 0..1 loop
// time the canonical kernels receive. Returns the params unchanged when no
// param carries automation, so every pre-existing program keeps its exact
// identity and byte-identical render path. An int parameter with choices is
// rounded to the selected integer; an int parameter without choices keeps the
// unrounded scaled value, which the int uniform truncates at upload, as
// upstream's uniform1i does.
#[must_use]
pub fn resolve_effect_automation(
    effect: &EffectDefinition,
    params: &BTreeMap<String, ParamValue>,
    normalized_time: f32,
) -> BTreeMap<String, ParamValue> {
    let has_automation = params
        .values()
        .any(|value| matches!(value, ParamValue::Automation(_)));
    if !has_automation {
        return params.clone();
    }
    let mut resolved = params.clone();
    for (name, value) in resolved.iter_mut() {
        let ParamValue::Automation(descriptor) = value else {
            continue;
        };
        let Some(spec) = effect.params.get(name) else {
            continue;
        };
        let automation_spec = automation_param_spec(spec);
        let number =
            resolve_automation_uniform(descriptor, f64::from(normalized_time), automation_spec);
        *value = match automation_spec {
            Some(spec) if spec.round => ParamValue::Int(number as i32),
            Some(_) if spec.parameter_type == "int" => ParamValue::Int(number as i32),
            _ => ParamValue::number(number, uses_authored_precision(effect)),
        };
    }
    resolved
}

pub fn normalize_parameters(
    effect: &EffectDefinition,
    supplied: &BTreeMap<String, ParamValue>,
    render_seed: i32,
) -> Result<NormalizedParameters, RenderError> {
    normalize_bindings(effect, supplied, render_seed, BindingMode::Standalone)
}

// The graph binds one shader-visible key per parameter. In particular, a
// preprocessor define replaces an ordinary uniform of the same parameter.
pub(crate) fn graph_uniform_name<'a>(name: &'a str, spec: &'a ParameterSpec) -> &'a str {
    spec.define
        .as_deref()
        .filter(|key| !key.is_empty())
        .or(spec.uniform.as_deref())
        .unwrap_or(name)
}

pub(crate) fn normalize_graph_parameters(
    effect: &EffectDefinition,
    supplied: &BTreeMap<String, ParamValue>,
    render_seed: i32,
) -> Result<NormalizedParameters, RenderError> {
    normalize_bindings(effect, supplied, render_seed, BindingMode::Graph)
}

#[derive(Clone, Copy, PartialEq)]
enum BindingMode {
    Standalone,
    Graph,
}

fn normalize_bindings(
    effect: &EffectDefinition,
    supplied: &BTreeMap<String, ParamValue>,
    render_seed: i32,
    mode: BindingMode,
) -> Result<NormalizedParameters, RenderError> {
    // A joined group step inherits the owner's stateSize even when it declares
    // no stateSize param of its own: upstream's initializeGroupStepState
    // injects it so the step's `inherit: true` base-color textures size from
    // the group's shared stateSize. The renderer's texture sizing reads it
    // from the normalized values, so it must survive normalization.
    let inherits_state_size = supplied.contains_key("stateSize")
        && !effect.params.contains_key("stateSize")
        && effect.textures.values().any(|texture| {
            ["width", "height"].iter().any(|dimension| {
                texture
                    .get(*dimension)
                    .and_then(serde_json::Value::as_object)
                    .is_some_and(|spec| {
                        spec.get("inherit").and_then(serde_json::Value::as_bool) == Some(true)
                            && spec.get("param").and_then(serde_json::Value::as_str)
                                == Some("stateSize")
                    })
            })
        });
    if let Some(name) = supplied.keys().find(|name| {
        !effect.params.contains_key(name.as_str())
            && !(inherits_state_size && name.as_str() == "stateSize")
    }) {
        return Err(RenderError::UnknownParameter { name: name.clone() });
    }
    let mut normalized = NormalizedParameters::default();
    let keep_precision = uses_authored_precision(effect);
    // buildBindings walks the declared parameter order. If two descriptors
    // name the same graph uniform, the later declaration supplies its value.
    let names: &mut dyn Iterator<Item = &String> = match mode {
        BindingMode::Graph => &mut effect.param_names.iter(),
        BindingMode::Standalone => &mut effect.params.keys(),
    };
    let mut palette = None;
    for name in names {
        let spec = &effect.params[name];
        if matches!(
            spec.parameter_type.as_str(),
            "surface" | "volume" | "geometry"
        ) {
            continue;
        }
        let value = if name == "seed" && !supplied.contains_key(name) {
            coerce(
                name,
                spec,
                Some(&ParamValue::Int(render_seed)),
                keep_precision,
            )?
        } else {
            coerce(name, spec, supplied.get(name), keep_precision)?
        };
        // The packed GLSL block consumes f32 values. The native remap adapter
        // reads the semantic uniforms at the precision authored in the DSL.
        let shader_value = if keep_precision {
            match &value {
                Value::Float(number) => Value::Float(f64::from(*number as f32)),
                Value::Array(lanes) => Value::Vec(
                    lanes
                        .iter()
                        .map(|lane| {
                            let Value::Float(number) = lane else {
                                unreachable!("numeric parameter vector")
                            };
                            *number as f32
                        })
                        .collect(),
                ),
                _ => value.clone(),
            }
        } else {
            value.clone()
        };
        normalized.values.insert(name.clone(), shader_value);
        if spec.parameter_type == "palette" && palette.is_none() {
            if let Value::Int(index) = &value {
                palette = Some(*index);
            }
        }
        match mode {
            BindingMode::Graph => {
                normalized
                    .uniforms
                    .insert(graph_uniform_name(name, spec).to_owned(), value);
            }
            BindingMode::Standalone => {
                if let Some(uniform) = &spec.uniform {
                    normalized.uniforms.insert(uniform.clone(), value.clone());
                }
                if let Some(define) = &spec.define {
                    normalized.uniforms.insert(define.clone(), value);
                }
            }
        }
    }
    if inherits_state_size {
        if let Some(value) = supplied.get("stateSize") {
            match value {
                ParamValue::Int(number) => {
                    normalized
                        .values
                        .insert("stateSize".into(), Value::Int(*number));
                }
                ParamValue::Float(number) => {
                    normalized
                        .values
                        .insert("stateSize".into(), Value::Float(f64::from(*number)));
                }
                _ => {}
            }
        }
    }
    const CLASSIC_PALETTE_UNIFORMS: [&str; 5] = [
        "paletteAmp",
        "paletteFreq",
        "paletteOffset",
        "palettePhase",
        "paletteMode",
    ];
    if effect.namespace == "classicNoisedeck"
        && (mode == BindingMode::Graph
            || CLASSIC_PALETTE_UNIFORMS
                .iter()
                .all(|name| effect.params.contains_key(*name)))
    {
        if let Some(index) = palette {
            apply_classic_palette_uniforms(index, &mut normalized.uniforms)?;
        }
    }
    Ok(normalized)
}

pub(crate) fn validate_parameter_number(
    name: &str,
    spec: &ParameterSpec,
    value: f64,
    integer: bool,
) -> Result<(), String> {
    if !value.is_finite()
        || (integer && (value.fract() != 0.0 || value < i32::MIN as f64 || value > i32::MAX as f64))
    {
        return Err(format!(
            "Parameter {name:?} must be a finite {}",
            if integer { "integer" } else { "number" }
        ));
    }
    if let Some(minimum) = spec.metadata.get("min").and_then(JsonValue::as_f64) {
        if value < minimum {
            return Err(format!("Parameter {name:?} must be at least {minimum}"));
        }
    }
    if let Some(maximum) = spec.metadata.get("max").and_then(JsonValue::as_f64) {
        if value > maximum {
            return Err(format!("Parameter {name:?} must be at most {maximum}"));
        }
    }
    Ok(())
}

fn validate_parameter_float32(name: &str, spec: &ParameterSpec, value: f32) -> Result<(), String> {
    if !value.is_finite() {
        return Err(format!("Parameter {name:?} must be a finite number"));
    }
    if let Some(minimum) = spec.metadata.get("min").and_then(JsonValue::as_f64) {
        if value < minimum as f32 {
            return Err(format!("Parameter {name:?} must be at least {minimum}"));
        }
    }
    if let Some(maximum) = spec.metadata.get("max").and_then(JsonValue::as_f64) {
        if value > maximum as f32 {
            return Err(format!("Parameter {name:?} must be at most {maximum}"));
        }
    }
    Ok(())
}

pub(crate) fn validate_parameter_vector(
    name: &str,
    spec: &ParameterSpec,
    values: &[f64],
) -> Result<(), String> {
    let width_is_valid = match spec.parameter_type.as_str() {
        "color" => matches!(values.len(), 3 | 4),
        "vec2" => values.len() == 2,
        "vec3" => values.len() == 3,
        "vec4" => values.len() == 4,
        "mat3" => values.len() == 9,
        _ => false,
    };
    if !width_is_valid || values.iter().any(|&value| !(value as f32).is_finite()) {
        return Err(format!(
            "Parameter {name:?} must be a finite {}",
            spec.parameter_type
        ));
    }
    Ok(())
}

fn invalid_parameter(name: &str, detail: impl Into<String>) -> RenderError {
    RenderError::InvalidParameter {
        message: format!("Parameter {name:?} {}", detail.into()),
    }
}

fn coerce(
    name: &str,
    spec: &ParameterSpec,
    supplied: Option<&ParamValue>,
    keep_precision: bool,
) -> Result<Value, RenderError> {
    // Validate numeric catalog defaults at their original f64 precision. A
    // round-trip through ParamValue::Float would move some decimal boundary
    // values before their min/max checks.
    if supplied.is_none() {
        if let Some(number) = spec.default.as_f64() {
            return match spec.parameter_type.as_str() {
                "float" => {
                    validate_parameter_number(name, spec, number, false)
                        .map_err(|message| RenderError::InvalidParameter { message })?;
                    if !(number as f32).is_finite() {
                        return Err(invalid_parameter(
                            name,
                            "must be representable as a finite f32 number",
                        ));
                    }
                    Ok(Value::Float(if keep_precision {
                        number
                    } else {
                        f64::from(number as f32)
                    }))
                }
                "int" | "enum" | "member" | "palette" => {
                    validate_parameter_number(name, spec, number, true)
                        .map_err(|message| RenderError::InvalidParameter { message })?;
                    Ok(Value::Int(number as i32))
                }
                "bool" | "boolean" if number == 0.0 => Ok(Value::Bool(false)),
                "bool" | "boolean" if number == 1.0 => Ok(Value::Bool(true)),
                _ => Err(invalid_parameter(
                    name,
                    format!(
                        "cannot coerce numeric default {number} to {}",
                        spec.parameter_type
                    ),
                )),
            };
        }
    }
    let owned_default;
    let value = if let Some(value) = supplied {
        value
    } else {
        owned_default = from_json(name, &spec.default, keep_precision)?;
        &owned_default
    };
    let bad = || {
        invalid_parameter(
            name,
            format!("cannot coerce {value:?} to {}", spec.parameter_type),
        )
    };
    match spec.parameter_type.as_str() {
        "color" | "vec2" | "vec3" | "vec4" | "mat3" => {
            let values = match value {
                ParamValue::Vector(values) => values.iter().copied().map(f64::from).collect(),
                ParamValue::Vector64(values) => values.clone(),
                ParamValue::String(value) if spec.parameter_type != "mat3" => {
                    if spec.parameter_type == "color" && value.starts_with('#') {
                        parse_hex(value).map_err(|detail| invalid_parameter(name, detail))?
                    } else {
                        value
                            .split(',')
                            .map(|component| component.trim().parse::<f64>().map_err(|_| bad()))
                            .collect::<Result<Vec<_>, _>>()?
                    }
                }
                _ => return Err(bad()),
            };
            validate_parameter_vector(name, spec, &values)
                .map_err(|message| RenderError::InvalidParameter { message })?;
            if keep_precision {
                return Ok(Value::Array(values.into_iter().map(Value::Float).collect()));
            }
            let values = values.into_iter().map(|x| x as f32).collect();
            Ok(if spec.parameter_type == "mat3" {
                Value::Mat {
                    dimension: 3,
                    columns: values,
                }
            } else {
                Value::Vec(values)
            })
        }
        "float" => {
            let number = match value {
                ParamValue::Float(value) => {
                    validate_parameter_float32(name, spec, *value)
                        .map_err(|message| RenderError::InvalidParameter { message })?;
                    return Ok(Value::Float(f64::from(*value)));
                }
                ParamValue::Float64(value) => *value,
                ParamValue::Int(value) => f64::from(*value),
                ParamValue::String(value) => value.parse::<f64>().map_err(|_| bad())?,
                _ => return Err(bad()),
            };
            validate_parameter_number(name, spec, number, false)
                .map_err(|message| RenderError::InvalidParameter { message })?;
            if !(number as f32).is_finite() {
                return Err(invalid_parameter(
                    name,
                    "must be representable as a finite f32 number",
                ));
            }
            Ok(Value::Float(if keep_precision {
                number
            } else {
                f64::from(number as f32)
            }))
        }
        "int" | "enum" | "member" | "palette" => {
            let number = match value {
                ParamValue::Int(value) => f64::from(*value),
                ParamValue::Float(value) => f64::from(*value),
                ParamValue::Float64(value) => *value,
                ParamValue::String(value) => {
                    let choices = spec.metadata.get("choices").and_then(JsonValue::as_object);
                    let short = value.rsplit('.').next().unwrap_or(value);
                    if let Some(selected) = choices
                        .and_then(|choices| choices.get(value).or_else(|| choices.get(short)))
                        .and_then(JsonValue::as_f64)
                    {
                        selected
                    } else if choices.is_none() && spec.parameter_type == "member" {
                        0.0
                    } else if choices.is_some() {
                        return Err(bad());
                    } else {
                        value.parse::<f64>().map_err(|_| bad())?
                    }
                }
                _ => return Err(bad()),
            };
            validate_parameter_number(name, spec, number, true)
                .map_err(|message| RenderError::InvalidParameter { message })?;
            Ok(Value::Int(number as i32))
        }
        "bool" | "boolean" => {
            let boolean = match value {
                ParamValue::Bool(value) => *value,
                ParamValue::Int(0) => false,
                ParamValue::Int(1) => true,
                ParamValue::String(value) => match value.trim().to_ascii_lowercase().as_str() {
                    "1" | "true" | "yes" | "on" => true,
                    "0" | "false" | "no" | "off" => false,
                    _ => return Err(bad()),
                },
                _ => return Err(bad()),
            };
            Ok(Value::Bool(boolean))
        }
        "string" => match value {
            ParamValue::String(_) => Ok(Value::Int(0)),
            _ => Err(bad()),
        },
        other => Err(invalid_parameter(
            name,
            format!("has unsupported type {other:?}"),
        )),
    }
}

fn from_json(
    name: &str,
    value: &JsonValue,
    keep_precision: bool,
) -> Result<ParamValue, RenderError> {
    match value {
        JsonValue::Bool(value) => Ok(ParamValue::Bool(*value)),
        JsonValue::Number(value) if value.is_i64() => {
            let value = value.as_i64().unwrap_or_default();
            i32::try_from(value)
                .map(ParamValue::Int)
                .map_err(|_| invalid_parameter(name, "integer default is outside i32"))
        }
        JsonValue::Number(value) => Ok(ParamValue::number(
            value.as_f64().unwrap_or_default(),
            keep_precision,
        )),
        JsonValue::String(value) => Ok(ParamValue::String(value.clone())),
        JsonValue::Array(values) => values
            .iter()
            .map(|value| {
                value
                    .as_f64()
                    .ok_or_else(|| invalid_parameter(name, "vector default contains a non-number"))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(|values| ParamValue::vector(values, keep_precision)),
        _ => Err(invalid_parameter(name, "has no usable default")),
    }
}

fn parse_hex(value: &str) -> Result<Vec<f64>, String> {
    let mut digits = value.trim_start_matches('#').to_string();
    if matches!(digits.len(), 3 | 4) {
        digits = digits.chars().flat_map(|c| [c, c]).collect();
    }
    if digits.len() != 6 && digits.len() != 8 {
        return Err(format!("has invalid color {value:?}"));
    }
    (0..digits.len() / 2)
        .map(|index| {
            u8::from_str_radix(&digits[index * 2..index * 2 + 2], 16)
                .map(|byte| f64::from(byte) / 255.0)
                .map_err(|_| format!("has invalid color {value:?}"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::effect_catalog;
    use crate::pass_runner::{pass_enabled, repeat_count};
    use serde_json::json;

    #[test]
    fn graph_binds_only_the_define_or_uniform_or_parameter_name_for_pass_conditions_and_repeats() {
        let catalog = effect_catalog().unwrap();
        let perlin = &catalog.effects["synth/perlin"];
        let standalone = normalize_parameters(perlin, &BTreeMap::new(), 1).unwrap();
        let graph = normalize_graph_parameters(perlin, &BTreeMap::new(), 1).unwrap();
        assert_eq!(standalone.uniforms["dimensions"], Value::Int(2));
        assert_eq!(graph.values["dimensions"], Value::Int(2));
        assert_eq!(graph.uniforms["DIMENSIONS"], Value::Int(2));
        assert!(!graph.uniforms.contains_key("dimensions"));
        assert!(!pass_enabled(
            &json!({"runIf":[{"uniform":"dimensions","equals":2}]}),
            &graph.uniforms
        ));
        assert!(pass_enabled(
            &json!({"runIf":[{"uniform":"DIMENSIONS","equals":2}]}),
            &graph.uniforms
        ));
        let feedback = &catalog.effects["filter/feedback"];
        let standalone = normalize_parameters(feedback, &BTreeMap::new(), 1).unwrap();
        let graph = normalize_graph_parameters(feedback, &BTreeMap::new(), 1).unwrap();
        assert!(!standalone.uniforms.contains_key("iterationCount"));
        assert_eq!(repeat_count(&json!("iterationCount"), &graph.uniforms), 60);
    }

    #[test]
    fn graph_binding_keeps_native_remap_precision_and_applies_palette_after_parameters() {
        let catalog = effect_catalog().unwrap();
        let remap = &catalog.effects["synth/remap"];
        let supplied = BTreeMap::from([(
            "zone7_bounds".into(),
            ParamValue::Vector64(vec![0.499999999, 0.2, 0.75, 1.0]),
        )]);
        let graph = normalize_graph_parameters(remap, &supplied, 1).unwrap();
        assert_eq!(
            graph.values["zone7_bounds"],
            Value::Vec(vec![0.5, 0.2, 0.75, 1.0])
        );
        assert_eq!(
            graph.uniforms["zone7_bounds"],
            Value::Array(
                [0.499999999, 0.2, 0.75, 1.0]
                    .into_iter()
                    .map(Value::Float)
                    .collect()
            )
        );

        // A classic palette supplies these five uniforms even if the effect
        // does not also declare editable paletteAmp/Freq/Offset/Phase/Mode.
        let with_palette: EffectDefinition = serde_json::from_value(json!({
            "namespace":"classicNoisedeck","func":"probe","kind":"generator","domain":"image","passes":[],
            "paramNames":["later","earlier","palette"],
            "params":{
                "palette":{"type":"palette","default":1},
                "later":{"type":"int","default":1,"uniform":"shared"},
                "earlier":{"type":"int","default":2,"uniform":"shared"}
            }
        })).unwrap();
        let standalone = normalize_parameters(&with_palette, &BTreeMap::new(), 1).unwrap();
        let graph = normalize_graph_parameters(&with_palette, &BTreeMap::new(), 1).unwrap();
        assert_eq!(standalone.uniforms["shared"], Value::Int(1));
        assert_eq!(graph.uniforms["shared"], Value::Int(2));
        assert!(!standalone.uniforms.contains_key("paletteAmp"));
        let mut palette = BTreeMap::new();
        apply_classic_palette_uniforms(1, &mut palette).unwrap();
        assert_eq!(palette.len(), 5);
        for (name, value) in palette {
            assert_eq!(graph.uniforms[&name], value);
        }
    }
}
