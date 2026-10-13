use std::collections::BTreeMap;

use noisemaker_cpu::catalog::{RenderPass, effect_catalog};
use noisemaker_cpu::draw_ops::{billboard_hash, billboard_shape_alpha, compute_clip_center};
use noisemaker_cpu::{
    CpuRenderer, RenderOptions, Surface, TextureFormat, Value, draw_op_keys, execute_draw_pass,
    scatter_point_pixel,
};

fn state(width: u32, height: u32, pixels: &[[f32; 4]]) -> Surface {
    Surface::from_f32(width, height, pixels.iter().flatten().copied().collect()).unwrap()
}

fn draw_pass(effect: &str, program: &str) -> &'static RenderPass {
    effect_catalog().unwrap().effects[effect]
        .passes
        .iter()
        .find(|pass| pass.program == program)
        .unwrap()
}

fn named_pass(effect: &str, name: &str) -> &'static RenderPass {
    effect_catalog().unwrap().effects[effect]
        .passes
        .iter()
        .find(|pass| pass.name == name)
        .unwrap()
}

fn billboard_pass(name: &str) -> RenderPass {
    let pass = named_pass("render/pointsBillboardRender", name);
    let mut inputs = pass.inputs.clone();
    inputs.insert("orderTex".into(), "depthOrderA".into());
    inputs.insert("spriteMeanTex".into(), "spriteMean".into());
    RenderPass {
        name: pass.name.clone(),
        program: pass.program.clone(),
        key: pass.key.clone(),
        inputs,
        outputs: pass.outputs.clone(),
        execution: pass.execution.clone(),
    }
}

#[test]
fn draw_registry_and_point_origin_rules_are_exact() {
    assert_eq!(
        draw_op_keys(),
        [
            "filter/wormhole:deposit",
            "filter3d/flow3d:deposit",
            "points/dla:depositGrid",
            "points/lenia:deposit",
            "points/physarum:deposit",
            "render/meshRender:render",
            "render/pointsBillboardRender:deposit",
            "render/pointsRender:deposit",
        ]
    );
    assert_eq!(scatter_point_pixel(0.0, 0.0, 1.0, 2, 3), Some(12));
    assert_eq!(scatter_point_pixel(-0.75, -0.8, 1.0, 4, 5), Some(64));
    for (x, y, w) in [
        (0.0, 0.0, 0.0),
        (0.0, 0.0, -1.0),
        (2.0, 0.0, 1.0),
        (f32::NAN, 0.0, 1.0),
        (0.0, f32::INFINITY, 1.0),
    ] {
        assert_eq!(scatter_point_pixel(x, y, w, 4, 5), None);
    }
}

#[test]
fn dla_colliding_stuck_agents_add_and_unstuck_agents_do_nothing() {
    let mut resources = BTreeMap::from([
        (
            "global_xyz".into(),
            state(2, 1, &[[0.5, 0.5, 0.0, 1.0], [0.5, 0.5, 0.0, 1.0]]),
        ),
        (
            "global_vel".into(),
            state(2, 1, &[[0.0, 1.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0]]),
        ),
        (
            "global_rgba".into(),
            state(2, 1, &[[0.5, 0.5, 0.0, 0.1], [0.5, 0.5, 0.0, 0.9]]),
        ),
    ]);
    let uniforms = BTreeMap::from([("deposit".into(), Value::Float(5.0))]);
    let formats = BTreeMap::from([("global_dla_grid".into(), TextureFormat::Rgba32f)]);
    let pixels = execute_draw_pass(
        "points/dla:depositGrid",
        draw_pass("points/dla", "depositGrid"),
        &uniforms,
        &mut resources,
        1,
        1,
        &formats,
    )
    .unwrap();
    assert_eq!(pixels, 2);
    assert_eq!(resources["global_dla_grid"].data(), &[0.5, 0.5, 0.0, 1.0]);

    resources.insert(
        "global_vel".into(),
        state(2, 1, &[[0.0, 0.0, 0.0, 0.0], [0.0, 0.49, 0.0, 0.0]]),
    );
    resources.remove("global_dla_grid");
    assert_eq!(
        execute_draw_pass(
            "points/dla:depositGrid",
            draw_pass("points/dla", "depositGrid"),
            &uniforms,
            &mut resources,
            1,
            1,
            &formats,
        )
        .unwrap(),
        0
    );
    assert_eq!(resources["global_dla_grid"].data(), &[0.0; 4]);
}

#[test]
fn unknown_draw_key_returns_typed_error_before_program_lookup() {
    let pass = RenderPass {
        name: "draw".into(),
        program: "missing".into(),
        key: None,
        inputs: BTreeMap::new(),
        outputs: BTreeMap::from([("fragColor".into(), "outputTex".into())]),
        execution: BTreeMap::from([("drawMode".into(), serde_json::json!("points"))]),
    };
    let error = execute_draw_pass(
        "synth/missing:missing",
        &pass,
        &BTreeMap::new(),
        &mut BTreeMap::new(),
        1,
        1,
        &BTreeMap::new(),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        noisemaker_cpu::RenderError::MissingDrawOp { key, pass }
            if key == "synth/missing:missing" && pass == "draw"
    ));
}

#[test]
fn lenia_physarum_and_points_render_match_constant_scaled_and_view_rules() {
    let xyz = state(2, 1, &[[0.5, 0.5, 0.0, 1.0], [0.25, 0.25, 0.0, 0.0]]);
    let rgba = state(2, 1, &[[0.2, 0.4, 0.6, 0.8], [1.0; 4]]);

    let mut lenia_resources = BTreeMap::from([("global_xyz".into(), xyz.clone())]);
    execute_draw_pass(
        "points/lenia:deposit",
        draw_pass("points/lenia", "deposit"),
        &BTreeMap::from([("depositAmount".into(), Value::Float(0.25))]),
        &mut lenia_resources,
        1,
        1,
        &BTreeMap::from([("global_lenia_density".into(), TextureFormat::Rgba32f)]),
    )
    .unwrap();
    assert_eq!(
        lenia_resources["global_lenia_density"].data(),
        &[0.25, 0.0, 0.0, 1.0]
    );

    let mut physarum_resources = BTreeMap::from([
        ("global_xyz".into(), xyz.clone()),
        ("global_rgba".into(), rgba.clone()),
    ]);
    execute_draw_pass(
        "points/physarum:deposit",
        draw_pass("points/physarum", "deposit"),
        &BTreeMap::from([("deposit".into(), Value::Float(0.5))]),
        &mut physarum_resources,
        1,
        1,
        &BTreeMap::from([("global_physarum_pheromone".into(), TextureFormat::Rgba32f)]),
    )
    .unwrap();
    assert_eq!(
        physarum_resources["global_physarum_pheromone"].data(),
        &[0.1, 0.2, 0.3, 0.4]
    );

    let view_uniforms = BTreeMap::from([
        ("density".into(), Value::Float(100.0)),
        ("viewMode".into(), Value::Int(0)),
        ("rotateX".into(), Value::Float(0.0)),
        ("rotateY".into(), Value::Float(0.0)),
        ("rotateZ".into(), Value::Float(0.0)),
        ("posX".into(), Value::Float(0.0)),
        ("posY".into(), Value::Float(0.0)),
        ("viewScale".into(), Value::Float(1.0)),
    ]);
    assert_eq!(
        compute_clip_center(0.5, 0.25, 999.0, &view_uniforms, 1, 1).unwrap(),
        Some([0.0, -0.5, 80.0, 0.0, 1.0])
    );
    let mut ortho = view_uniforms.clone();
    ortho.insert("viewMode".into(), Value::Int(1));
    assert_eq!(
        compute_clip_center(0.5, 0.5, 0.0, &ortho, 1, 1).unwrap(),
        Some([0.0, 0.0, 80.0, 80.0, 1.0])
    );
    let mut points_resources =
        BTreeMap::from([("global_xyz".into(), xyz), ("global_rgba".into(), rgba)]);
    execute_draw_pass(
        "render/pointsRender:deposit",
        draw_pass("render/pointsRender", "deposit"),
        &view_uniforms,
        &mut points_resources,
        1,
        1,
        &BTreeMap::from([("global_points_trail".into(), TextureFormat::Rgba32f)]),
    )
    .unwrap();
    assert_eq!(
        points_resources["global_points_trail"].data(),
        &[0.2, 0.4, 0.6, 0.8]
    );
}

#[test]
fn flow3d_flattens_z_limits_prefix_and_adds_opaque_alpha() {
    let mut resources = BTreeMap::from([
        (
            "global_flow3d_state1".into(),
            state(2, 1, &[[0.25, 0.25, 0.1, 1.0], [1.25, 0.25, 1.1, 1.0]]),
        ),
        (
            "global_flow3d_state2".into(),
            state(2, 1, &[[1.0, 0.0, 0.0, 0.1], [0.0, 1.0, 0.5, 0.2]]),
        ),
    ]);
    let formats = BTreeMap::from([("global_flow3d_trail".into(), TextureFormat::Rgba32f)]);
    let mut uniforms = BTreeMap::from([
        ("density".into(), Value::Float(5.0)),
        ("volumeSize".into(), Value::Int(2)),
    ]);
    assert_eq!(
        execute_draw_pass(
            "filter3d/flow3d:deposit",
            draw_pass("filter3d/flow3d", "deposit"),
            &uniforms,
            &mut resources,
            9,
            9,
            &formats,
        )
        .unwrap(),
        2
    );
    let output = &resources["global_flow3d_trail"];
    assert_eq!(&output.data()[24..28], &[1.0, 0.0, 0.0, 1.0]);
    assert_eq!(&output.data()[12..16], &[0.0, 1.0, 0.5, 1.0]);

    resources.remove("global_flow3d_trail");
    uniforms.insert("density".into(), Value::Float(2.0));
    assert_eq!(
        execute_draw_pass(
            "filter3d/flow3d:deposit",
            draw_pass("filter3d/flow3d", "deposit"),
            &uniforms,
            &mut resources,
            9,
            9,
            &formats,
        )
        .unwrap(),
        0
    );
}

/// Writes one agent texel through the shader-space row convention the deposit
/// adapters read with (`storageRow = height - 1 - shaderY`), mirroring the
/// upstream scatter-adapter fixtures.
fn poke_agent(surface: &mut Surface, sx: usize, sy: usize, rgba: [f32; 4]) {
    let storage_row = surface.height() as usize - 1 - sy;
    let offset = (storage_row * surface.width() as usize + sx) * 4;
    surface.data_mut()[offset..offset + 4].copy_from_slice(&rgba);
}

fn pixel_at(surface: &Surface, storage_row: usize, col: usize) -> [f32; 4] {
    let offset = (storage_row * surface.width() as usize + col) * 4;
    let data = surface.data();
    [
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]
}

fn flow3d_deposit_pixels(
    volume_size: i32,
    density: f64,
    state1: Surface,
    state2: Surface,
    destination: Surface,
) -> (usize, Surface) {
    let mut resources = BTreeMap::from([
        ("global_flow3d_state1".into(), state1),
        ("global_flow3d_state2".into(), state2),
        ("global_flow3d_trail".into(), destination),
    ]);
    let formats = BTreeMap::from([("global_flow3d_trail".into(), TextureFormat::Rgba32f)]);
    let uniforms = BTreeMap::from([
        ("density".into(), Value::Float(density)),
        ("volumeSize".into(), Value::Int(volume_size)),
    ]);
    let agent_width = resources["global_flow3d_state1"].width();
    let pixels = execute_draw_pass(
        "filter3d/flow3d:deposit",
        draw_pass("filter3d/flow3d", "deposit"),
        &uniforms,
        &mut resources,
        agent_width,
        1,
        &formats,
    )
    .unwrap();
    let trail = resources.remove("global_flow3d_trail").unwrap();
    (pixels, trail)
}

fn agents(width: u32, pokes: &[(usize, [f32; 4])]) -> Surface {
    let mut surface = Surface::new(width, 1).unwrap();
    for &(sx, rgba) in pokes {
        poke_agent(&mut surface, sx, 0, rgba);
    }
    surface
}

#[test]
fn flow3d_deposit_maps_a_3d_agent_position_into_the_flattened_volume_atlas() {
    // Shader row 0 is deliberately off-atlas; shader row 1 is the asserted agent.
    let mut off_atlas = Surface::new(1, 2).unwrap();
    poke_agent(&mut off_atlas, 0, 0, [99.0, 99.0, 99.0, 1.0]);
    poke_agent(&mut off_atlas, 0, 1, [1.25, 2.25, 2.9, 0.0]);
    let mut state2 = Surface::new(1, 2).unwrap();
    poke_agent(&mut state2, 0, 0, [1.0, 1.0, 1.0, 1.0]);
    poke_agent(&mut state2, 0, 1, [0.25, 0.5, 0.75, 0.1]);

    let (pixels, trail) =
        flow3d_deposit_pixels(4, 100.0, off_atlas, state2, Surface::new(4, 16).unwrap());
    assert_eq!(pixels, 1);
    // atlasX=1.25 -> col 1; atlasY=2.25+floor(2.9)*4=10.25 -> GL row 10
    // -> storage row 16-1-10=5. state1.w is intentionally ignored upstream.
    assert_eq!(pixel_at(&trail, 5, 1), [0.25, 0.5, 0.75, 1.0]);
    assert_eq!(pixel_at(&trail, 10, 1), [0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn flow3d_deposit_places_integer_atlas_boundaries_in_the_lower_gl_pixel() {
    let state1 = agents(
        2,
        &[
            (0, [20.0, 1.959_960_9, 5.7421875, 0.0]),
            (1, [20.71875, 17.0, 29.671875, 0.0]),
        ],
    );
    let state2 = agents(2, &[(0, [0.5, 0.0, 0.0, 1.0]), (1, [0.25, 0.0, 0.0, 1.0])]);

    let (_, trail) =
        flow3d_deposit_pixels(32, 100.0, state1, state2, Surface::new(32, 1024).unwrap());
    assert_eq!(pixel_at(&trail, 862, 19), [0.5, 0.0, 0.0, 1.0]);
    assert_eq!(pixel_at(&trail, 862, 20), [0.0, 0.0, 0.0, 0.0]);
    assert_eq!(pixel_at(&trail, 79, 20), [0.25, 0.0, 0.0, 1.0]);
    assert_eq!(pixel_at(&trail, 78, 20), [0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn flow3d_deposit_rounds_the_atlas_addition_as_glsl_float_before_raster_placement() {
    let state1 = agents(1, &[(0, [1.25, 0.000_122_070_31, 63.0, 0.0])]);
    let state2 = agents(1, &[(0, [0.5, 0.0, 0.0, 1.0])]);

    let (_, trail) =
        flow3d_deposit_pixels(64, 100.0, state1, state2, Surface::new(64, 4096).unwrap());
    assert_eq!(pixel_at(&trail, 64, 1), [0.5, 0.0, 0.0, 1.0]);
    assert_eq!(pixel_at(&trail, 63, 1), [0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn flow3d_deposit_culls_points_inside_the_m4_viewport_subpixel_edge() {
    for (volume_size, position) in [
        (32i32, [0.000_321_388_24_f32, 22.546875, 13.6015625, 0.0]),
        (32, [1.0 / 512.0, 22.546875, 13.6015625, 0.0]),
        (64, [1.25, 0.000_061_035_156, 0.0, 0.0]),
        (64, [1.25, 1.0 / 512.0, 0.0, 0.0]),
    ] {
        let state1 = agents(1, &[(0, position)]);
        let state2 = agents(1, &[(0, [0.5, 0.0, 0.0, 1.0])]);
        let (pixels, trail) = flow3d_deposit_pixels(
            volume_size,
            100.0,
            state1,
            state2,
            Surface::new(volume_size as u32, (volume_size * volume_size) as u32).unwrap(),
        );
        assert_eq!(pixels, 0);
        assert!(trail.data().iter().all(|value| *value == 0.0));
    }
}

#[test]
fn flow3d_deposit_snaps_near_integer_atlas_positions_to_the_m4_subpixel_grid() {
    let state1 = agents(
        2,
        &[
            (0, [1.000_976_6, 0.25, 1.1, 0.0]),
            (1, [2.25, 1.000_976_6, 1.1, 0.0]),
        ],
    );
    let state2 = agents(2, &[(0, [0.5, 0.0, 0.0, 1.0]), (1, [0.25, 0.0, 0.0, 1.0])]);

    let (_, trail) =
        flow3d_deposit_pixels(32, 100.0, state1, state2, Surface::new(32, 1024).unwrap());
    assert_eq!(pixel_at(&trail, 991, 0), [0.5, 0.0, 0.0, 1.0]);
    assert_eq!(pixel_at(&trail, 991, 1), [0.0, 0.0, 0.0, 0.0]);
    assert_eq!(pixel_at(&trail, 991, 2), [0.25, 0.0, 0.0, 1.0]);
    assert_eq!(pixel_at(&trail, 990, 2), [0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn flow3d_deposit_additively_accumulates_colliding_agent_colors_with_opaque_alpha() {
    let state1 = agents(
        2,
        &[(0, [2.25, 1.25, 1.1, 1.0]), (1, [2.25, 1.25, 1.9, 1.0])],
    );
    let state2 = agents(2, &[(0, [1.0, 0.25, 0.0, 0.0]), (1, [0.0, 0.5, 1.0, 0.0])]);

    let (pixels, trail) =
        flow3d_deposit_pixels(4, 100.0, state1, state2, Surface::new(4, 16).unwrap());
    assert_eq!(pixels, 2);
    assert_eq!(pixel_at(&trail, 10, 2), [1.0, 0.75, 1.0, 2.0]);
}

#[test]
fn billboard_hash_shapes_sprite_rotation_and_both_blends_are_exact() {
    assert!((billboard_hash(0.0, 42.0) - 0.07695067745562426).abs() < 1e-15);
    assert!((billboard_hash(1234.5, 42.0) - 0.9033963931499507).abs() < 1e-15);
    assert_eq!(billboard_shape_alpha(1, 0.5, 0.5), 1.0);
    assert_eq!(billboard_shape_alpha(2, 0.5, 0.5), 0.0);
    assert_eq!(billboard_shape_alpha(7, 0.5, 0.5), 1.0);

    let base_uniforms = BTreeMap::from([
        ("density".into(), Value::Float(100.0)),
        ("shapeMode".into(), Value::Int(1)),
        ("depositOpacity".into(), Value::Float(100.0)),
        ("seed".into(), Value::Int(42)),
        ("sizeVariation".into(), Value::Float(0.0)),
        ("rotationVar".into(), Value::Float(100.0)),
        ("pointSize".into(), Value::Float(1.0)),
        ("viewMode".into(), Value::Int(0)),
        ("rotateX".into(), Value::Float(0.0)),
        ("rotateY".into(), Value::Float(0.0)),
        ("rotateZ".into(), Value::Float(0.0)),
        ("posX".into(), Value::Float(0.0)),
        ("posY".into(), Value::Float(0.0)),
        ("viewScale".into(), Value::Float(1.0)),
    ]);
    let resources = || {
        BTreeMap::from([
            ("global_xyz".into(), state(1, 1, &[[0.5, 0.5, 0.0, 1.0]])),
            ("global_rgba".into(), state(1, 1, &[[0.5, 0.25, 1.0, 0.5]])),
            ("tex".into(), state(1, 1, &[[0.2, 0.4, 0.6, 0.8]])),
        ])
    };
    let formats = BTreeMap::from([("global_billboard_trail".into(), TextureFormat::Rgba32f)]);
    let mut additive = resources();
    assert_eq!(
        execute_draw_pass(
            "render/pointsBillboardRender:deposit",
            named_pass("render/pointsBillboardRender", "deposit_0"),
            &base_uniforms,
            &mut additive,
            3,
            3,
            &formats,
        )
        .unwrap(),
        1
    );
    assert_eq!(
        &additive["global_billboard_trail"].data()[16..20],
        &[0.5, 0.25, 1.0, 0.5]
    );

    let mut over = resources();
    over.insert(
        "global_billboard_trail".into(),
        Surface::from_f32(3, 3, vec![0.2; 36]).unwrap(),
    );
    execute_draw_pass(
        "render/pointsBillboardRender:deposit",
        named_pass("render/pointsBillboardRender", "deposit_alpha_0"),
        &base_uniforms,
        &mut over,
        3,
        3,
        &formats,
    )
    .unwrap();
    assert_eq!(
        &over["global_billboard_trail"].data()[16..20],
        &[0.6, 0.35, 1.1, 0.6]
    );

    let mut sprite_uniforms = base_uniforms.clone();
    sprite_uniforms.insert("shapeMode".into(), Value::Int(0));
    sprite_uniforms.insert("rotationVar".into(), Value::Float(0.0));
    let mut sprite = resources();
    execute_draw_pass(
        "render/pointsBillboardRender:deposit",
        named_pass("render/pointsBillboardRender", "deposit_0"),
        &sprite_uniforms,
        &mut sprite,
        3,
        3,
        &formats,
    )
    .unwrap();
    assert_eq!(
        &sprite["global_billboard_trail"].data()[16..20],
        &[0.1, 0.1, 0.6, 0.4]
    );
}

#[test]
fn billboard_alpha_uses_order_texture_for_overlapping_nonflat_particles() {
    let uniforms = BTreeMap::from([
        ("density".into(), Value::Float(100.0)),
        ("shapeMode".into(), Value::Int(3)),
        ("depositOpacity".into(), Value::Float(100.0)),
        ("seed".into(), Value::Int(42)),
        ("sizeVariation".into(), Value::Float(0.0)),
        ("rotationVar".into(), Value::Float(0.0)),
        ("pointSize".into(), Value::Float(3.0)),
        ("viewMode".into(), Value::Int(1)),
        ("blendMode".into(), Value::Int(1)),
        ("rotateX".into(), Value::Float(0.0)),
        ("rotateY".into(), Value::Float(0.0)),
        ("rotateZ".into(), Value::Float(0.0)),
        ("posX".into(), Value::Float(0.0)),
        ("posY".into(), Value::Float(0.0)),
        ("viewScale".into(), Value::Float(1.0)),
    ]);
    let mut resources = BTreeMap::from([
        (
            "global_xyz".into(),
            state(2, 1, &[[0.0, 0.0, 1.0, 1.0], [0.0, 0.0, 2.0, 1.0]]),
        ),
        (
            "global_rgba".into(),
            state(2, 1, &[[1.0, 0.0, 0.0, 0.5], [0.0, 0.0, 1.0, 0.5]]),
        ),
        (
            "depthOrderA".into(),
            state(2, 1, &[[0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0]]),
        ),
        ("tex".into(), state(1, 1, &[[1.0, 1.0, 1.0, 1.0]])),
    ]);
    let formats = BTreeMap::from([("global_billboard_trail".into(), TextureFormat::Rgba32f)]);
    execute_draw_pass(
        "render/pointsBillboardRender:deposit",
        &billboard_pass("deposit_alpha_1"),
        &uniforms,
        &mut resources,
        5,
        5,
        &formats,
    )
    .unwrap();
    assert_eq!(
        &resources["global_billboard_trail"].data()[48..52],
        &[1.0, 0.0, 0.5, 0.75]
    );

    resources.remove("global_billboard_trail");
    let mut perspective = uniforms;
    perspective.insert("viewMode".into(), Value::Int(2));
    perspective.insert("posZ".into(), Value::Float(69.0));
    perspective.insert("fieldOfView".into(), Value::Float(60.0));
    execute_draw_pass(
        "render/pointsBillboardRender:deposit",
        &billboard_pass("deposit_alpha_2"),
        &perspective,
        &mut resources,
        5,
        5,
        &formats,
    )
    .unwrap();
    assert_eq!(
        &resources["global_billboard_trail"].data()[48..52],
        &[1.0, 0.0, 0.5, 0.75]
    );
}

#[test]
fn billboard_additive_defocus_layer_uses_sprite_mean_and_blur_weight() {
    let uniforms = BTreeMap::from([
        ("density".into(), Value::Float(100.0)),
        ("shapeMode".into(), Value::Int(3)),
        ("depositOpacity".into(), Value::Float(100.0)),
        ("seed".into(), Value::Int(42)),
        ("sizeVariation".into(), Value::Float(0.0)),
        ("rotationVar".into(), Value::Float(0.0)),
        ("pointSize".into(), Value::Float(2.0)),
        ("viewMode".into(), Value::Int(1)),
        ("blendMode".into(), Value::Int(0)),
        ("blurLayer".into(), Value::Int(1)),
        ("aperture".into(), Value::Float(20.0)),
        ("focalDistance".into(), Value::Float(40.0)),
        ("rotateX".into(), Value::Float(0.0)),
        ("rotateY".into(), Value::Float(0.0)),
        ("rotateZ".into(), Value::Float(0.0)),
        ("posX".into(), Value::Float(0.0)),
        ("posY".into(), Value::Float(0.0)),
        ("viewScale".into(), Value::Float(1.0)),
    ]);
    let mut resources = BTreeMap::from([
        ("global_xyz".into(), state(1, 1, &[[0.0, 0.0, 1.0, 1.0]])),
        ("global_rgba".into(), state(1, 1, &[[1.0, 0.0, 0.0, 1.0]])),
        ("tex".into(), state(1, 1, &[[1.0, 1.0, 1.0, 1.0]])),
        ("spriteMean".into(), state(1, 1, &[[1.0, 1.0, 1.0, 1.0]])),
    ]);
    let formats = BTreeMap::from([("defocus".into(), TextureFormat::Rgba32f)]);
    execute_draw_pass(
        "render/pointsBillboardRender:deposit",
        &billboard_pass("depositDefocus_1"),
        &uniforms,
        &mut resources,
        5,
        5,
        &formats,
    )
    .unwrap();
    let blur_pixels = 20.0_f64 * 39.0 / 79.0;
    let expansion = 1.0 + blur_pixels;
    let expected = (1.0 / (0.19724318 * expansion * expansion)) as f32;
    assert!((resources["defocus"].data()[48] - expected).abs() < 1e-6);

    resources.insert("defocus".into(), Surface::new(2, 2).unwrap());
    execute_draw_pass(
        "render/pointsBillboardRender:deposit",
        &billboard_pass("depositDefocus_1"),
        &uniforms,
        &mut resources,
        8,
        8,
        &formats,
    )
    .unwrap();
    assert_eq!(
        (resources["defocus"].width(), resources["defocus"].height()),
        (2, 2)
    );
}

#[test]
fn wormhole_end_to_end_matches_oracle_and_dimension_mismatch_is_typed_graph_error() {
    let result = CpuRenderer::new()
        .unwrap()
        .render(
            "search synth,filter; solid(color:#fff).wormhole(stride:0,alpha:1).write(o0)",
            &RenderOptions {
                width: 1,
                height: 1,
                ..RenderOptions::default()
            },
        )
        .unwrap();
    for channel in &result.surface.data()[..3] {
        assert!((*channel - 0.5).abs() < 1e-6, "{channel}");
    }
    assert_eq!(result.surface.data()[3], 1.0);

    let mut resources = BTreeMap::from([("inputTex".into(), Surface::new(2, 1).unwrap())]);
    let error = execute_draw_pass(
        "filter/wormhole:deposit",
        draw_pass("filter/wormhole", "deposit"),
        &BTreeMap::from([
            ("kink".into(), Value::Float(0.0)),
            ("stride".into(), Value::Float(0.0)),
            ("rotation".into(), Value::Float(0.0)),
            ("wrap".into(), Value::Int(1)),
        ]),
        &mut resources,
        1,
        1,
        &BTreeMap::new(),
    )
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("wormhole deposit requires matching source and destination dimensions")
    );
}
