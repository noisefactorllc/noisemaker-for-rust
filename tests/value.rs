use std::collections::BTreeMap;

use noisemaker_cpu::{Value, VmError};

#[test]
fn values_store_float_lanes_at_f32_boundaries_and_copy_by_value() {
    let value = Value::float_vector([1.0_f64 / 3.0, 2.0]);
    assert_eq!(
        value.float_lanes().unwrap()[0].to_bits(),
        (1.0_f32 / 3.0).to_bits()
    );

    let mut copy = value.clone();
    copy.write_swizzle("x", Value::Float(9.0)).unwrap();
    assert_eq!(
        value.read_swizzle("x").unwrap(),
        Value::Float(1.0_f32 / 3.0)
    );
    assert_eq!(copy.read_swizzle("xy").unwrap(), Value::Vec(vec![9.0, 2.0]));
}

#[test]
fn swizzles_arrays_structs_and_samplers_remain_typed() {
    let mut value = Value::Vec(vec![1.0, 2.0, 3.0, 4.0]);
    assert_eq!(
        value.read_swizzle("bgra").unwrap(),
        Value::Vec(vec![3.0, 2.0, 1.0, 4.0])
    );
    value
        .write_swizzle("yz", Value::Vec(vec![8.0, 9.0]))
        .unwrap();
    assert_eq!(value, Value::Vec(vec![1.0, 8.0, 9.0, 4.0]));

    let array = Value::Array(vec![Value::Int(7), Value::Int(8)]);
    assert_eq!(array.index(1).unwrap(), &Value::Int(8));
    let structure = Value::Struct(BTreeMap::from([("field".into(), Value::Uint(5))]));
    assert_eq!(structure.member("field").unwrap(), &Value::Uint(5));
    assert_eq!(Value::Sampler(3), Value::Sampler(3));
}

#[test]
fn invalid_value_access_is_a_typed_error() {
    assert!(matches!(
        Value::Bool(true).read_swizzle("x"),
        Err(VmError::Type { .. })
    ));
    assert!(matches!(
        Value::Vec(vec![1.0, 2.0]).read_swizzle("z"),
        Err(VmError::Bounds { .. })
    ));
    assert!(matches!(Value::Int(1).index(0), Err(VmError::Type { .. })));
}
