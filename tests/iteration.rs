use noisemaker_cpu::{
    CpuRenderer, ITERATION_DELTA_TIME, IterationStep, RenderOptions, compute_iteration_groups,
    is_particle_state_name, iteration_schedule, wrap01,
};
use sha2::{Digest, Sha256};

fn effect(id: &str) -> IterationStep {
    IterationStep::Effect {
        effect_id: id.into(),
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
    ] {
        assert!(is_particle_state_name(name), "{name}");
    }
    for name in ["global_rd_state", "global_flow3d_state1", "global_accum"] {
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
        "27776d19891e6feb4b7df91836fdc318a86edf3b999c081597d8bbf95cddd925"
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(n4.to_rgba8())),
        "72ef208d71700639a38f397fc2b71aec637c441e539769c228d364dbebd5fce7"
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
    assert_eq!(
        format!("{:x}", Sha256::digest(n1.to_rgba8())),
        "cf84752f2ff5e9b493c3e9fe2a05574c333fc96602147d763d4e250542c06296"
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(n4.to_rgba8())),
        "ac1bd04a4754ef4d78d3f20c403d535537f180ae018603de63c6a760216481e2"
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
