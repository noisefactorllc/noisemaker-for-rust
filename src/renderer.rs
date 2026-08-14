use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use thiserror::Error;

use crate::catalog::{CatalogError, Expression, effect_catalog, shader_bundle};
use crate::dsl::{DslError, RenderPlan, RenderStep, SurfaceBinding, compile_dsl};
use crate::iteration::{
    IterationStep, compute_iteration_groups, is_particle_state_name, iteration_schedule,
};
use crate::pass_runner::{
    RESERVED_FEEDBACK_RESOURCE, apply_pass_uniform_aliases, canonical_uniforms, execute_pass,
    normalize_parameters, pass_enabled, repeat_count, texture_dimensions,
};
use crate::{ParamValue, Surface, SurfaceError, TextureFormat, Value, VmError};

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
}

#[cfg(test)]
fn trace_particle_group(shared: &BTreeMap<String, Surface>) -> ParticleGroupTrace {
    let original_allocations = shared
        .iter()
        .map(|(name, surface)| (name.clone(), surface.data().as_ptr() as usize))
        .collect();
    ParticleGroupTrace {
        resources: shared.clone(),
        original_allocations,
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
    #[error("effect {effect_id:?} produced non-finite output {value} at channel {channel}")]
    NonFiniteOutput {
        effect_id: String,
        channel: usize,
        value: f32,
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

#[derive(Clone, Debug, Default)]
pub(crate) struct ChainBundle {
    pub image: Option<Surface>,
    pub volume: Option<Surface>,
    pub geometry: Option<Surface>,
    pub volume_size: Option<u32>,
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
        mut observe_particle_group: impl FnMut(&BTreeMap<String, Surface>),
    ) -> Result<RenderResult, RenderError> {
        let mut surfaces = BTreeMap::<String, Surface>::new();

        for chain in plan.chains {
            // Current image/volume/geometry state is deliberately local to a
            // chain. Only an explicit write/read crosses a chain boundary.
            let mut bundle = ChainBundle::default();
            let grouping_steps = chain
                .steps
                .iter()
                .map(|step| match step {
                    RenderStep::Read { .. } => IterationStep::Read,
                    RenderStep::Write { .. } => IterationStep::Write,
                    RenderStep::Effect { effect_id, .. } => IterationStep::Effect {
                        effect_id: effect_id.clone(),
                    },
                })
                .collect::<Vec<_>>();
            let groups = compute_iteration_groups(&grouping_steps).map_err(|error| {
                RenderError::InvalidGraph {
                    message: error.to_string(),
                }
            })?;
            let mut step_offset = 0;
            for group in groups {
                let step_count = group.steps.len();
                let steps = &chain.steps[step_offset..step_offset + step_count];
                step_offset += step_count;
                if group.iterated {
                    let execution = execute_iteration_group(
                        steps,
                        &bundle,
                        &surfaces,
                        options,
                        &mut self.cpu_texture_cache,
                    )?;
                    if execution
                        .shared
                        .keys()
                        .any(|name| is_particle_state_name(name))
                    {
                        observe_particle_group(&execution.shared);
                    }
                    bundle = execution.bundle;
                    continue;
                }
                for step in steps {
                    match step {
                        RenderStep::Read { surface, .. } => {
                            bundle.image = Some(surfaces.get(surface).cloned().ok_or(
                                RenderError::UnwrittenSurface {
                                    surface: surface.clone(),
                                },
                            )?);
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
                            bundle = execute_effect_bundle_with_cache(
                                effect_id,
                                &params,
                                &inputs,
                                &bundle,
                                options,
                                &mut self.cpu_texture_cache,
                            )?;
                        }
                        RenderStep::Write { surface, .. } => {
                            let image = bundle.image.clone().ok_or_else(|| {
                                RenderError::InvalidGraph {
                                    message: format!(
                                        "write({surface}) reached execution without a current image"
                                    ),
                                }
                            })?;
                            surfaces.insert(surface.clone(), image);
                        }
                    }
                }
            }
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
    execute_effect_bundle_with_cache(effect_id, params, inputs, input_bundle, options, &mut cache)
}

fn execute_effect_bundle_with_cache(
    effect_id: &str,
    params: &BTreeMap<String, ParamValue>,
    inputs: &BTreeMap<String, Surface>,
    input_bundle: &ChainBundle,
    options: &RenderOptions,
    cache: &mut CpuTextureCache,
) -> Result<ChainBundle, RenderError> {
    execute_effect_bundle_with_state(
        effect_id,
        params,
        inputs,
        input_bundle,
        options,
        &mut BTreeMap::new(),
        cache,
    )
}

fn execute_effect_bundle_with_state(
    effect_id: &str,
    params: &BTreeMap<String, ParamValue>,
    inputs: &BTreeMap<String, Surface>,
    input_bundle: &ChainBundle,
    options: &RenderOptions,
    persistent: &mut BTreeMap<String, Surface>,
    cache: &mut CpuTextureCache,
) -> Result<ChainBundle, RenderError> {
    let catalog = effect_catalog()?;
    let effect = catalog
        .effects
        .get(effect_id)
        .ok_or_else(|| RenderError::UnknownEffect {
            effect_id: effect_id.into(),
        })?;
    let mut normalized = normalize_parameters(effect, params, options.seed)?;
    inherit_volume_size(effect_id, effect, &mut normalized, input_bundle)?;
    if effect_id == "synth/remap" {
        normalized.uniforms.insert(
            "data".into(),
            remap_uniform_data(&normalized.values, inputs, options.width, options.height),
        );
    }
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
    let mut resources = persistent.clone();
    resources.extend(inputs.clone());
    resources.extend(options.external_textures.clone());
    if let Some(image) = &input_bundle.image {
        resources.insert("inputTex".into(), image.clone());
    }
    if let Some(volume) = &input_bundle.volume {
        resources.insert("inputTex3d".into(), volume.clone());
    }
    if let Some(geometry) = &input_bundle.geometry {
        resources.insert("inputGeo".into(), geometry.clone());
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
    for (name, spec) in &effect.textures {
        if !resources.contains_key(name) {
            let (width, height) = texture_dimensions(
                spec,
                &normalized.values,
                options.width,
                options.height,
                &resources,
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
                        Some(Value::Float(value)) => *value,
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
        let pass_uniforms = apply_pass_uniform_aliases(&uniforms, &alias_sources, &aliases);
        if !pass_enabled(
            render_pass
                .execution
                .get("conditions")
                .unwrap_or(&serde_json::Value::Null),
            &pass_uniforms,
        ) {
            continue;
        }
        let repeats = repeat_count(
            render_pass
                .execution
                .get("repeat")
                .unwrap_or(&serde_json::Value::Null),
            &pass_uniforms,
        );
        if crate::draw_ops::is_draw_pass(render_pass) {
            let key = format!("{effect_id}:{}", render_pass.program);
            for _ in 0..repeats {
                crate::draw_ops::execute_draw_pass(
                    &key,
                    render_pass,
                    &pass_uniforms,
                    &mut resources,
                    options.width,
                    options.height,
                    &formats,
                )?;
            }
            if let Some(name) = render_pass.outputs.values().next() {
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
        for _ in 0..repeats {
            execute_pass(
                &program.ir,
                render_pass,
                &pass_uniforms,
                &mut resources,
                options.width,
                options.height,
                &formats,
                effect.external_texture.as_deref(),
                derivatives,
            )?;
        }
        if let Some(name) = render_pass.outputs.values().next() {
            last_output = Some(name.clone());
        }
    }
    let volume_domain = matches!(
        effect.domain.as_str(),
        "volume-generator" | "volume-filter" | "volume-renderer"
    );
    let image = resources.get("outputTex").cloned().or_else(|| {
        if volume_domain {
            input_bundle.image.clone()
        } else {
            last_output.and_then(|name| resources.get(&name).cloned())
        }
    });
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
    for output in [image.as_ref(), volume.as_ref(), geometry.as_ref()]
        .into_iter()
        .flatten()
    {
        if let Some((index, value)) = output
            .data()
            .iter()
            .copied()
            .enumerate()
            .find(|(_, value)| !value.is_finite())
        {
            return Err(RenderError::NonFiniteOutput {
                effect_id: effect_id.into(),
                channel: index,
                value,
            });
        }
    }
    *persistent = resources;
    Ok(ChainBundle {
        image,
        volume,
        geometry,
        volume_size,
    })
}

fn remap_uniform_data(
    values: &BTreeMap<String, Value>,
    inputs: &BTreeMap<String, Surface>,
    width: u32,
    height: u32,
) -> Value {
    let mut data = vec![Value::Vec(vec![0.0; 4]); 267];
    let background = match values.get("bgColor") {
        Some(Value::Vec(color)) => color.clone(),
        _ => vec![0.0; 3],
    };
    let alpha = match values.get("bgAlpha") {
        Some(Value::Float(value)) => *value,
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
        Some(Value::Float(value)) => *value,
        _ => 0.04,
    };
    data[1] = Value::Vec(vec![zone_count, smooth_edge, 0.0, 0.0]);
    for zone in 0..8 {
        let active = inputs.contains_key(&format!("zone{zone}_tex")) as u8 as f32;
        data[2 + zone] = Value::Vec(vec![0.0, active, 0.0, 1.0]);
    }
    data[266] = Value::Vec(vec![width as f32, height as f32, 0.0, 0.0]);
    Value::Array(data)
}

struct IterationExecution {
    bundle: ChainBundle,
    shared: BTreeMap<String, Surface>,
}

fn execute_iteration_group(
    steps: &[RenderStep],
    group_input: &ChainBundle,
    surfaces: &BTreeMap<String, Surface>,
    options: &RenderOptions,
    cache: &mut CpuTextureCache,
) -> Result<IterationExecution, RenderError> {
    let catalog = effect_catalog()?;
    let Some(RenderStep::Effect {
        effect_id: owner_id,
        params: owner_params,
        ..
    }) = steps.first()
    else {
        return Err(RenderError::InvalidGraph {
            message: "iterated group must begin with an effect".into(),
        });
    };
    let count = match owner_params.get("iterationCount") {
        Some(ParamValue::Int(count)) => *count,
        _ => 60,
    };
    if count <= 0 {
        return Ok(IterationExecution {
            bundle: zero_iteration_bundle(
                &catalog.effects[owner_id],
                owner_params,
                group_input,
                options,
            )?,
            shared: BTreeMap::new(),
        });
    }
    let count = u32::try_from(count).map_err(|_| RenderError::InvalidParameter {
        message: "iterationCount cannot be represented as u32".into(),
    })?;
    let owner_state_size = owner_params.get("stateSize").cloned();
    let mut states = vec![BTreeMap::<String, Surface>::new(); steps.len()];
    let mut shared = BTreeMap::<String, Surface>::new();
    let mut declared_particle_textures = BTreeMap::new();
    for step in steps {
        let RenderStep::Effect { effect_id, .. } = step else {
            continue;
        };
        for (name, spec) in &catalog.effects[effect_id].textures {
            if is_particle_state_name(name) {
                declared_particle_textures
                    .entry(name.clone())
                    .or_insert_with(|| spec.clone());
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
        shared.insert(
            "global_accum".into(),
            Surface::new(image.width(), image.height())?,
        );
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
            let normalized = normalize_parameters(definition, &params, options.seed)?;
            ensure_group_particle_resources(
                definition,
                &normalized.values,
                &declared_particle_textures,
                &mut shared,
                options,
            )?;
            for (name, surface) in &shared {
                states[index].insert(name.clone(), surface.clone());
            }
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
                effect_id,
                &params,
                &inputs,
                &step_input,
                &iteration_options,
                &mut states[index],
                cache,
            )?;
            for (name, surface) in &states[index] {
                if is_particle_state_name(name) || name == "global_accum" {
                    shared.insert(name.clone(), surface.clone());
                }
            }
        }
        final_output = step_input;
        Ok(())
    })?;
    Ok(IterationExecution {
        bundle: final_output,
        shared,
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

fn inherit_group_state_size(
    index: usize,
    definition: &crate::catalog::EffectDefinition,
    owner_state_size: Option<&ParamValue>,
    params: &mut BTreeMap<String, ParamValue>,
) {
    if index > 0 && definition.params.contains_key("stateSize") {
        if let Some(size) = owner_state_size {
            params.insert("stateSize".into(), size.clone());
        }
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

fn ensure_group_particle_resources(
    effect: &crate::catalog::EffectDefinition,
    values: &BTreeMap<String, Value>,
    declared_textures: &BTreeMap<String, serde_json::Value>,
    shared: &mut BTreeMap<String, Surface>,
    options: &RenderOptions,
) -> Result<(), RenderError> {
    let referenced = effect
        .passes
        .iter()
        .flat_map(|pass| pass.inputs.values().chain(pass.outputs.values()))
        .filter(|name| is_particle_state_name(name))
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    for name in referenced {
        if shared.contains_key(&name) {
            continue;
        }
        let fallback;
        let spec = if let Some(spec) = declared_textures.get(&name) {
            spec
        } else {
            let state_size = values
                .get("stateSize")
                .and_then(value_as_positive_u32)
                .unwrap_or(256);
            fallback = serde_json::json!({"width": state_size, "height": state_size});
            &fallback
        };
        let (width, height) =
            texture_dimensions(spec, values, options.width, options.height, shared)?;
        shared.insert(name, Surface::new(width, height)?);
    }
    Ok(())
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

fn bundle_output(
    name: Option<&str>,
    input: Option<&Surface>,
    resources: &BTreeMap<String, Surface>,
) -> Option<Surface> {
    match name {
        None | Some("inputTex" | "inputTex3d" | "inputGeo") => input.cloned(),
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
        let normalized = normalize_parameters(effect, params, options.seed)?;
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
            "outputTex3d": null,
            "outputGeo": null,
            "loopRole": null
        }))
        .unwrap();
        let values = BTreeMap::from([("stateSize".into(), Value::Int(3))]);
        let mut shared = BTreeMap::new();
        ensure_group_particle_resources(
            &effect,
            &values,
            &BTreeMap::new(),
            &mut shared,
            &RenderOptions::default(),
        )
        .unwrap();
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
            &RenderOptions::default(),
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
            "externalTexture": null, "outputTex3d": null, "outputGeo": null, "loopRole": null,
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
            &RenderOptions::default(),
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
            &group_input,
            &BTreeMap::new(),
            &options(1),
            &mut cache,
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
            &group_input,
            &BTreeMap::new(),
            &options(1),
            &mut cache,
        )
        .unwrap();
        let first_xyz = first.shared["global_xyz"].clone();
        let first_allocation = first.shared["global_xyz"].data().as_ptr();
        let changed = execute_iteration_group(
            &emitter_plan.chains[0].steps[1..2],
            &group_input,
            &BTreeMap::new(),
            &options(9),
            &mut cache,
        )
        .unwrap();
        let repeated = execute_iteration_group(
            &emitter_plan.chains[0].steps[1..2],
            &group_input,
            &BTreeMap::new(),
            &options(1),
            &mut cache,
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
        let trace = trace_particle_group(&shared);

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
        assert_eq!(data.len(), 267);
        assert_eq!(data[0], Value::Vec(vec![0.25, 0.5, 0.75, 0.8]));
        assert_eq!(data[1], Value::Vec(vec![2.0, 0.125, 0.0, 0.0]));
        assert_eq!(data[2], Value::Vec(vec![0.0, 0.0, 0.0, 1.0]));
        assert_eq!(data[3], Value::Vec(vec![0.0, 1.0, 0.0, 1.0]));
        assert_eq!(data[10], Value::Vec(vec![0.0; 4]));
        assert_eq!(data[265], Value::Vec(vec![0.0; 4]));
        assert_eq!(data[266], Value::Vec(vec![7.0, 5.0, 0.0, 0.0]));
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
