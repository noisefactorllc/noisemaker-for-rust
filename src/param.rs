use std::collections::BTreeMap;

use serde_json::Value as JsonValue;

use crate::adapters::apply_classic_palette_uniforms;
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

pub fn normalize_parameters(
    effect: &EffectDefinition,
    supplied: &BTreeMap<String, ParamValue>,
    render_seed: i32,
) -> Result<NormalizedParameters, RenderError> {
    if let Some(name) = supplied
        .keys()
        .find(|name| !effect.params.contains_key(name.as_str()))
    {
        return Err(RenderError::UnknownParameter { name: name.clone() });
    }
    let mut normalized = NormalizedParameters::default();
    for (name, spec) in &effect.params {
        if matches!(
            spec.parameter_type.as_str(),
            "surface" | "volume" | "geometry"
        ) {
            continue;
        }
        let value = if name == "seed" && !supplied.contains_key(name) {
            coerce(name, spec, Some(&ParamValue::Int(render_seed)))?
        } else {
            coerce(name, spec, supplied.get(name))?
        };
        normalized.values.insert(name.clone(), value.clone());
        if let Some(uniform) = &spec.uniform {
            normalized.uniforms.insert(uniform.clone(), value.clone());
        }
        if let Some(define) = &spec.define {
            normalized.uniforms.insert(define.clone(), value);
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
        && CLASSIC_PALETTE_UNIFORMS
            .iter()
            .all(|name| effect.params.contains_key(*name))
    {
        if let Some(Value::Int(index)) = effect
            .params
            .iter()
            .find(|(_, spec)| spec.parameter_type == "palette")
            .and_then(|(name, _)| normalized.values.get(name))
        {
            apply_classic_palette_uniforms(*index, &mut normalized.uniforms)?;
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
    values: &[f32],
) -> Result<(), String> {
    let width_is_valid = match spec.parameter_type.as_str() {
        "color" => matches!(values.len(), 3 | 4),
        "vec2" => values.len() == 2,
        "vec3" => values.len() == 3,
        "vec4" => values.len() == 4,
        "mat3" => values.len() == 9,
        _ => false,
    };
    if !width_is_valid || values.iter().any(|value| !value.is_finite()) {
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
                    let number = number as f32;
                    if !number.is_finite() {
                        return Err(invalid_parameter(
                            name,
                            "must be representable as a finite f32 number",
                        ));
                    }
                    Ok(Value::Float(number))
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
        owned_default = from_json(name, &spec.default)?;
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
                ParamValue::Vector(values) => values.clone(),
                ParamValue::String(value) if spec.parameter_type != "mat3" => {
                    if spec.parameter_type == "color" && value.starts_with('#') {
                        parse_hex(value).map_err(|detail| invalid_parameter(name, detail))?
                    } else {
                        value
                            .split(',')
                            .map(|component| {
                                component
                                    .trim()
                                    .parse::<f64>()
                                    .map(|component| component as f32)
                                    .map_err(|_| bad())
                            })
                            .collect::<Result<Vec<_>, _>>()?
                    }
                }
                _ => return Err(bad()),
            };
            validate_parameter_vector(name, spec, &values)
                .map_err(|message| RenderError::InvalidParameter { message })?;
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
                    return Ok(Value::Float(*value));
                }
                ParamValue::Int(value) => f64::from(*value),
                ParamValue::String(value) => value.parse::<f64>().map_err(|_| bad())?,
                _ => return Err(bad()),
            };
            validate_parameter_number(name, spec, number, false)
                .map_err(|message| RenderError::InvalidParameter { message })?;
            let number = number as f32;
            if !number.is_finite() {
                return Err(invalid_parameter(
                    name,
                    "must be representable as a finite f32 number",
                ));
            }
            Ok(Value::Float(number))
        }
        "int" | "enum" | "member" | "palette" => {
            let number = match value {
                ParamValue::Int(value) => f64::from(*value),
                ParamValue::Float(value) => f64::from(*value),
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

fn from_json(name: &str, value: &JsonValue) -> Result<ParamValue, RenderError> {
    match value {
        JsonValue::Bool(value) => Ok(ParamValue::Bool(*value)),
        JsonValue::Number(value) if value.is_i64() => {
            let value = value.as_i64().unwrap_or_default();
            i32::try_from(value)
                .map(ParamValue::Int)
                .map_err(|_| invalid_parameter(name, "integer default is outside i32"))
        }
        JsonValue::Number(value) => {
            Ok(ParamValue::Float(value.as_f64().unwrap_or_default() as f32))
        }
        JsonValue::String(value) => Ok(ParamValue::String(value.clone())),
        JsonValue::Array(values) => values
            .iter()
            .map(|value| {
                value
                    .as_f64()
                    .map(|value| value as f32)
                    .ok_or_else(|| invalid_parameter(name, "vector default contains a non-number"))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(ParamValue::Vector),
        _ => Err(invalid_parameter(name, "has no usable default")),
    }
}

fn parse_hex(value: &str) -> Result<Vec<f32>, String> {
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
                .map(|byte| f32::from(byte) / 255.0)
                .map_err(|_| format!("has invalid color {value:?}"))
        })
        .collect()
}
