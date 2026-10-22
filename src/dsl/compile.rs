use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value as JsonValue;

use super::{
    DslArgument, DslBinding, DslCall, DslChain, DslChainElement, DslError, DslProgram, DslValue,
    Location, SurfaceRef, parse_dsl,
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
    // A `midi()`/`audio()` value call: unsupported in CPU ports, warned about
    // and ignored. Parameter consumers keep their declared default, matching
    // upstream's undefined-argument skip.
    Undefined,
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
    // A value binding resolves lazily at its first use: statement order
    // (upstream 9cc041d0) makes a chain see only the lets declared at or
    // before its own statement index, so a name declared by a later let stays
    // pending until its let declares. A bare identifier value must name
    // something resolvable at declaration, as upstream's declareVar requires.
    Value {
        expr: DslValue,
        resolved: bool,
        value: Option<Evaluated>,
    },
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
    // Names of the program's lets that a later statement declares. Reading one
    // before its let reports S003, as upstream's validator does.
    pending_names: Option<BTreeSet<String>>,
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
        pending_names: None,
    };
    // Statement order (upstream 9cc041d0): a chain sees only the lets declared
    // at or before its own statement index; a later `let` of the same name
    // reassigns it for later statements; reading a pending name reports S003.
    let mut next_binding = 0usize;
    let mut chains = Vec::new();
    for (chain_index, chain) in ast.chains.iter().enumerate() {
        declare_through(chain_index, &ast.bindings, &mut next_binding, &mut scope)?;
        // pendingNames covers the whole chain compile, including inline
        // chains: every argument position reports a pending read, as upstream
        // does.
        scope.pending_names = Some(
            ast.bindings[next_binding..]
                .iter()
                .map(|binding| binding.name.clone())
                .collect(),
        );
        let compiled = compile_chain(chain, &mut scope, false);
        scope.pending_names = None;
        chains.push(compiled?);
    }
    declare_through(usize::MAX, &ast.bindings, &mut next_binding, &mut scope)?;
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

// Declares every binding whose statement index has arrived. A binding's
// planIndex is the number of chain statements written before its `let`.
fn declare_through(
    chain_index: usize,
    bindings: &[DslBinding],
    next_binding: &mut usize,
    scope: &mut CompileScope,
) -> Result<(), DslError> {
    while *next_binding < bindings.len() && bindings[*next_binding].plan_index <= chain_index {
        declare_binding(&bindings[*next_binding], scope)?;
        *next_binding += 1;
    }
    Ok(())
}

// Declares one let. Statement order (upstream 9cc041d0): a later `let` of the
// same name reassigns it for later statements, so a duplicate name is not an
// error. A bare identifier value must name something resolvable now (an
// earlier binding, an alias, or an effect), as upstream's declareVar requires.
fn declare_binding(binding: &DslBinding, scope: &mut CompileScope) -> Result<(), DslError> {
    if let DslValue::Call(call) = &binding.value {
        if !matches!(call.name.as_str(), "osc" | "midi" | "audio") {
            // A partial keeps its call unresolved. Its arguments resolve where
            // it is called, as upstream's validator substitutes a symbol at
            // each use.
            scope.bindings.insert(
                binding.name.clone(),
                Binding::Partial(Partial {
                    call: (**call).clone(),
                }),
            );
            return Ok(());
        }
    }
    if let Some(alias) = alias_binding(&binding.value, scope) {
        scope.bindings.insert(binding.name.clone(), alias);
        return Ok(());
    }
    if let DslValue::Identifier { name, loc } = &binding.value {
        let name = name.as_str();
        if !name.contains('.')
            && !scope.bindings.contains_key(name)
            && !resolvable_effect(scope, name)
        {
            let loc = loc.clone();
            return Err(DslError::new(
                format!("Variable used before assignment: '{name}'"),
                &loc,
            ));
        }
    }
    scope.bindings.insert(
        binding.name.clone(),
        Binding::Value {
            expr: binding.value.clone(),
            resolved: false,
            value: None,
        },
    );
    Ok(())
}

// An effect alias declares when the name resolves in the search namespaces, as
// upstream's registry lookup does.
fn resolvable_effect(scope: &CompileScope, name: &str) -> bool {
    scope.search.iter().any(|namespace| {
        scope
            .catalog
            .effects
            .contains_key(&format!("{namespace}/{name}"))
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
                value: evaluate_value(&argument.value, scope, 0, &[])?,
                loc: argument.loc.clone(),
            })
        })
        .collect()
}

// `resolving` threads the chain of binding names currently being resolved so a
// cyclic let set (`let a = b`, `let b = a`) fails instead of looping forever,
// as upstream's validator substitution detects the same cycle. `osc_depth` is
// the automation nesting depth: only oscillator fields deepen it.
fn evaluate_value(
    value: &DslValue,
    scope: &mut CompileScope,
    osc_depth: usize,
    resolving: &[String],
) -> Result<Evaluated, DslError> {
    match value {
        DslValue::Number(value) => Ok(Evaluated::Number(*value)),
        DslValue::String(value) => Ok(Evaluated::String(value.clone())),
        DslValue::Bool(value) => Ok(Evaluated::Bool(*value)),
        DslValue::Surface(surface) => Ok(Evaluated::Surface(surface.clone())),
        DslValue::Array { values, .. } => values
            .iter()
            .map(|value| evaluate_value(value, scope, osc_depth, resolving))
            .collect::<Result<Vec<_>, _>>()
            .map(Evaluated::Array),
        DslValue::Identifier { name, loc } => match scope.bindings.get(name) {
            Some(Binding::Value {
                resolved,
                value: bound,
                ..
            }) if *resolved => Ok(bound.clone().expect("resolved bindings hold a value")),
            Some(Binding::Partial(_)) => Err(DslError::new(
                format!("Effect partial {name:?} cannot be used as a value"),
                loc,
            )),
            // An effect alias used as a value stays the effect's bare name.
            Some(Binding::Alias(alias)) => Ok(Evaluated::String(alias.clone())),
            Some(_) => {
                // A value binding resolves lazily at its first use. A name
                // declared by a later let is pending here and reading it
                // reports S003, as upstream does.
                if resolving.contains(name) {
                    let mut chain = resolving.to_vec();
                    chain.push(name.clone());
                    return Err(DslError::new(
                        format!("Automation cycle detected: {}", chain.join(" -> ")),
                        loc,
                    ));
                }
                let expr = match scope.bindings.get(name) {
                    Some(Binding::Value { expr, .. }) => expr.clone(),
                    _ => unreachable!("only value bindings reach lazy resolution"),
                };
                let mut next_resolving = resolving.to_vec();
                next_resolving.push(name.clone());
                let value = evaluate_value(&expr, scope, osc_depth, &next_resolving)?;
                if let Some(Binding::Value {
                    resolved,
                    value: bound,
                    ..
                }) = scope.bindings.get_mut(name)
                {
                    *resolved = true;
                    *bound = Some(value.clone());
                }
                Ok(value)
            }
            None => {
                if scope
                    .pending_names
                    .as_ref()
                    .is_some_and(|pending| pending.contains(name))
                {
                    return Err(DslError::new(
                        format!("Variable used before assignment: '{name}'"),
                        loc,
                    ));
                }
                Ok(Evaluated::String(name.clone()))
            }
        },
        DslValue::Vector { width, values, loc } => {
            let values = values
                .iter()
                .map(|value| evaluate_value(value, scope, 0, resolving))
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
            let Evaluated::Number(value) = evaluate_value(argument, scope, 0, resolving)? else {
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
                evaluate_value(left, scope, 0, resolving)?,
                evaluate_value(right, scope, 0, resolving)?,
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
        // Upstream parses `osc(...)`, `midi(...)` and `audio(...)` as automation
        // value calls. The CPU port has no live MIDI or audio input, so midi()
        // and audio() warn and leave their consumers' offline defaults in
        // place.
        DslValue::Call(call) if matches!(call.name.as_str(), "osc" | "midi" | "audio") => {
            compile_automation_call(call, scope, osc_depth, resolving)
        }
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
    // Canonical names supplied explicitly by named arguments: an alias never
    // overrides an explicitly supplied canonical name; the canonical name wins.
    let canonical_supplied = args
        .iter()
        .filter_map(|argument| {
            let name = argument.name.as_ref()?;
            if matches!(argument.value, Evaluated::Undefined) {
                return None;
            }
            Some(name.clone())
        })
        .collect::<std::collections::BTreeSet<_>>();
    let mut warned_aliases = BTreeSet::new();
    for (index, argument) in args.iter().enumerate() {
        let supplied_name = argument
            .name
            .clone()
            .or_else(|| definition.param_names.get(index).cloned());
        let resolved_name = supplied_name.as_ref().and_then(|supplied| {
            definition
                .param_aliases
                .as_ref()
                .and_then(|aliases| aliases.get(supplied).cloned())
                .or_else(|| Some(supplied.clone()))
        });
        let Some(name) = resolved_name
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
        // Deprecated aliases resolve exactly as upstream's resolveParamAliases:
        // an alias migrates its value to the canonical name, and every alias
        // name warns once per call, as upstream's S007 diagnostic does.
        if let Some(supplied) = &supplied_name {
            if supplied != &name
                && !matches!(argument.value, Evaluated::Undefined)
                && warned_aliases.insert(supplied.clone())
            {
                eprintln!(
                    "param '{}' is deprecated, use '{}' instead. Aliases will be removed on 2027-06-01.",
                    supplied, name
                );
            }
        }
        // A midi()/audio() value is undefined in CPU ports: skip it so the
        // parameter keeps its declared default, as upstream's parameter loop
        // skips undefined arguments.
        if matches!(argument.value, Evaluated::Undefined) {
            continue;
        }
        if supplied_name != Some(name.clone()) && canonical_supplied.contains(&name) {
            continue;
        }
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
    // An `osc(...)` automation value (numeric params only, matching upstream's
    // uniformSpecs contract) is kept as-is here and resolved to a concrete
    // number per render in the renderer; static values validate as before.
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

// --- osc(), midi() and audio() value calls ------------------------------------
//
// Only `osc(...)` compiles to a live automation value. MIDI and audio input are
// unsupported in CPU ports: their value calls warn and resolve to nothing, so
// consumers keep their offline defaults.

// Mirrors upstream std_enums.js oscKind (sine..noise2d; noise/noise1d alias
// kind 5, noise2d is the two-stage periodic noise added upstream at eabb537e).
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

fn compile_oscillator(
    call: &DslCall,
    scope: &mut CompileScope,
    depth: usize,
    resolving: &[String],
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
        Some(node) => evaluate_value(node, scope, depth + 1, resolving)?,
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
        let value = evaluate_value(node, scope, depth + 1, resolving)?;
        match value {
            Evaluated::Undefined => Ok(AutomationField::Number(fallback)),
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

fn compile_automation_call(
    call: &DslCall,
    scope: &mut CompileScope,
    depth: usize,
    resolving: &[String],
) -> Result<Evaluated, DslError> {
    if call.name != "osc" {
        eprintln!(
            "Warning: {}() is unsupported in CPU ports; real-time input is unavailable. Ignoring this unsupported input value.",
            call.name
        );
        return Ok(Evaluated::Undefined);
    }
    compile_oscillator(call, scope, depth, resolving).map(Evaluated::Automation)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn definition_with_alias() -> EffectDefinition {
        EffectDefinition {
            namespace: "points".into(),
            func: "test".into(),
            kind: "filter".into(),
            domain: "image".into(),
            params: BTreeMap::from([(
                "canonical".to_string(),
                ParameterSpec {
                    parameter_type: "int".into(),
                    default: serde_json::json!(5),
                    uniform: None,
                    define: None,
                    metadata: BTreeMap::new(),
                },
            )]),
            param_names: vec!["canonical".to_string()],
            textures: BTreeMap::new(),
            passes: vec![],
            iterated: false,
            external_texture: None,
            output_tex3d: None,
            output_geo: None,
            loop_role: None,
            param_aliases: Some(BTreeMap::from([(
                "alias".to_string(),
                "canonical".to_string(),
            )])),
        }
    }

    fn loc() -> Location {
        Location {
            source_name: "test.dsl".into(),
            line: 1,
            column: 1,
            index: 0,
        }
    }

    fn named(name: &str, value: Evaluated) -> ResolvedArgument {
        ResolvedArgument {
            name: Some(name.to_string()),
            value,
            loc: loc(),
        }
    }

    #[test]
    fn alias_resolves_and_canonical_wins() {
        let definition = definition_with_alias();
        let args = vec![named("alias", Evaluated::Number(10.0))];
        let normalized = normalize_arguments("points/test", &definition, &args, &loc()).unwrap();
        assert_eq!(
            normalized.params.get("canonical"),
            Some(&ParamValue::Int(10))
        );

        // An explicitly supplied canonical name wins over the alias.
        let args = vec![
            named("canonical", Evaluated::Number(20.0)),
            named("alias", Evaluated::Number(10.0)),
        ];
        let normalized = normalize_arguments("points/test", &definition, &args, &loc()).unwrap();
        assert_eq!(
            normalized.params.get("canonical"),
            Some(&ParamValue::Int(20))
        );
    }

    #[test]
    fn undefined_alias_skipped() {
        let definition = definition_with_alias();
        let args = vec![named("alias", Evaluated::Undefined)];
        let normalized = normalize_arguments("points/test", &definition, &args, &loc()).unwrap();
        // An undefined alias value is skipped so the canonical parameter keeps
        // its declared default, as upstream's parameter loop does.
        assert_eq!(
            normalized.params.get("canonical"),
            Some(&ParamValue::Int(5))
        );
    }
}
