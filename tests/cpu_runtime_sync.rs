use noisemaker_cpu::{Runtime, Value};

#[test]
fn cpu_metal_division_broadcasts_captured_results() {
    let runtime = Runtime::new();
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/cpu-sync/metal-division-m4.json")).unwrap();
    for pair in fixture["runtime"].as_array().unwrap() {
        let bits = |i: usize| pair[i].as_u64().unwrap() as u32;
        let a = f32::from_bits(bits(0));
        let b = f32::from_bits(bits(1));
        for (left, right) in [
            (Value::Vec(vec![a; 2]), Value::Float(f64::from(b))),
            (Value::Float(f64::from(a)), Value::Vec(vec![b; 2])),
            (Value::Vec(vec![a; 2]), Value::Vec(vec![b; 2])),
        ] {
            let Value::Vec(actual) = runtime.binary("metal/", &left, &right).unwrap() else {
                panic!("vector division returned a scalar");
            };
            assert_eq!(
                actual.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                vec![bits(2); 2],
                "{pair}"
            );
        }
    }
    assert!(
        runtime
            .binary(
                "metal/",
                &Value::Vec(vec![1.0; 2]),
                &Value::Vec(vec![1.0; 3])
            )
            .is_err()
    );
}

#[test]
fn cpu_metal_scalar_retains_product_precision_and_generic_division() {
    let runtime = Runtime::new();
    let a = Value::Float(f64::from(1.3_f32));
    let b = Value::Int(41);
    assert_eq!(
        runtime.binary("metal/", &a, &b).unwrap(),
        Value::Float(0.031707317150402314)
    );
    assert_eq!(
        runtime.binary("/", &a, &b).unwrap(),
        Value::Float(f64::from(1.3_f32) / 41.0)
    );
    assert_ne!(
        runtime.binary("metal/", &a, &b).unwrap(),
        runtime.binary("/", &a, &b).unwrap()
    );
}

#[test]
fn cpu_metal_mat2_retains_its_column_shape() {
    let runtime = Runtime::new();
    let a = f32::from_bits(1_034_395_935);
    let b = f32::from_bits(3_072_200_160);
    let expected = f32::from_bits(3_322_396_964);
    let matrix = |value| Value::Mat {
        dimension: 2,
        columns: vec![value; 4],
    };
    for (left, right) in [
        (matrix(a), Value::Float(f64::from(b))),
        (Value::Float(f64::from(a)), matrix(b)),
        (matrix(a), matrix(b)),
    ] {
        let Value::Mat { dimension, columns } = runtime.binary("metal/", &left, &right).unwrap()
        else {
            panic!("matrix division lost its column shape");
        };
        assert_eq!(dimension, 2);
        assert_eq!(columns, vec![expected; 4]);
    }
}
