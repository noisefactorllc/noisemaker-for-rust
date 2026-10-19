use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value as JsonValue;

use super::{
    DslArgument, DslCall, DslChain, DslChainElement, DslError, DslProgram, DslValue, Location,
    SurfaceRef, parse_dsl,
};
use crate::ParamValue;
use crate::automation::{AutomationField, AutomationValue};
use crate::catalog::{EffectCatalog, EffectDefinition, ParameterSpec, effect_catalog};
use crate::param::{validate_parameter_number, validate_parameter_vector};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SurfaceBinding {
    Surface(String),
    Current,
}

// A chain argument (`tex: solid(color: #f00)`) compiles to its own chain that
// writes a hidden surface. The consuming step carries it in `inline_chains`,
// and the renderer runs it before the step, as upstream's validator compiles
// the chain into temporary steps ahead of the consuming effect.
#[derive(Clone, Debug, PartialEq)]
pub struct InlineChain {
    pub surface: String,
    pub chain: CompiledChain,
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
        inline_chains: Vec<InlineChain>,
        loc: Location,
    },
    Write {
        surface: String,
        loc: Location,
    },
    // Built-in volume I/O (upstream `_write3d` and `_read3d`). write3d stores
    // the current volume and geometry under the two names and passes the chain
    // through; read3d starts a chain from them.
    Read3d {
        volume: String,
        geometry: String,
        loc: Location,
    },
    Write3d {
        volume: String,
        geometry: String,
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

#[derive(Clone, Debug)]
enum Evaluated {
    Number(f64),
    String(String),
    Bool(bool),
    Array(Vec<Evaluated>),
    Surface(SurfaceRef),
    Automation(AutomationValue),
}

#[derive(Clone, Debug)]
struct ResolvedArgument {
    name: Option<String>,
    value: Evaluated,
    loc: Location,
}

// A `let` partial keeps its call unresolved. Its arguments resolve where it is
// called, as upstream's validator substitutes a symbol at each use.
#[derive(Clone, Debug)]
struct Partial {
    call: DslCall,
}

#[derive(Clone, Debug)]
enum Binding {
    // A value binding keeps its evaluated value and its source expression: an
    // automation field substitutes the source expression, as upstream's
    // validator substitutes symbols before it compiles an automation
    // descriptor.
    Value { value: Evaluated, ast: DslValue },
    // Upstream `let x = noise` makes `x` an alias of the effect function, and
    // `let y = x` copies a partial or an alias.
    Alias(String),
    Partial(Partial),
}

// The `let` bindings of a program, plus the compiler of chain arguments.
struct CompileScope {
    catalog: &'static EffectCatalog,
    search: Vec<String>,
    bindings: BTreeMap<String, Binding>,
    // Chain arguments of the call whose arguments are resolving. `None` outside
    // chain compilation, where a chain is not a usable value.
    pending: Option<Vec<InlineChain>>,
    inline_count: usize,
}

impl CompileScope {
    fn compile_inline_chain(&mut self, chain: &DslChain) -> Result<Evaluated, DslError> {
        let compiled = compile_chain(chain, self, true)?;
        let surface = format!("inline:{}", self.inline_count);
        self.inline_count += 1;
        let name = surface.clone();
        let loc = chain.loc.clone();
        self.pending
            .as_mut()
            .expect("chain arguments compile only while arguments resolve")
            .push(InlineChain {
                surface: name,
                chain: compiled,
            });
        Ok(Evaluated::Surface(SurfaceRef { name: surface, loc }))
    }
}

pub fn compile_dsl(source: &str, source_name: &str) -> Result<RenderPlan, DslError> {
    let ast = parse_dsl(source, source_name)?;
    let catalog = effect_catalog().map_err(|error| DslError::new(error.to_string(), &ast.loc))?;
    if ast.search.is_empty() {
        return Err(DslError::new("Missing required search directive", &ast.loc));
    }

    let mut scope = CompileScope {
        catalog,
        search: ast.search.clone(),
        bindings: BTreeMap::new(),
        pending: None,
        inline_count: 0,
    };
    for binding in &ast.bindings {
        if scope.bindings.contains_key(&binding.name) {
            return Err(DslError::new(
                format!("Duplicate binding {:?}", binding.name),
                &binding.loc,
            ));
        }
        let value = match &binding.value {
            DslValue::Call(call) if !matches!(call.name.as_str(), "osc" | "midi" | "audio") => {
                Binding::Partial(Partial {
                    call: (**call).clone(),
                })
            }
            value => match alias_binding(value, &scope) {
                Some(binding) => binding,
                None => Binding::Value {
                    value: evaluate_value(value, &mut scope)?,
                    ast: value.clone(),
                },
            },
        };
        scope.bindings.insert(binding.name.clone(), value);
    }

    let mut chains = Vec::new();
    for chain in &ast.chains {
        chains.push(compile_chain(chain, &mut scope, false)?);
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

// Upstream `let x = noise` makes `x` an alias of the effect function, and
// `let y = x` copies a partial or an alias (the validator stores the symbol's
// expression and substitutes it where `y` is used).
fn alias_binding(value: &DslValue, scope: &CompileScope) -> Option<Binding> {
    let DslValue::Identifier { name, .. } = value else {
        return None;
    };
    if name.contains('.') {
        return None;
    }
    match scope.bindings.get(name) {
        Some(Binding::Partial(partial)) => Some(Binding::Partial(partial.clone())),
        Some(Binding::Alias(alias)) => Some(Binding::Alias(alias.clone())),
        Some(Binding::Value { .. }) => None,
        None => scope
            .search
            .iter()
            .any(|namespace| {
                scope
                    .catalog
                    .effects
                    .contains_key(&format!("{namespace}/{name}"))
            })
            .then(|| Binding::Alias(name.clone())),
    }
}

// Compiles one chain. A statement chain that begins with a generator must end
// with write(oN). A chain argument (`inline`) needs no write(): its last image
// is the argument's surface.
fn compile_chain(
    chain: &DslChain,
    scope: &mut CompileScope,
    inline: bool,
) -> Result<CompiledChain, DslError> {
    let catalog = scope.catalog;
    let mut steps = Vec::new();
    let mut has_image = false;
    let mut has_volume = false;
    let mut starts_with_generator = false;
    let mut open_loop: Option<Location> = None;
    // A subchain block contributes its body calls in place. Upstream's
    // validator wraps them in `_subchain_begin`/`_subchain_end` markers, which
    // its expander passes through without a render pass.
    let calls = chain
        .calls
        .iter()
        .flat_map(|element| match element {
            DslChainElement::Call(call) => vec![call.clone()],
            DslChainElement::Subchain(subchain) => subchain.body.clone(),
        })
        .collect::<Vec<_>>();
    for (index, source_call) in calls.iter().enumerate() {
        let mut call = source_call.clone();
        let bound = scope.bindings.get(&call.name).cloned();
        match bound {
            Some(Binding::Alias(alias)) => call.name = alias,
            Some(Binding::Partial(partial)) => call = merge_partial(&partial.call, &call)?,
            Some(_) => {
                return Err(DslError::new(
                    format!("Binding {:?} is not callable", call.name),
                    &call.loc,
                ));
            }
            None => {}
        }
        // Chain arguments compile while the arguments resolve; they belong to
        // this call.
        let outer_pending = scope.pending.take();
        scope.pending = Some(Vec::new());
        let resolved = resolve_args(&call.args, scope);
        let inline_chains = scope.pending.take().unwrap_or_default();
        scope.pending = outer_pending;
        let args = resolved?;
        let call_loc = call.loc.clone();

        if call.name == "read" {
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
        if call.name == "write" {
            if open_loop.is_some() {
                return Err(DslError::new(
                    "loopBegin must be closed by loopEnd before write",
                    &call_loc,
                ));
            }
            let surface =
                one_surface_argument(&args, &call_loc, "write(surface) requires a current image")?;
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
        // Built-in volume I/O (upstream `_write3d` and `_read3d`). write3d
        // stores the current volume and geometry under the two names and passes
        // the chain through; read3d starts a chain from them. A mid-chain
        // read3d() is not a starter, so it fails below as an unknown effect, as
        // a one-argument read3d() at a chain start does.
        if call.name == "write3d" {
            // Upstream's lexer reserves write3d, so it cannot start a chain.
            if index == 0 {
                return Err(DslError::new(
                    "Expected effect or IO function name",
                    &call_loc,
                ));
            }
            if open_loop.is_some() {
                return Err(DslError::new(
                    "loopBegin must be closed by loopEnd before write",
                    &call_loc,
                ));
            }
            let Some((volume, geometry)) = volume_references(&call) else {
                return Err(DslError::new("Expected surface reference", &call_loc));
            };
            steps.push(RenderStep::Write3d {
                volume,
                geometry,
                loc: call_loc,
            });
            continue;
        }
        if index == 0 && call.name == "read3d" {
            if let Some((volume, geometry)) = read3d_references(&call) {
                steps.push(RenderStep::Read3d {
                    volume,
                    geometry,
                    loc: call_loc,
                });
                has_volume = true;
                continue;
            }
        }

        let namespaces: Vec<String> = call
            .namespace
            .clone()
            .map(|namespace| vec![namespace])
            .unwrap_or_else(|| scope.search.clone());
        let effect_id = namespaces
            .iter()
            .map(|namespace| format!("{namespace}/{}", call.name))
            .find(|effect_id| catalog.effects.contains_key(effect_id.as_str()))
            .ok_or_else(|| {
                DslError::new(
                    format!(
                        "Unknown effect {:?} in search namespaces {}",
                        call.name,
                        namespaces.join(", ")
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
        // External-input effects (render/meshLoader) begin a chain with no
        // surface binding at all: their pass inputs are host-fed data
        // textures, so only effects that declare surface parameters must
        // have one bound to start.
        if requires_bound_surface_to_start
            && normalized.surface_params.is_empty()
            && definition
                .params
                .values()
                .any(|spec| spec.parameter_type == "surface")
        {
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
            inline_chains,
            loc: call_loc,
        });
    }
    if let Some(location) = open_loop {
        return Err(DslError::new(
            "loopBegin must be closed by loopEnd before the chain ends",
            &location,
        ));
    }
    if !inline
        && starts_with_generator
        && !matches!(
            steps.last(),
            Some(RenderStep::Write { .. }) | Some(RenderStep::Write3d { .. })
        )
    {
        return Err(DslError::new(
            "Generator chain must end with write(oN)",
            &chain.loc,
        ));
    }
    Ok(CompiledChain {
        steps,
        loc: chain.loc.clone(),
    })
}

fn resolve_args(
    arguments: &[DslArgument],
    scope: &mut CompileScope,
) -> Result<Vec<ResolvedArgument>, DslError> {
    arguments
        .iter()
        .map(|argument| {
            Ok(ResolvedArgument {
                name: argument.name.clone(),
                value: evaluate_value(&argument.value, scope)?,
                loc: argument.loc.clone(),
            })
        })
        .collect()
}

fn evaluate_value(value: &DslValue, scope: &mut CompileScope) -> Result<Evaluated, DslError> {
    match value {
        DslValue::Number(value) => Ok(Evaluated::Number(*value)),
        DslValue::String(value) => Ok(Evaluated::String(value.clone())),
        DslValue::Bool(value) => Ok(Evaluated::Bool(*value)),
        DslValue::Surface(surface) => Ok(Evaluated::Surface(surface.clone())),
        DslValue::Array { values, .. } => values
            .iter()
            .map(|value| evaluate_value(value, scope))
            .collect::<Result<Vec<_>, _>>()
            .map(Evaluated::Array),
        DslValue::Identifier { name, loc } => match scope.bindings.get(name) {
            Some(Binding::Value { value, .. }) => Ok(value.clone()),
            Some(Binding::Partial(_)) => Err(DslError::new(
                format!("Effect partial {name:?} cannot be used as a value"),
                loc,
            )),
            // An effect alias used as a value stays the effect's bare name.
            Some(Binding::Alias(alias)) => Ok(Evaluated::String(alias.clone())),
            None => Ok(Evaluated::String(name.clone())),
        },
        DslValue::Vector { width, values, loc } => {
            let values = values
                .iter()
                .map(|value| evaluate_value(value, scope))
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
            let Evaluated::Number(value) = evaluate_value(argument, scope)? else {
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
            let (Evaluated::Number(left), Evaluated::Number(right)) =
                (evaluate_value(left, scope)?, evaluate_value(right, scope)?)
            else {
                return Err(DslError::new("Arithmetic requires numeric values", loc));
            };
            Ok(Evaluated::Number(match operator.as_str() {
                "+" => left + right,
                "-" => left - right,
                "*" => left * right,
                _ => left / right,
            }))
        }
        // Upstream parses `osc(...)`, `midi(...)` and `audio(...)` as automation
        // values. The CPU port has no live MIDI or audio input, so those take
        // upstream's no-input branch and resolve to their `min` field.
        DslValue::Call(call) if matches!(call.name.as_str(), "osc" | "midi" | "audio") => Ok(
            Evaluated::Automation(compile_automation_call(call, scope, 0)?),
        ),
        DslValue::Call(call) => Err(DslError::new(
            "Effect call cannot be used as a value",
            &call.loc,
        )),
        // A chain in an argument compiles to a hidden surface. Outside argument
        // resolution (a `let` value) a chain is not a usable value.
        DslValue::Chain(chain) => match scope.pending {
            Some(_) => scope.compile_inline_chain(chain),
            None => Err(DslError::new("Unsupported DSL value Chain", &chain.loc)),
        },
    }
}

fn merge_args(stored: &DslCall, call: &DslCall) -> Result<DslCall, DslError> {
    if stored.arg_mode.is_none() {
        return Ok(DslCall {
            name: stored.name.clone(),
            args: call.args.clone(),
            arg_mode: call.arg_mode.clone(),
            namespace: call.namespace.clone(),
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
        return Ok(DslCall {
            name: stored.name.clone(),
            args,
            arg_mode: stored.arg_mode.clone(),
            namespace: call.namespace.clone(),
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
    Ok(DslCall {
        name: stored.name.clone(),
        args,
        arg_mode: Some("named".into()),
        namespace: call.namespace.clone(),
        loc: call.loc.clone(),
    })
}

fn merge_partial(stored: &DslCall, call: &DslCall) -> Result<DslCall, DslError> {
    // A `from(ns, ...)` namespace on the call site wins over one stored in the
    // partial, as in upstream's resolveCall.
    let mut merged = merge_args(stored, call)?;
    merged.namespace = call.namespace.clone().or_else(|| stored.namespace.clone());
    Ok(merged)
}

// `read3d(vol, geo)` and `write3d(vol, geo)` take two positional references:
// identifiers such as vol0 and geo0, or surface references (upstream
// parseWriteCall and the validator's Read3D handling).
fn volume_references(call: &DslCall) -> Option<(String, String)> {
    if call.arg_mode.as_deref() != Some("positional") || call.args.len() != 2 {
        return None;
    }
    let mut names = Vec::new();
    for argument in &call.args {
        match &argument.value {
            DslValue::Surface(surface) => names.push(surface.name.clone()),
            DslValue::Identifier { name, .. } if !name.contains('.') => names.push(name.clone()),
            _ => return None,
        }
    }
    Some((names.remove(0), names.remove(0)))
}

fn read3d_references(call: &DslCall) -> Option<(String, String)> {
    if call.arg_mode.as_deref() == Some("named") {
        let named = call
            .args
            .iter()
            .filter_map(|argument| {
                argument
                    .name
                    .as_deref()
                    .map(|name| (name.to_owned(), argument.clone()))
            })
            .collect::<BTreeMap<String, DslArgument>>();
        let tex3d = named.get("tex3d")?.clone();
        let geo = named.get("geo")?.clone();
        return volume_references(&DslCall {
            name: String::new(),
            args: vec![tex3d, geo],
            arg_mode: Some("positional".into()),
            namespace: None,
            loc: call.loc.clone(),
        });
    }
    volume_references(call)
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
        let Some(name) = supplied_name
            .as_ref()
            .filter(|name| definition.params.contains_key(*name))
            .cloned()
        else {
            let bad = supplied_name.unwrap_or_else(|| format!("argument {}", index + 1));
            let accepted = definition.param_names.join(", ");
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

// A float parameter may declare inline choices (the `rotation` parameter of the
// palette filters). Upstream's validator resolves a bare identifier against
// them and its unparser writes the choice name, so they resolve here as for
// `int`.
fn float_choice_value(
    name: &str,
    spec: &ParameterSpec,
    value: Evaluated,
    loc: &Location,
) -> Result<Evaluated, DslError> {
    let Evaluated::String(text) = &value else {
        return Ok(value);
    };
    let Some(choices) = spec.metadata.get("choices").and_then(JsonValue::as_object) else {
        return Ok(value);
    };
    let key = text.rsplit('.').next().unwrap_or(text);
    match choices.get(key).and_then(JsonValue::as_f64) {
        Some(number) => Ok(Evaluated::Number(number)),
        None => Err(DslError::new(
            format!("Parameter {name:?} has invalid enum value {text:?}"),
            loc,
        )),
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
    // An `osc(...)`/`midi(...)`/`audio(...)` automation value (numeric params
    // only, matching upstream's uniformSpecs contract) is kept as-is here and
    // resolved to a concrete number per render in the renderer; static values
    // validate as before.
    if let Evaluated::Automation(descriptor) = &value {
        if matches!(spec.parameter_type.as_str(), "float" | "int") {
            params.insert(name.into(), ParamValue::Automation(descriptor.clone()));
            return Ok(());
        }
    }
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
            let value = float_choice_value(name, spec, value, loc)?;
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
                            .map_err(fail)?
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

// --- osc(), midi() and audio() automation values ------------------------------
//
// A port of upstream's parser transformOscInvocation/transformMidiInvocation/
// transformAudioInvocation and the validator's compileAutomationDescriptor.
// Errors that upstream reports with severity "error" (its P003 parser errors,
// S001 and S003) throw DslError. Upstream warnings (S002) keep the field's
// fallback value and mark the descriptor `_invalid` where upstream does, so the
// program still compiles. The errors use the port's existing message templates;
// the P003 errors name the call.

const OSC_KIND_VALUES: [(i32, &str); 8] = [
    (0, "sine"),
    (1, "tri"),
    (2, "saw"),
    (3, "sawInv"),
    (4, "square"),
    (5, "noise"),
    (5, "noise1d"),
    (6, "noise2d"),
];

const OSC_PARAM_ORDER: [&str; 6] = ["type", "min", "max", "speed", "offset", "seed"];
const MIDI_POSITIONAL: [&str; 5] = ["channel", "mode", "min", "max", "sensitivity"];
const MIDI_KEYWORD_ONLY: [&str; 6] = ["name", "id", "cc", "nrpn", "zone", "members"];
const AUDIO_POSITIONAL: [&str; 3] = ["band", "min", "max"];
const AUDIO_KEYWORD_ONLY: [&str; 3] = ["channel", "name", "id"];

fn automation_unknown_parameter(call: &DslCall, bad_name: &str, valid: &[&str]) -> DslError {
    DslError::new(
        format!(
            "Unknown parameter {:?} for {}(); accepted: {}",
            bad_name,
            call.name,
            valid.join(", ")
        ),
        &call.loc,
    )
}

fn automation_missing_parameter(call: &DslCall, name: &str) -> DslError {
    DslError::new(
        format!("Missing required parameter {name:?} for {}()", call.name),
        &call.loc,
    )
}

fn automation_unsupported_call(call: &DslCall) -> DslError {
    DslError::new(
        format!("Unsupported DSL value Call {:?}", call.name),
        &call.loc,
    )
}

// Replaces an identifier bound by `let` with the bound expression, as upstream's
// validator substitutes symbols before it compiles an automation descriptor, and
// folds arithmetic to a number, as upstream's parser does.
fn substitute_field(
    value: &DslValue,
    scope: &mut CompileScope,
    seen: &mut BTreeSet<String>,
) -> DslValue {
    match value {
        DslValue::Unary { .. } | DslValue::Binary { .. } => match evaluate_value(value, scope) {
            Ok(Evaluated::Number(number)) => DslValue::Number(number),
            _ => value.clone(),
        },
        DslValue::Identifier { name, loc } if !name.contains('.') => {
            if seen.contains(name) {
                return value.clone();
            }
            let bound = match scope.bindings.get(name) {
                Some(Binding::Value { ast, .. }) => Some(ast.clone()),
                Some(Binding::Alias(alias)) => {
                    return DslValue::Identifier {
                        name: alias.clone(),
                        loc: loc.clone(),
                    };
                }
                // An effect partial is a call that is not an automation source.
                Some(Binding::Partial(partial)) => {
                    return DslValue::Call(Box::new(DslCall {
                        name: partial.call.name.clone(),
                        args: Vec::new(),
                        arg_mode: None,
                        namespace: None,
                        loc: loc.clone(),
                    }));
                }
                None => None,
            };
            match bound {
                Some(bound) => {
                    seen.insert(name.clone());
                    substitute_field(&bound, scope, seen)
                }
                None => value.clone(),
            }
        }
        _ => value.clone(),
    }
}

fn resolve_automation_enum(
    node: &DslValue,
    enum_name: &str,
    fallback: Option<f64>,
    valid_values: &[f64],
    call: &DslCall,
    field: &str,
) -> Result<Option<f64>, DslError> {
    // Upstream resolveAutomationEnum: a number, an enum path, or a bare member
    // name of `enumName`. A string literal is an error; any other unresolved
    // value is a warning that keeps the fallback.
    let mut resolved = None;
    match node {
        DslValue::Number(value) => resolved = Some(*value),
        DslValue::Identifier { name, .. } => {
            resolved = if name.contains('.') {
                crate::automation::resolve_std_enum(name)
            } else {
                crate::automation::resolve_std_enum(&format!("{enum_name}.{name}"))
            };
        }
        _ => {}
    }
    if let Some(value) = resolved.filter(|value| valid_values.contains(value)) {
        return Ok(Some(value));
    }
    if matches!(node, DslValue::String(_)) {
        return Err(DslError::new(
            format!("Parameter {field:?} must be an enum value"),
            &call.loc,
        ));
    }
    Ok(fallback)
}

// Upstream resolveAutomationNumber. Returns the field and whether it is valid;
// `valid` is false where upstream reports a warning and calls the field's
// onInvalid callback.
#[allow(clippy::too_many_arguments)]
fn resolve_automation_number(
    node: Option<DslValue>,
    scope: &mut CompileScope,
    depth: usize,
    call: &DslCall,
    field: &str,
    fallback: f64,
    allow_boolean: bool,
    allow_automation: bool,
    allow_member: bool,
    integer: bool,
    min: Option<f64>,
    max: Option<f64>,
    clamp01: bool,
) -> Result<(AutomationField, bool), DslError> {
    let Some(node) = node else {
        return Ok((AutomationField::Number(fallback), true));
    };
    let invalid = AutomationField::Number(fallback);
    let value = match node {
        DslValue::Number(value) => Some(value),
        DslValue::Bool(value) if allow_boolean => Some(f64::from(value)),
        DslValue::Identifier { name, .. } if name.contains('.') && allow_member => {
            crate::automation::resolve_std_enum(&name)
        }
        DslValue::Call(ref call)
            if allow_automation && matches!(call.name.as_str(), "osc" | "midi" | "audio") =>
        {
            let compiled = compile_automation_call(call, scope, depth + 1)?;
            let invalid = compiled.is_invalid();
            return Ok((AutomationField::Ref(Box::new(compiled)), !invalid));
        }
        DslValue::String(_) => {
            // Upstream S001 (a string literal) and S003 (an undefined
            // automation source).
            return Err(DslError::new(
                format!("Parameter {field:?} must be a finite number"),
                &call.loc,
            ));
        }
        DslValue::Identifier { name, .. } if !name.contains('.') => {
            return Err(DslError::new(
                format!("Parameter {field:?} must be a finite number"),
                &call.loc,
            ));
        }
        _ => None,
    };
    let Some(value) = value else {
        return Ok((invalid, false));
    };
    if !value.is_finite() {
        return Ok((invalid, false));
    }
    if integer && value.fract() != 0.0 {
        return Ok((invalid, false));
    }
    if min.is_some_and(|min| value < min) || max.is_some_and(|max| value > max) {
        return Ok((invalid, false));
    }
    Ok((
        AutomationField::Number(if clamp01 {
            value.clamp(0.0, 1.0)
        } else {
            value
        }),
        true,
    ))
}

// Splits a midi()/audio() call into upstream's resolved fields: positional
// arguments fill the positional order densely, keyword arguments take their own
// slots (the last duplicate keyword wins), and omitted fields take defaults.
fn automation_fields(
    call: &DslCall,
    scope: &mut CompileScope,
    positional_order: &[&str],
    keyword_only: &[&str],
    defaults: &[(&str, f64)],
) -> Result<BTreeMap<String, DslValue>, DslError> {
    let mut valid: Vec<&str> = positional_order.to_vec();
    valid.extend_from_slice(keyword_only);
    let mut positional: Vec<(usize, &DslArgument)> = Vec::new();
    let mut kwargs: BTreeMap<String, &DslArgument> = BTreeMap::new();
    for (index, argument) in call.args.iter().enumerate() {
        match &argument.name {
            Some(name) => {
                kwargs.insert(name.clone(), argument);
            }
            None => positional.push((index, argument)),
        }
    }
    // Upstream: the keyword-only fields cannot take a positional argument.
    if positional.len() > positional_order.len() {
        let (index, _) = positional[positional_order.len()];
        return Err(automation_unknown_parameter(
            call,
            &format!("argument {}", index + 1),
            &valid,
        ));
    }
    for name in kwargs.keys() {
        if !valid.contains(&name.as_str()) {
            return Err(automation_unknown_parameter(call, name, &valid));
        }
    }
    let mut fields: BTreeMap<String, DslValue> = BTreeMap::new();
    let mut cursor = 0;
    for field in positional_order {
        if let Some(argument) = kwargs.get(*field) {
            fields.insert((*field).into(), argument.value.clone());
        } else if cursor < positional.len() {
            let (_, argument) = positional[cursor];
            cursor += 1;
            fields.insert((*field).into(), argument.value.clone());
        } else if let Some((_, default)) = defaults.iter().find(|(name, _)| name == field) {
            fields.insert((*field).into(), DslValue::Number(*default));
        }
    }
    if cursor < positional.len() {
        let (index, _) = positional[cursor];
        return Err(automation_unknown_parameter(
            call,
            &format!("argument {}", index + 1),
            &valid,
        ));
    }
    for field in keyword_only {
        if let Some(argument) = kwargs.get(*field) {
            fields.insert((*field).into(), argument.value.clone());
        }
    }
    for (name, field) in fields.iter_mut() {
        if name != "name" && name != "id" {
            let mut seen = BTreeSet::new();
            *field = substitute_field(field, scope, &mut seen);
        }
    }
    Ok(fields)
}

// The selector strings must be non-empty string literals (upstream parser).
fn check_selector_strings(
    call: &DslCall,
    fields: &BTreeMap<String, DslValue>,
) -> Result<(), DslError> {
    for field in ["name", "id"] {
        if let Some(DslValue::String(value)) = fields.get(field) {
            if value.is_empty() {
                return Err(automation_unsupported_call(call));
            }
            continue;
        }
        if fields.contains_key(field) {
            return Err(automation_unsupported_call(call));
        }
    }
    Ok(())
}

fn compile_oscillator(
    call: &DslCall,
    scope: &mut CompileScope,
    depth: usize,
) -> Result<AutomationValue, DslError> {
    // Mirrors upstream std_enums.js oscKind (sine..noise2d; noise/noise1d alias
    // kind 5, noise2d is the two-stage periodic noise). Compiles an `osc(...)`
    // value-position call into the automation value shape the runtime evaluator
    // consumes, mirroring upstream's parser transformOscInvocation plus the
    // validator's compileAutomationDescriptor: positional args fill
    // type/min/max/speed/offset/seed in order, kwargs select by name, every
    // field defaults as upstream defaults, min/max clamp into [0,1], nested
    // osc() fields are allowed, and the oscType resolves from an integer 0..6
    // or an oscKind name (bare or oscKind-qualified). Invalid programs throw
    // DslError, as everywhere else in this compiler.
    if depth > crate::automation::MAX_AUTOMATION_DEPTH {
        return Err(DslError::new(
            format!(
                "Automation nesting exceeds the maximum depth of {}",
                crate::automation::MAX_AUTOMATION_DEPTH
            ),
            &call.loc,
        ));
    }
    let mut fields: BTreeMap<&str, &DslValue> = BTreeMap::new();
    if call.arg_mode.as_deref() == Some("named") {
        for argument in &call.args {
            let Some(name) = &argument.name else {
                continue;
            };
            if !OSC_PARAM_ORDER.contains(&name.as_str()) {
                return Err(DslError::new(
                    format!(
                        "osc() unknown parameter '{name}'; valid: {}",
                        OSC_PARAM_ORDER.join(", ")
                    ),
                    &call.loc,
                ));
            }
            fields.insert(name.as_str(), &argument.value);
        }
    } else {
        for (argument, name) in call.args.iter().zip(OSC_PARAM_ORDER) {
            fields.insert(name, &argument.value);
        }
    }

    let raw_type = match fields.get("type") {
        Some(node) => evaluate_value(node, scope)?,
        None => Evaluated::Number(0.0),
    };
    let osc_type = match raw_type {
        Evaluated::Number(value) => {
            if !value.is_finite() || value.fract() != 0.0 || !(0.0..=6.0).contains(&value) {
                return Err(DslError::new(
                    "osc() type must resolve to a supported oscKind value (0-6)",
                    &call.loc,
                ));
            }
            value as i32
        }
        Evaluated::String(raw) => {
            let kind_name = raw.strip_prefix("oscKind.").unwrap_or(&raw);
            match OSC_KIND_VALUES.iter().find(|(_, name)| *name == kind_name) {
                Some((kind, _)) => *kind,
                None => {
                    return Err(DslError::new(
                        format!(
                            "osc() type must resolve to a supported oscKind value; got {:?}",
                            raw
                        ),
                        &call.loc,
                    ));
                }
            }
        }
        _ => {
            return Err(DslError::new(
                "osc() type must resolve to a supported oscKind value",
                &call.loc,
            ));
        }
    };

    let number_field = |node: Option<&&DslValue>,
                        name: &str,
                        fallback: f64,
                        clamp: bool,
                        scope: &mut CompileScope|
     -> Result<AutomationField, DslError> {
        let Some(node) = node else {
            return Ok(AutomationField::Number(fallback));
        };
        let value = evaluate_value(node, scope)?;
        match value {
            Evaluated::Automation(descriptor) => Ok(AutomationField::Ref(Box::new(descriptor))),
            Evaluated::Bool(true) => Ok(AutomationField::Number(1.0)),
            Evaluated::Bool(false) => Ok(AutomationField::Number(0.0)),
            Evaluated::Number(value) if value.is_finite() => {
                Ok(AutomationField::Number(if clamp {
                    value.clamp(0.0, 1.0)
                } else {
                    value
                }))
            }
            _ => Err(DslError::new(
                format!("osc() {name} must be a number or a nested osc()"),
                &call.loc,
            )),
        }
    };

    Ok(AutomationValue::Oscillator {
        osc_type,
        min: number_field(fields.get("min"), "min", 0.0, true, scope)?,
        max: number_field(fields.get("max"), "max", 1.0, true, scope)?,
        speed: number_field(fields.get("speed"), "speed", 1.0, false, scope)?,
        offset: number_field(fields.get("offset"), "offset", 0.0, false, scope)?,
        seed: number_field(fields.get("seed"), "seed", 1.0, false, scope)?,
    })
}

fn compile_midi(
    call: &DslCall,
    scope: &mut CompileScope,
    depth: usize,
) -> Result<AutomationValue, DslError> {
    if depth > crate::automation::MAX_AUTOMATION_DEPTH {
        return Err(DslError::new(
            format!(
                "Automation nesting exceeds the maximum depth of {}",
                crate::automation::MAX_AUTOMATION_DEPTH
            ),
            &call.loc,
        ));
    }
    let fields = automation_fields(
        call,
        scope,
        &MIDI_POSITIONAL,
        &MIDI_KEYWORD_ONLY,
        &[
            ("mode", 4.0),
            ("min", 0.0),
            ("max", 1.0),
            ("sensitivity", 1.0),
        ],
    )?;
    let has_zone = fields.contains_key("zone");
    if !fields.contains_key("channel") && !has_zone {
        return Err(automation_missing_parameter(call, "channel"));
    }
    if fields.contains_key("channel") && has_zone {
        return Err(automation_unsupported_call(call));
    }
    if fields.contains_key("members") && !has_zone {
        return Err(automation_missing_parameter(call, "zone"));
    }
    if fields.contains_key("id") && !fields.contains_key("name") {
        return Err(automation_missing_parameter(call, "name"));
    }
    check_selector_strings(call, &fields)?;

    let mode = resolve_automation_enum(
        fields.get("mode").unwrap_or(&DslValue::Number(4.0)),
        "midiMode",
        Some(4.0),
        &[0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0],
        call,
        "mode",
    )?
    .unwrap_or(4.0) as i64;
    let zone = if has_zone {
        resolve_automation_enum(
            fields.get("zone").unwrap_or(&DslValue::Number(0.0)),
            "midiZone",
            None,
            &[0.0, 1.0],
            call,
            "zone",
        )?
    } else {
        None
    };
    let mut valid_selection = !has_zone || zone.is_some();
    let mut members = None;
    if let Some(node) = fields.get("members") {
        let (value, valid) = resolve_automation_number(
            Some(node.clone()),
            scope,
            depth,
            call,
            "members",
            f64::NAN,
            false,
            false,
            false,
            true,
            Some(1.0),
            Some(15.0),
            false,
        )?;
        members = valid.then(|| value.as_number()).flatten();
        if !valid {
            valid_selection = false;
        }
    }
    let mut valid_channel = true;
    let mut channel = None;
    if !has_zone {
        let (value, valid) = resolve_automation_number(
            fields
                .get("channel")
                .cloned()
                .or(Some(DslValue::Number(1.0))),
            scope,
            depth,
            call,
            "channel",
            1.0,
            false,
            false,
            false,
            true,
            Some(1.0),
            Some(16.0),
            false,
        )?;
        channel = value.as_number().map(|value| value as i64);
        valid_channel = valid;
    }
    let mut valid_cc = true;
    let mut cc = None;
    if fields.contains_key("cc") || mode == 5 || mode == 6 {
        let (value, valid) = resolve_automation_number(
            fields.get("cc").cloned().or(Some(DslValue::Number(1.0))),
            scope,
            depth,
            call,
            "cc",
            1.0,
            false,
            false,
            false,
            true,
            Some(0.0),
            Some(if mode == 6 { 31.0 } else { 127.0 }),
            false,
        )?;
        cc = value.as_number().map(|value| value as i64);
        valid_cc = valid;
    }
    let mut nrpn = None;
    if fields.contains_key("nrpn") || mode == 7 {
        if !fields.contains_key("nrpn") {
            valid_selection = false;
        }
        let (value, valid) = resolve_automation_number(
            fields.get("nrpn").cloned(),
            scope,
            depth,
            call,
            "nrpn",
            f64::NAN,
            false,
            false,
            false,
            true,
            Some(0.0),
            Some(16382.0),
            false,
        )?;
        nrpn = valid
            .then(|| value.as_number().map(|number| number as i64))
            .flatten();
        if !valid {
            valid_selection = false;
        }
    }
    let (min, _) = resolve_automation_number(
        fields.get("min").cloned().or(Some(DslValue::Number(0.0))),
        scope,
        depth,
        call,
        "min",
        0.0,
        true,
        true,
        true,
        false,
        None,
        None,
        true,
    )?;
    let (max, _) = resolve_automation_number(
        fields.get("max").cloned().or(Some(DslValue::Number(1.0))),
        scope,
        depth,
        call,
        "max",
        1.0,
        true,
        true,
        true,
        false,
        None,
        None,
        true,
    )?;
    let (sensitivity, _) = resolve_automation_number(
        fields
            .get("sensitivity")
            .cloned()
            .or(Some(DslValue::Number(1.0))),
        scope,
        depth,
        call,
        "sensitivity",
        1.0,
        true,
        true,
        true,
        false,
        None,
        None,
        false,
    )?;
    let selector = |field: &str| match fields.get(field) {
        Some(DslValue::String(value)) => Some(value.clone()),
        _ => None,
    };
    Ok(AutomationValue::Midi {
        channel,
        mode,
        cc,
        nrpn,
        zone,
        members,
        min,
        max,
        sensitivity,
        name: selector("name"),
        id: selector("id"),
        invalid: !valid_cc || !valid_channel || !valid_selection,
    })
}

fn compile_audio(
    call: &DslCall,
    scope: &mut CompileScope,
    depth: usize,
) -> Result<AutomationValue, DslError> {
    if depth > crate::automation::MAX_AUTOMATION_DEPTH {
        return Err(DslError::new(
            format!(
                "Automation nesting exceeds the maximum depth of {}",
                crate::automation::MAX_AUTOMATION_DEPTH
            ),
            &call.loc,
        ));
    }
    let fields = automation_fields(
        call,
        scope,
        &AUDIO_POSITIONAL,
        &AUDIO_KEYWORD_ONLY,
        &[("min", 0.0), ("max", 1.0)],
    )?;
    if !fields.contains_key("band") {
        return Err(automation_missing_parameter(call, "band"));
    }
    if fields.contains_key("id") && !fields.contains_key("name") {
        return Err(automation_missing_parameter(call, "name"));
    }
    if fields.contains_key("name") && !fields.contains_key("channel") {
        return Err(automation_missing_parameter(call, "channel"));
    }
    check_selector_strings(call, &fields)?;

    let band = resolve_automation_enum(
        fields.get("band").unwrap_or(&DslValue::Number(0.0)),
        "audioBand",
        None,
        &[0.0, 1.0, 2.0, 3.0, 4.0],
        call,
        "band",
    )?;
    let (min, min_valid) = resolve_automation_number(
        fields.get("min").cloned().or(Some(DslValue::Number(0.0))),
        scope,
        depth,
        call,
        "min",
        0.0,
        false,
        true,
        false,
        false,
        None,
        None,
        true,
    )?;
    let (max, max_valid) = resolve_automation_number(
        fields.get("max").cloned().or(Some(DslValue::Number(1.0))),
        scope,
        depth,
        call,
        "max",
        1.0,
        false,
        true,
        false,
        false,
        None,
        None,
        true,
    )?;
    let mut channel = None;
    let mut valid_channel = true;
    if let Some(node) = fields.get("channel") {
        if let DslValue::Number(value) = node {
            if value.fract() == 0.0 && (1.0..=32.0).contains(value) {
                channel = Some(*value as i64);
            } else {
                valid_channel = false;
            }
        } else if matches!(node, DslValue::String(_)) {
            return Err(DslError::new(
                "Parameter \"channel\" must be an integer",
                &call.loc,
            ));
        } else {
            valid_channel = false;
        }
    }
    let selector = |field: &str| match fields.get(field) {
        Some(DslValue::String(value)) => Some(value.clone()),
        _ => None,
    };
    Ok(AutomationValue::Audio {
        band,
        min,
        max,
        channel,
        name: selector("name"),
        id: selector("id"),
        invalid: band.is_none() || !min_valid || !max_valid || !valid_channel,
    })
}

fn compile_automation_call(
    call: &DslCall,
    scope: &mut CompileScope,
    depth: usize,
) -> Result<AutomationValue, DslError> {
    match call.name.as_str() {
        "midi" => compile_midi(call, scope, depth),
        "audio" => compile_audio(call, scope, depth),
        _ => compile_oscillator(call, scope, depth),
    }
}
