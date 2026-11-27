use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};
use std::{cell::RefCell, rc::Rc};

use thiserror::Error;

use crate::catalog::{
    CatalogError, EffectCatalog, EffectDefinition, Expression, effect_catalog, shader_bundle,
};
use crate::dsl::{DslError, RenderPlan, RenderStep, SurfaceBinding, compile_dsl};
use crate::external_input::{ExternalInputs, MeshData, empty_mesh_data, flip_rgba_rows};
use crate::iteration::{
    IterationStep, compute_groups_with_catalog, is_particle_state_name, iteration_schedule,
};
use crate::pass_runner::{
    RESERVED_FEEDBACK_RESOURCE, apply_pass_uniform_aliases, canonical_uniforms,
    execute_pass_with_bindings, first_output, normalize_parameters, pass_enabled, repeat_count,
    resolve_effect_automation, texture_dimensions,
};
use crate::{FilterMode, ParamValue, Surface, SurfaceError, TextureFormat, Value, VmError};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum OneShot {
    Initial,
    #[default]
    Ready,
}

#[derive(Clone, Debug)]
pub struct RenderOptions {
    pub width: u32,
    pub height: u32,
    pub time: f32,
    pub frame: u32,
    pub delta_time: f32,
    pub seed: i32,
    pub external_textures: BTreeMap<String, Surface>,
    pub external_inputs: ExternalInputs,
    pub one_shot: OneShot,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            width: 512,
            height: 512,
            time: 0.0,
            frame: 0,
            delta_time: 0.0,
            seed: 1,
            external_textures: BTreeMap::new(),
            external_inputs: ExternalInputs::default(),
            one_shot: OneShot::Ready,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RenderResult {
    pub surface: Surface,
    pub elapsed: Duration,
}

#[cfg(test)]
struct RenderTrace {
    result: RenderResult,
    particle_groups: Vec<ParticleGroupTrace>,
}

#[cfg(test)]
struct ParticleGroupTrace {
    resources: BTreeMap<String, Surface>,
    original_allocations: BTreeMap<String, usize>,
    // Retain the observed allocation for as long as this trace compares addresses.
    _retained: BTreeMap<String, Surface>,
}

#[cfg(test)]
fn trace_particle_group(shared: BTreeMap<String, Surface>) -> ParticleGroupTrace {
    let original_allocations = shared
        .iter()
        .map(|(name, surface)| (name.clone(), surface.data().as_ptr() as usize))
        .collect();
    ParticleGroupTrace {
        resources: shared.clone(),
        original_allocations,
        _retained: shared,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CpuTextureCacheStats {
    pub entries: usize,
    pub bytes: usize,
    pub byte_limit: usize,
}

struct CpuTextureCacheEntry {
    surface: Surface,
    last_used: u64,
}

struct CpuTextureCache {
    entries: BTreeMap<String, CpuTextureCacheEntry>,
    bytes: usize,
    byte_limit: usize,
    clock: u64,
}

impl CpuTextureCache {
    fn new(byte_limit: usize) -> Self {
        Self {
            entries: BTreeMap::new(),
            bytes: 0,
            byte_limit,
            clock: 0,
        }
    }
    fn get(&mut self, key: &str) -> Option<Surface> {
        self.clock = self.clock.wrapping_add(1);
        let entry = self.entries.get_mut(key)?;
        entry.last_used = self.clock;
        Some(entry.surface.clone())
    }
    fn insert(&mut self, key: String, surface: Surface) {
        let bytes = std::mem::size_of_val(surface.data());
        if bytes > self.byte_limit {
            return;
        }
        if let Some(previous) = self.entries.remove(&key) {
            self.bytes -= std::mem::size_of_val(previous.surface.data());
        }
        self.clock = self.clock.wrapping_add(1);
        self.entries.insert(
            key,
            CpuTextureCacheEntry {
                surface,
                last_used: self.clock,
            },
        );
        self.bytes += bytes;
        while self.bytes > self.byte_limit {
            let Some(oldest) = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(key, _)| key.clone())
            else {
                break;
            };
            let removed = self.entries.remove(&oldest).unwrap();
            self.bytes -= std::mem::size_of_val(removed.surface.data());
        }
    }
    fn clear(&mut self) {
        self.entries.clear();
        self.bytes = 0;
    }
    fn stats(&self) -> CpuTextureCacheStats {
        CpuTextureCacheStats {
            entries: self.entries.len(),
            bytes: self.bytes,
            byte_limit: self.byte_limit,
        }
    }
}

#[derive(Debug, Error)]
pub enum RenderError {
    #[error(transparent)]
    Dsl(#[from] DslError),
    #[error("surface {surface:?} has not been written in this render")]
    UnwrittenSurface { surface: String },
    #[error(
        "effect {effect_id:?} parameter {parameter:?} references unwritten surface {surface:?}"
    )]
    MissingSurfaceInput {
        effect_id: String,
        parameter: String,
        surface: String,
    },
    #[error(
        "effect {effect_id:?} uses unsupported CPU render domain {domain:?} (chain domains: {chain_domains:?}, volume size: {volume_size:?})"
    )]
    UnsupportedDomain {
        effect_id: String,
        domain: String,
        chain_domains: Vec<&'static str>,
        volume_size: Option<u32>,
    },
    #[error("unknown effect {effect_id:?}")]
    UnknownEffect { effect_id: String },
    #[error("unknown parameter {name:?}")]
    UnknownParameter { name: String },
    #[error("invalid parameter: {message}")]
    InvalidParameter { message: String },
    #[error("effect {effect_id:?} did not produce required output {output:?}")]
    MissingOutput { effect_id: String, output: String },
    #[error("pass {pass:?} references missing shader program {program:?}")]
    MissingProgram { pass: String, program: String },
    #[error("missing CPU draw adapter {key:?} for pass {pass:?}")]
    MissingDrawOp { key: String, pass: String },
    #[error("pass {pass:?} MRT destinations must share dimensions: {detail}")]
    MrtDestinationDimensions {
        pass: String,
        destinations: Vec<(String, u32, u32)>,
        detail: String,
    },
    #[error("invalid render graph: {message}")]
    InvalidGraph { message: String },
    #[error(transparent)]
    Catalog(#[from] CatalogError),
    #[error(transparent)]
    Surface(#[from] SurfaceError),
    #[error(transparent)]
    Vm(#[from] VmError),
}

const AGENT_CHANNELS: [(&str, &str); 3] = [
    ("outputXyz", "inputXyz"),
    ("outputVel", "inputVel"),
    ("outputRgba", "inputRgba"),
];

// A published channel denotes a logical texture. Writing either name replaces
// its surface, including for other names bound to this same slot.
#[derive(Clone, Debug)]
struct TextureBinding {
    surface: Rc<RefCell<Surface>>,
    format: TextureFormat,
}

impl TextureBinding {
    fn new(surface: Surface, format: TextureFormat) -> Self {
        Self {
            surface: Rc::new(RefCell::new(surface)),
            format,
        }
    }
}

type Bindings = BTreeMap<String, TextureBinding>;

#[derive(Clone, Copy, Default, PartialEq)]
enum StepMode {
    #[default]
    Ordinary,
    Iterated,
    Direct,
}

#[derive(Clone, Default)]
struct StepResources {
    mode: StepMode,
    particle_targets: BTreeMap<String, ParticleTarget>,
    surfaces: BTreeMap<String, Surface>,
    bindings: Bindings,
}

struct StorageTarget<'a> {
    owner: &'a EffectDefinition,
    params: BTreeMap<String, ParamValue>,
    users: BTreeSet<String>,
    binding: Option<TextureBinding>,
}

type ChainStorage<'a> = Vec<(String, StorageTarget<'a>)>;

fn chain_storage<'a>(steps: &[RenderStep], catalog: &'a EffectCatalog) -> ChainStorage<'a> {
    let effects: Vec<_> = steps
        .iter()
        .filter_map(|step| match step {
            RenderStep::Effect {
                effect_id, params, ..
            } => catalog
                .effects
                .get(effect_id)
                .map(|effect| (effect_id, params, effect)),
            _ => None,
        })
        .collect();
    let mut storage = ChainStorage::new();
    for (id, params, effect) in &effects {
        for pass in &effect.passes {
            let Some(targets) = pass
                .execution
                .get("storageTextures")
                .and_then(serde_json::Value::as_object)
            else {
                continue;
            };
            for key in ordered_keys(pass.execution.get("storageTextureOrder"), targets.keys()) {
                let Some(name) = targets.get(key).and_then(serde_json::Value::as_str) else {
                    continue;
                };
                if !name.starts_with("global_")
                    || is_particle_state_name(name)
                    || name == "global_accum"
                {
                    continue;
                }
                let writer = StorageTarget {
                    owner: effect,
                    params: (*params).clone(),
                    users: BTreeSet::from([(*id).clone()]),
                    binding: None,
                };
                if let Some((_, target)) = storage.iter_mut().find(|(target, _)| target == name) {
                    *target = writer;
                } else {
                    storage.push((name.to_owned(), writer));
                }
            }
        }
    }
    for (name, target) in &mut storage {
        for (id, params, effect) in &effects {
            if effect.textures.contains_key(name) {
                target.owner = effect;
                target.params = (*params).clone();
            } else if !effect.passes.iter().any(|pass| {
                pass.inputs
                    .values()
                    .chain(pass.outputs.values())
                    .any(|value| value == name)
            }) && ![
                &effect.output_tex,
                &effect.output_tex3d,
                &effect.output_geo,
                &effect.output_xyz,
                &effect.output_vel,
                &effect.output_rgba,
            ]
            .into_iter()
            .any(|field| field.as_ref() == Some(name))
            {
                continue;
            }
            target.users.insert((*id).clone());
        }
    }
    storage.retain(|(_, target)| target.users.len() > 1);
    storage
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ChainBundle {
    pub image: Option<Surface>,
    pub volume: Option<Surface>,
    pub geometry: Option<Surface>,
    pub volume_size: Option<u32>,
    agents: Bindings,
    typed: bool,
    image_binding: Option<TextureBinding>,
}

pub struct CpuRenderer {
    cpu_texture_cache: CpuTextureCache,
}

impl CpuRenderer {
    pub fn new() -> Result<Self, RenderError> {
        crate::catalog::effect_catalog()?;
        crate::catalog::shader_bundle()?;
        Ok(Self {
            cpu_texture_cache: CpuTextureCache::new(64 * 1024 * 1024),
        })
    }

    pub fn with_cpu_texture_cache_byte_limit(byte_limit: usize) -> Result<Self, RenderError> {
        crate::catalog::effect_catalog()?;
        crate::catalog::shader_bundle()?;
        Ok(Self {
            cpu_texture_cache: CpuTextureCache::new(byte_limit),
        })
    }

    #[must_use]
    pub fn cpu_texture_cache_stats(&self) -> CpuTextureCacheStats {
        self.cpu_texture_cache.stats()
    }

    pub fn clear_cpu_texture_cache(&mut self) {
        self.cpu_texture_cache.clear();
    }

    pub fn dispose(&mut self) {
        self.clear_cpu_texture_cache();
    }

    /// Compile and render one DSL program without retaining named surfaces or
    /// chain state between calls.
    pub fn render(
        &mut self,
        source: &str,
        options: &RenderOptions,
    ) -> Result<RenderResult, RenderError> {
        let started = Instant::now();
        let plan = compile_dsl(source, "<dsl>")?;
        self.execute_plan(plan, options, started)
    }

    /// Render an already compiled DSL plan. This is useful for callers that
    /// cache or intentionally transform a validated plan before execution.
    pub fn render_plan(
        &mut self,
        plan: RenderPlan,
        options: &RenderOptions,
    ) -> Result<RenderResult, RenderError> {
        self.execute_plan(plan, options, Instant::now())
    }

    fn execute_plan(
        &mut self,
        plan: RenderPlan,
        options: &RenderOptions,
        started: Instant,
    ) -> Result<RenderResult, RenderError> {
        self.execute_plan_with_observer(plan, options, started, |_| {})
    }

    #[cfg(test)]
    fn render_plan_with_trace(
        &mut self,
        plan: RenderPlan,
        options: &RenderOptions,
    ) -> Result<RenderTrace, RenderError> {
        let mut particle_groups = Vec::new();
        let result = self.execute_plan_with_observer(plan, options, Instant::now(), |shared| {
            particle_groups.push(trace_particle_group(shared))
        })?;
        Ok(RenderTrace {
            result,
            particle_groups,
        })
    }

    fn execute_plan_with_observer(
        &mut self,
        plan: RenderPlan,
        options: &RenderOptions,
        started: Instant,
        mut observe_particle_group: impl FnMut(BTreeMap<String, Surface>),
    ) -> Result<RenderResult, RenderError> {
        let mut surfaces = BTreeMap::<String, Surface>::new();
        for chain in &plan.chains {
            self.execute_steps(
                &chain.steps,
                effect_catalog()?,
                &mut surfaces,
                options,
                &mut observe_particle_group,
            )?;
        }

        let surface =
            surfaces
                .remove(&plan.render_surface)
                .ok_or(RenderError::UnwrittenSurface {
                    surface: plan.render_surface,
                })?;
        Ok(RenderResult {
            surface,
            elapsed: started.elapsed(),
        })
    }

    // Runs one compiled chain and returns its final bundle. A step's
    // `inline_chains` (chain arguments such as `tex: solid(color: #f00)`) run
    // before the group that holds the step, and each writes the hidden surface
    // that the step's argument names. Upstream compiles those chains into steps
    // directly ahead of the consuming effect; they read nothing that the group
    // itself writes.
    fn execute_steps(
        &mut self,
        steps: &[RenderStep],
        catalog: &EffectCatalog,
        surfaces: &mut BTreeMap<String, Surface>,
        options: &RenderOptions,
        observe_particle_group: &mut dyn FnMut(BTreeMap<String, Surface>),
    ) -> Result<ChainBundle, RenderError> {
        // Current image/volume/geometry state is deliberately local to a
        // chain. Only an explicit write/read crosses a chain boundary.
        let mut bundle = ChainBundle::default();
        // The chain-scoped loop accumulator carries a loop region's feedback
        // surface to the chain's next loop region, matching upstream's
        // chain-scoped shared globals. Each inline chain argument is its own
        // chain scope (a fresh accumulator inside the recursive call).
        let mut chain_accum = Option::<Surface>::None;
        let mut storage = chain_storage(steps, catalog);
        let grouping_steps = steps
            .iter()
            .map(|step| match step {
                RenderStep::Read { .. } => IterationStep::Read,
                RenderStep::Write { .. } => IterationStep::Write,
                RenderStep::Read3d { .. } => IterationStep::Read3d,
                RenderStep::Write3d { .. } => IterationStep::Write3d,
                RenderStep::Effect {
                    effect_id, params, ..
                } => IterationStep::Effect {
                    effect_id: effect_id.clone(),
                    params: params.clone(),
                },
            })
            .collect::<Vec<_>>();
        let groups = compute_groups_with_catalog(&grouping_steps, catalog).map_err(|error| {
            RenderError::InvalidGraph {
                message: error.to_string(),
            }
        })?;
        let mut step_offset = 0;
        for group in groups {
            let step_count = group.steps.len();
            let group_steps = &steps[step_offset..step_offset + step_count];
            step_offset += step_count;
            let only = if group_steps.len() == 1 {
                Some(&group_steps[0])
            } else {
                None
            };
            if let Some(step) = only {
                if matches!(
                    step,
                    RenderStep::Read { .. }
                        | RenderStep::Write { .. }
                        | RenderStep::Read3d { .. }
                        | RenderStep::Write3d { .. }
                ) {
                    bundle = run_surface_io_step(step, bundle, surfaces)?;
                    continue;
                }
            }
            for step in group_steps {
                if let RenderStep::Effect { inline_chains, .. } = step {
                    for inline in inline_chains {
                        let value = self.execute_steps(
                            &inline.chain.steps,
                            catalog,
                            surfaces,
                            options,
                            observe_particle_group,
                        )?;
                        let image = value.image.ok_or_else(|| RenderError::InvalidGraph {
                            message: format!(
                                "inline chain for {} reached execution without a current image",
                                inline.surface
                            ),
                        })?;
                        surfaces.insert(inline.surface.clone(), image);
                    }
                }
            }
            if group.iterated {
                let execution = execute_iteration_group(
                    group_steps,
                    catalog,
                    &bundle,
                    surfaces,
                    options,
                    &mut self.cpu_texture_cache,
                    chain_accum.take(),
                    &mut storage,
                )?;
                if execution
                    .shared
                    .keys()
                    .any(|name| is_particle_state_name(name))
                {
                    observe_particle_group(execution.shared);
                }
                bundle = execution.bundle;
                chain_accum = execution.chain_accum;
                continue;
            }
            for step in group_steps {
                match step {
                    RenderStep::Read { .. } | RenderStep::Write { .. } => {
                        return Err(RenderError::InvalidGraph {
                            message: "surface IO reached execution outside its own group".into(),
                        });
                    }
                    RenderStep::Read3d { .. } | RenderStep::Write3d { .. } => {
                        return Err(RenderError::InvalidGraph {
                            message: "surface IO reached execution outside its own group".into(),
                        });
                    }
                    RenderStep::Effect {
                        effect_id,
                        params,
                        surface_params,
                        explicit_params,
                        ..
                    } => {
                        let mut params = params.clone();
                        // An omitted effect seed inherits the render seed. The
                        // compiler still materializes other defaults so the
                        // execution layer receives fully normalized values.
                        if !explicit_params.iter().any(|name| name == "seed") {
                            params.remove("seed");
                        }

                        let mut inputs = BTreeMap::<String, Surface>::new();
                        if let Some(image) = &bundle.image {
                            inputs.insert("inputTex".into(), image.clone());
                        }
                        for (parameter, binding) in surface_params {
                            let input = match binding {
                                SurfaceBinding::Current => {
                                    bundle.image.clone().ok_or_else(|| {
                                        RenderError::MissingSurfaceInput {
                                            effect_id: effect_id.clone(),
                                            parameter: parameter.clone(),
                                            surface: "inputTex".into(),
                                        }
                                    })?
                                }
                                SurfaceBinding::Surface(surface) => surfaces
                                    .get(surface)
                                    .cloned()
                                    .ok_or_else(|| RenderError::MissingSurfaceInput {
                                        effect_id: effect_id.clone(),
                                        parameter: parameter.clone(),
                                        surface: surface.clone(),
                                    })?,
                            };
                            inputs.insert(parameter.clone(), input);
                        }
                        bundle = execute_effect_bundle_with_state(
                            &catalog.effects[effect_id],
                            effect_id,
                            &params,
                            &inputs,
                            &bundle,
                            options,
                            &mut StepResources::default(),
                            &mut storage,
                            surface_params,
                            &mut self.cpu_texture_cache,
                        )?;
                    }
                }
            }
        }
        Ok(bundle)
    }
}

// The built-in surface I/O steps. read3d starts a chain bundle from a stored
// volume and geometry; write3d stores the current bundle's volume and geometry
// under its two names (`none` stores nothing) and passes the bundle through, as
// upstream's `_write3d` blits and passes through.
fn run_surface_io_step(
    step: &RenderStep,
    bundle: ChainBundle,
    surfaces: &mut BTreeMap<String, Surface>,
) -> Result<ChainBundle, RenderError> {
    match step {
        RenderStep::Read { surface, .. } => {
            let image = surfaces
                .get(surface)
                .cloned()
                .ok_or(RenderError::UnwrittenSurface {
                    surface: surface.clone(),
                })?;
            Ok(ChainBundle {
                image: Some(image),
                volume: None,
                geometry: None,
                volume_size: None,
                ..ChainBundle::default()
            })
        }
        RenderStep::Write { surface, .. } => {
            let image = bundle
                .image
                .clone()
                .ok_or_else(|| RenderError::InvalidGraph {
                    message: format!("write({surface}) reached execution without a current image"),
                })?;
            surfaces.insert(surface.clone(), image);
            Ok(bundle)
        }
        RenderStep::Write3d {
            volume, geometry, ..
        } => {
            if volume != "none" {
                if let Some(volume_surface) = &bundle.volume {
                    surfaces.insert(volume.clone(), volume_surface.clone());
                }
            }
            if geometry != "none" {
                if let Some(geometry_surface) = &bundle.geometry {
                    surfaces.insert(geometry.clone(), geometry_surface.clone());
                }
            }
            Ok(bundle)
        }
        RenderStep::Read3d {
            volume, geometry, ..
        } => {
            let volume_surface =
                surfaces
                    .get(volume)
                    .cloned()
                    .ok_or(RenderError::UnwrittenSurface {
                        surface: volume.clone(),
                    })?;
            let volume_size = volume_surface.width();
            Ok(ChainBundle {
                image: None,
                volume: Some(volume_surface),
                geometry: surfaces.get(geometry).cloned(),
                volume_size: Some(volume_size),
                typed: true,
                ..ChainBundle::default()
            })
        }
        RenderStep::Effect { .. } => Err(RenderError::InvalidGraph {
            message: "effect step reached the surface IO runner".into(),
        }),
    }
}

pub fn render_effect(
    effect_id: &str,
    params: &BTreeMap<String, ParamValue>,
    inputs: &BTreeMap<String, Surface>,
    options: &RenderOptions,
) -> Result<Surface, RenderError> {
    let input_bundle = ChainBundle {
        image: inputs.get("inputTex").cloned(),
        volume: inputs.get("inputTex3d").cloned(),
        geometry: inputs.get("inputGeo").cloned(),
        volume_size: inputs.get("inputTex3d").map(Surface::width),
        agents: AGENT_CHANNELS
            .iter()
            .filter_map(|(_, name)| {
                inputs.get(*name).map(|surface| {
                    (
                        (*name).to_owned(),
                        TextureBinding::new(surface.clone(), TextureFormat::default()),
                    )
                })
            })
            .collect(),
        image_binding: None,
        typed: inputs.contains_key("inputTex3d") || inputs.contains_key("inputGeo"),
    };
    execute_effect_bundle(effect_id, params, inputs, &input_bundle, options)?
        .image
        .ok_or_else(|| RenderError::MissingOutput {
            effect_id: effect_id.into(),
            output: "outputTex".into(),
        })
}

fn execute_effect_bundle(
    effect_id: &str,
    params: &BTreeMap<String, ParamValue>,
    inputs: &BTreeMap<String, Surface>,
    input_bundle: &ChainBundle,
    options: &RenderOptions,
) -> Result<ChainBundle, RenderError> {
    let mut cache = CpuTextureCache::new(64 * 1024 * 1024);
    let catalog = effect_catalog()?;
    let effect = catalog
        .effects
        .get(effect_id)
        .ok_or_else(|| RenderError::UnknownEffect {
            effect_id: effect_id.into(),
        })?;
    execute_effect_bundle_with_state(
        effect,
        effect_id,
        params,
        inputs,
        input_bundle,
        options,
        &mut StepResources {
            mode: StepMode::Direct,
            ..StepResources::default()
        },
        &mut ChainStorage::new(),
        &BTreeMap::new(),
        &mut cache,
    )
}

#[allow(clippy::too_many_arguments)]
fn execute_effect_bundle_with_state(
    effect: &EffectDefinition,
    effect_id: &str,
    params: &BTreeMap<String, ParamValue>,
    inputs: &BTreeMap<String, Surface>,
    input_bundle: &ChainBundle,
    options: &RenderOptions,
    persistent: &mut StepResources,
    storage: &mut ChainStorage,
    surface_params: &BTreeMap<String, SurfaceBinding>,
    cache: &mut CpuTextureCache,
) -> Result<ChainBundle, RenderError> {
    let params = resolve_effect_automation(effect, params, options.time);
    let normalizer = if persistent.mode == StepMode::Direct {
        normalize_parameters
    } else {
        crate::param::normalize_graph_parameters
    };
    let mut normalized = normalizer(effect, &params, options.seed)?;
    // Upstream binds a surface parameter's declared `colorModeUniform` to 1
    // when the parameter carries a surface and 0 when it resolves to none
    // (buildBindings). Surface parameters carry no numeric uniform of their
    // own, so the flag derives from the resolved input bindings.
    for (name, spec) in &effect.params {
        if let Some(serde_json::Value::String(mode_uniform)) = spec.metadata.get("colorModeUniform")
        {
            let active = inputs.contains_key(name);
            normalized.uniforms.insert(
                mode_uniform.clone(),
                Value::Float(if active { 1.0 } else { 0.0 }),
            );
        }
    }
    inherit_volume_size(
        effect_id,
        effect,
        &mut normalized,
        input_bundle,
        persistent.mode,
    )?;
    if effect_id == "synth/remap" {
        normalized.uniforms.insert(
            "data".into(),
            remap_uniform_data(&normalized.values, inputs, options.width, options.height),
        );
    }
    bind_external_inputs(
        effect_id,
        &mut normalized.uniforms,
        &options.external_inputs,
    )?;
    let effective_seed = match normalized.values.get("seed") {
        Some(Value::Int(seed)) => *seed,
        Some(Value::Float(seed)) => *seed as i32,
        _ => options.seed,
    };
    let uniforms = canonical_uniforms(
        options.width,
        options.height,
        options.time,
        effective_seed,
        options.frame,
        options.delta_time,
        &normalized.uniforms,
    );
    let mut resources = std::mem::take(&mut persistent.surfaces);
    let bindings = &mut persistent.bindings;
    if persistent.mode == StepMode::Direct {
        resources.extend(inputs.clone());
    }
    // A surface parameter that resolved to none still binds: upstream's
    // buildBindings substitutes its 1x1 `emptySurface` for a null surface value
    // (`if (value === null) surface = this.emptySurface`), so every pass input
    // referencing the parameter's texture name finds a blank surface.
    for (name, spec) in &effect.params {
        if spec.parameter_type == "surface" && !inputs.contains_key(name) {
            resources.insert(
                spec.metadata
                    .get("texture")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(name)
                    .to_string(),
                Surface::new(1, 1)?,
            );
        }
    }
    if persistent.mode == StepMode::Direct {
        resources.extend(options.external_textures.clone());
    } else if let Some(name) = &effect.external_texture {
        let texture =
            options
                .external_textures
                .get(name)
                .ok_or_else(|| RenderError::InvalidGraph {
                    message: format!("{effect_id} requires external texture {name:?}"),
                })?;
        let mut external_view = texture.clone();
        external_view.set_filter_mode(FilterMode::Linear);
        resources.insert(name.clone(), external_view);
    }
    bind_external_input_resources(effect_id, effect, &options.external_inputs, &mut resources)?;
    if let Some(image) = &input_bundle.image {
        resources.insert("inputTex".into(), image.clone());
    }
    if let Some(volume) = &input_bundle.volume {
        resources.insert("inputTex3d".into(), volume.clone());
    }
    if let Some(geometry) = &input_bundle.geometry {
        resources.insert("inputGeo".into(), geometry.clone());
    }
    // Refresh ephemeral input names every tick, especially when a new emitter
    // opens a private particle scope. A named surface argument is a snapshot.
    bindings.remove("inputTex");
    if let Some(binding) = &input_bundle.image_binding {
        bindings.insert("inputTex".into(), binding.clone());
    }
    for name in surface_params.keys() {
        let texture = effect
            .params
            .get(name)
            .and_then(|spec| spec.metadata.get("texture"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or(name);
        if let Some(surface) = inputs.get(name) {
            resources.insert(texture.to_owned(), surface.clone());
        }
        bindings.remove(texture);
        if inputs
            .get(name)
            .zip(input_bundle.image.as_ref())
            .is_some_and(|(surface, image)| surface.same_image(image))
        {
            if let Some(binding) = &input_bundle.image_binding {
                bindings.insert(texture.to_owned(), binding.clone());
            }
        }
    }
    for (_, name) in AGENT_CHANNELS {
        bindings.remove(name);
        resources.remove(name);
        if !effect.textures.contains_key("global_xyz") {
            if let Some(binding) = input_bundle.agents.get(name) {
                bindings.insert(name.into(), binding.clone());
            }
        }
    }
    for (name, binding) in bindings.iter() {
        resources.insert(name.clone(), binding.surface.borrow().clone());
    }
    for (name, target) in storage
        .iter_mut()
        .filter(|(_, target)| target.users.contains(effect_id))
    {
        if target.binding.is_none() {
            let values;
            let (definition, parameters) = if std::ptr::eq(target.owner, effect) {
                (effect, &normalized.values)
            } else {
                let owner = target.owner;
                let params = resolve_effect_automation(owner, &target.params, options.time);
                values = crate::param::normalize_graph_parameters(owner, &params, options.seed)?;
                (owner, &values.values)
            };
            let spec = definition
                .textures
                .get(name)
                .unwrap_or(&serde_json::Value::Null);
            let (width, height) =
                texture_dimensions(spec, parameters, options.width, options.height, &resources)?;
            let format = spec
                .get("format")
                .and_then(serde_json::Value::as_str)
                .map(texture_format)
                .transpose()?
                .unwrap_or_default();
            target.binding = Some(TextureBinding::new(Surface::new(width, height)?, format));
        }
        let binding = target.binding.as_ref().unwrap();
        resources.insert(name.clone(), binding.surface.borrow().clone());
        bindings.insert(name.clone(), binding.clone());
    }
    let reserved_feedback_names = effect
        .passes
        .iter()
        .flat_map(|pass| pass.inputs.values())
        .filter(|name| matches!(name.as_str(), "selfTex" | "feedback"))
        .cloned()
        .collect::<Vec<_>>();
    let expected_feedback_dimensions = effect
        .textures
        .get("outputTex")
        .map(|spec| {
            texture_dimensions(
                spec,
                &normalized.values,
                options.width,
                options.height,
                &resources,
            )
        })
        .transpose()?
        .unwrap_or((options.width, options.height));
    ensure_reserved_feedback(
        &reserved_feedback_names,
        expected_feedback_dimensions,
        &mut resources,
    )?;
    seed_typed_resources(
        effect_id,
        effect,
        &normalized.values,
        input_bundle,
        &mut resources,
        options,
    )?;
    let produced: BTreeSet<_> = effect
        .passes
        .iter()
        .flat_map(|pass| pass.outputs.values())
        .collect();
    let consumed: BTreeSet<_> = effect
        .passes
        .iter()
        .flat_map(|pass| pass.inputs.values())
        .collect();
    let empty_resources = BTreeMap::new();
    for name in effect.texture_order.iter().chain(
        effect
            .textures
            .keys()
            .filter(|name| !effect.texture_order.contains(name)),
    ) {
        if persistent.mode == StepMode::Ordinary
            && produced.contains(name)
            && !(name.starts_with('_') && consumed.contains(name))
        {
            continue;
        }
        if persistent.mode == StepMode::Iterated && is_particle_state_name(name) {
            continue;
        }
        let spec = &effect.textures[name];
        if !resources.contains_key(name) {
            let (width, height) = texture_dimensions(
                spec,
                &normalized.values,
                options.width,
                options.height,
                if persistent.mode == StepMode::Iterated {
                    &empty_resources
                } else {
                    &resources
                },
            )?;
            let surface = if name == "overlayTex"
                && matches!(
                    effect_id,
                    "filter/fibers" | "filter/scratches" | "filter/strayHair"
                ) {
                if options.one_shot == OneShot::Initial {
                    Surface::new(width, height)?
                } else {
                    let density = match normalized.values.get("density") {
                        Some(Value::Float(value)) => *value as f32,
                        Some(Value::Int(value)) => *value as f32,
                        _ => 0.0,
                    };
                    let key = format!("{effect_id}:{width}x{height}:{effective_seed}:{density}");
                    if let Some(surface) = cache.get(&key) {
                        surface
                    } else {
                        let surface = crate::overlay::render_overlay(
                            effect_id,
                            width,
                            height,
                            effective_seed,
                            density,
                        )?;
                        cache.insert(key, surface.clone());
                        surface
                    }
                }
            } else {
                Surface::new(width, height)?
            };
            resources.insert(name.clone(), surface);
        }
    }
    let mut formats = effect
        .textures
        .iter()
        .map(|(name, spec)| {
            let format = spec
                .get("format")
                .and_then(serde_json::Value::as_str)
                .map(texture_format)
                .transpose()?
                .unwrap_or_default();
            Ok((name.clone(), format))
        })
        .collect::<Result<BTreeMap<_, _>, RenderError>>()?;
    for name in effect.passes.iter().flat_map(|pass| {
        pass.inputs
            .values()
            .chain(pass.outputs.values())
            .map(String::as_str)
    }) {
        if let Some(format) = particle_texture_format(name) {
            formats.entry(name.into()).or_insert(format);
        }
    }
    formats.extend(
        bindings
            .iter()
            .map(|(name, binding)| (name.clone(), binding.format)),
    );
    formats.extend(
        persistent
            .particle_targets
            .iter()
            .map(|(name, target)| (name.clone(), target.format)),
    );
    let shaders = shader_bundle()?;
    let mut last_output = None;
    for render_pass in &effect.passes {
        let mut alias_sources = uniforms.clone();
        alias_sources.extend(normalized.uniforms.clone());
        alias_sources.extend(normalized.values.clone());
        let empty_aliases = serde_json::Map::new();
        let aliases = render_pass
            .execution
            .get("uniforms")
            .and_then(serde_json::Value::as_object)
            .unwrap_or(&empty_aliases);
        let aliases = aliases
            .iter()
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect::<BTreeMap<_, _>>();
        let mut pass_uniforms = apply_pass_uniform_aliases(&uniforms, &alias_sources, &aliases);
        if let Some(defines) = render_pass
            .execution
            .get("defines")
            .and_then(serde_json::Value::as_object)
        {
            for (name, value) in defines {
                if let Some(value) = crate::pass_runner::json_to_value(value) {
                    pass_uniforms.insert(name.clone(), value);
                }
            }
        }
        let gate_uniforms = if persistent.mode == StepMode::Direct {
            &pass_uniforms
        } else {
            &normalized.uniforms
        };
        if !pass_enabled(
            render_pass
                .execution
                .get("conditions")
                .unwrap_or(&serde_json::Value::Null),
            gate_uniforms,
        ) {
            continue;
        }
        let repeats = repeat_count(
            render_pass
                .execution
                .get("repeat")
                .unwrap_or(&serde_json::Value::Null),
            gate_uniforms,
        );
        if repeats == 0 {
            continue;
        }
        if persistent.mode != StepMode::Direct {
            for key in ordered_keys(
                render_pass.execution.get("inputOrder"),
                render_pass.inputs.keys(),
            ) {
                if let Some(name) = render_pass.inputs.get(key) {
                    if crate::pass_runner::resource_surface(&resources, name).is_none() {
                        return Err(RenderError::InvalidGraph {
                            message: format!(
                                "{effect_id} pass {:?} requires texture {name:?}",
                                render_pass.name
                            ),
                        });
                    }
                }
            }
        }
        // The CPU allocates each write from the declaration and the per-axis
        // viewport. Only an explicitly handed-on or group-owned texture keeps
        // its allocation extent. Retain the old pixels separately for blending.
        let target_size = |name: &str,
                           resources: &BTreeMap<String, Surface>|
         -> Result<Option<(u32, u32)>, RenderError> {
            if let Some(target) = persistent.particle_targets.get(name) {
                return Ok(Some(target.size));
            }
            if bindings.contains_key(name) {
                return Ok(resources
                    .get(name)
                    .map(|surface| (surface.width(), surface.height())));
            }
            if persistent.mode == StepMode::Direct {
                return Ok(None);
            }
            crate::pass_runner::declared_attachment_dimensions(
                effect.textures.get(name),
                render_pass.execution.get("viewport"),
                &normalized.values,
                options.width,
                options.height,
                resources,
            )
            .map(Some)
        };
        if crate::draw_ops::is_draw_pass(render_pass) {
            let key = format!("{effect_id}:{}", render_pass.program);
            for _ in 0..repeats {
                let dimensions = first_output(render_pass)
                    .map(|name| target_size(name, &resources))
                    .transpose()?
                    .flatten();
                crate::draw_ops::execute_draw_pass_at_size(
                    &key,
                    render_pass,
                    &pass_uniforms,
                    &mut resources,
                    options.width,
                    options.height,
                    &formats,
                    &options.external_inputs,
                    dimensions,
                    persistent.mode != StepMode::Direct,
                )?;
                if let Some(name) = first_output(render_pass) {
                    refresh_written_bindings(std::iter::once(name), bindings, &mut resources);
                }
            }
            if let Some(name) = first_output(render_pass) {
                last_output = Some(name.clone());
            }
            continue;
        }
        let key = render_pass
            .key
            .as_ref()
            .ok_or_else(|| RenderError::MissingProgram {
                pass: render_pass.name.clone(),
                program: render_pass.program.clone(),
            })?;
        let program = shaders
            .programs
            .get(key)
            .ok_or_else(|| RenderError::MissingProgram {
                pass: render_pass.name.clone(),
                program: key.clone(),
            })?;
        let derivatives = uses_derivatives(&program.ir);
        let graph = persistent.mode != StepMode::Direct;
        let attachments = if !crate::pass_runner::uses_mrt(&program.ir, render_pass, graph) {
            first_output(render_pass).into_iter().collect::<Vec<_>>()
        } else {
            program
                .ir
                .outputs
                .iter()
                .filter_map(|name| render_pass.outputs.get(name))
                .collect()
        };
        for _ in 0..repeats {
            let mut dimensions = BTreeMap::new();
            for name in &attachments {
                if let Some(size) = target_size(name, &resources)? {
                    dimensions.insert((*name).clone(), size);
                }
            }
            execute_pass_with_bindings(
                &program.ir,
                render_pass,
                &pass_uniforms,
                &mut resources,
                options.width,
                options.height,
                &formats,
                effect.external_texture.as_deref(),
                derivatives,
                &dimensions,
                graph,
            )?;
            refresh_written_bindings(attachments.iter().copied(), bindings, &mut resources);
        }
        if let Some(name) = attachments.last() {
            last_output = Some((*name).clone());
        }
    }
    let volume_domain = matches!(
        effect.domain.as_str(),
        "volume-generator" | "volume-filter" | "volume-renderer"
    );
    let image_name = output_texture_name(
        effect,
        "outputTex",
        effect.output_tex.as_deref(),
        &resources,
    );
    let mut agents = Bindings::new();
    for ((field, input_name), configured) in AGENT_CHANNELS.into_iter().zip([
        effect.output_xyz.as_deref(),
        effect.output_vel.as_deref(),
        effect.output_rgba.as_deref(),
    ]) {
        let name = output_texture_name(effect, field, configured, &resources).unwrap_or(input_name);
        if let Some(surface) = resources.get(name) {
            let binding = bindings.entry(name.into()).or_insert_with(|| {
                TextureBinding::new(
                    surface.clone(),
                    formats.get(name).copied().unwrap_or_default(),
                )
            });
            agents.insert(input_name.into(), binding.clone());
        }
    }
    let (image, mut image_binding) = match image_name {
        Some("inputGeo" | "inputTex3d") => (
            input_bundle.image.clone(),
            input_bundle.image_binding.clone(),
        ),
        Some(name) => (resources.get(name).cloned(), bindings.get(name).cloned()),
        None if volume_domain || effect.output_tex.is_some() => (
            input_bundle.image.clone(),
            input_bundle.image_binding.clone(),
        ),
        // The implicit last attachment is its physical snapshot. A subsequent
        // write through a shared target cannot change that snapshot.
        None => (
            last_output
                .as_ref()
                .and_then(|name| resources.get(name))
                .cloned(),
            None,
        ),
    };
    if let (Some(image), None) = (&image, &image_binding) {
        image_binding = agents
            .values()
            .find(|binding| binding.surface.borrow().same_image(image))
            .cloned();
    }
    let typed =
        input_bundle.typed || volume_domain || !agents.is_empty() || image_binding.is_some();
    let volume = bundle_output(
        effect.output_tex3d.as_deref(),
        input_bundle.volume.as_ref(),
        &resources,
    );
    if let Some(output) = image.as_ref() {
        copy_reserved_feedback(effect_id, &reserved_feedback_names, output, &mut resources)?;
    }
    let geometry = bundle_output(
        effect.output_geo.as_deref(),
        input_bundle.geometry.as_ref(),
        &resources,
    );
    let normalized_volume_size = normalized
        .values
        .get("volumeSize")
        .and_then(value_as_positive_u32);
    let volume_size = if effect.domain == "volume-generator" {
        normalized_volume_size.or_else(|| volume.as_ref().map(Surface::width))
    } else {
        input_bundle
            .volume_size
            .or(normalized_volume_size)
            .or_else(|| volume.as_ref().map(Surface::width))
    };
    if matches!(effect.domain.as_str(), "volume-generator" | "volume-filter") {
        let volume = volume.as_ref().ok_or_else(|| RenderError::MissingOutput {
            effect_id: effect_id.into(),
            output: "outputTex3d".into(),
        })?;
        validate_volume_atlas(
            effect_id,
            volume,
            volume_size.unwrap_or(volume.width()),
            false,
        )?;
    } else if image.is_none() && !matches!(effect.domain.as_str(), "loop-begin" | "loop-end") {
        return Err(RenderError::MissingOutput {
            effect_id: effect_id.into(),
            output: "outputTex".into(),
        });
    }
    // The oracle's float surfaces may carry non-finite pixels (the canonical
    // zero-sum convolution kernels divide by a zero kernel weight). The CPU
    // renderer keeps those floats and maps them to 0 only at RGBA8 encode
    // (Surface byteFromFloat), so mirror that contract instead of rejecting
    // the surface: any downstream pass consumes the same NaN floats the
    // oracle's next pass would see.
    persistent.surfaces = resources;
    Ok(ChainBundle {
        image,
        image_binding,
        agents,
        typed,
        volume: if typed { volume } else { None },
        geometry: if typed { geometry } else { None },
        volume_size: if typed { volume_size } else { None },
    })
}

/// Reactive (MIDI/audio) uniform defaults and mesh/external data-texture bindings.
/// Mirrors the upstream pipeline's global-uniform stage (updateGlobalUniforms): the
/// 128-float audio arrays and the MIDI clock counter are bound only for the effects
/// whose kernels declare them, zero-initialized when no external state is supplied.
const REACTIVE_EFFECT_IDS: [&str; 3] = ["synth/roll", "synth/scope", "synth/spectrum"];
const MESH_TEX_WIDTH: usize = 256;
const MESH_TEX_HEIGHT: usize = 256;

fn bind_external_inputs(
    effect_id: &str,
    uniforms: &mut BTreeMap<String, Value>,
    external_inputs: &ExternalInputs,
) -> Result<(), RenderError> {
    if REACTIVE_EFFECT_IDS.contains(&effect_id) {
        let clock_count = external_inputs
            .midi_state
            .as_ref()
            .map_or(0.0_f64, |midi| f64::from(midi.clock_count));
        uniforms.insert("midiClockCount".into(), Value::Float(clock_count));
        // GLSL uniform arrays are zero-initialized when the upstream pipeline has no
        // external state; the CPU kernels index them unconditionally, so always bind
        // 128-float arrays (zeros when no audio state is supplied).
        let zero = vec![Value::Float(0.0); 128];
        let audio = external_inputs.audio_state.as_ref();
        uniforms.insert(
            "audioWaveform".into(),
            audio.map_or_else(
                || Value::Array(zero.clone()),
                |state| {
                    Value::Array(
                        state
                            .waveform
                            .iter()
                            .map(|&v| Value::Float(f64::from(v)))
                            .collect(),
                    )
                },
            ),
        );
        uniforms.insert(
            "audioSpectrum".into(),
            audio.map_or_else(
                || Value::Array(zero),
                |state| {
                    Value::Array(
                        state
                            .spectrum
                            .iter()
                            .map(|&v| Value::Float(f64::from(v)))
                            .collect(),
                    )
                },
            ),
        );
    }
    Ok(())
}

fn bind_external_input_resources(
    effect_id: &str,
    effect: &EffectDefinition,
    external_inputs: &ExternalInputs,
    resources: &mut BTreeMap<String, Surface>,
) -> Result<(), RenderError> {
    let pass_input_names: std::collections::BTreeSet<&str> = effect
        .passes
        .iter()
        .flat_map(|pass| pass.inputs.values())
        .map(String::as_str)
        .collect();
    if REACTIVE_EFFECT_IDS.contains(&effect_id) && pass_input_names.contains("midiNoteGrid") {
        let grid: Vec<f32> = external_inputs
            .midi_state
            .as_ref()
            .map_or_else(|| vec![0.0; 128 * 16 * 4], |midi| midi.note_grid.clone());
        let mut surface =
            Surface::from_f32(128, 16, flip_rgba_rows(&grid, 128, 16)).map_err(|error| {
                RenderError::InvalidGraph {
                    message: format!("midiNoteGrid data texture rejected: {error}"),
                }
            })?;
        surface.set_filter_mode(FilterMode::Nearest);
        resources.insert("midiNoteGrid".into(), surface);
    }
    let mesh_names: Vec<&str> = pass_input_names
        .iter()
        .copied()
        .filter(|name| name.starts_with("global_mesh0_"))
        .collect();
    if mesh_names.is_empty() {
        return Ok(());
    }
    // With no mesh_data, bind the zero mesh upstream samples before an upload.
    let mesh_data: &MeshData = if let Some(mesh_data) = &external_inputs.mesh_data {
        mesh_data
    } else {
        // The zero mesh is constant; a lazily shared instance avoids rebuilding
        // the 256x256 buffers on every pass.
        static EMPTY: std::sync::OnceLock<MeshData> = std::sync::OnceLock::new();
        EMPTY.get_or_init(empty_mesh_data)
    };
    let tex_width = if mesh_data.tex_width == 0 {
        MESH_TEX_WIDTH
    } else {
        mesh_data.tex_width
    };
    let tex_height = if mesh_data.tex_height == 0 {
        MESH_TEX_HEIGHT
    } else {
        mesh_data.tex_height
    };
    for (name, data) in [
        ("global_mesh0_positions", &mesh_data.position_data),
        ("global_mesh0_normals", &mesh_data.normal_data),
        ("global_mesh0_uvs", &mesh_data.uv_data),
    ] {
        if mesh_names.contains(&name) {
            let mut surface = Surface::from_f32(
                tex_width as u32,
                tex_height as u32,
                flip_rgba_rows(data, tex_width, tex_height),
            )
            .map_err(|error| RenderError::InvalidGraph {
                message: format!("{name} data texture rejected: {error}"),
            })?;
            surface.set_filter_mode(FilterMode::Nearest);
            resources.insert(name.into(), surface);
        }
    }
    Ok(())
}

fn remap_uniform_data(
    values: &BTreeMap<String, Value>,
    inputs: &BTreeMap<String, Surface>,
    width: u32,
    height: u32,
) -> Value {
    // Reference synth/remap packs zone metadata at 2..9, vertex pairs at
    // 10..265, resolution at 266, and zone bounds at 267..274.
    let mut data = vec![Value::Vec(vec![0.0; 4]); 267];
    data.resize(275, Value::Vec(vec![0.0, 0.0, 1.0, 1.0]));
    let background = match values.get("bgColor") {
        Some(Value::Vec(color)) => color.clone(),
        _ => vec![0.0; 3],
    };
    let alpha = match values.get("bgAlpha") {
        Some(Value::Float(value)) => *value as f32,
        Some(Value::Int(value)) => *value as f32,
        _ => 1.0,
    };
    data[0] = Value::Vec(vec![
        *background.first().unwrap_or(&0.0),
        *background.get(1).unwrap_or(&0.0),
        *background.get(2).unwrap_or(&0.0),
        alpha,
    ]);
    let zone_count = match values.get("zoneCount") {
        Some(Value::Int(value)) => *value as f32,
        _ => 0.0,
    };
    let smooth_edge = match values.get("smoothEdge") {
        Some(Value::Float(value)) => *value as f32,
        _ => 0.04,
    };
    data[1] = Value::Vec(vec![zone_count, smooth_edge, 0.0, 0.0]);
    for zone in 0..8 {
        let active = inputs.contains_key(&format!("zone{zone}_tex")) as u8 as f32;
        let count = match values.get(&format!("zone{zone}_count")) {
            Some(Value::Int(value)) => *value as f32,
            _ => 0.0,
        };
        let alpha = match values.get(&format!("zone{zone}_alpha")) {
            Some(Value::Float(value)) => *value as f32,
            _ => 1.0,
        };
        data[2 + zone] = Value::Vec(vec![count, active, 0.0, alpha]);
        for pair in 0..32 {
            if let Some(Value::Vec(vertices)) = values.get(&format!("zone{zone}_v{pair}")) {
                data[10 + zone * 32 + pair] = Value::Vec(vertices.clone());
            }
        }
        if let Some(Value::Vec(bounds)) = values.get(&format!("zone{zone}_bounds")) {
            data[267 + zone] = Value::Vec(bounds.clone());
        }
    }
    data[266] = Value::Vec(vec![width as f32, height as f32, 0.0, 0.0]);
    Value::Array(data)
}

struct IterationExecution {
    bundle: ChainBundle,
    shared: BTreeMap<String, Surface>,
    // The chain-scoped loop accumulator a group hands back to the chain.
    chain_accum: Option<Surface>,
}

#[allow(clippy::too_many_arguments)]
fn execute_iteration_group(
    steps: &[RenderStep],
    catalog: &EffectCatalog,
    group_input: &ChainBundle,
    surfaces: &BTreeMap<String, Surface>,
    options: &RenderOptions,
    cache: &mut CpuTextureCache,
    mut chain_accum: Option<Surface>,
    storage: &mut ChainStorage,
) -> Result<IterationExecution, RenderError> {
    let Some(RenderStep::Effect {
        effect_id: first_id,
        params: first_params,
        ..
    }) = steps.first()
    else {
        return Err(RenderError::InvalidGraph {
            message: "iterated group must begin with an effect".into(),
        });
    };
    // The CPU renderer picks the group's iteration-count owner as the first step
    // whose definition is iterated: a joined group (animated noise3d feeding
    // flow3d) is owned by its later iterated step, not by steps[0].
    let owner_index = steps
        .iter()
        .position(|step| {
            matches!(
                step,
                RenderStep::Effect { effect_id, .. }
                    if catalog
                        .effects
                        .get(effect_id)
                        .is_some_and(|definition| definition.iterated)
            )
        })
        .unwrap_or(0);
    let Some(RenderStep::Effect {
        effect_id: owner_id,
        params: owner_params,
        ..
    }) = steps.get(owner_index)
    else {
        return Err(RenderError::InvalidGraph {
            message: "iterated group must begin with an effect".into(),
        });
    };
    let requested_count = match owner_params.get("iterationCount") {
        Some(ParamValue::Int(count)) => *count,
        _ => 60,
    };
    // A pass with its own repeat count already performs the requested work in
    // one frame. Do not multiply it by the enclosing group's iteration count.
    let count = if catalog.effects[owner_id]
        .passes
        .iter()
        .any(|pass| pass.execution.contains_key("repeat"))
    {
        requested_count.min(1)
    } else {
        requested_count
    };
    if count <= 0 {
        return Ok(IterationExecution {
            bundle: zero_iteration_bundle(
                &catalog.effects[first_id],
                first_params,
                group_input,
                options,
            )?,
            shared: BTreeMap::new(),
            // A zero-iteration group never touches the loop accumulator, so it
            // stays in the chain scope untouched.
            chain_accum,
        });
    }
    let count = u32::try_from(count).map_err(|_| RenderError::InvalidParameter {
        message: "iterationCount cannot be represented as u32".into(),
    })?;
    // Only multi-step groups inherit the lead step's stateSize (the CPU
    // renderer's groupOwnerStateSize returns undefined for single-step groups).
    // A particle group's owner is its pointsEmit step, which an animated input
    // generator may precede.
    let owner_state_size = if steps.len() > 1 {
        match steps.iter().find(|step| {
            matches!(step, RenderStep::Effect { effect_id, .. }
                if catalog.effects[effect_id].textures.contains_key("global_xyz"))
        }) {
            Some(RenderStep::Effect { params, .. }) => params.get("stateSize").cloned(),
            Some(_) => None,
            None => first_params.get("stateSize").cloned(),
        }
    } else {
        None
    };
    let mut states = vec![
        StepResources {
            mode: StepMode::Iterated,
            ..StepResources::default()
        };
        steps.len()
    ];
    let mut shared_bindings = Bindings::new();
    let mut shared = BTreeMap::<String, Surface>::new();
    let mut declared_particle_textures = BTreeMap::new();
    for (index, step) in steps.iter().enumerate() {
        let RenderStep::Effect {
            effect_id, params, ..
        } = step
        else {
            continue;
        };
        let definition = &catalog.effects[effect_id];
        if !definition
            .textures
            .keys()
            .any(|name| is_particle_state_name(name))
        {
            continue;
        }
        let mut params = params.clone();
        inherit_group_state_size(index, definition, owner_state_size.as_ref(), &mut params);
        let params = resolve_effect_automation(definition, &params, options.time);
        let normalized =
            crate::param::normalize_graph_parameters(definition, &params, options.seed)?;
        for (name, spec) in &definition.textures {
            if is_particle_state_name(name) && !declared_particle_textures.contains_key(name) {
                let target = particle_target(spec, &normalized.values, options)?;
                declared_particle_textures.insert(name.clone(), (index, target));
            }
        }
    }
    let loop_group = catalog.effects[owner_id].loop_role.as_deref() == Some("begin");
    if loop_group {
        let image = group_input
            .image
            .as_ref()
            .ok_or_else(|| RenderError::InvalidGraph {
                message: "loop iteration group requires an input image".into(),
            })?;
        // Upstream scopes the loop accumulator by chain (a global that different
        // effects share keeps its chain scope), so a second loop region in the
        // same chain continues the first region's feedback surface instead of a
        // zeroed one. An inherited surface of the wrong size (cannot arise in one
        // chain's image stream, but guarded) is released and re-created.
        let wrong_size = chain_accum.as_ref().is_some_and(|accum| {
            accum.width() != image.width() || accum.height() != image.height()
        });
        if wrong_size {
            chain_accum = None;
        }
        let accum = match chain_accum.take() {
            Some(accum) => accum,
            None => Surface::new(image.width(), image.height())?,
        };
        shared.insert("global_accum".into(), accum);
    }
    let mut final_output = group_input.clone();
    run_group_frames(count, options.time, |frame| {
        let mut step_input = group_input.clone();
        for (index, step) in steps.iter().enumerate() {
            let RenderStep::Effect {
                effect_id,
                params,
                surface_params,
                explicit_params,
                ..
            } = step
            else {
                return Err(RenderError::InvalidGraph {
                    message: "iteration group cannot contain read/write".into(),
                });
            };
            let definition = &catalog.effects[effect_id];
            let mut params = params.clone();
            inherit_group_state_size(index, definition, owner_state_size.as_ref(), &mut params);
            if !explicit_params.iter().any(|name| name == "seed") {
                params.remove("seed");
            }
            // Automation-driven params (`osc(...)`) re-resolve against each
            // iteration's own normalized time: the iteration loop rewinds time
            // to emulate the upstream frames the persistent-texture feedback
            // accumulated, so an automation param must read the same
            // per-iteration time the kernels receive.
            let params = resolve_effect_automation(definition, &params, frame.time);
            let normalized =
                crate::param::normalize_graph_parameters(definition, &params, options.seed)?;
            for (name, (owner, target)) in &mut declared_particle_textures {
                if *owner == index {
                    *target =
                        particle_target(&definition.textures[name], &normalized.values, options)?;
                }
            }
            states[index].particle_targets = ensure_group_particle_resources(
                definition,
                &normalized.values,
                &declared_particle_textures,
                &mut shared,
            )?;
            for (name, surface) in &shared {
                states[index].surfaces.insert(name.clone(), surface.clone());
            }
            states[index].bindings.extend(shared_bindings.clone());
            let mut inputs = BTreeMap::new();
            if let Some(image) = &step_input.image {
                inputs.insert("inputTex".into(), image.clone());
            }
            for (parameter, binding) in surface_params {
                let input = match binding {
                    SurfaceBinding::Current => step_input.image.clone().ok_or_else(|| {
                        RenderError::MissingSurfaceInput {
                            effect_id: effect_id.clone(),
                            parameter: parameter.clone(),
                            surface: "inputTex".into(),
                        }
                    })?,
                    SurfaceBinding::Surface(surface) => {
                        surfaces.get(surface).cloned().ok_or_else(|| {
                            RenderError::MissingSurfaceInput {
                                effect_id: effect_id.clone(),
                                parameter: parameter.clone(),
                                surface: surface.clone(),
                            }
                        })?
                    }
                };
                inputs.insert(parameter.clone(), input);
            }
            let iteration_options = RenderOptions {
                time: frame.time,
                frame: frame.frame,
                delta_time: frame.delta_time,
                ..options.clone()
            };
            step_input = execute_effect_bundle_with_state(
                definition,
                effect_id,
                &params,
                &inputs,
                &step_input,
                &iteration_options,
                &mut states[index],
                storage,
                surface_params,
                cache,
            )?;
            shared_bindings.extend(
                states[index]
                    .bindings
                    .iter()
                    .filter(|(name, _)| is_particle_state_name(name))
                    .map(|(name, binding)| (name.clone(), binding.clone())),
            );
            for (name, surface) in &states[index].surfaces {
                if is_particle_state_name(name) || name == "global_accum" {
                    shared.insert(name.clone(), surface.clone());
                }
            }
        }
        final_output = step_input;
        Ok(())
    })?;
    // The loop accumulator is chain-scoped: hand it to the chain's next loop
    // region instead of releasing it with the group's other scratch resources.
    // A non-loop iterated group never touches it, so the chain scope keeps it.
    let chain_accum = shared.remove("global_accum").or(chain_accum);
    Ok(IterationExecution {
        bundle: final_output,
        shared,
        chain_accum,
    })
}

fn run_group_frames(
    count: u32,
    time: f32,
    mut execute: impl FnMut(crate::iteration::IterationFrame) -> Result<(), RenderError>,
) -> Result<(), RenderError> {
    for frame in iteration_schedule(count, time) {
        execute(frame)?;
    }
    Ok(())
}

// A texture dimension spec reading the group's shared `stateSize` through the `inherit: true`
// flag (upstream's inherited texture dimension): it resolves against the scope's stateSize —
// `pointsEmit`'s value — and never the behavior's own default over it. Every particle behavior's
// `global_<behavior>_base_color` memory is sized this way, and several behaviors (flock,
// hydraulic, lenia, physarum) declare no `stateSize` global of their own.
fn declares_inherited_state_size_textures(definition: &crate::catalog::EffectDefinition) -> bool {
    definition.textures.values().any(|texture| {
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
    })
}

fn inherit_group_state_size(
    index: usize,
    definition: &crate::catalog::EffectDefinition,
    owner_state_size: Option<&ParamValue>,
    params: &mut BTreeMap<String, ParamValue>,
) {
    // Mirrors upstream's initializeGroupStepState: a joined step's stateSize is
    // forcibly overridden to the owner's normalized value whenever the step
    // declares a stateSize param at all — or when its own textures read the
    // scope's stateSize through an `inherit: true` dimension (a joining
    // behavior's base-color memory). A step with no declared stateSize that
    // still declares inherited-stateSize textures falls back to the synthetic
    // particle-state size (256) when nothing to inherit exists.
    let declares_state_size = params.contains_key("stateSize");
    let declares_inherited = declares_inherited_state_size_textures(definition);
    if index > 0 {
        if owner_state_size.is_some() && (declares_state_size || declares_inherited) {
            if let Some(size) = owner_state_size {
                params.insert("stateSize".into(), size.clone());
            }
        }
    } else if !declares_state_size && declares_inherited {
        params.insert("stateSize".into(), ParamValue::Int(256));
    }
}

fn particle_texture_format(name: &str) -> Option<TextureFormat> {
    match name {
        "global_xyz" | "global_vel" => Some(TextureFormat::Rgba32f),
        "global_rgba" => Some(TextureFormat::Rgba8),
        "global_life_data" => Some(TextureFormat::Rgba16f),
        name if is_particle_state_name(name) => Some(TextureFormat::Rgba16f),
        _ => None,
    }
}

#[derive(Clone, Copy)]
struct ParticleTarget {
    size: (u32, u32),
    format: TextureFormat,
}

fn particle_target(
    spec: &serde_json::Value,
    values: &BTreeMap<String, Value>,
    options: &RenderOptions,
) -> Result<ParticleTarget, RenderError> {
    let size = texture_dimensions(
        spec,
        values,
        options.width,
        options.height,
        &BTreeMap::new(),
    )?;
    let format = spec
        .get("format")
        .and_then(serde_json::Value::as_str)
        .map(texture_format)
        .transpose()?
        .unwrap_or_default();
    Ok(ParticleTarget { size, format })
}

fn ensure_group_particle_resources(
    effect: &EffectDefinition,
    values: &BTreeMap<String, Value>,
    declared_textures: &BTreeMap<String, (usize, ParticleTarget)>,
    shared: &mut BTreeMap<String, Surface>,
) -> Result<BTreeMap<String, ParticleTarget>, RenderError> {
    let mut targets = BTreeMap::new();
    for name in effect
        .passes
        .iter()
        .flat_map(|pass| pass.inputs.values().chain(pass.outputs.values()))
        .filter(|name| is_particle_state_name(name))
    {
        if targets.contains_key(name) {
            continue;
        }
        let target = if let Some((_, target)) = declared_textures.get(name) {
            *target
        } else {
            let size = values
                .get("stateSize")
                .and_then(value_as_positive_u32)
                .unwrap_or(256);
            ParticleTarget {
                size: (size, size),
                format: particle_texture_format(name).unwrap(),
            }
        };
        if !shared.contains_key(name) {
            shared.insert(name.clone(), Surface::new(target.size.0, target.size.1)?);
        }
        targets.insert(name.clone(), target);
    }
    Ok(targets)
}

fn copy_reserved_feedback(
    effect_id: &str,
    names: &[String],
    output: &Surface,
    resources: &mut BTreeMap<String, Surface>,
) -> Result<(), RenderError> {
    let Some(name) = names.first() else {
        return Ok(());
    };
    let feedback = &resources[RESERVED_FEEDBACK_RESOURCE];
    if feedback.width() != output.width() || feedback.height() != output.height() {
        return Err(RenderError::InvalidGraph {
            message: format!(
                "{effect_id} {name} ({}x{}) must match the step's output ({}x{})",
                feedback.width(),
                feedback.height(),
                output.width(),
                output.height()
            ),
        });
    }
    resources
        .get_mut(RESERVED_FEEDBACK_RESOURCE)
        .expect("reserved feedback resource was preallocated")
        .data_mut()
        .copy_from_slice(output.data());
    Ok(())
}

fn ensure_reserved_feedback(
    names: &[String],
    expected: (u32, u32),
    resources: &mut BTreeMap<String, Surface>,
) -> Result<(), RenderError> {
    if !names.is_empty() && !resources.contains_key(RESERVED_FEEDBACK_RESOURCE) {
        resources.insert(
            RESERVED_FEEDBACK_RESOURCE.into(),
            Surface::new(expected.0, expected.1)?,
        );
    }
    Ok(())
}

fn inherit_volume_size(
    effect_id: &str,
    effect: &crate::catalog::EffectDefinition,
    normalized: &mut crate::pass_runner::NormalizedParameters,
    input: &ChainBundle,
    mode: StepMode,
) -> Result<(), RenderError> {
    let Some(volume) = &input.volume else {
        return Ok(());
    };
    if !matches!(
        effect.domain.as_str(),
        "volume-generator" | "volume-filter" | "volume-renderer"
    ) || !normalized.values.contains_key("volumeSize")
    {
        return Ok(());
    }
    let size = volume.width();
    validate_volume_atlas(effect_id, volume, size, true)?;
    let value = Value::Int(i32::try_from(size).map_err(|_| RenderError::InvalidGraph {
        message: format!("{effect_id} volume width {size} cannot bind to volumeSize"),
    })?);
    normalized.values.insert("volumeSize".into(), value.clone());
    if let Some(spec) = effect.params.get("volumeSize") {
        if mode != StepMode::Direct {
            normalized.uniforms.insert(
                crate::param::graph_uniform_name("volumeSize", spec).to_owned(),
                value,
            );
            return Ok(());
        }
        if let Some(uniform) = &spec.uniform {
            normalized.uniforms.insert(uniform.clone(), value.clone());
        }
        if let Some(define) = &spec.define {
            normalized.uniforms.insert(define.clone(), value);
        }
    }
    Ok(())
}

fn validate_volume_atlas(
    effect_id: &str,
    volume: &Surface,
    size: u32,
    input: bool,
) -> Result<(), RenderError> {
    let height = size
        .checked_mul(size)
        .ok_or_else(|| RenderError::InvalidGraph {
            message: format!("{effect_id} volume atlas dimensions overflow for size {size}"),
        })?;
    if volume.width() != size || volume.height() != height {
        return Err(RenderError::InvalidGraph {
            message: format!(
                "{effect_id} {}volume atlas expected {size}x{height}, received {}x{}",
                if input { "input " } else { "" },
                volume.width(),
                volume.height()
            ),
        });
    }
    Ok(())
}

fn value_as_positive_u32(value: &Value) -> Option<u32> {
    let value = match value {
        Value::Int(value) => i64::from(*value),
        Value::Uint(value) => i64::from(*value),
        Value::Float(value) if value.is_finite() && value.fract() == 0.0 => *value as i64,
        _ => return None,
    };
    u32::try_from(value).ok().filter(|value| *value > 0)
}

fn ordered_keys<'a>(
    order: Option<&'a serde_json::Value>,
    keys: impl Iterator<Item = &'a String> + 'a,
) -> impl Iterator<Item = &'a str> {
    let order = order
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    order.iter().filter_map(serde_json::Value::as_str).chain(
        keys.map(String::as_str)
            .filter(move |key| !order.iter().any(|item| item.as_str() == Some(key))),
    )
}

fn output_texture_name<'a>(
    effect: &EffectDefinition,
    field: &'a str,
    configured: Option<&'a str>,
    resources: &BTreeMap<String, Surface>,
) -> Option<&'a str> {
    if resources.contains_key(field)
        && ((field == "outputTex" && configured.is_none())
            || effect
                .passes
                .iter()
                .any(|pass| pass.outputs.values().any(|name| name == field)))
    {
        Some(field)
    } else {
        configured
    }
}

fn refresh_written_bindings<'a>(
    names: impl Iterator<Item = &'a String>,
    bindings: &Bindings,
    resources: &mut BTreeMap<String, Surface>,
) {
    if bindings.is_empty() {
        return;
    }
    // All MRT attachments were rendered against the same pre-pass contents.
    // Commit their values before refreshing aliases, in shader location order.
    let mut changed = Vec::new();
    for name in names {
        if let Some(binding) = bindings.get(name) {
            *binding.surface.borrow_mut() = resources[name].clone();
            changed.push((name, binding));
        }
    }
    for (name, binding) in bindings {
        if changed
            .iter()
            .rev()
            .find(|(_, updated)| Rc::ptr_eq(&updated.surface, &binding.surface))
            .is_some_and(|(written, _)| name != *written)
        {
            resources.insert(name.clone(), binding.surface.borrow().clone());
        }
    }
}

fn bundle_output(
    name: Option<&str>,
    input: Option<&Surface>,
    resources: &BTreeMap<String, Surface>,
) -> Option<Surface> {
    match name {
        Some("inputTex") => resources.get("inputTex").or(input).cloned(),
        None | Some("inputTex3d" | "inputGeo") => input.cloned(),
        Some(name) => resources.get(name).cloned(),
    }
}

fn allocate_declared_surface(
    effect: &crate::catalog::EffectDefinition,
    name: &str,
    values: &BTreeMap<String, Value>,
    resources: &BTreeMap<String, Surface>,
    options: &RenderOptions,
) -> Result<Surface, RenderError> {
    let spec = effect
        .textures
        .get(name)
        .ok_or_else(|| RenderError::InvalidGraph {
            message: format!("effect output {name:?} has no allocatable texture declaration"),
        })?;
    let (width, height) =
        texture_dimensions(spec, values, options.width, options.height, resources)?;
    Ok(Surface::new(width, height)?)
}

fn seed_typed_resources(
    effect_id: &str,
    effect: &crate::catalog::EffectDefinition,
    values: &BTreeMap<String, Value>,
    input: &ChainBundle,
    resources: &mut BTreeMap<String, Surface>,
    options: &RenderOptions,
) -> Result<(), RenderError> {
    for name in &effect.param_names {
        let parameter_type = effect.params[name].parameter_type.as_str();
        if !matches!(parameter_type, "volume" | "geometry") {
            continue;
        }
        let incoming = if parameter_type == "volume" {
            input.volume.as_ref()
        } else {
            input.geometry.as_ref()
        };
        if let Some(incoming) = incoming {
            resources.insert(name.clone(), incoming.clone());
            continue;
        }
        if resources.contains_key(name) {
            continue;
        }
        let output_name = if parameter_type == "volume" {
            effect.output_tex3d.as_deref()
        } else {
            effect.output_geo.as_deref()
        };
        let Some(output_name) = output_name.filter(|name| effect.textures.contains_key(*name))
        else {
            return Err(RenderError::InvalidGraph {
                message: format!(
                    "{effect_id} parameter {name:?} requires a {parameter_type} input"
                ),
            });
        };
        let starter = allocate_declared_surface(effect, output_name, values, resources, options)?;
        resources.insert(name.clone(), starter);
    }
    Ok(())
}

fn zero_iteration_bundle(
    effect: &crate::catalog::EffectDefinition,
    params: &BTreeMap<String, ParamValue>,
    input: &ChainBundle,
    options: &RenderOptions,
) -> Result<ChainBundle, RenderError> {
    if input.image.is_some() || input.volume.is_some() || input.geometry.is_some() {
        return Ok(input.clone());
    }
    if effect.domain == "volume-generator" {
        let params = resolve_effect_automation(effect, params, options.time);
        let normalized = crate::param::normalize_graph_parameters(effect, &params, options.seed)?;
        let resources = BTreeMap::new();
        let output = effect
            .output_tex3d
            .as_deref()
            .ok_or_else(|| RenderError::MissingOutput {
                effect_id: format!("{}/{}", effect.namespace, effect.func),
                output: "outputTex3d".into(),
            })?;
        let volume =
            allocate_declared_surface(effect, output, &normalized.values, &resources, options)?;
        let geometry = effect
            .output_geo
            .as_deref()
            .filter(|name| *name != "inputGeo" && effect.textures.contains_key(*name))
            .map(|name| {
                allocate_declared_surface(effect, name, &normalized.values, &resources, options)
            })
            .transpose()?;
        let volume_size = normalized
            .values
            .get("volumeSize")
            .and_then(value_as_positive_u32)
            .or(Some(volume.width()));
        validate_volume_atlas(
            &format!("{}/{}", effect.namespace, effect.func),
            &volume,
            volume_size.unwrap(),
            false,
        )?;
        return Ok(ChainBundle {
            image: None,
            volume: Some(volume),
            geometry,
            volume_size,
            typed: true,
            ..ChainBundle::default()
        });
    }
    Ok(ChainBundle {
        image: Some(Surface::new(options.width, options.height)?),
        ..ChainBundle::default()
    })
}

fn texture_format(name: &str) -> Result<TextureFormat, RenderError> {
    match name {
        "rgba8" | "rgba8unorm" => Ok(TextureFormat::Rgba8),
        "rgba16f" | "rgba16float" => Ok(TextureFormat::Rgba16f),
        "rgba32f" | "rgba32float" => Ok(TextureFormat::Rgba32f),
        _ => Err(RenderError::InvalidGraph {
            message: format!("unknown texture format {name:?}"),
        }),
    }
}

fn uses_derivatives(program: &crate::catalog::ProgramIr) -> bool {
    let mut found = false;
    for function in &program.functions {
        for statement in &function.body {
            statement.visit_expressions(&mut |expression| {
                if let Expression::Call { target, .. } = expression {
                    if matches!(
                        target.as_str(),
                        "builtin:dFdx" | "builtin:dFdy" | "builtin:fwidth"
                    ) {
                        found = true;
                    }
                }
            });
        }
    }
    found
}

#[cfg(test)]
mod cache_tests {
    use std::collections::BTreeMap;

    use super::{
        ChainBundle, CpuRenderer, CpuTextureCache, RenderError, copy_reserved_feedback,
        ensure_group_particle_resources, ensure_reserved_feedback, execute_iteration_group,
        inherit_group_state_size, particle_texture_format, remap_uniform_data, run_group_frames,
        trace_particle_group,
    };
    use crate::catalog::{EffectDefinition, effect_catalog};
    use crate::draw_ops::execute_draw_pass;
    use crate::dsl::compile_dsl;
    use crate::pass_runner::{RESERVED_FEEDBACK_RESOURCE, execute_pass, resource_surface};
    use crate::{ParamValue, RenderOptions, Surface, TextureFormat, Value};

    fn literal(value_type: &str, value: serde_json::Value) -> serde_json::Value {
        serde_json::json!({"kind":"literal","type":value_type,"value":value,"source":null})
    }

    fn inherited_state_size_definition() -> EffectDefinition {
        EffectDefinition {
            namespace: "points".into(),
            func: "test".into(),
            kind: "filter".into(),
            domain: "image".into(),
            params: Default::default(),
            param_names: Vec::new(),
            textures: serde_json::from_value(serde_json::json!({
                "global_test_base_color": {
                    "width": {"param": "stateSize", "inherit": true},
                    "height": {"param": "stateSize", "inherit": true},
                    "format": "rgba8"
                }
            }))
            .unwrap(),
            passes: Vec::new(),
            iterated: false,
            external_texture: None,
            texture_order: Vec::new(),
            output_tex: None,
            output_xyz: None,
            output_vel: None,
            output_rgba: None,
            output_tex3d: None,
            output_geo: None,
            loop_role: None,
            param_aliases: None,
        }
    }

    #[test]
    fn joined_group_steps_inherit_owner_state_size_and_first_steps_fall_back_to_256() {
        let definition = inherited_state_size_definition();
        // A joined step (index > 0) inherits the owner's normalized stateSize.
        let mut joined = BTreeMap::new();
        inherit_group_state_size(1, &definition, Some(&ParamValue::Int(64)), &mut joined);
        assert_eq!(joined.get("stateSize"), Some(&ParamValue::Int(64)));

        // The owner step (index 0) with no declared stateSize falls back to the
        // synthetic particle-state size.
        let mut owner = BTreeMap::new();
        inherit_group_state_size(0, &definition, None, &mut owner);
        assert_eq!(owner.get("stateSize"), Some(&ParamValue::Int(256)));

        // A joined step with nothing to inherit stays untouched.
        let mut unjoined = BTreeMap::new();
        inherit_group_state_size(1, &definition, None, &mut unjoined);
        assert_eq!(unjoined.get("stateSize"), None);
    }

    fn identifier(value_type: &str, name: &str, storage: &str) -> serde_json::Value {
        serde_json::json!({"kind":"identifier","type":value_type,"name":name,"storage":storage})
    }

    fn dual_feedback_program() -> crate::catalog::ProgramIr {
        let sample = |name: &str| {
            serde_json::json!({
                "kind":"call", "type":"vec4", "name":"texture", "target":"builtin:texture",
                "arguments":[
                    identifier("sampler2D", name, "uniform"),
                    {"kind":"construct", "type":"vec2", "arguments":[literal("float", serde_json::json!(0.5))]}
                ]
            })
        };
        serde_json::from_value(serde_json::json!({
            "outputs":["fragColor"], "varyings":["v_texCoord"], "structs":[],
            "uniforms":[
                {"name":"previousSelf","type":"sampler2D","initializer":null,"arraySize":null},
                {"name":"previousFeedback","type":"sampler2D","initializer":null,"arraySize":null}
            ],
            "globals":[{"name":"fragColor","type":"vec4","initializer":null,"arraySize":null}],
            "functions":[{
                "name":"main", "mangledName":"main__void", "returnType":"void", "parameters":[],
                "body":[{"kind":"expression","expression":{
                    "kind":"assignment", "type":"vec4", "operator":"=",
                    "lvalue":identifier("vec4", "fragColor", "global"),
                    "value":{"kind":"binary","type":"vec4","operator":"+","left":sample("previousSelf"),"right":sample("previousFeedback")}
                }}]
            }]
        }))
        .unwrap()
    }

    #[test]
    fn cache_hit_refreshes_lru_recency() {
        let mut cache = CpuTextureCache::new(2 * 1024);
        let surface = || Surface::new(8, 8).unwrap();
        cache.insert("a".into(), surface());
        cache.insert("b".into(), surface());
        assert!(cache.get("a").is_some());
        cache.insert("c".into(), surface());
        assert!(cache.get("a").is_some());
        assert!(cache.get("b").is_none());
        assert!(cache.get("c").is_some());
    }

    #[test]
    fn reserved_aliases_share_one_canonical_surface_through_pass_execution() {
        let mut resources = BTreeMap::new();
        let names = vec!["selfTex".to_string(), "feedback".to_string()];
        ensure_reserved_feedback(&names, (1, 1), &mut resources).unwrap();
        assert_eq!(resources.len(), 1);
        assert!(resources.contains_key(RESERVED_FEEDBACK_RESOURCE));
        resources
            .get_mut(RESERVED_FEEDBACK_RESOURCE)
            .unwrap()
            .data_mut()
            .copy_from_slice(&[0.25, 0.0, 0.0, 1.0]);
        let allocation = resources[RESERVED_FEEDBACK_RESOURCE].data().as_ptr();
        assert_eq!(
            resource_surface(&resources, "selfTex")
                .unwrap()
                .data()
                .as_ptr(),
            allocation
        );
        assert_eq!(
            resource_surface(&resources, "feedback")
                .unwrap()
                .data()
                .as_ptr(),
            allocation
        );

        let pass = crate::catalog::RenderPass {
            name: "dualFeedback".into(),
            program: "dualFeedback".into(),
            key: None,
            inputs: BTreeMap::from([
                ("previousSelf".into(), "selfTex".into()),
                ("previousFeedback".into(), "feedback".into()),
            ]),
            outputs: BTreeMap::from([("fragColor".into(), "outputTex".into())]),
            execution: BTreeMap::new(),
        };
        execute_pass(
            &dual_feedback_program(),
            &pass,
            &BTreeMap::new(),
            &mut resources,
            1,
            1,
            &BTreeMap::from([("outputTex".into(), TextureFormat::Rgba32f)]),
            None,
            false,
        )
        .unwrap();
        assert_eq!(resources["outputTex"].data()[0], 0.5);
        let output = resources["outputTex"].clone();
        copy_reserved_feedback("filter/dualFeedback", &names, &output, &mut resources).unwrap();
        assert_eq!(
            resources[RESERVED_FEEDBACK_RESOURCE].data().as_ptr(),
            allocation
        );
        assert_eq!(resources[RESERVED_FEEDBACK_RESOURCE].data()[0], 0.5);
        assert_eq!(
            resources
                .keys()
                .filter(|name| {
                    matches!(name.as_str(), "selfTex" | "feedback")
                        || name.as_str() == RESERVED_FEEDBACK_RESOURCE
                })
                .count(),
            1
        );

        let mut mismatch = BTreeMap::from([(
            RESERVED_FEEDBACK_RESOURCE.into(),
            Surface::new(2, 2).unwrap(),
        )]);
        let error = copy_reserved_feedback(
            "filter/selfTexSizeMismatch",
            &names,
            &Surface::new(5, 5).unwrap(),
            &mut mismatch,
        )
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("selfTex (2x2) must match the step's output (5x5)")
        );
    }

    #[test]
    fn synthetic_undeclared_particle_resources_use_state_size_formats_and_persist() {
        let effect: EffectDefinition = serde_json::from_value(serde_json::json!({
            "namespace": "points",
            "func": "synthetic",
            "kind": "filter",
            "domain": "image",
            "params": {},
            "paramNames": [],
            "textures": {},
            "passes": [{
                "name": "agent",
                "program": "agent",
                "key": "points/synthetic:agent",
                "inputs": {
                    "xyzTex": "global_xyz",
                    "velTex": "global_vel",
                    "rgbaTex": "global_rgba",
                    "lifeTex": "global_life_data",
                    "trailTex": "global_synthetic_trail"
                },
                "outputs": {}
            }],
            "iterated": true,
            "externalTexture": null,
            "loopRole": null
        }))
        .unwrap();
        let values = BTreeMap::from([("stateSize".into(), Value::Int(3))]);
        let mut shared = BTreeMap::new();
        ensure_group_particle_resources(&effect, &values, &BTreeMap::new(), &mut shared).unwrap();
        for name in [
            "global_xyz",
            "global_vel",
            "global_rgba",
            "global_life_data",
            "global_synthetic_trail",
        ] {
            assert_eq!(
                (shared[name].width(), shared[name].height()),
                (3, 3),
                "{name}"
            );
        }
        assert_eq!(
            particle_texture_format("global_xyz"),
            Some(TextureFormat::Rgba32f)
        );
        assert_eq!(
            particle_texture_format("global_vel"),
            Some(TextureFormat::Rgba32f)
        );
        assert_eq!(
            particle_texture_format("global_rgba"),
            Some(TextureFormat::Rgba8)
        );
        assert_eq!(
            particle_texture_format("global_life_data"),
            Some(TextureFormat::Rgba16f)
        );
        assert_eq!(
            particle_texture_format("global_synthetic_trail"),
            Some(TextureFormat::Rgba16f)
        );

        shared.get_mut("global_xyz").unwrap().data_mut()[0] = 0.75;
        let allocation = shared["global_xyz"].data().as_ptr();
        ensure_group_particle_resources(
            &effect,
            &BTreeMap::from([("stateSize".into(), Value::Int(5))]),
            &BTreeMap::new(),
            &mut shared,
        )
        .unwrap();
        assert_eq!(shared["global_xyz"].data().as_ptr(), allocation);
        assert_eq!(shared["global_xyz"].data()[0], 0.75);
        assert_eq!(
            (shared["global_xyz"].width(), shared["global_xyz"].height()),
            (3, 3)
        );

        let only_xyz: EffectDefinition = serde_json::from_value(serde_json::json!({
            "namespace": "points", "func": "defaultSize", "kind": "filter", "domain": "image",
            "params": {}, "paramNames": [], "textures": {}, "iterated": true,
            "externalTexture": null, "loopRole": null,
            "passes": [{"name":"agent", "program":"agent", "key":"points/defaultSize:agent",
                "inputs":{"xyzTex":"global_xyz"}, "outputs":{}}]
        }))
        .unwrap();
        let mut default_shared = BTreeMap::new();
        ensure_group_particle_resources(
            &only_xyz,
            &BTreeMap::new(),
            &BTreeMap::new(),
            &mut default_shared,
        )
        .unwrap();
        assert_eq!(
            (
                default_shared["global_xyz"].width(),
                default_shared["global_xyz"].height()
            ),
            (256, 256)
        );
    }

    #[test]
    fn production_particle_groups_size_undeclared_state_and_reset_seeded_owners() {
        let options = |seed| RenderOptions {
            width: 2,
            height: 2,
            seed,
            ..RenderOptions::default()
        };
        let group_input = ChainBundle {
            image: Some(Surface::new(2, 2).unwrap()),
            ..ChainBundle::default()
        };
        let plan = compile_dsl(
            "search synth,points; solid().attractor(stateSize:4,iterationCount:1).write(o0)",
            "<particle-production-test>",
        )
        .unwrap();
        let mut cache = CpuTextureCache::new(0);
        let execution = execute_iteration_group(
            &plan.chains[0].steps[1..2],
            effect_catalog().unwrap(),
            &group_input,
            &BTreeMap::new(),
            &options(1),
            &mut cache,
            None,
            &mut Vec::new(),
        )
        .unwrap();
        for name in ["global_xyz", "global_vel", "global_rgba"] {
            assert_eq!(
                (
                    execution.shared[name].width(),
                    execution.shared[name].height()
                ),
                (4, 4),
                "{name} must not bind or render through the 1x1 fallback"
            );
        }
        assert_eq!((execution.bundle.image.unwrap().width(), 2), (2, 2));

        let emitter_plan = compile_dsl(
            "search synth,render; solid().pointsEmit(stateSize:64,iterationCount:1).write(o0)",
            "<particle-owner-reset-test>",
        )
        .unwrap();
        let first = execute_iteration_group(
            &emitter_plan.chains[0].steps[1..2],
            effect_catalog().unwrap(),
            &group_input,
            &BTreeMap::new(),
            &options(1),
            &mut cache,
            None,
            &mut Vec::new(),
        )
        .unwrap();
        let first_xyz = first.shared["global_xyz"].clone();
        let first_allocation = first.shared["global_xyz"].data().as_ptr();
        let changed = execute_iteration_group(
            &emitter_plan.chains[0].steps[1..2],
            effect_catalog().unwrap(),
            &group_input,
            &BTreeMap::new(),
            &options(9),
            &mut cache,
            None,
            &mut Vec::new(),
        )
        .unwrap();
        let repeated = execute_iteration_group(
            &emitter_plan.chains[0].steps[1..2],
            effect_catalog().unwrap(),
            &group_input,
            &BTreeMap::new(),
            &options(1),
            &mut cache,
            None,
            &mut Vec::new(),
        )
        .unwrap();
        assert_ne!(first_xyz, changed.shared["global_xyz"]);
        assert_eq!(first_xyz, repeated.shared["global_xyz"]);
        assert_ne!(
            first_allocation,
            changed.shared["global_xyz"].data().as_ptr()
        );
    }

    #[test]
    fn two_particle_owners_are_isolated_seed_sensitive_and_repeatable() {
        let source = |second_seed| {
            format!(
                "search synth,render; solid(color:#000).pointsEmit(seed:1,stateSize:64,iterationCount:1).write(o0); solid(color:#000).pointsEmit(seed:{second_seed},stateSize:64,iterationCount:1).write(o1); render(o1)"
            )
        };
        let plan = |second_seed| compile_dsl(&source(second_seed), "<two-owner-test>").unwrap();
        let options = RenderOptions {
            width: 4,
            height: 4,
            time: 0.25,
            seed: 1,
            ..RenderOptions::default()
        };
        let mut renderer = CpuRenderer::new().unwrap();
        let first = renderer.render_plan_with_trace(plan(2), &options).unwrap();
        let changed = renderer.render_plan_with_trace(plan(9), &options).unwrap();
        let repeated = renderer.render_plan_with_trace(plan(2), &options).unwrap();
        assert_eq!(
            (first.result.surface.width(), first.result.surface.height()),
            (4, 4)
        );
        assert_eq!(first.particle_groups.len(), 2);
        assert_eq!(changed.particle_groups.len(), 2);
        assert_eq!(repeated.particle_groups.len(), 2);
        assert_ne!(
            first.particle_groups[0].original_allocations["global_xyz"],
            first.particle_groups[1].original_allocations["global_xyz"],
            "independent owners must not share one allocation"
        );
        for name in ["global_xyz", "global_vel", "global_rgba"] {
            assert_eq!(
                first.particle_groups[0].resources[name],
                changed.particle_groups[0].resources[name],
                "changing owner two must not alter owner one {name}"
            );
            assert_eq!(
                first.particle_groups[0].resources[name],
                repeated.particle_groups[0].resources[name],
                "owner one {name} must repeat exactly"
            );
            assert_eq!(
                first.particle_groups[1].resources[name],
                repeated.particle_groups[1].resources[name],
                "owner two {name} must repeat exactly"
            );
        }
        assert_ne!(
            first.particle_groups[1].resources["global_xyz"],
            changed.particle_groups[1].resources["global_xyz"],
            "owner two position state must remain seed-sensitive"
        );
    }

    #[test]
    fn particle_trace_records_source_identity_before_deep_clone() {
        let original = Surface::new(2, 2).unwrap();
        let original_allocation = original.data().as_ptr() as usize;
        let shared = BTreeMap::from([("global_xyz".to_owned(), original)]);
        let trace = trace_particle_group(shared);

        assert_eq!(
            trace.original_allocations["global_xyz"], original_allocation,
            "trace identity must be captured from the source allocation"
        );
        assert_ne!(
            trace.resources["global_xyz"].data().as_ptr() as usize,
            original_allocation,
            "the cloned surface must not be mistaken for the source allocation"
        );
    }

    #[test]
    fn remap_uniform_adapter_has_exact_header_active_and_dimension_records() {
        let mut zone = Surface::new(2, 2).unwrap();
        zone.clear([1.0, 0.0, 0.0, 1.0]);
        let values = BTreeMap::from([
            ("bgColor".into(), Value::Vec(vec![0.25, 0.5, 0.75])),
            ("bgAlpha".into(), Value::Float(0.8)),
            ("zoneCount".into(), Value::Int(2)),
            ("smoothEdge".into(), Value::Float(0.125)),
        ]);
        let Value::Array(data) =
            remap_uniform_data(&values, &BTreeMap::from([("zone1_tex".into(), zone)]), 7, 5)
        else {
            panic!("remap data must be an array")
        };
        assert_eq!(data.len(), 275);
        assert_eq!(data[0], Value::Vec(vec![0.25, 0.5, 0.75, 0.8]));
        assert_eq!(data[1], Value::Vec(vec![2.0, 0.125, 0.0, 0.0]));
        assert_eq!(data[2], Value::Vec(vec![0.0, 0.0, 0.0, 1.0]));
        assert_eq!(data[3], Value::Vec(vec![0.0, 1.0, 0.0, 1.0]));
        assert_eq!(data[10], Value::Vec(vec![0.0; 4]));
        assert_eq!(data[265], Value::Vec(vec![0.0; 4]));
        assert_eq!(data[266], Value::Vec(vec![7.0, 5.0, 0.0, 0.0]));
        // reference 0ed489ec: 8 new zoneN_bounds slots, each defaulting to
        // [0,0,1,1] so an unset bounds box never clips a zone's polygon.
        assert_eq!(data[267], Value::Vec(vec![0.0, 0.0, 1.0, 1.0]));
        assert_eq!(data[274], Value::Vec(vec![0.0, 0.0, 1.0, 1.0]));
    }

    #[test]
    fn remap_uniform_adapter_packs_host_active_zone_geometry() {
        let values = BTreeMap::from([
            ("zoneCount".into(), Value::Int(8)),
            ("zone0_count".into(), Value::Int(3)),
            ("zone0_alpha".into(), Value::Float(0.625)),
            ("zone0_bounds".into(), Value::Vec(vec![0.1, 0.2, 0.8, 0.9])),
            ("zone0_v0".into(), Value::Vec(vec![0.1, 0.2, 0.3, 0.4])),
            ("zone0_v31".into(), Value::Vec(vec![0.5, 0.6, 0.7, 0.8])),
            ("zone7_count".into(), Value::Int(64)),
            ("zone7_alpha".into(), Value::Float(0.25)),
            ("zone7_bounds".into(), Value::Vec(vec![0.2, 0.3, 0.9, 1.0])),
            ("zone7_v31".into(), Value::Vec(vec![0.9, 0.8, 0.7, 0.6])),
        ]);
        let inputs = BTreeMap::from([("zone0_tex".into(), Surface::new(1, 1).unwrap())]);
        let Value::Array(data) = remap_uniform_data(&values, &inputs, 8, 8) else {
            panic!("remap data must be an array")
        };

        assert_eq!(data[2], Value::Vec(vec![3.0, 1.0, 0.0, 0.625]));
        assert_eq!(data[9], Value::Vec(vec![64.0, 0.0, 0.0, 0.25]));
        assert_eq!(data[10], Value::Vec(vec![0.1, 0.2, 0.3, 0.4]));
        assert_eq!(data[41], Value::Vec(vec![0.5, 0.6, 0.7, 0.8]));
        assert_eq!(data[265], Value::Vec(vec![0.9, 0.8, 0.7, 0.6]));
        assert_eq!(data[267], Value::Vec(vec![0.1, 0.2, 0.8, 0.9]));
        assert_eq!(data[274], Value::Vec(vec![0.2, 0.3, 0.9, 1.0]));
    }

    #[test]
    fn group_frame_executor_interleaves_move_and_deposit() {
        let (mut position, mut trail) = (0_u32, 0_u32);
        run_group_frames(4, 0.0, |_| {
            position += 1;
            trail += position;
            Ok(())
        })
        .unwrap();
        assert_eq!(trail, 10);
        assert_ne!(trail, position * 4);
    }

    #[test]
    fn grouped_frames_dispatch_real_draw_op_and_advance_shared_trail() {
        let catalog = effect_catalog().unwrap();
        let pass = catalog.effects["render/pointsRender"]
            .passes
            .iter()
            .find(|pass| pass.program == "deposit")
            .unwrap();
        let mut resources = BTreeMap::from([
            (
                "global_xyz".into(),
                Surface::from_f32(1, 1, vec![0.5, 0.5, 0.0, 1.0]).unwrap(),
            ),
            (
                "global_rgba".into(),
                Surface::from_f32(1, 1, vec![0.2, 0.4, 0.6, 0.8]).unwrap(),
            ),
        ]);
        let uniforms = BTreeMap::from([
            ("density".into(), Value::Float(100.0)),
            ("viewMode".into(), Value::Int(0)),
            ("rotateX".into(), Value::Float(0.0)),
            ("rotateY".into(), Value::Float(0.0)),
            ("rotateZ".into(), Value::Float(0.0)),
            ("posX".into(), Value::Float(0.0)),
            ("posY".into(), Value::Float(0.0)),
            ("viewScale".into(), Value::Float(1.0)),
        ]);
        let formats = BTreeMap::from([("global_points_trail".into(), TextureFormat::Rgba32f)]);
        let mut calls = 0;
        let mut red = Vec::new();
        run_group_frames(3, 0.25, |_| {
            calls += 1;
            assert_eq!(
                execute_draw_pass(
                    "render/pointsRender:deposit",
                    pass,
                    &uniforms,
                    &mut resources,
                    1,
                    1,
                    &formats,
                )?,
                1
            );
            red.push(resources["global_points_trail"].data()[0]);
            Ok(())
        })
        .unwrap();
        assert_eq!(calls, 3);
        assert_eq!(red, [0.2, 0.4, 0.6]);
    }

    #[test]
    fn loop_executor_freezes_input_and_only_advances_accumulator() {
        let input = 0.2_f32;
        let mut accumulator = 0.0_f32;
        run_group_frames(3, 0.0, |_| {
            let begin = input + accumulator;
            accumulator = begin + 0.1;
            Ok(())
        })
        .unwrap();
        assert!((accumulator - 0.9).abs() < 0.002);
        let mut bypass = input;
        run_group_frames(0, 0.0, |_| {
            bypass = 999.0;
            Ok(())
        })
        .unwrap();
        assert_eq!(bypass, 0.2);
    }

    #[test]
    fn joining_state_size_inherits_owner_while_standalone_keeps_its_own() {
        let catalog = effect_catalog().unwrap();
        let joiner = &catalog.effects["points/life"];
        let mut grouped = BTreeMap::from([("stateSize".into(), ParamValue::Int(4))]);
        inherit_group_state_size(1, joiner, Some(&ParamValue::Int(8)), &mut grouped);
        assert_eq!(grouped["stateSize"], ParamValue::Int(8));
        let mut standalone = BTreeMap::from([("stateSize".into(), ParamValue::Int(4))]);
        inherit_group_state_size(0, joiner, None, &mut standalone);
        assert_eq!(standalone["stateSize"], ParamValue::Int(4));
    }

    #[test]
    fn iterated_unknown_draw_error_propagates_from_first_frame() {
        let error = run_group_frames(3, 0.0, |_| {
            Err(RenderError::MissingDrawOp {
                key: "points/missingScatterIter:deposit".into(),
                pass: "deposit".into(),
            })
        })
        .unwrap_err();
        assert!(
            matches!(error, RenderError::MissingDrawOp { key, .. } if key == "points/missingScatterIter:deposit")
        );
    }
}

#[cfg(test)]
mod handed_on_textures {
    use super::*;
    use serde_json::{Value as Json, json};

    // Use the shipped solid/passthrough kernels in small authored pass graphs,
    // exercising both the normal graph scheduler and its iterated branch.
    fn fill(name: &str, alpha: f64) -> Json {
        json!({"name":"fill","program":"solid","key":"synth/solid:solid",
            "uniforms":{"alpha":alpha},"outputs":{"fragColor":name}})
    }
    fn storage_fill(name: &str, alpha: f64) -> Json {
        let mut pass = fill(name, alpha);
        let outputs = pass.as_object_mut().unwrap().remove("outputs").unwrap();
        pass["storageTextures"] = outputs;
        pass
    }
    fn copy(input: &str) -> Json {
        json!({"name":"copy","program":"passthrough","key":"render/pointsEmit:passthrough",
            "inputs":{"inputTex":input},"outputs":{"fragColor":"outputTex"}})
    }
    fn run(specs: Vec<Json>, steps: &[&str]) -> Result<ChainBundle, RenderError> {
        run_with(specs, steps, |_| {})
    }
    fn run_with(
        specs: Vec<Json>,
        steps: &[&str],
        edit: impl FnOnce(&mut Vec<RenderStep>),
    ) -> Result<ChainBundle, RenderError> {
        run_with_options(
            specs,
            steps,
            edit,
            &RenderOptions {
                width: 2,
                height: 2,
                ..RenderOptions::default()
            },
        )
    }

    fn run_with_options(
        specs: Vec<Json>,
        steps: &[&str],
        edit: impl FnOnce(&mut Vec<RenderStep>),
        options: &RenderOptions,
    ) -> Result<ChainBundle, RenderError> {
        let mut catalog: EffectCatalog = serde_json::from_value(json!({
            "effects": {}, "excludedEffects": [],
            "provenance":{"source":"","version":"","base":"","lockSource":"","lockVersion":""}
        }))
        .unwrap();
        for (index, mut spec) in specs.into_iter().enumerate() {
            let name = spec["func"].as_str().unwrap().to_owned();
            spec["namespace"] = json!("probe");
            spec["kind"] = json!(if index == 0 { "generator" } else { "filter" });
            spec["domain"] = json!("image");
            spec["params"]["color"] =
                json!({"type":"color","default":[0.5,0.5,0.5],"uniform":"color"});
            spec["params"]["iterationCount"] = json!({"type":"int","default":1});
            spec["paramNames"] = json!(
                spec["params"]
                    .as_object()
                    .unwrap()
                    .keys()
                    .collect::<Vec<_>>()
            );
            catalog
                .effects
                .insert(name, serde_json::from_value(spec).unwrap());
        }
        let mut steps = steps
            .iter()
            .map(|name| RenderStep::Effect {
                id: None,
                effect_id: (*name).into(),
                params: BTreeMap::from([("iterationCount".into(), ParamValue::Int(1))]),
                surface_params: BTreeMap::new(),
                explicit_params: vec![],
                inline_chains: vec![],
                loc: crate::dsl::Location {
                    source_name: "probe".into(),
                    line: 1,
                    column: 1,
                    index: 0,
                },
            })
            .collect::<Vec<_>>();
        edit(&mut steps);
        CpuRenderer::new()?.execute_steps(
            &steps,
            &catalog,
            &mut BTreeMap::new(),
            options,
            &mut |_| {},
        )
    }

    #[test]
    fn ordinary_and_grouped_writes_update_all_published_particle_channels() {
        for (field, input, global) in [
            ("outputXyz", "inputXyz", "global_xyz"),
            ("outputVel", "inputVel", "global_vel"),
            ("outputRgba", "inputRgba", "global_rgba"),
        ] {
            for grouped in [false, true] {
                let target = if grouped { global } else { "agent" };
                let mut source = json!({"func":"emit","iterated":grouped,field:target,
                    "textures":{target:{"width":1,"height":1,"format":"rgba32f"}},
                    "passes":[fill(target,0.25),fill("outputTex",0.125)]});
                if grouped {
                    source["textures"]["global_xyz"] =
                        json!({"width":1,"height":1,"format":"rgba32f"});
                }
                let written = if grouped { global } else { input };
                let graph = vec![
                    source,
                    json!({"func":"edit","passes":[fill(written,0.75),fill("outputTex",0.5)]}),
                    json!({"func":"readback","textures":{field:{}},"passes":[copy(input)]}),
                ];
                let result = run(graph, &["emit", "edit", "readback"]).unwrap();
                let agent = result.agents[input].surface.borrow();
                assert_eq!(
                    (agent.width(), agent.height()),
                    (1, 1),
                    "{input} grouped={grouped}"
                );
                assert_eq!(agent.data()[3], 0.75);
                assert_eq!(result.image.unwrap().data()[3], 0.75);
            }
        }
    }

    #[test]
    fn shared_storage_image_and_particle_aliases_observe_writes_before_sampling() {
        for iterated in [false, true] {
            for with_agent in [false, true] {
                let mut source = json!({"func":"source","iterated":iterated,"outputTex":"global_work",
                    "textures":{"global_work":{"width":1,"height":1,"format":"rgba32f"}},
                    "passes":[storage_fill("global_work",0.25)]});
                let edit = if with_agent {
                    source["outputXyz"] = json!("global_work");
                    fill("inputXyz", 0.75)
                } else {
                    storage_fill("global_work", 0.75)
                };
                let result = run(
                    vec![
                        source,
                        json!({"func":"edit","passes":[edit,copy("inputTex")]}),
                    ],
                    &["source", "edit"],
                )
                .unwrap();
                assert_eq!(
                    result.image.unwrap().data()[3],
                    0.75,
                    "iterated={iterated}, with_agent={with_agent}"
                );
            }
        }
    }

    #[test]
    fn storage_shares_between_definitions_but_is_private_to_repeated_instances() {
        let producer = json!({"func":"producer","textures":{"global_work":{"width":1,"height":1}},
            "passes":[storage_fill("global_work",0.25), fill("outputTex",0.5)]});
        let middle = json!({"func":"middle","passes":[fill("outputTex",0.125)]});
        let reader = json!({"func":"reader","passes":[copy("global_work")]});
        let result = run(
            vec![producer.clone(), middle.clone(), reader.clone()],
            &["producer", "middle", "reader"],
        )
        .unwrap();
        assert_eq!(result.image.unwrap().data()[3], 0.25);
        let isolated = run(vec![producer, middle, reader], &["reader"]).unwrap_err();
        assert!(isolated.to_string().contains("global_work"));

        let start = json!({"func":"start","passes":[fill("outputTex",0.125)]});
        let mut private = json!({"func":"private","textures":{"global_work":{}},
            "passes":[copy("global_work"),storage_fill("global_work",0.75)]});
        let chain = ["start", "private", "private"];
        let error = run(vec![start.clone(), private.clone()], &chain).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("requires texture \"global_work\"")
        );
        private["iterated"] = json!(true);
        assert_eq!(
            run(vec![start, private], &chain)
                .unwrap()
                .image
                .unwrap()
                .data()[3],
            0.
        );
    }

    #[test]
    fn an_ordinary_step_cannot_sample_the_unwritten_target_of_a_skipped_pass() {
        let mut skipped = fill("scratch", 0.25);
        skipped["conditions"] = json!({"runIf":[{"uniform":"iterationCount","equals":0}]});
        let source = json!({"func":"skipped","textures":{"scratch":{}},
            "passes":[skipped,copy("scratch")]});
        let error = run(vec![source], &["skipped"]).unwrap_err();
        assert!(error.to_string().contains("requires texture \"scratch\""));
    }

    #[test]
    fn publishing_the_image_input_updates_both_volume_and_geometry_channels() {
        let initial = json!({"func":"initial","outputTex3d":"aux","outputGeo":"aux","outputXyz":"aux",
            "passes":[fill("aux",0.25),fill("outputTex",0.375)]});
        let link = json!({"func":"link","outputTex3d":"inputTex","outputGeo":"inputTex","passes":[copy("inputTex")]});
        let result = run(vec![initial, link], &["initial", "link"]).unwrap();
        assert_eq!(result.volume.unwrap().data()[3], 0.375);
        assert_eq!(result.geometry.unwrap().data()[3], 0.375);
        assert_eq!(result.agents["inputXyz"].surface.borrow().data()[3], 0.25);
    }

    #[test]
    fn fresh_attachments_use_each_declared_axis_and_a_matching_blend_backdrop() {
        for iterated in [false, true] {
            for (viewport, expected) in [
                (json!({}), (5, 3)),
                (json!({"w":4}), (4, 3)),
                (json!({"width":2,"height":1}), (2, 1)),
            ] {
                let mut first = fill("work", 0.25);
                first["viewport"] = json!({"width":2,"height":1});
                let mut second = fill("work", 0.75);
                if viewport != json!({}) {
                    second["viewport"] = viewport;
                }
                second["blend"] = json!(["zero", "one"]);
                let producer = json!({"func":"producer","iterated":iterated,"outputTex":"work",
                    "textures":{"work":{"width":5,"height":3,"format":"rgba32f"}},"passes":[first,second]});
                let result = run(vec![producer], &["producer"]).unwrap().image.unwrap();
                assert_eq!((result.width(), result.height()), expected);
                assert_eq!(
                    result.data()[3],
                    if expected == (2, 1) { 0.25 } else { 0.0 }
                );
            }
        }
    }

    #[test]
    fn pass_aliases_cannot_activate_a_skipped_publisher_or_increase_its_repeat_count() {
        let mut skipped = fill("work", 0.75);
        skipped["uniforms"]["iterationCount"] = json!(0);
        skipped["conditions"] = json!({"runIf":[{"uniform":"iterationCount","equals":0}]});
        let mut repeated = fill("work", 0.125);
        repeated["uniforms"]["iterationCount"] = json!(3);
        repeated["repeat"] = json!("iterationCount");
        repeated["blend"] = json!(true);
        let producer = json!({"func":"producer","outputTex":"work","passes":[fill("work",0.25),skipped,repeated]});
        assert_eq!(
            run(vec![producer], &["producer"])
                .unwrap()
                .image
                .unwrap()
                .data()[3],
            0.375
        );
    }

    #[test]
    fn shared_particle_targets_use_the_first_declaring_steps_parameters_and_format() {
        let first = json!({"func":"first","iterated":true,"params":{"w":{"type":"int","default":1}},
            "textures":{"global_xyz":{}},"outputXyz":"global_vel","passes":[copy("global_vel")]});
        let owner = json!({"func":"owner","params":{"w":{"type":"int","default":3}},
            "textures":{"global_vel":{"width":{"param":"w"},"height":2}},
            "passes":[copy("global_vel")]});
        let state = run(vec![first, owner], &["first", "owner"]).unwrap();
        let vel = state.agents["inputXyz"].surface.borrow();
        assert_eq!((vel.width(), vel.height()), (3, 2));

        let first = json!({"func":"first","iterated":true,"textures":{"global_xyz":{},
            "global_vel":{"width":2,"height":2,"format":"rgba8"}},"outputVel":"global_vel",
            "passes":[fill("global_vel",0.125),fill("outputTex",0.25)]});
        let mut update = fill("global_vel", 0.12345);
        update["viewport"] = json!({"width":5,"height":4});
        let behavior = json!({"func":"behavior","textures":{"global_vel":{"width":5,"height":5,"format":"rgba32f"}},
            "passes":[update,copy("global_vel")]});
        let state = run(vec![first, behavior], &["first", "behavior"]).unwrap();
        let vel = state.agents["inputVel"].surface.borrow();
        assert_eq!((vel.width(), vel.height()), (2, 2));
        assert_eq!(state.image.unwrap().data()[3], 0.121520996);
    }

    #[test]
    fn group_scratch_initialization_sizes_each_texture_without_other_scratch_inputs() {
        let producer = json!({"func":"producer","iterated":true,"outputXyz":"work","textureOrder":["reference","work"],
            "textures":{"reference":{"width":3,"height":3},"work":{"width":{"inputOverride":"reference","param":"n","default":1},"height":2}},
            "passes":[copy("work")]});
        let result = run(vec![producer], &["producer"]).unwrap();
        let work = result.agents["inputXyz"].surface.borrow();
        assert_eq!((work.width(), work.height()), (1, 2));
    }

    #[test]
    fn named_surface_inputs_follow_only_the_current_images_physical_identity() {
        let source = json!({"func":"source","outputTex":"work","outputXyz":"work","passes":[fill("work",0.25)]});
        let user = json!({"func":"edit","params":{"named":{"type":"surface","default":"none","texture":"sample"}},
            "passes":[fill("inputXyz",0.75),copy("sample")]});
        for (new_image, read_saved, expected) in [
            (false, false, 0.75),
            (true, false, 0.25),
            (false, true, 0.25),
        ] {
            let result = run_with(
                vec![source.clone(), user.clone()],
                &["source", "edit"],
                |steps| {
                    let loc = crate::dsl::Location {
                        source_name: "probe".into(),
                        line: 1,
                        column: 1,
                        index: 0,
                    };
                    let RenderStep::Effect { surface_params, .. } = &mut steps[1] else {
                        unreachable!()
                    };
                    surface_params.insert("named".into(), SurfaceBinding::Surface("o1".into()));
                    steps.insert(
                        1,
                        RenderStep::Write {
                            id: None,
                            surface: "o1".into(),
                            loc: loc.clone(),
                        },
                    );
                    if new_image {
                        steps.insert(2, steps[0].clone());
                    }
                    if read_saved {
                        steps.push(RenderStep::Read {
                            id: None,
                            skip: false,
                            surface: "o1".into(),
                            loc,
                        });
                    }
                },
            )
            .unwrap();
            assert_eq!(result.image.unwrap().data()[3], expected);
        }
        let mut unbound = user;
        unbound["passes"] = json!([copy("named")]);
        let error = run_with(vec![source, unbound], &["source", "edit"], |steps| {
            let RenderStep::Effect { surface_params, .. } = &mut steps[1] else {
                unreachable!()
            };
            surface_params.insert("named".into(), SurfaceBinding::Current);
        })
        .unwrap_err();
        assert!(error.to_string().contains("requires texture \"named\""));
    }

    #[test]
    fn publishing_a_typed_input_name_as_image_retains_the_incoming_image() {
        let first = json!({"func":"initial","outputTex3d":"volume","outputGeo":"geo",
            "passes":[fill("volume",0.25),fill("geo",0.5),fill("outputTex",0.375)]});
        for name in ["inputTex3d", "inputGeo"] {
            let link = json!({"func":"link","outputTex":name,"passes":[fill("side",0.875)]});
            let result = run(vec![first.clone(), link], &["initial", "link"]).unwrap();
            assert_eq!(result.image.unwrap().data()[3], 0.375);
        }
        let mut skipped = fill("outputTex", 0.75);
        skipped["conditions"] = json!({"runIf":[{"uniform":"iterationCount","equals":0}]});
        let link = json!({"func":"skipped","textures":{"outputTex":{}},"passes":[skipped]});
        assert!(
            matches!(run(vec![first,link], &["initial","skipped"]), Err(RenderError::MissingOutput{output,..}) if output=="outputTex")
        );
    }

    #[test]
    fn a_single_output_kernel_cannot_publish_an_unused_declared_attachment() {
        let source = json!({"func":"source","textures":{"scratch":{}},
            "passes":[{"name":"fill","program":"solid","key":"synth/solid:solid",
                "outputs":{"a":"scratch","z":"first"},"outputOrder":["z","a"]},copy("scratch")]});
        assert!(
            run(vec![source], &["source"])
                .unwrap_err()
                .to_string()
                .contains("requires texture \"scratch\"")
        );
    }

    #[test]
    fn implicit_last_attachment_is_the_actual_written_surface() {
        for iterated in [false, true] {
            for name in ["inputGeo", "inputTex3d"] {
                let source =
                    json!({"func":"source","iterated":iterated,"passes":[fill(name,0.625)]});
                assert_eq!(
                    run(vec![source], &["source"])
                        .unwrap()
                        .image
                        .unwrap()
                        .data()[3],
                    0.625
                );
            }
            for (agent, expected) in [(false, 0.25), (true, 0.75)] {
                let mut source = json!({"func":"source","iterated":iterated,"textures":{"global_work":{}},
                    "passes":[storage_fill("global_work",0.25)]});
                if agent {
                    source["outputXyz"] = json!("global_work");
                }
                let edit = json!({"func":"edit","passes":[storage_fill("global_work",0.75),copy("inputTex")]});
                assert_eq!(
                    run(vec![source, edit], &["source", "edit"])
                        .unwrap()
                        .image
                        .unwrap()
                        .data()[3],
                    expected
                );
            }
        }
    }

    #[test]
    fn an_image_only_step_publishes_auxiliary_outputs_in_a_typed_bundle() {
        for with_agent in [false, true] {
            let mut source = json!({"func":"source","outputGeo":"aux","outputTex3d":"aux",
                "passes":[fill("aux",0.25),fill("outputTex",0.5)]});
            if with_agent {
                source["outputXyz"] = json!("aux");
            }
            let result = run(vec![source], &["source"]).unwrap();
            assert_eq!(result.geometry.is_some(), with_agent);
            assert_eq!(result.volume.is_some(), with_agent);
        }
    }

    #[test]
    fn shader_output_count_and_draw_buffers_choose_the_active_mrt_slots() {
        for iterated in [false, true] {
            for (mrt, expected) in [(false, [128, 128, 128, 255]), (true, [0, 255, 0, 255])] {
                let mut pass = json!({"name":"split","program":"mrt","key":"synth3d/heightmap3d:precompute",
                    "inputs":{"tex":"source","heightTex":"source"},
                    "outputs":{"extra":"unused","fragColor":"color","geoOut":"geometry"},"outputOrder":["fragColor","geoOut","extra"],
                    "uniforms":{"volumeSize":2,"baseHeight":0.5,"heightScale":1.0}});
                if mrt {
                    pass["drawBuffers"] = json!(2);
                }
                let source =
                    json!({"func":"mrt","iterated":iterated,"passes":[fill("source",1.0),pass]});
                let bytes = run(vec![source], &["mrt"])
                    .unwrap()
                    .image
                    .unwrap()
                    .to_rgba8();
                assert_eq!(&bytes[..4], &expected);
            }
        }
    }

    #[test]
    fn external_linear_filter_follows_the_supplied_resource_view() {
        for (external, local, expected) in [
            ("ext", None, 0.5),
            ("inputTex", Some("work"), 1.0),
            ("inputTex", Some("inputTex"), 1.0),
        ] {
            let texture = Surface::from_f32(2, 1, vec![0., 0., 0., 0., 1., 1., 1., 1.]).unwrap();
            let options = RenderOptions {
                width: 2,
                height: 1,
                external_textures: BTreeMap::from([(external.to_owned(), texture)]),
                ..RenderOptions::default()
            };
            let mut passes = Vec::new();
            if let Some(local) = local {
                let mut make_local = copy(external);
                make_local["outputs"] = json!({"fragColor":local});
                passes.push(make_local);
            }
            passes.push(copy(local.unwrap_or(external)));
            let source = json!({"func":"sample","externalTexture":external,"textures":{"outputTex":{"width":1,"height":1,"format":"rgba32f"}},"passes":passes});
            let image = run_with_options(vec![source], &["sample"], |_| {}, &options)
                .unwrap()
                .image
                .unwrap();
            assert_eq!(
                image.data()[3],
                expected,
                "external={external} local={local:?}"
            );
        }
    }

    #[test]
    fn drawing_over_an_external_texture_replaces_its_linear_sampler() {
        let catalog: Json = serde_json::from_str(crate::generated::CATALOG_JSON).unwrap();
        let mesh = &catalog["effects"]["render/meshRender"];
        let draw = mesh["passes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|pass| pass["program"] == "render")
            .unwrap();
        let external = Surface::from_f32(2, 1, vec![0., 0., 0., 0., 1., 1., 1., 1.]).unwrap();
        let options = RenderOptions {
            width: 2,
            height: 1,
            external_textures: BTreeMap::from([("ext".into(), external)]),
            ..RenderOptions::default()
        };
        for (replace, expected) in [(false, 0.5), (true, 1.)] {
            let mut passes = Vec::new();
            if replace {
                // A single vertex draws no triangle. The replacement keeps the
                // original two texels, but now uses nearest sampling.
                passes.push(json!({"name":"draw", "program":"render", "drawMode":"triangles",
                                   "count":1, "uniforms":draw["uniforms"], "outputs":{"color":"ext"}}));
            }
            passes.push(copy("ext"));
            let source = json!({"func":"render/meshRender", "externalTexture":"ext",
                "params":mesh["params"],
                "textures":{"outputTex":{"width":1,"height":1,"format":"rgba32f"}}, "passes":passes});
            let image = run_with_options(vec![source], &["render/meshRender"], |_| {}, &options)
                .unwrap()
                .image
                .unwrap();
            assert_eq!(image.data()[3], expected, "draw={replace}");
        }
    }

    #[test]
    fn shared_storage_dimensions_follow_authored_target_order() {
        let first = json!({"func":"producer","textures":{
            "global_z":{"width":3,"height":1},
            "global_a":{"width":{"inputOverride":"global_z","param":"iterationCount"},"height":1}},
            "passes":[{"name":"fill","program":"solid","key":"synth/solid:solid",
                "storageTextures":{"a":"global_a","z":"global_z"},"storageTextureOrder":["z","a"]}]});
        let reader = json!({"func":"reader","outputXyz":"global_a","outputVel":"global_z",
            "passes":[copy("global_z"),copy("global_a")]});
        let result = run(vec![first, reader], &["producer", "reader"]).unwrap();
        assert_eq!(result.agents["inputVel"].surface.borrow().width(), 3);
        assert_eq!(result.agents["inputXyz"].surface.borrow().width(), 3);
    }

    #[test]
    fn bound_draw_target_keeps_its_size_and_format_over_a_conflicting_viewport() {
        for iterated in [false, true] {
            let source = json!({"func":"source","outputTex":"work","outputXyz":"work",
                "textures":{"work":{"width":1,"height":1,"format":"rgba32f"}},"passes":[fill("work",1.0)]});
            let draw = json!({"func":"points/dla","iterated":iterated,"outputTex":"inputXyz","passes":[{
                "name":"draw","program":"depositGrid","drawMode":"points","uniforms":{"deposit":1},
                "viewport":{"width":2,"height":2},"inputs":{"xyzTex":"inputXyz","velTex":"inputXyz","rgbaTex":"inputXyz"},
                "outputs":{"fragColor":"inputXyz"}}]});
            let image = run(vec![source, draw], &["source", "points/dla"])
                .unwrap()
                .image
                .unwrap();
            assert_eq!((image.width(), image.height()), (1, 1));
            assert_eq!(image.data(), &[0.55, 0.55, 0.55, 1.1]);
        }
    }

    #[test]
    fn fallback_uses_last_mrt_shader_output_and_named_outputs_take_precedence() {
        // CPU 170d7ba, 2x2 heightmap3d:precompute probe: color encodes
        // [128,128,128,255], while the geometry normal encodes [0,255,0,255].
        let mrt = json!({"name":"split","program":"mrt","key":"synth3d/heightmap3d:precompute",
            "drawBuffers":2,"inputs":{"tex":"source","heightTex":"source"},
            "outputs":{"fragColor":"color","geoOut":"geometry"},
            "uniforms":{"volumeSize":2,"baseHeight":0.5,"heightScale":1.0}});
        for iterated in [false, true] {
            for (output, expected) in [
                (None, [0, 255, 0, 255]),
                (Some("color"), [128, 128, 128, 255]),
            ] {
                let mut spec =
                    json!({"func":"mrt","iterated":iterated,"passes":[fill("source",1.0),mrt]});
                if let Some(name) = output {
                    spec["outputTex"] = json!(name);
                }
                let image = run(vec![spec], &["mrt"]).unwrap().image.unwrap();
                assert_eq!(&image.to_rgba8()[..4], &expected);
            }
        }
    }

    #[test]
    fn a_new_particle_emitter_clears_the_previous_emitters_unpublished_channels() {
        let first = json!({"func":"first","iterated":true,"outputVel":"v","textures":{"global_xyz":{}},
            "passes":[fill("v",0.75),fill("outputTex",0.125)]});
        let next = json!({"func":"next","iterated":true,"outputXyz":"global_xyz","textures":{"global_xyz":{}},
            "passes":[fill("global_xyz",0.25),fill("outputTex",0.5)]});
        let reader = json!({"func":"reader","passes":[copy("inputVel")]});
        let error = run(vec![first, next, reader], &["first", "next", "reader"]).unwrap_err();
        assert!(error.to_string().contains("inputVel"));
    }
}
