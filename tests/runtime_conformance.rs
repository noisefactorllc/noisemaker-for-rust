use noisemaker_cpu::{FilterMode, Runtime, Surface, Value};

fn scalar(runtime: &mut Runtime, name: &str, arguments: &[Value]) -> f32 {
    match runtime.call_builtin(name, arguments).unwrap() {
        Value::Float(value) => value,
        value => panic!("{name} returned {}", value.kind()),
    }
}

fn vector(runtime: &mut Runtime, name: &str, arguments: &[Value]) -> Vec<f32> {
    match runtime.call_builtin(name, arguments).unwrap() {
        Value::Vec(value) => value,
        value => panic!("{name} returned {}", value.kind()),
    }
}

fn close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= 2.0e-6,
        "{actual:?} != {expected:?}"
    );
}

#[test]
fn standard_sine_matches_javascript_math_sin_then_float32() {
    let mut runtime = Runtime::new();
    assert_eq!(
        scalar(&mut runtime, "sin", &[Value::Float(1.0)]),
        f32::from_bits(0x3f57_6aa4)
    );
    assert_eq!(
        scalar(
            &mut runtime,
            "sin",
            &[Value::Float(f32::from_bits(0x4308_d58e))]
        ),
        f32::from_bits(0xbf7c_17fb)
    );
}

#[test]
fn float_component_builtins_match_sibling_edge_semantics() {
    let rt = &mut Runtime::new();
    assert_eq!(scalar(rt, "abs", &[Value::Float(-3.0)]), 3.0);
    assert_eq!(
        scalar(rt, "abs", &[Value::Float(f32::NEG_INFINITY)]),
        f32::INFINITY
    );
    close(
        scalar(rt, "acos", &[Value::Float(0.5)]),
        std::f32::consts::FRAC_PI_3,
    );
    close(
        scalar(rt, "atan", &[Value::Float(1.0), Value::Float(-1.0)]),
        3.0 * std::f32::consts::PI / 4.0,
    );
    assert_eq!(scalar(rt, "ceil", &[Value::Float(-1.25)]), -1.0);
    assert_eq!(
        scalar(
            rt,
            "clamp",
            &[Value::Float(2.0), Value::Float(-1.0), Value::Float(1.0)]
        ),
        1.0
    );
    assert!(
        scalar(
            rt,
            "clamp",
            &[Value::Float(f32::NAN), Value::Float(0.0), Value::Float(1.0)]
        )
        .is_nan()
    );
    close(
        scalar(rt, "cos", &[Value::Float(std::f32::consts::PI)]),
        -1.0,
    );
    close(
        scalar(rt, "degrees", &[Value::Float(std::f32::consts::PI)]),
        180.0,
    );
    assert_eq!(scalar(rt, "exp", &[Value::Float(0.0)]), 1.0);
    assert_eq!(scalar(rt, "floor", &[Value::Float(-1.25)]), -2.0);
    assert_eq!(scalar(rt, "fract", &[Value::Float(-1.25)]), 0.75);
    assert_eq!(scalar(rt, "inversesqrt", &[Value::Float(4.0)]), 0.5);
    assert_eq!(
        rt.call_builtin("isnan", &[Value::Vec(vec![f32::NAN, f32::INFINITY, -0.0])])
            .unwrap(),
        Value::BVec(vec![true, false, false])
    );
    close(scalar(rt, "log", &[Value::Float(std::f32::consts::E)]), 1.0);
    assert_eq!(scalar(rt, "log2", &[Value::Float(8.0)]), 3.0);
    assert!(scalar(rt, "min", &[Value::Float(f32::NAN), Value::Float(1.0)]).is_nan());
    assert!(scalar(rt, "max", &[Value::Float(1.0), Value::Float(f32::NAN)]).is_nan());
    assert_eq!(
        scalar(
            rt,
            "mix",
            &[Value::Float(2.0), Value::Float(10.0), Value::Float(0.25)]
        ),
        4.0
    );
    assert_eq!(
        scalar(rt, "mod", &[Value::Float(-1.0), Value::Float(3.0)]),
        2.0
    );
    assert_eq!(
        scalar(rt, "pow", &[Value::Float(2.0), Value::Float(3.0)]),
        8.0
    );
    close(
        scalar(rt, "radians", &[Value::Float(180.0)]),
        std::f32::consts::PI,
    );
    assert_eq!(scalar(rt, "round", &[Value::Float(-1.5)]), -1.0);
    assert_eq!(scalar(rt, "round", &[Value::Float(1.5)]), 2.0);
    assert_eq!(scalar(rt, "sign", &[Value::Float(-3.0)]), -1.0);
    assert_eq!(
        scalar(rt, "sign", &[Value::Float(-0.0)]).to_bits(),
        (-0.0_f32).to_bits()
    );
    close(
        scalar(rt, "sin", &[Value::Float(std::f32::consts::FRAC_PI_2)]),
        1.0,
    );
    let large_angle = 12_345.678_f32;
    let standard = scalar(rt, "sin", &[Value::Float(large_angle)]);
    assert_eq!(
        standard.to_bits(),
        (f64::from(large_angle).sin() as f32).to_bits()
    );
    let turns = large_angle * (1.0_f32 / std::f32::consts::TAU);
    let reduced =
        (f64::from(turns - turns.floor()) * f64::from(std::f32::consts::TAU)).sin() as f32;
    assert_ne!(standard.to_bits(), reduced.to_bits());
    assert_eq!(
        scalar(
            rt,
            "smoothstep",
            &[Value::Float(0.0), Value::Float(1.0), Value::Float(0.5)]
        ),
        0.5
    );
    assert_eq!(scalar(rt, "sqrt", &[Value::Float(9.0)]), 3.0);
    assert_eq!(
        scalar(rt, "step", &[Value::Float(0.5), Value::Float(0.49)]),
        0.0
    );
    assert_eq!(
        scalar(rt, "step", &[Value::Float(0.5), Value::Float(0.5)]),
        1.0
    );
    close(scalar(rt, "tanh", &[Value::Float(0.5)]), 0.462_117_17);
}

#[test]
fn relational_and_boolean_builtins_return_lane_exact_results() {
    let rt = &mut Runtime::new();
    assert_eq!(
        rt.call_builtin("all", &[Value::BVec(vec![true, true, false])])
            .unwrap(),
        Value::Bool(false)
    );
    assert_eq!(
        rt.call_builtin("any", &[Value::BVec(vec![false, false, true])])
            .unwrap(),
        Value::Bool(true)
    );
    let a = Value::Vec(vec![1.0, 2.0, 3.0]);
    let b = Value::Vec(vec![1.0, 3.0, 2.0]);
    assert_eq!(
        rt.call_builtin("equal", &[a.clone(), b.clone()]).unwrap(),
        Value::BVec(vec![true, false, false])
    );
    assert_eq!(
        rt.call_builtin("notEqual", &[a.clone(), b.clone()])
            .unwrap(),
        Value::BVec(vec![false, true, true])
    );
    assert_eq!(
        rt.call_builtin("lessThan", &[a.clone(), b.clone()])
            .unwrap(),
        Value::BVec(vec![false, true, false])
    );
    assert_eq!(
        rt.call_builtin("lessThanEqual", &[a.clone(), b.clone()])
            .unwrap(),
        Value::BVec(vec![true, true, false])
    );
    assert_eq!(
        rt.call_builtin("greaterThan", &[a.clone(), b.clone()])
            .unwrap(),
        Value::BVec(vec![false, false, true])
    );
    assert_eq!(
        rt.call_builtin("greaterThanEqual", &[a, b]).unwrap(),
        Value::BVec(vec![true, false, true])
    );
}

#[test]
fn geometry_builtins_match_nontrivial_reference_vectors() {
    let rt = &mut Runtime::new();
    assert_eq!(
        scalar(
            rt,
            "dot",
            &[
                Value::Vec(vec![1.0, 2.0, 3.0]),
                Value::Vec(vec![4.0, -5.0, 6.0])
            ]
        ),
        12.0
    );
    assert_eq!(
        scalar(
            rt,
            "distance",
            &[Value::Vec(vec![1.0, 2.0]), Value::Vec(vec![4.0, 6.0])]
        ),
        5.0
    );
    assert_eq!(scalar(rt, "length", &[Value::Vec(vec![3.0, 4.0])]), 5.0);
    assert_eq!(
        vector(rt, "normalize", &[Value::Vec(vec![3.0, 4.0])]),
        vec![0.6, 0.8]
    );
    assert_eq!(
        vector(rt, "normalize", &[Value::Vec(vec![0.0, 0.0])]),
        vec![0.0, 0.0]
    );
    assert_eq!(
        vector(
            rt,
            "cross",
            &[
                Value::Vec(vec![1.0, 0.0, 0.0]),
                Value::Vec(vec![0.0, 1.0, 0.0])
            ]
        ),
        vec![0.0, 0.0, 1.0]
    );
    assert_eq!(
        vector(
            rt,
            "reflect",
            &[
                Value::Vec(vec![1.0, -1.0, 0.0]),
                Value::Vec(vec![0.0, 1.0, 0.0])
            ]
        ),
        vec![1.0, 1.0, 0.0]
    );
    assert_eq!(
        vector(
            rt,
            "refract",
            &[
                Value::Vec(vec![0.0, -1.0, 0.0]),
                Value::Vec(vec![0.0, 1.0, 0.0]),
                Value::Float(0.5)
            ]
        ),
        vec![0.0, -1.0, 0.0]
    );
    assert_eq!(
        vector(
            rt,
            "refract",
            &[
                Value::Vec(vec![1.0, 0.0, 0.0]),
                Value::Vec(vec![0.0, 1.0, 0.0]),
                Value::Float(2.0)
            ]
        ),
        vec![0.0, 0.0, 0.0]
    );
}

#[test]
fn integer_bit_half_and_matrix_operations_are_bit_exact() {
    let rt = &mut Runtime::new();
    assert_eq!(
        rt.binary("+", &Value::Int(i32::MAX), &Value::Int(1))
            .unwrap(),
        Value::Int(i32::MIN)
    );
    assert_eq!(
        rt.binary("/", &Value::Int(i32::MIN), &Value::Int(-1))
            .unwrap(),
        Value::Int(i32::MIN)
    );
    assert_eq!(
        rt.binary("/", &Value::Int(7), &Value::Int(0)).unwrap(),
        Value::Int(0)
    );
    assert_eq!(
        rt.binary("%", &Value::Int(-7), &Value::Int(3)).unwrap(),
        Value::Int(-1)
    );
    assert_eq!(
        rt.binary("<<", &Value::Uint(1), &Value::Int(32)).unwrap(),
        Value::Uint(1)
    );
    assert_eq!(
        rt.binary(">>", &Value::Uint(0x8000_0000), &Value::Int(16))
            .unwrap(),
        Value::Uint(0x8000)
    );
    assert_eq!(
        rt.call_builtin("abs", &[Value::Int(i32::MIN)]).unwrap(),
        Value::Int(i32::MIN)
    );
    assert_eq!(
        rt.call_builtin("floatBitsToUint", &[Value::Float(-1.0)])
            .unwrap(),
        Value::Uint(3_212_836_864)
    );
    assert_eq!(
        rt.call_builtin("uintBitsToFloat", &[Value::Uint(0x3f80_0000)])
            .unwrap(),
        Value::Float(1.0)
    );
    for (input, bits, decoded) in [
        (
            [std::f32::consts::PI, std::f32::consts::E],
            1_097_876_040,
            [3.140625, 2.71875],
        ),
        ([1e-7, -1e-7], 2_147_614_722, [1.1920929e-7, -1.1920929e-7]),
        (
            [f32::INFINITY, f32::NEG_INFINITY],
            4_227_890_176,
            [f32::INFINITY, f32::NEG_INFINITY],
        ),
    ] {
        let packed = rt
            .call_builtin("packHalf2x16", &[Value::Vec(input.to_vec())])
            .unwrap();
        assert_eq!(packed, Value::Uint(bits));
        assert_eq!(
            rt.call_builtin("unpackHalf2x16", &[packed]).unwrap(),
            Value::Vec(decoded.to_vec())
        );
    }
    let a = Value::Mat {
        dimension: 2,
        columns: vec![1.0, 2.0, 3.0, 4.0],
    };
    let b = Value::Mat {
        dimension: 2,
        columns: vec![5.0, 6.0, 7.0, 8.0],
    };
    assert_eq!(
        rt.binary("*", &a, &Value::Vec(vec![5.0, 6.0])).unwrap(),
        Value::Vec(vec![23.0, 34.0])
    );
    assert_eq!(
        rt.binary("*", &Value::Vec(vec![5.0, 6.0]), &a).unwrap(),
        Value::Vec(vec![17.0, 39.0])
    );
    assert_eq!(
        rt.binary("*", &a, &b).unwrap(),
        Value::Mat {
            dimension: 2,
            columns: vec![23.0, 34.0, 31.0, 46.0]
        }
    );
}

#[test]
fn emitted_unary_binary_and_logical_operator_families_are_value_exact() {
    let rt = Runtime::new();
    for (operator, expected) in [("+", 7.0), ("-", 3.0), ("*", 10.0), ("/", 2.5), ("%", 1.0)] {
        assert_eq!(
            rt.binary(operator, &Value::Float(5.0), &Value::Float(2.0))
                .unwrap(),
            Value::Float(expected)
        );
    }
    assert_eq!(
        rt.binary("&", &Value::Int(0b1100), &Value::Int(0b1010))
            .unwrap(),
        Value::Int(0b1000)
    );
    assert_eq!(
        rt.binary("|", &Value::Int(0b1100), &Value::Int(0b0011))
            .unwrap(),
        Value::Int(0b1111)
    );
    assert_eq!(
        rt.binary("^", &Value::Uint(0xffff_0000), &Value::Uint(0x00ff_00ff))
            .unwrap(),
        Value::Uint(0xff00_00ff)
    );
    assert_eq!(
        rt.binary("<<", &Value::Int(1), &Value::Int(31)).unwrap(),
        Value::Int(i32::MIN)
    );
    assert_eq!(
        rt.binary(">>", &Value::Int(-8), &Value::Int(2)).unwrap(),
        Value::Int(-2)
    );
    assert_eq!(
        rt.binary("&&", &Value::Bool(true), &Value::Bool(false))
            .unwrap(),
        Value::Bool(false)
    );
    assert_eq!(
        rt.binary("||", &Value::Bool(false), &Value::Bool(true))
            .unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        rt.binary(
            "==",
            &Value::Vec(vec![1.0, 2.0]),
            &Value::Vec(vec![1.0, 2.0])
        )
        .unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        rt.binary(
            "!=",
            &Value::Vec(vec![1.0, 2.0]),
            &Value::Vec(vec![1.0, 3.0])
        )
        .unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        rt.binary("<", &Value::Float(1.0), &Value::Float(2.0))
            .unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        rt.binary(">=", &Value::Int(2), &Value::Int(2)).unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        rt.unary("+", &Value::Float(-2.0)).unwrap(),
        Value::Float(-2.0)
    );
    assert_eq!(
        rt.unary("-", &Value::Int(i32::MIN)).unwrap(),
        Value::Int(i32::MIN)
    );
    assert_eq!(
        rt.unary("!", &Value::Bool(true)).unwrap(),
        Value::Bool(false)
    );
    assert_eq!(
        rt.unary("~", &Value::Uint(0x0f0f_0000)).unwrap(),
        Value::Uint(0xf0f0_ffff)
    );
}

#[test]
fn texture_builtins_match_filter_origin_lod_fetch_and_size_contracts() {
    let rt = &mut Runtime::new();
    let data = vec![
        1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
    ];
    let nearest = rt.add_texture(Surface::from_f32(2, 2, data.clone()).unwrap());
    let mut linear_surface = Surface::from_f32(2, 2, data).unwrap();
    linear_surface.set_filter_mode(FilterMode::Linear);
    let linear = rt.add_texture(linear_surface);
    assert_eq!(
        rt.call_builtin("texture", &[nearest.clone(), Value::Vec(vec![0.25, 0.25])])
            .unwrap(),
        Value::Vec(vec![0.0, 0.0, 1.0, 1.0])
    );
    assert_eq!(
        rt.call_builtin(
            "textureLod",
            &[
                nearest.clone(),
                Value::Vec(vec![0.75, 0.75]),
                Value::Float(3.0)
            ]
        )
        .unwrap(),
        Value::Vec(vec![0.0, 1.0, 0.0, 1.0])
    );
    assert_eq!(
        rt.call_builtin(
            "texelFetch",
            &[nearest.clone(), Value::IVec(vec![0, 0]), Value::Int(0)]
        )
        .unwrap(),
        Value::Vec(vec![0.0, 0.0, 1.0, 1.0])
    );
    assert_eq!(
        rt.call_builtin("textureSize", &[nearest, Value::Int(0)])
            .unwrap(),
        Value::IVec(vec![2, 2])
    );
    assert_eq!(
        rt.call_builtin("texture", &[linear, Value::Vec(vec![0.5, 0.5])])
            .unwrap(),
        Value::Vec(vec![0.5, 0.5, 0.5, 1.0])
    );
}
