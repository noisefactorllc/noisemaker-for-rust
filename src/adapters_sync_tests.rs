use super::*;

// Raw little-endian f32 outputs captured from CPU 4aeb3e1e028b2dc412401f7b7f17c014c603bf14
// bindCanonicalKernel(kernelFactories.get(key)), with Float32 Surface input (no PNG conversion).
fn input(width: u32, height: u32, negative: bool) -> Surface {
    let mut data = Vec::new();
    for i in 0..width * height {
        let (x, y) = (i % width, i / width);
        data.extend_from_slice(&[
            f64::from((3 * x + 5 * y) % 31) as f32 / 31.0,
            (f64::from((11 * x + 7 * y) % 37) / 37.0) as f32,
            (f64::from((17 * x + 23 * y) % 41) / 41.0) as f32,
            (f64::from(i % 7) / 7.0) as f32,
        ]);
    }
    if negative {
        data[4] = -0.75;
        data[10] = -0.25;
    }
    Surface::from_f32(width, height, data).unwrap()
}
fn uniforms(width: u32, height: u32) -> BTreeMap<String, Value> {
    BTreeMap::from([
        (
            "resolution".into(),
            Value::Vec(vec![width as f32, height as f32]),
        ),
        (
            "fullResolution".into(),
            Value::Vec(vec![width as f32, height as f32]),
        ),
        ("tileOffset".into(), Value::Vec(vec![0.0, 0.0])),
        ("renderScale".into(), Value::Float(1.0)),
        ("alpha".into(), Value::Float(1.0)),
        ("seed".into(), Value::Float(1.0)),
        ("time".into(), Value::Float(0.25)),
        ("speed".into(), Value::Float(1.0)),
    ])
}
fn run(
    key: &str,
    u: &BTreeMap<String, Value>,
    surface: Surface,
    width: u32,
    height: u32,
) -> Surface {
    let pass = RenderPass {
        name: "sync".into(),
        program: "sync".into(),
        key: Some(key.into()),
        inputs: BTreeMap::from([
            ("inputTex".into(), "input".into()),
            ("bufTex".into(), "buffer".into()),
        ]),
        outputs: BTreeMap::from([("fragColor".into(), "out".into())]),
        execution: BTreeMap::new(),
    };
    let resources = BTreeMap::from([
        ("input".into(), surface),
        ("buffer".into(), Surface::new(width, height).unwrap()),
    ]);
    execute_semantic_fragment_adapter(key, &pass, u, &resources, width, height)
        .unwrap()
        .or_else(|| execute_fragment_adapter(key, &pass, u, &resources, width, height).unwrap())
        .expect("CPU adapter key registered")
        .1
}
fn exact(actual: &Surface, bytes: &[u8]) {
    let expected: Vec<u32> = bytes
        .chunks_exact(4)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
        .collect();
    assert_eq!(actual.data().len(), expected.len());
    for (i, (&a, &e)) in actual.data().iter().zip(&expected).enumerate() {
        assert_eq!(a.to_bits(), e, "lane {i}: {a} != {}", f32::from_bits(e));
    }
}
#[test]
fn cpu_sync_crt_direct_and_tiled() {
    let mut u = uniforms(8, 6);
    exact(
        &run("filter/crt:crt", &u, input(8, 6, false), 8, 6),
        include_bytes!("../tests/fixtures/cpu-adapter-sync/crt.f32"),
    );
    u.insert("alpha".into(), Value::Float(0.7));
    u.insert("seed".into(), Value::Float(5.0));
    u.insert("time".into(), Value::Float(0.4));
    u.insert("speed".into(), Value::Float(2.5));
    u.insert("renderScale".into(), Value::Float(2.0));
    u.insert("fullResolution".into(), Value::Vec(vec![32.0, 18.0]));
    u.insert("tileOffset".into(), Value::Vec(vec![4.0, 2.0]));
    exact(
        &run("filter/crt:crt", &u, input(8, 6, false), 8, 6),
        include_bytes!("../tests/fixtures/cpu-adapter-sync/crt-tiled.f32"),
    );
    exact(
        &run("filter/crt:crt", &uniforms(8, 6), input(8, 6, false), 8, 6),
        include_bytes!("../tests/fixtures/cpu-adapter-sync/crt.f32"),
    );
}
#[test]
fn cpu_sync_crt_width_one_and_alpha_zero() {
    let mut u = uniforms(1, 4);
    let source = input(1, 4, false);
    let out = run("filter/crt:crt", &u, source.clone(), 1, 4);
    for (a, b) in out
        .data()
        .chunks_exact(4)
        .zip(source.data().chunks_exact(4))
    {
        assert!(a[..3].iter().all(|v| v.is_nan()));
        assert_eq!(a[3].to_bits(), b[3].to_bits());
    }
    u.insert("alpha".into(), Value::Float(0.0));
    assert_eq!(run("filter/crt:crt", &u, source.clone(), 1, 4), source);
}
#[test]
fn cpu_sync_snow_quarter_turn_and_animation() {
    let mut u = uniforms(8, 6);
    u.insert("density".into(), Value::Float(100.0));
    u.insert("pause".into(), Value::Bool(false));
    exact(
        &run("filter/snow:snow", &u, input(8, 6, false), 8, 6),
        include_bytes!("../tests/fixtures/cpu-adapter-sync/snow.f32"),
    );
    u.insert("alpha".into(), Value::Float(0.6));
    u.insert("density".into(), Value::Float(75.0));
    u.insert("time".into(), Value::Float(0.4));
    exact(
        &run("filter/snow:snow", &u, input(8, 6, false), 8, 6),
        include_bytes!("../tests/fixtures/cpu-adapter-sync/snow-moving.f32"),
    );
}
#[test]
fn cpu_sync_navier_whole_vector_and_hash_boundaries() {
    let mut u = uniforms(5, 3);
    u.insert("seed".into(), Value::Float(7.0));
    u.insert("speed".into(), Value::Float(10.0));
    u.insert("inputForce".into(), Value::Float(0.0));
    u.insert("inputDye".into(), Value::Float(0.0));
    u.insert("resetState".into(), Value::Bool(true));
    exact(
        &run("synth/navierStokes:nsSplat", &u, input(5, 3, false), 5, 3),
        include_bytes!("../tests/fixtures/cpu-adapter-sync/navier.f32"),
    );
}
#[test]
fn cpu_sync_median_active_window_unsigned_and_threshold() {
    for (radius, threshold, bytes) in [
        (
            1,
            0.0,
            include_bytes!("../tests/fixtures/cpu-adapter-sync/median1.f32"),
        ),
        (
            2,
            15.0,
            include_bytes!("../tests/fixtures/cpu-adapter-sync/median2.f32"),
        ),
        (
            3,
            0.0,
            include_bytes!("../tests/fixtures/cpu-adapter-sync/median3.f32"),
        ),
    ] {
        let mut u = uniforms(6, 5);
        u.insert("RADIUS".into(), Value::Int(radius));
        u.insert("threshold".into(), Value::Float(threshold));
        exact(
            &run("filter/median:median", &u, input(6, 5, true), 6, 5),
            bytes,
        );
    }
}

#[test]
fn cpu_sync_nonfinite_fetch_coercion_and_small_input() {
    let surface = input(4, 3, false);
    // Direct CPU snow.js texelOffset probes, including ToInt32 wrap and nonfinite input.
    for (x, y, offset) in [
        (f64::NAN, f64::NAN, 32),
        (f64::INFINITY, f64::NEG_INFINITY, 32),
        (4_294_967_297.0, 4_294_967_298.0, 4),
        (-1.9, 1.9, 16),
    ] {
        assert_eq!(adapter_texel_offset(&surface, x, y), offset);
    }
    let mut u = uniforms(2, 2);
    u.insert("alpha".into(), Value::Float(0.0));
    u.insert("density".into(), Value::Float(100.0));
    u.insert("pause".into(), Value::Bool(false));
    let single = Surface::from_f32(1, 1, vec![-0.0, -1.0, 2.0, 0.25]).unwrap();
    assert_eq!(
        run("filter/snow:snow", &u, single, 2, 2).data(),
        [-0.0, -1.0, 2.0, 0.25].repeat(4)
    );
}
#[test]
fn cpu_sync_fma_ties_and_m2_reciprocal() {
    // CPU crt.js fma32 TwoSum midpoint probes; f32::mul_add rounds the exact product+sum.
    for (a, b, c, bits) in [
        (
            1.1836352348327637_f32,
            1.7818914651870728_f32,
            -1.931027560075904e-9_f32,
            1074199463_u32,
        ),
        (
            1.715787649154663_f32,
            1.694210410118103_f32,
            -3.285236749306364e-9_f32,
            1077545661,
        ),
        (
            1.5398544073104858_f32,
            1.8304558992385864_f32,
            -9.061266736942741e-10_f32,
            1077175431,
        ),
        (134217744.0_f32, 201326592.0_f32, 1.0_f32, 1522532354),
        (134217744.0_f32, 201326592.0_f32, -1.0_f32, 1522532353),
    ] {
        assert_eq!(a.mul_add(b, c).to_bits(), bits);
    }
    assert_eq!(crt_mix(-0.0, -0.0, 0.0).to_bits(), 0);
    assert_eq!((-0.0_f32).mul_add(1.0, -0.0).to_bits(), 0x80000000);
    assert_eq!(crt_div(1.0, 3.0).to_bits(), (1.0_f32 / 3.0).to_bits());
    assert_eq!(
        crt_div(7.0, 3.0).to_bits(),
        (7.0_f32 * (1.0_f32 / 3.0)).to_bits()
    );
    assert_eq!(snow_hash(0.5, 0.5, 53.0).to_bits(), 1060721664);
    assert_eq!(snow_hash(7.5, 5.5, 113.0).to_bits(), 1054375936);
}

#[test]
fn cpu_sync_fractional_scalar_boundaries() {
    let mut u = uniforms(8, 6);
    u.insert("alpha".into(), Value::Float(0.37));
    u.insert("time".into(), Value::Float(f64::from(0.4_f32)));
    u.insert("seed".into(), Value::Int(7));
    u.insert("speed".into(), Value::Float(3.1));
    exact(
        &run("filter/crt:crt", &u, input(8, 6, false), 8, 6),
        include_bytes!("../tests/fixtures/cpu-adapter-sync/crt-fractional.f32"),
    );
    u.insert("density".into(), Value::Float(83.7));
    u.insert("pause".into(), Value::Bool(false));
    exact(
        &run("filter/snow:snow", &u, input(8, 6, false), 8, 6),
        include_bytes!("../tests/fixtures/cpu-adapter-sync/snow-fractional.f32"),
    );
}
#[test]
fn cpu_sync_crt_nonfinite_scale_and_signed_zero() {
    let mut u = uniforms(2, 2);
    u.insert("renderScale".into(), Value::Float(f64::NAN));
    let source = input(2, 2, false);
    let out = run("filter/crt:crt", &u, source.clone(), 2, 2);
    for (a, b) in out
        .data()
        .chunks_exact(4)
        .zip(source.data().chunks_exact(4))
    {
        assert!(a[..3].iter().all(|v| v.is_nan()));
        assert_eq!(a[3].to_bits(), b[3].to_bits());
    }
    assert_eq!(crt_min(0.0, -0.0).to_bits(), 0x80000000);
    assert_eq!(crt_min(-0.0, 0.0).to_bits(), 0x80000000);
    assert_eq!(crt_max(0.0, -0.0).to_bits(), 0);
    assert_eq!(crt_max(-0.0, 0.0).to_bits(), 0);
}
