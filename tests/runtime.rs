use noisemaker_cpu::{DerivativeMode, Runtime, Surface, Value, VmError};

fn runtime() -> Runtime {
    Runtime::new()
}

#[test]
fn constructors_flatten_splat_and_reject_invalid_conversions() {
    let rt = runtime();
    assert_eq!(
        rt.construct("vec3", &[Value::Float(2.0)]).unwrap(),
        Value::Vec(vec![2.0; 3])
    );
    assert_eq!(
        rt.construct(
            "vec4",
            &[
                Value::Vec(vec![1.0, 2.0]),
                Value::Float(3.0),
                Value::Float(4.0)
            ]
        )
        .unwrap(),
        Value::Vec(vec![1.0, 2.0, 3.0, 4.0])
    );
    assert!(matches!(
        rt.construct("bool", &[Value::Sampler(0)]),
        Err(VmError::Type { .. })
    ));
}

#[test]
fn integer_ops_wrap_and_float_division_preserves_signed_zero_semantics() {
    let rt = runtime();
    assert_eq!(
        rt.binary("+", &Value::Int(i32::MAX), &Value::Int(1))
            .unwrap(),
        Value::Int(i32::MIN)
    );
    assert_eq!(
        rt.binary("*", &Value::Uint(u32::MAX), &Value::Uint(2))
            .unwrap(),
        Value::Uint(u32::MAX.wrapping_mul(2))
    );
    assert_eq!(
        rt.binary(">>", &Value::Uint(0x8000_0000), &Value::Uint(16))
            .unwrap(),
        Value::Uint(0x8000)
    );
    let positive = rt
        .binary("/", &Value::Float(1.0), &Value::Float(0.0))
        .unwrap();
    let negative = rt
        .binary("/", &Value::Float(1.0), &Value::Float(-0.0))
        .unwrap();
    assert_eq!(positive, Value::Float(f32::INFINITY));
    assert_eq!(negative, Value::Float(f32::NEG_INFINITY));
}

#[test]
fn comparisons_reduce_for_operators_and_relational_builtins_return_vectors() {
    let mut rt = runtime();
    let a = Value::Vec(vec![1.0, 2.0]);
    let b = Value::Vec(vec![1.0, 3.0]);
    assert_eq!(rt.binary("==", &a, &b).unwrap(), Value::Bool(false));
    assert_eq!(rt.binary("!=", &a, &b).unwrap(), Value::Bool(true));
    assert_eq!(
        rt.call_builtin("lessThan", &[a, b]).unwrap(),
        Value::BVec(vec![false, true])
    );
}

#[test]
fn matrix_products_pcg_and_bitcasts_match_golden_values() {
    let mut rt = runtime();
    let identity = Value::Mat {
        dimension: 2,
        columns: vec![1.0, 0.0, 0.0, 1.0],
    };
    let vector = Value::Vec(vec![4.0, 7.0]);
    assert_eq!(rt.binary("*", &identity, &vector).unwrap(), vector);
    assert_eq!(
        rt.pcg3d([1, 2, 3]),
        [4_204_755_366, 1_223_881_804, 1_500_469_937]
    );
    assert_eq!(
        rt.call_builtin("floatBitsToUint", &[Value::Float(1.0)])
            .unwrap(),
        Value::Uint(0x3f80_0000)
    );
    assert_eq!(
        rt.call_builtin("uintBitsToFloat", &[Value::Uint(0x3f80_0000)])
            .unwrap(),
        Value::Float(1.0)
    );
    let packed = rt
        .call_builtin("packHalf2x16", &[Value::Vec(vec![0.5, -2.0])])
        .unwrap();
    assert_eq!(packed, Value::Uint(3_221_239_808));
    assert_eq!(
        rt.call_builtin("unpackHalf2x16", &[packed]).unwrap(),
        Value::Vec(vec![0.5, -2.0])
    );
}

#[test]
fn texture_handles_are_stable_indices_and_use_bottom_left_coordinates() {
    let mut rt = runtime();
    let surface = Surface::from_f32(1, 2, vec![1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0]).unwrap();
    let handle = rt.add_texture(surface);
    assert_eq!(handle, Value::Sampler(0));
    assert_eq!(
        rt.call_builtin("textureSize", &[handle.clone(), Value::Int(0)])
            .unwrap(),
        Value::IVec(vec![1, 2])
    );
    assert_eq!(
        rt.call_builtin(
            "texelFetch",
            &[handle.clone(), Value::IVec(vec![0, 0]), Value::Int(0)]
        )
        .unwrap(),
        Value::Vec(vec![0.0, 1.0, 0.0, 1.0])
    );
    assert_eq!(
        rt.call_builtin("texture", &[handle, Value::Vec(vec![0.5, 0.75])])
            .unwrap(),
        Value::Vec(vec![1.0, 0.0, 0.0, 1.0])
    );
}

#[test]
fn derivatives_record_replay_and_reject_divergent_shapes() {
    let mut lanes = Vec::new();
    for value in [1.0, 4.0, 11.0, 20.0] {
        let mut rt = runtime();
        rt.set_derivative_mode(DerivativeMode::Record);
        assert_eq!(
            rt.call_builtin("dFdx", &[Value::Float(value)]).unwrap(),
            Value::Float(0.0)
        );
        lanes.push(rt.take_derivative_log());
    }
    let diffs = Runtime::fine_derivatives(&lanes, 0, 0).unwrap();
    let mut rt = runtime();
    rt.set_derivative_replay(diffs);
    assert_eq!(
        rt.call_builtin("dFdx", &[Value::Float(1.0)]).unwrap(),
        Value::Float(3.0)
    );

    lanes[3].push(("dFdy".into(), Value::Float(2.0)));
    assert!(matches!(
        Runtime::fine_derivatives(&lanes, 0, 0),
        Err(VmError::DivergentDerivatives { .. })
    ));
}

#[test]
fn unknown_builtin_and_wrong_arity_are_typed_errors() {
    let mut rt = runtime();
    assert!(matches!(
        rt.call_builtin("imaginary", &[]),
        Err(VmError::UnknownBuiltin { .. })
    ));
    assert!(matches!(
        rt.call_builtin("sin", &[]),
        Err(VmError::Arity { .. })
    ));
}
