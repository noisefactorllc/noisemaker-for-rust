use noisemaker_cpu::{
    CpuRenderer, ITERATION_DELTA_TIME, IterationStep, ParamValue, RenderOptions,
    compute_iteration_groups, is_particle_state_name, iteration_schedule, wrap01,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

fn effect(id: &str) -> IterationStep {
    IterationStep::Effect {
        effect_id: id.into(),
        params: BTreeMap::new(),
    }
}

fn effect_with(id: &str, params: &[(&str, i32)]) -> IterationStep {
    IterationStep::Effect {
        effect_id: id.into(),
        params: params
            .iter()
            .map(|(name, value)| (name.to_string(), ParamValue::Int(*value)))
            .collect(),
    }
}

#[test]
fn particle_name_contract_and_pure_grouping_are_exact() {
    for name in [
        "global_xyz",
        "global_vel",
        "global_rgba",
        "global_life_data",
        "global_points_trail",
        "global_dla_base_color",
        "global_flock_base_color",
    ] {
        assert!(is_particle_state_name(name), "{name}");
    }
    for name in [
        "global_rd_state",
        "global_flow3d_state1",
        "global_accum",
        "global_base_color",
        "global__base_color",
        "global_dla_BaseColor",
        "global_dla_base_color2",
    ] {
        assert!(!is_particle_state_name(name), "{name}");
    }
    let groups = compute_iteration_groups(&[
        effect("render/pointsEmit"),
        effect("points/flow"),
        effect("filter/invert"),
        effect("synth/reactionDiffusion"),
    ])
    .unwrap();
    assert_eq!(
        groups
            .iter()
            .map(|group| (group.iterated, group.is_loop, group.steps.len()))
            .collect::<Vec<_>>(),
        [(true, false, 2), (false, false, 1), (true, false, 1)]
    );
}

#[test]
fn read_write_and_second_particle_owner_split_groups() {
    let groups = compute_iteration_groups(&[
        effect("render/pointsEmit"),
        effect("points/flow"),
        IterationStep::Write,
        IterationStep::Read,
        effect("render/pointsEmit"),
        effect("points/life"),
    ])
    .unwrap();
    assert_eq!(
        groups
            .iter()
            .map(|group| (group.iterated, group.steps.len()))
            .collect::<Vec<_>>(),
        [(true, 2), (false, 1), (false, 1), (true, 2)]
    );
}

#[test]
fn flow3d_reevaluates_animated_noise3d_per_tick_while_preserving_static_and_zero_iteration_chains()
{
    let flow = effect_with("filter3d/flow3d", &[("iterationCount", 2)]);
    let animated = effect_with("synth3d/noise3d", &[("speed", 1)]);
    let static_noise = effect_with("synth3d/noise3d", &[("speed", 0)]);
    let zero_flow = effect_with("filter3d/flow3d", &[("iterationCount", 0)]);
    let groups = |steps: &[IterationStep]| {
        compute_iteration_groups(steps)
            .unwrap()
            .into_iter()
            .map(|group| (group.iterated, group.steps.len()))
            .collect::<Vec<_>>()
    };
    assert_eq!(groups(&[animated.clone(), flow.clone()]), [(true, 2)]);
    assert_eq!(
        groups(&[static_noise.clone(), flow.clone()]),
        [(false, 1), (true, 1)]
    );
    assert_eq!(
        groups(&[animated.clone(), zero_flow]),
        [(false, 1), (true, 1)]
    );
    // The join survives boundary-free neighbors exactly like the CPU planner:
    // a second flow3d after a merged pair stays its own group.
    assert_eq!(
        groups(&[animated.clone(), flow.clone(), flow.clone()]),
        [(true, 2), (true, 1)]
    );
}

#[test]
fn points_emit_reruns_animated_image_generator_input_per_iteration_and_leaves_other_shapes_alone() {
    // The authority renders the whole chain every frame, so pointsEmit colors
    // agents from its input at that frame's time; evaluating an animated perlin
    // once would color every agent from the final frame.
    let emit = effect_with("render/pointsEmit", &[("iterationCount", 3)]);
    let zero_emit = effect_with("render/pointsEmit", &[("iterationCount", 0)]);
    let animated = |speed: f32| IterationStep::Effect {
        effect_id: "synth/perlin".into(),
        params: BTreeMap::from([("speed".to_string(), ParamValue::Float(speed))]),
    };
    let shape = |steps: &[IterationStep]| {
        compute_iteration_groups(steps)
            .unwrap()
            .into_iter()
            .map(|group| (group.iterated, group.steps.len()))
            .collect::<Vec<_>>()
    };
    let merged = compute_iteration_groups(&[animated(1.0), emit.clone()]).unwrap();
    assert_eq!(
        merged
            .iter()
            .map(|group| (group.iterated, group.steps.len()))
            .collect::<Vec<_>>(),
        [(true, 2)]
    );
    assert!(
        matches!(&merged[0].steps[0], IterationStep::Effect { effect_id, .. } if effect_id == "synth/perlin"),
        "the animated input generator re-runs inside the group"
    );
    assert_eq!(
        shape(&[animated(0.0), emit.clone()]),
        [(false, 1), (true, 1)]
    );
    assert_eq!(
        shape(&[effect("synth/perlin"), emit.clone()]),
        [(false, 1), (true, 1)]
    );
    assert_eq!(shape(&[animated(1.0), zero_emit]), [(false, 1), (true, 1)]);
    // A volume generator keeps its shape, and an iterated image generator
    // already re-runs as its own group.
    assert_eq!(
        shape(&[effect_with("synth3d/noise3d", &[("speed", 1)]), emit]),
        [(false, 1), (true, 1)]
    );
    assert_eq!(
        shape(&[
            effect_with("synth/cellularAutomata", &[("speed", 10)]),
            effect_with("render/pointsEmit", &[("iterationCount", 3)]),
        ]),
        [(true, 1), (true, 1)]
    );
}

#[test]
fn balanced_loop_is_one_group_and_planner_defends_invalid_regions() {
    let balanced = compute_iteration_groups(&[
        effect("render/loopBegin"),
        effect("filter/invert"),
        effect("render/loopEnd"),
    ])
    .unwrap();
    assert_eq!(
        (balanced.len(), balanced[0].is_loop, balanced[0].steps.len()),
        (1, true, 3)
    );

    for (steps, message) in [
        (
            vec![effect("render/loopEnd")],
            "loopEnd has no matching loopBegin",
        ),
        (
            vec![effect("render/loopBegin"), effect("filter/invert")],
            "loopBegin has no matching loopEnd",
        ),
        (
            vec![effect("render/loopBegin"), effect("render/loopBegin")],
            "Nested loop iteration groups are not supported",
        ),
        (
            vec![effect("render/loopBegin"), IterationStep::Write],
            "Loop iteration group cannot cross a read/write boundary",
        ),
    ] {
        assert!(
            compute_iteration_groups(&steps)
                .unwrap_err()
                .to_string()
                .contains(message)
        );
    }
}

#[test]
fn schedule_uses_exact_fixed_delta_frame_and_positive_wrap() {
    let schedule = iteration_schedule(4, 0.5);
    assert_eq!(schedule.len(), 4);
    assert_eq!(ITERATION_DELTA_TIME, 1.0_f32 / 600.0_f32);
    assert_eq!(schedule[3].frame, 3);
    assert_eq!(schedule[3].time, 0.5);
    assert_eq!(schedule[3].delta_time, ITERATION_DELTA_TIME);
    assert!((schedule[0].time - 0.495).abs() < 1e-6);
    let near_zero = iteration_schedule(4, 0.001);
    assert!(near_zero[0].time > 0.99 && near_zero[0].time < 1.0);
    assert_eq!(wrap01(-0.25), 0.75);
    assert!(iteration_schedule(0, 0.5).is_empty());
}

#[test]
fn cellular_automata_resets_and_matches_js_n1_n4_oracles() {
    let source = |count| {
        format!("search synth; cellularAutomata(zoom:1,seed:1,iterationCount:{count}).write(o0)")
    };
    let options = RenderOptions {
        width: 8,
        height: 8,
        time: 0.25,
        seed: 1,
        ..RenderOptions::default()
    };
    let mut renderer = CpuRenderer::new().unwrap();
    let n1 = renderer.render(&source(1), &options).unwrap().surface;
    let n4 = renderer.render(&source(4), &options).unwrap().surface;
    let n4_again = renderer.render(&source(4), &options).unwrap().surface;
    assert_ne!(n1, n4);
    assert_eq!(n4, n4_again);
    assert_eq!(
        format!("{:x}", Sha256::digest(n1.to_rgba8())),
        "880a61dab9be88cc3429b0cab3a0f8afe4c82886b921a4cbca5c39a466843810"
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(n4.to_rgba8())),
        "2e3905c0cce91bc2eab96059a46428e68232cab214fb7b1b88a064a2b0ff9ce8"
    );
}

#[test]
fn navier_stokes_matches_js_n1_n4_full_frame_oracles() {
    let source = |count| format!("search synth; navierStokes(iterationCount:{count}).write(o0)");
    let options = RenderOptions {
        width: 8,
        height: 8,
        time: 0.25,
        seed: 1,
        ..RenderOptions::default()
    };
    let mut renderer = CpuRenderer::new().unwrap();
    let n1 = renderer.render(&source(1), &options).unwrap().surface;
    let n4 = renderer.render(&source(4), &options).unwrap().surface;
    // CPU 4aeb3e1e028b2dc412401f7b7f17c014c603bf14, same DSL/options above.
    // Fresh CpuRenderer renders at n1 and n4 both produce this complete RGBA8 hash.
    assert_eq!(
        format!("{:x}", Sha256::digest(n1.to_rgba8())),
        "d7e9262577f69833a40c660be5e8dc15ae12a150d8cb72702782d3da0b87dcfa"
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(n4.to_rgba8())),
        "d7e9262577f69833a40c660be5e8dc15ae12a150d8cb72702782d3da0b87dcfa"
    );
}

#[test]
fn zero_iteration_image_effect_returns_byte_exact_input() {
    let options = RenderOptions {
        width: 4,
        height: 4,
        ..RenderOptions::default()
    };
    let mut renderer = CpuRenderer::new().unwrap();
    let direct = renderer
        .render("search synth; solid(color:#369).write(o0)", &options)
        .unwrap()
        .surface;
    let bypassed = renderer
        .render(
            "search synth,filter; solid(color:#369).feedback(iterationCount:0).write(o0)",
            &options,
        )
        .unwrap()
        .surface;
    assert_eq!(direct, bypassed);
}
