use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value as JsonValue;

use super::{DslArgument, DslError, DslProgram, DslValue, Location, SurfaceRef, parse_dsl};
use crate::ParamValue;
use crate::catalog::{EffectDefinition, ParameterSpec, effect_catalog};
use crate::param::{validate_parameter_number, validate_parameter_vector};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SurfaceBinding {
    Surface(String),
    Current,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RenderStep {
    Read {
        surface: String,
        loc: Location,
    },
    Effect {
        effect_id: String,
        params: BTreeMap<String, ParamValue>,
        surface_params: BTreeMap<String, SurfaceBinding>,
        explicit_params: Vec<String>,
        loc: Location,
    },
    Write {
        surface: String,
        loc: Location,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompiledChain {
    pub steps: Vec<RenderStep>,
    pub loc: Location,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderPlan {
    pub search: Vec<String>,
    pub chains: Vec<CompiledChain>,
    pub render_surface: String,
    pub ast: DslProgram,
}

#[derive(Clone, Debug, PartialEq)]
enum Evaluated {
    Number(f64),
    String(String),
    Bool(bool),
    Array(Vec<Evaluated>),
    Surface(SurfaceRef),
}

#[derive(Clone, Debug)]
struct ResolvedArgument {
    name: Option<String>,
    value: Evaluated,
    loc: Location,
}

#[derive(Clone, Debug)]
struct Partial {
    name: String,
    args: Vec<ResolvedArgument>,
    arg_mode: Option<String>,
    loc: Location,
}

#[derive(Clone, Debug)]
enum Binding {
    Value(Evaluated),
    Partial(Partial),
}

pub fn compile_dsl(source: &str, source_name: &str) -> Result<RenderPlan, DslError> {
    let ast = parse_dsl(source, source_name)?;
    let catalog = effect_catalog().map_err(|error| DslError::new(error.to_string(), &ast.loc))?;
    if ast.search.is_empty() {
        return Err(DslError::new("Missing required search directive", &ast.loc));
    }

    let mut bindings = BTreeMap::new();
    for binding in &ast.bindings {
        if bindings.contains_key(&binding.name) {
            return Err(DslError::new(
                format!("Duplicate binding {:?}", binding.name),
                &binding.loc,
            ));
        }
        let value = match &binding.value {
            DslValue::Call(call) => Binding::Partial(Partial {
                name: call.name.clone(),
                args: resolve_args(&call.args, &bindings)?,
                arg_mode: call.arg_mode.clone(),
                loc: call.loc.clone(),
            }),
            value => Binding::Value(evaluate_value(value, &bindings)?),
        };
        bindings.insert(binding.name.clone(), value);
    }

    let mut chains = Vec::new();
    for chain in &ast.chains {
        let mut steps = Vec::new();
        let mut has_image = false;
        let mut has_volume = false;
        let mut starts_with_generator = false;
        let mut open_loop: Option<Location> = None;
        for (index, source_call) in chain.calls.iter().enumerate() {
            let (call_name, args, call_loc) = if let Some(binding) = bindings.get(&source_call.name)
            {
                let Binding::Partial(partial) = binding else {
                    return Err(DslError::new(
                        format!("Binding {:?} is not callable", source_call.name),
                        &source_call.loc,
                    ));
                };
                let invocation = Partial {
                    name: source_call.name.clone(),
                    args: resolve_args(&source_call.args, &bindings)?,
                    arg_mode: source_call.arg_mode.clone(),
                    loc: source_call.loc.clone(),
                };
                let merged = merge_partial(partial, &invocation)?;
                (merged.name, merged.args, source_call.loc.clone())
            } else {
                (
                    source_call.name.clone(),
                    resolve_args(&source_call.args, &bindings)?,
                    source_call.loc.clone(),
                )
            };

            if call_name == "read" {
                let surface =
                    one_surface_argument(&args, &call_loc, "read(surface) must begin a chain")?;
                if index != 0 {
                    return Err(DslError::new("read(surface) must begin a chain", &call_loc));
                }
                steps.push(RenderStep::Read {
                    surface,
                    loc: call_loc,
                });
                has_image = true;
                continue;
            }
            if call_name == "write" {
                if open_loop.is_some() {
                    return Err(DslError::new(
                        "loopBegin must be closed by loopEnd before write",
                        &call_loc,
                    ));
                }
                let surface = one_surface_argument(
                    &args,
                    &call_loc,
                    "write(surface) requires a current image",
                )?;
                if !has_image {
                    return Err(DslError::new(
                        "write(surface) requires a current image",
                        &call_loc,
                    ));
                }
                steps.push(RenderStep::Write {
                    surface,
                    loc: call_loc,
                });
                continue;
            }

            let effect_id = ast
                .search
                .iter()
                .map(|namespace| format!("{namespace}/{call_name}"))
                .find(|effect_id| catalog.effects.contains_key(effect_id))
                .ok_or_else(|| {
                    DslError::new(
                        format!(
                            "Unknown effect {call_name:?} in search namespaces {}",
                            ast.search.join(", ")
                        ),
                        &call_loc,
                    )
                })?;
            let definition = &catalog.effects[&effect_id];
            let mut requires_bound_surface_to_start = false;
            match definition.domain.as_str() {
                "volume-generator" => {
                    if index != 0 && !(definition.iterated && has_volume) {
                        return Err(DslError::new(
                            format!("Generator {effect_id} must begin a chain"),
                            &call_loc,
                        ));
                    }
                    if index == 0 {
                        starts_with_generator = true;
                    }
                    has_volume = true;
                }
                "volume-filter" => {
                    if !has_volume {
                        return Err(DslError::new(
                            format!("volume filter {effect_id} requires a volume input"),
                            &call_loc,
                        ));
                    }
                }
                "volume-renderer" => {
                    if !has_volume {
                        return Err(DslError::new(
                            format!("volume renderer {effect_id} requires a volume input"),
                            &call_loc,
                        ));
                    }
                    has_image = true;
                }
                "loop-begin" => {
                    if !has_image {
                        return Err(DslError::new(
                            format!("{effect_id} requires a current image"),
                            &call_loc,
                        ));
                    }
                    if open_loop.is_some() {
                        return Err(DslError::new(
                            "nested loopBegin regions are not supported",
                            &call_loc,
                        ));
                    }
                    open_loop = Some(call_loc.clone());
                }
                "loop-end" => {
                    if open_loop.is_none() {
                        return Err(DslError::new(
                            "loopEnd has no matching loopBegin",
                            &call_loc,
                        ));
                    }
                    if !has_image {
                        return Err(DslError::new(
                            format!("{effect_id} requires a current image"),
                            &call_loc,
                        ));
                    }
                    open_loop = None;
                }
                _ if definition.kind == "generator" => {
                    if index != 0 {
                        return Err(DslError::new(
                            format!("Generator {effect_id} must begin a chain"),
                            &call_loc,
                        ));
                    }
                    starts_with_generator = true;
                    has_image = true;
                }
                _ if !has_image => {
                    let requires_input = definition
                        .passes
                        .iter()
                        .any(|pass| pass.inputs.values().any(|input| input == "inputTex"));
                    if requires_input {
                        return Err(DslError::new(
                            format!(
                                "{} {effect_id} requires an input; begin with a generator or read(oN)",
                                definition.kind
                            ),
                            &call_loc,
                        ));
                    }
                    requires_bound_surface_to_start = true;
                    has_image = true;
                }
                _ => {}
            }
            let normalized = normalize_arguments(&effect_id, definition, &args, &call_loc)?;
            if requires_bound_surface_to_start && normalized.surface_params.is_empty() {
                return Err(DslError::new(
                    format!(
                        "{} {effect_id} requires at least one surface input to begin a chain",
                        definition.kind
                    ),
                    &call_loc,
                ));
            }
            steps.push(RenderStep::Effect {
                effect_id,
                params: normalized.params,
                surface_params: normalized.surface_params,
                explicit_params: normalized.explicit_params,
                loc: call_loc,
            });
        }
        if let Some(location) = open_loop {
            return Err(DslError::new(
                "loopBegin must be closed by loopEnd before the chain ends",
                &location,
            ));
        }
        if starts_with_generator && !matches!(steps.last(), Some(RenderStep::Write { .. })) {
            return Err(DslError::new(
                "Generator chain must end with write(oN)",
                &chain.loc,
            ));
        }
        chains.push(CompiledChain {
            steps,
            loc: chain.loc.clone(),
        });
    }
    let last_written = chains
        .iter()
        .flat_map(|chain| &chain.steps)
        .fold(None, |last, step| match step {
            RenderStep::Write { surface, .. } => Some(surface.clone()),
            _ => last,
        });
    let render_surface = ast
        .render
        .as_ref()
        .map(|surface| surface.name.clone())
        .or(last_written)
        .ok_or_else(|| {
            DslError::new(
                "No render surface specified and no write() found - add render(oN) or write(oN)",
                &ast.loc,
            )
        })?;
    Ok(RenderPlan {
        search: ast.search.clone(),
        chains,
        render_surface,
        ast,
    })
}

fn resolve_args(
    arguments: &[DslArgument],
    bindings: &BTreeMap<String, Binding>,
) -> Result<Vec<ResolvedArgument>, DslError> {
    arguments
        .iter()
        .map(|argument| {
            Ok(ResolvedArgument {
                name: argument.name.clone(),
                value: evaluate_value(&argument.value, bindings)?,
                loc: argument.loc.clone(),
            })
        })
        .collect()
}

fn evaluate_value(
    value: &DslValue,
    bindings: &BTreeMap<String, Binding>,
) -> Result<Evaluated, DslError> {
    match value {
        DslValue::Number(value) => Ok(Evaluated::Number(*value)),
        DslValue::String(value) => Ok(Evaluated::String(value.clone())),
        DslValue::Bool(value) => Ok(Evaluated::Bool(*value)),
        DslValue::Surface(surface) => Ok(Evaluated::Surface(surface.clone())),
        DslValue::Array { values, .. } => values
            .iter()
            .map(|value| evaluate_value(value, bindings))
            .collect::<Result<Vec<_>, _>>()
            .map(Evaluated::Array),
        DslValue::Identifier { name, loc } => match bindings.get(name) {
            Some(Binding::Value(value)) => Ok(value.clone()),
            Some(Binding::Partial(_)) => Err(DslError::new(
                format!("Effect partial {name:?} cannot be used as a value"),
                loc,
            )),
            None => Ok(Evaluated::String(name.clone())),
        },
        DslValue::Vector { width, values, loc } => {
            let values = values
                .iter()
                .map(|value| evaluate_value(value, bindings))
                .collect::<Result<Vec<_>, _>>()?;
            if values.len() != *width
                || values
                    .iter()
                    .any(|value| !matches!(value, Evaluated::Number(_)))
            {
                return Err(DslError::new(
                    format!("vec{width} requires {width} numeric values"),
                    loc,
                ));
            }
            Ok(Evaluated::Array(values))
        }
        DslValue::Unary {
            operator,
            argument,
            loc,
        } => {
            let Evaluated::Number(value) = evaluate_value(argument, bindings)? else {
                return Err(DslError::new("Unary arithmetic requires a number", loc));
            };
            Ok(Evaluated::Number(if operator == "-" {
                -value
            } else {
                value
            }))
        }
        DslValue::Binary {
            operator,
            left,
            right,
            loc,
        } => {
            let (Evaluated::Number(left), Evaluated::Number(right)) = (
                evaluate_value(left, bindings)?,
                evaluate_value(right, bindings)?,
            ) else {
                return Err(DslError::new("Arithmetic requires numeric values", loc));
            };
            Ok(Evaluated::Number(match operator.as_str() {
                "+" => left + right,
                "-" => left - right,
                "*" => left * right,
                _ => left / right,
            }))
        }
        DslValue::Call(call) => Err(DslError::new(
            "Effect call cannot be used as a value",
            &call.loc,
        )),
    }
}

fn merge_partial(stored: &Partial, call: &Partial) -> Result<Partial, DslError> {
    if stored.arg_mode.is_none() {
        return Ok(Partial {
            name: stored.name.clone(),
            args: call.args.clone(),
            arg_mode: call.arg_mode.clone(),
            loc: call.loc.clone(),
        });
    }
    if call.arg_mode.is_none() {
        let mut retained = stored.clone();
        retained.loc = call.loc.clone();
        return Ok(retained);
    }
    if stored.arg_mode != call.arg_mode {
        return Err(DslError::new(
            "Partial and call arguments must use the same named or positional form",
            &call.loc,
        ));
    }
    if stored.arg_mode.as_deref() == Some("positional") {
        let mut args = stored.args.clone();
        args.extend(call.args.clone());
        return Ok(Partial {
            name: stored.name.clone(),
            args,
            arg_mode: stored.arg_mode.clone(),
            loc: call.loc.clone(),
        });
    }
    let mut args = stored.args.clone();
    for replacement in &call.args {
        if let Some(existing) = args
            .iter_mut()
            .find(|argument| argument.name == replacement.name)
        {
            *existing = replacement.clone();
        } else {
            args.push(replacement.clone());
        }
    }
    Ok(Partial {
        name: stored.name.clone(),
        args,
        arg_mode: Some("named".into()),
        loc: call.loc.clone(),
    })
}

fn one_surface_argument(
    args: &[ResolvedArgument],
    loc: &Location,
    message: &str,
) -> Result<String, DslError> {
    if args.len() != 1 {
        return Err(DslError::new(message, loc));
    }
    let Evaluated::Surface(surface) = &args[0].value else {
        return Err(DslError::new(message, loc));
    };
    Ok(surface.name.clone())
}

struct NormalizedArguments {
    params: BTreeMap<String, ParamValue>,
    surface_params: BTreeMap<String, SurfaceBinding>,
    explicit_params: Vec<String>,
}

fn normalize_arguments(
    effect_id: &str,
    definition: &EffectDefinition,
    args: &[ResolvedArgument],
    loc: &Location,
) -> Result<NormalizedArguments, DslError> {
    let mut params = BTreeMap::new();
    let mut surface_params = BTreeMap::new();
    for name in &definition.param_names {
        let spec = &definition.params[name];
        if let Some(value) = json_default(&spec.default) {
            apply_parameter(name, spec, value, &mut params, &mut surface_params, loc)?;
        }
    }
    let mut explicit_params = Vec::new();
    let mut explicit_seen = BTreeSet::new();
    for (index, argument) in args.iter().enumerate() {
        let supplied_name = argument
            .name
            .clone()
            .or_else(|| definition.param_names.get(index).cloned());
        let canonical_name = supplied_name.as_ref().and_then(|name| {
            definition
                .param_aliases
                .get(name)
                .cloned()
                .or_else(|| Some(name.clone()))
        });
        let Some(name) = canonical_name.filter(|name| definition.params.contains_key(name)) else {
            let bad = supplied_name.unwrap_or_else(|| format!("argument {}", index + 1));
            let accepted = definition
                .param_names
                .iter()
                .chain(definition.param_aliases.keys())
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            return Err(DslError::new(
                format!("Unknown parameter {bad:?} for {effect_id}; accepted: {accepted}"),
                loc,
            ));
        };
        apply_parameter(
            &name,
            &definition.params[&name],
            argument.value.clone(),
            &mut params,
            &mut surface_params,
            &argument.loc,
        )?;
        if explicit_seen.insert(name.clone()) {
            explicit_params.push(name);
        }
    }
    for name in &definition.param_names {
        let parameter_type = definition.params[name].parameter_type.as_str();
        let present = if matches!(parameter_type, "surface") {
            surface_params.contains_key(name)
                || definition.params[name].default == JsonValue::String("none".into())
                || definition.params[name].default.is_null()
        } else {
            params.contains_key(name)
        };
        if !present {
            return Err(DslError::new(
                format!("Missing required parameter {name:?} for {effect_id}"),
                loc,
            ));
        }
    }
    Ok(NormalizedArguments {
        params,
        surface_params,
        explicit_params,
    })
}

fn json_default(value: &JsonValue) -> Option<Evaluated> {
    match value {
        JsonValue::Null => None,
        JsonValue::Bool(value) => Some(Evaluated::Bool(*value)),
        JsonValue::Number(value) => value.as_f64().map(Evaluated::Number),
        JsonValue::String(value) => Some(Evaluated::String(value.clone())),
        JsonValue::Array(values) => Some(Evaluated::Array(
            values.iter().filter_map(json_default).collect(),
        )),
        JsonValue::Object(_) => None,
    }
}

fn apply_parameter(
    name: &str,
    spec: &ParameterSpec,
    value: Evaluated,
    params: &mut BTreeMap<String, ParamValue>,
    surfaces: &mut BTreeMap<String, SurfaceBinding>,
    loc: &Location,
) -> Result<(), DslError> {
    let fail = |message: String| DslError::new(format!("Parameter {name:?} {message}"), loc);
    match spec.parameter_type.as_str() {
        "surface" => match value {
            Evaluated::String(value) if value == "none" => {
                surfaces.remove(name);
            }
            Evaluated::String(value) if value == "inputTex" => {
                surfaces.insert(name.into(), SurfaceBinding::Current);
            }
            Evaluated::Surface(surface) => {
                surfaces.insert(name.into(), SurfaceBinding::Surface(surface.name));
            }
            _ => return Err(fail("must be a surface reference".into())),
        },
        "float" => {
            let Evaluated::Number(value) = value else {
                return Err(fail("must be a finite number".into()));
            };
            validate_parameter_number(name, spec, value, false)
                .map_err(|message| DslError::new(message, loc))?;
            let value = value as f32;
            if !value.is_finite() {
                return Err(fail("must be representable as a finite f32 number".into()));
            }
            params.insert(name.into(), ParamValue::Float(value));
        }
        "int" | "enum" | "member" | "palette" => {
            let integer = enum_or_integer(name, spec, value, loc)?;
            validate_parameter_number(name, spec, f64::from(integer), true)
                .map_err(|message| DslError::new(message, loc))?;
            params.insert(name.into(), ParamValue::Int(integer));
        }
        "bool" | "boolean" => {
            let Evaluated::Bool(value) = value else {
                return Err(fail("must be boolean".into()));
            };
            params.insert(name.into(), ParamValue::Bool(value));
        }
        "color" | "vec2" | "vec3" | "vec4" | "mat3" => {
            let value = match value {
                Evaluated::String(value)
                    if spec.parameter_type == "color" && value.starts_with('#') =>
                {
                    Evaluated::Array(
                        decode_hex_color(&value)
                            .map_err(&fail)?
                            .into_iter()
                            .map(Evaluated::Number)
                            .collect(),
                    )
                }
                value => value,
            };
            let Evaluated::Array(values) = value else {
                return Err(fail(format!("must be a {}", spec.parameter_type)));
            };
            let components = values
                .into_iter()
                .map(|value| match value {
                    Evaluated::Number(value) if value.is_finite() => {
                        let value = value as f32;
                        if value.is_finite() {
                            Ok(value)
                        } else {
                            Err(fail(format!(
                                "must be representable as a finite {}",
                                spec.parameter_type
                            )))
                        }
                    }
                    _ => Err(fail(format!("must be a {}", spec.parameter_type))),
                })
                .collect::<Result<Vec<_>, _>>()?;
            validate_parameter_vector(name, spec, &components)
                .map_err(|message| DslError::new(message, loc))?;
            params.insert(name.into(), ParamValue::Vector(components));
        }
        "string" => {
            let Evaluated::String(value) = value else {
                return Err(fail("must be a string".into()));
            };
            params.insert(name.into(), ParamValue::String(value));
        }
        "volume" | "geometry" => {
            let Evaluated::String(value) = value else {
                return Err(fail(format!("must be a {} reference", spec.parameter_type)));
            };
            if value.is_empty() {
                return Err(fail(format!("must be a {} reference", spec.parameter_type)));
            }
            params.insert(name.into(), ParamValue::String(value));
        }
        other => return Err(fail(format!("has unsupported type {other:?}"))),
    }
    Ok(())
}

fn decode_hex_color(value: &str) -> Result<Vec<f64>, String> {
    let mut digits = value.trim_start_matches('#').to_owned();
    if matches!(digits.len(), 3 | 4) {
        digits = digits.chars().flat_map(|digit| [digit, digit]).collect();
    }
    if !matches!(digits.len(), 6 | 8) {
        return Err(format!("has invalid color default {value:?}"));
    }
    (0..digits.len() / 2)
        .map(|index| {
            u8::from_str_radix(&digits[index * 2..index * 2 + 2], 16)
                .map(|byte| f64::from(byte) / 255.0)
                .map_err(|_| format!("has invalid color default {value:?}"))
        })
        .collect()
}

fn enum_or_integer(
    name: &str,
    spec: &ParameterSpec,
    value: Evaluated,
    loc: &Location,
) -> Result<i32, DslError> {
    if let Evaluated::String(value) = value {
        let key = value.rsplit('.').next().unwrap_or(&value);
        let choices = spec.metadata.get("choices").and_then(JsonValue::as_object);
        if choices.is_none() && spec.parameter_type == "member" {
            return Ok(0);
        }
        let selected = choices
            .and_then(|choices| choices.get(key))
            .and_then(JsonValue::as_i64)
            .ok_or_else(|| {
                DslError::new(
                    format!("Parameter {name:?} has invalid enum value {value:?}"),
                    loc,
                )
            })?;
        return Ok(selected as i32);
    }
    let Evaluated::Number(value) = value else {
        return Err(DslError::new(
            format!("Parameter {name:?} must be an integer"),
            loc,
        ));
    };
    if !value.is_finite()
        || value.fract() != 0.0
        || value < i32::MIN as f64
        || value > i32::MAX as f64
    {
        return Err(DslError::new(
            format!("Parameter {name:?} must be an integer"),
            loc,
        ));
    }
    Ok(value as i32)
}
