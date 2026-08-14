use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use noisemaker_cpu::catalog::{EffectDefinition, ParameterSpec, effect_catalog, shader_bundle};
use noisemaker_cpu::{
    CpuRenderer, ParamValue, RenderOptions, RenderStep, Surface, compile_dsl, draw_op_keys,
    fragment_adapter_keys,
};
use serde_json::Value as JsonValue;

// Exact Task 7 smoke checkpoints. Keeping the evidence ranges executable
// prevents individually green batch reports from concealing a gap or overlap.
const DEFAULT_SMOKE_BATCHES: &[(usize, usize)] = &[
    (0, 20),
    (20, 40),
    (40, 60),
    (60, 80),
    (80, 100),
    (100, 120),
    (120, 136),
    (136, 137),
    (137, 140),
    (140, 160),
    (160, 180),
    (180, 205),
];

fn literal(value: &JsonValue) -> String {
    match value {
        JsonValue::Null => "none".into(),
        JsonValue::Bool(value) => value.to_string(),
        JsonValue::Number(value) => value.to_string(),
        JsonValue::String(value) if value == "none" => value.clone(),
        JsonValue::String(value) => value.clone(),
        JsonValue::Array(values) => format!(
            "[{}]",
            values.iter().map(literal).collect::<Vec<_>>().join(",")
        ),
        JsonValue::Object(_) => panic!("object is not a DSL literal: {value}"),
    }
}

fn bounded_arguments(effect: &EffectDefinition, choice: Option<(&str, &JsonValue)>) -> Vec<String> {
    let mut args = BTreeMap::<String, String>::new();
    if effect.iterated {
        args.insert(
            "iterationCount".into(),
            if effect.domain == "image" { "4" } else { "1" }.into(),
        );
    }
    if effect.params.contains_key("volumeSize") {
        args.insert("volumeSize".into(), "2".into());
    }
    if effect.params.contains_key("stateSize") {
        args.insert("stateSize".into(), "64".into());
    }
    if let Some(spec) = effect.params.get("searchRadius") {
        if let Some(minimum) = spec.metadata.get("min") {
            args.insert("searchRadius".into(), literal(minimum));
        }
    }
    if effect.namespace == "synth3d" && effect.func == "flythrough3d" {
        args.insert("type".into(), "1".into());
    }
    if let Some((name, value)) = choice {
        args.insert(name.into(), literal(value));
    }
    args.into_iter()
        .map(|(name, value)| format!("{name}:{value}"))
        .collect()
}

fn surface_params(effect: &EffectDefinition) -> Vec<String> {
    effect
        .param_names
        .iter()
        .filter(|name| effect.params[*name].parameter_type == "surface")
        .cloned()
        .collect()
}

fn smoke_program(
    id: &str,
    effect: &EffectDefinition,
    choice: Option<(&str, &JsonValue)>,
) -> String {
    let mut args = bounded_arguments(effect, choice);
    let surfaces = if effect.kind == "generator" {
        Vec::new()
    } else {
        surface_params(effect)
    };
    let mut prefix = Vec::new();
    for (index, name) in surfaces.into_iter().enumerate() {
        if index == 0 {
            args.push(format!("{name}:inputTex"));
            continue;
        }
        let next_surface = index - 1;
        prefix.push(format!("solid(color:#58c).write(o{next_surface})"));
        args.push(format!("{name}:o{next_surface}"));
    }
    let call = format!("{}({})", effect.func, args.join(","));
    let search = format!(
        "search {},synth,synth3d,filter3d,filter,render,points,mixer,classicNoisedeck",
        effect.namespace
    );
    let chain = match effect.domain.as_str() {
        "loop-begin" => format!("solid(color:#58c).{call}.loopEnd().write(o0)"),
        "loop-end" => format!("solid(color:#58c).loopBegin(iterationCount:1).{call}.write(o0)"),
        "volume-generator" => format!("{call}.render3d(volumeSize:2).write(o0)"),
        "volume-filter" => {
            format!("noise3d(volumeSize:2).{call}.render3d(volumeSize:2).write(o0)")
        }
        "volume-renderer" => format!("noise3d(volumeSize:2).{call}.write(o0)"),
        "image"
            if effect.namespace == "points"
                || matches!(
                    id,
                    "render/pointsEmit" | "render/pointsRender" | "render/pointsBillboardRender"
                ) =>
        {
            let middle = if id == "render/pointsEmit" {
                call.clone()
            } else {
                format!("pointsEmit(stateSize:64,iterationCount:4).{call}")
            };
            let render = if matches!(id, "render/pointsRender" | "render/pointsBillboardRender") {
                String::new()
            } else {
                ".pointsRender(iterationCount:4)".into()
            };
            format!("solid(color:#58c).{middle}{render}.write(o0)")
        }
        "image" if effect.kind == "generator" => format!("{call}.write(o0)"),
        "image" => {
            format!("solid(color:#58c).{call}.write(o7)")
        }
        domain => panic!("unhandled domain {domain:?} for {id}"),
    };
    let mut statements = prefix;
    statements.push(chain);
    let target = if effect.domain == "image"
        && effect.kind != "generator"
        && effect.namespace != "points"
        && !matches!(
            id,
            "render/pointsEmit" | "render/pointsRender" | "render/pointsBillboardRender"
        ) {
        "o7"
    } else {
        "o0"
    };
    format!("{search}; {}; render({target})", statements.join("; "))
}

fn render_smoke(
    renderer: &mut CpuRenderer,
    source: &str,
    options: &RenderOptions,
) -> Result<noisemaker_cpu::RenderResult, noisemaker_cpu::RenderError> {
    let plan = compile_dsl(source, "<smoke>")?;
    renderer.render_plan(plan, options)
}

fn smoke_options(effect: &EffectDefinition) -> RenderOptions {
    let size = if effect.iterated { 16 } else { 2 };
    let mut external_textures = BTreeMap::new();
    let mut texture = Surface::new(size, size).unwrap();
    texture.clear([0.2, 0.4, 0.6, 1.0]);
    external_textures.insert("imageTex".into(), texture.clone());
    external_textures.insert("textTex".into(), texture);
    RenderOptions {
        width: size,
        height: size,
        time: 0.25,
        seed: 3,
        external_textures,
        ..RenderOptions::default()
    }
}

#[test]
fn exact_smoke_inventory_and_registry_coverage_are_locked() {
    let catalog = effect_catalog().unwrap();
    assert_eq!(catalog.effects.len(), 205);
    assert_eq!(shader_bundle().unwrap().programs.len(), 288);
    assert_eq!(
        catalog
            .effects
            .values()
            .filter(|effect| effect.domain != "image")
            .count(),
        15
    );
    assert_eq!(
        catalog
            .effects
            .values()
            .filter(|effect| effect.iterated)
            .count(),
        25
    );
    assert_eq!(fragment_adapter_keys().len(), 4);
    assert_eq!(draw_op_keys().len(), 7);
    let shaders = &shader_bundle().unwrap().programs;
    let adapters = fragment_adapter_keys().into_iter().collect::<BTreeSet<_>>();
    let draws = draw_op_keys().into_iter().collect::<BTreeSet<_>>();
    let mut missing = Vec::new();
    for (id, effect) in &catalog.effects {
        for pass in &effect.passes {
            let key = format!("{id}:{}", pass.program);
            if noisemaker_cpu::draw_ops::is_draw_pass(pass) {
                if !draws.contains(key.as_str()) {
                    missing.push(key);
                }
            } else if !shaders.contains_key(pass.key.as_deref().unwrap_or_default())
                && !adapters.contains(key.as_str())
            {
                missing.push(key);
            }
        }
    }
    assert!(
        missing.is_empty(),
        "missing execution registrations: {missing:?}"
    );
}

#[test]
fn completed_default_smoke_batches_cover_205_exactly_once() {
    let mut coverage = vec![0_u8; 205];
    for &(start, end) in DEFAULT_SMOKE_BATCHES {
        assert!(
            start < end && end <= coverage.len(),
            "invalid batch {start}:{end}"
        );
        for count in &mut coverage[start..end] {
            *count += 1;
        }
    }
    assert_eq!(coverage, vec![1; 205]);
}

#[test]
fn all_205_default_programs_are_finite_deterministic_and_aggregated() {
    let started = Instant::now();
    let catalog = effect_catalog().unwrap();
    let mut failures = Vec::new();
    let mut completed = 0;
    let start = std::env::var("NM_SMOKE_START")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let end = std::env::var("NM_SMOKE_END")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(catalog.effects.len());
    let only = std::env::var("NM_SMOKE_ONLY").ok();
    let selected = catalog
        .effects
        .iter()
        .enumerate()
        .filter(|(index, (id, _))| {
            only.as_ref()
                .map_or(*index >= start && *index < end, |only| id.as_str() == only)
        })
        .map(|(_, effect)| effect)
        .collect::<Vec<_>>();
    let expected = selected.len();
    for (id, effect) in selected {
        let case_started = Instant::now();
        eprintln!("default smoke begin: {id}");
        let source = smoke_program(id, effect, None);
        let options = smoke_options(effect);
        let result = (|| {
            let mut renderer = CpuRenderer::new()?;
            let first = render_smoke(&mut renderer, &source, &options)?.surface;
            let repeated = render_smoke(&mut renderer, &source, &options)?.surface;
            if first != repeated {
                return Err(noisemaker_cpu::RenderError::InvalidGraph {
                    message: "immediate repeated render differs".into(),
                });
            }
            if first.width() != options.width
                || first.height() != options.height
                || first.data().iter().any(|value| !value.is_finite())
            {
                return Err(noisemaker_cpu::RenderError::InvalidGraph {
                    message: "output dimensions or finiteness violated".into(),
                });
            }
            Ok(())
        })();
        match result {
            Ok(()) => {
                completed += 1;
                eprintln!("default smoke ok: {id} ({:?})", case_started.elapsed());
            }
            Err(error) => {
                eprintln!("default smoke failed: {id} ({:?})", case_started.elapsed());
                failures.push(format!("{id}\n  {source}\n  {error}"));
            }
        }
    }
    eprintln!(
        "default smoke batch: {completed}/{expected} ({:?})",
        started.elapsed()
    );
    assert_eq!(completed, expected, "{}", failures.join("\n"));
}

fn choice_values(spec: &ParameterSpec) -> Vec<&JsonValue> {
    if spec.define.is_none() {
        return Vec::new();
    }
    spec.metadata
        .get("choices")
        .and_then(JsonValue::as_object)
        .into_iter()
        .flat_map(|choices| choices.values())
        .filter(|value| !value.is_null())
        .collect()
}

fn oracle_non_finite_choice(id: &str, name: &str, value: &JsonValue) -> Option<&'static str> {
    if id == "classicNoisedeck/effects"
        && name == "effect"
        && matches!(value.as_i64(), Some(2 | 3 | 9))
    {
        Some(concat!(
            "the canonical zero-sum convolution divides by its zero kernel weight; the JS CPU ",
            "renderer returns NaN floats and only Surface.toRgba8 maps them to zero, while ",
            "Rust intentionally rejects a non-finite final surface"
        ))
    } else {
        None
    }
}

#[test]
fn all_456_non_null_compile_choices_execute_without_skips() {
    let catalog = effect_catalog().unwrap();
    let mut failures = Vec::new();
    let mut executed = 0;
    let mut finite = 0;
    let mut expected_non_finite = BTreeSet::new();
    let start = std::env::var("NM_CHOICE_START")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let end = std::env::var("NM_CHOICE_END")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(456);
    let only = std::env::var("NM_CHOICE_ONLY").ok();
    let mut catalog_index = 0;
    for (id, effect) in &catalog.effects {
        for (name, spec) in &effect.params {
            for value in choice_values(spec) {
                let selected = only
                    .as_ref()
                    .map_or(catalog_index >= start && catalog_index < end, |only| {
                        id == only
                    });
                catalog_index += 1;
                if !selected {
                    continue;
                }
                executed += 1;
                let case_started = Instant::now();
                eprintln!("choice smoke begin: {id}:{name}={}", literal(value));
                let source = smoke_program(id, effect, Some((name, value)));
                let options = RenderOptions {
                    width: 2,
                    height: 2,
                    ..smoke_options(effect)
                };
                match CpuRenderer::new()
                    .and_then(|mut renderer| render_smoke(&mut renderer, &source, &options))
                {
                    Ok(result) if result.surface.data().iter().all(|value| value.is_finite()) => {
                        finite += 1;
                        eprintln!(
                            "choice smoke ok: {id}:{name}={} ({:?})",
                            literal(value),
                            case_started.elapsed()
                        );
                    }
                    Ok(_) => failures.push(format!("{id}:{name}={} non-finite", literal(value))),
                    Err(noisemaker_cpu::RenderError::NonFiniteOutput {
                        effect_id: ref error_id,
                        channel: 0,
                        value: output,
                    }) if oracle_non_finite_choice(id, name, value).is_some()
                        && error_id == id
                        && output.is_nan() =>
                    {
                        expected_non_finite.insert(format!("{id}:{name}={}", literal(value)));
                        eprintln!(
                            "choice smoke expected typed non-finite rejection: {}:{}={} ({:?}): {}",
                            id,
                            name,
                            literal(value),
                            case_started.elapsed(),
                            oracle_non_finite_choice(id, name, value).unwrap()
                        );
                    }
                    Err(error) => failures.push(format!(
                        "{id}:{name}={}\n  {source}\n  {error}",
                        literal(value)
                    )),
                }
            }
        }
    }
    assert_eq!(catalog_index, 456);
    let expected = if only.is_some() {
        executed
    } else {
        end.min(456).saturating_sub(start.min(456))
    };
    assert_eq!(executed, expected);
    if only.is_none() && start == 0 && end >= 456 {
        assert_eq!(finite, 453);
        assert_eq!(
            expected_non_finite,
            BTreeSet::from([
                "classicNoisedeck/effects:effect=2".into(),
                "classicNoisedeck/effects:effect=3".into(),
                "classicNoisedeck/effects:effect=9".into(),
            ])
        );
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_iterated_default_is_60_and_one_default_reaches_execution() {
    let catalog = effect_catalog().unwrap();
    for (id, effect) in &catalog.effects {
        if effect.iterated {
            assert_eq!(
                effect.params["iterationCount"].default,
                JsonValue::from(60),
                "{id}"
            );
        }
        if let Some(state_size) = effect.params.get("stateSize") {
            assert_ne!(
                state_size.default,
                JsonValue::from(4),
                "{id} stateSize default must remain independent of smoke override"
            );
        }
        if let Some(search_radius) = effect.params.get("searchRadius") {
            assert_ne!(
                search_radius.default, search_radius.metadata["min"],
                "{id} searchRadius default must remain independent of smoke minimum"
            );
        }
    }
    let mut renderer = CpuRenderer::new().unwrap();
    let output = renderer
        .render(
            "search synth; cellularAutomata(zoom:32,seed:4).write(o0)",
            &RenderOptions {
                width: 2,
                height: 2,
                ..RenderOptions::default()
            },
        )
        .unwrap();
    assert_eq!((output.surface.width(), output.surface.height()), (2, 2));
}

#[test]
fn smoke_builder_wires_all_nine_mashup_surfaces() {
    let effect = &effect_catalog().unwrap().effects["mixer/mashup"];
    let source = smoke_program("mixer/mashup", effect, None);
    for name in [
        "source",
        "layer0_tex",
        "layer1_tex",
        "layer2_tex",
        "layer3_tex",
        "layer4_tex",
        "layer5_tex",
        "layer6_tex",
        "layer7_tex",
    ] {
        assert!(
            source.contains(&format!("{name}:o")) || source.contains(&format!("{name}:inputTex")),
            "{source}"
        );
    }
}

#[test]
fn smoke_plan_keeps_particle_state_at_the_schema_minimum() {
    let effect = &effect_catalog().unwrap().effects["points/flock"];
    let source = smoke_program("points/flock", effect, None);
    assert!(source.contains("pointsEmit(stateSize:64,iterationCount:4)"));
    let plan = compile_dsl(&source, "<fixture-test>").unwrap();
    assert!(plan.chains.iter().flat_map(|chain| &chain.steps).any(|step| {
        matches!(step, RenderStep::Effect { params, .. } if params.get("stateSize") == Some(&ParamValue::Int(64)))
    }));
}

#[test]
fn smoke_generator_omits_optional_current_surface_default() {
    let effect = &effect_catalog().unwrap().effects["synth/cellularAutomata"];
    let source = smoke_program("synth/cellularAutomata", effect, None);
    assert!(!source.contains("tex:inputTex"), "{source}");
    compile_dsl(&source, "<fixture-test>").unwrap();
}
