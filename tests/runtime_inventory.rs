use std::collections::BTreeSet;

use noisemaker_cpu::catalog::{Expression, Type, shader_bundle};
use noisemaker_cpu::{Runtime, Surface, Value};

#[test]
fn every_locked_ir_builtin_signature_dispatches_with_its_annotated_type() {
    let mut signatures = BTreeSet::new();
    for program in shader_bundle().unwrap().programs.values() {
        for function in &program.ir.functions {
            for statement in &function.body {
                statement.visit_expressions(&mut |expression| {
                    if let Expression::Call {
                        value_type,
                        target,
                        arguments,
                        ..
                    } = expression
                    {
                        if let Some(name) = target.strip_prefix("builtin:") {
                            signatures.insert((
                                name.to_owned(),
                                arguments
                                    .iter()
                                    .map(expression_type)
                                    .map(|ty| ty.0.clone())
                                    .collect::<Vec<_>>(),
                                value_type.0.clone(),
                            ));
                        }
                    }
                });
            }
        }
    }
    assert_eq!(
        signatures
            .iter()
            .map(|(name, _, _)| name)
            .collect::<BTreeSet<_>>()
            .len(),
        53
    );

    let mut runtime = Runtime::new();
    let sampler = runtime.add_texture(Surface::from_f32(2, 2, vec![0.25; 16]).unwrap());
    for (name, argument_types, return_type) in signatures {
        let arguments = argument_types
            .iter()
            .map(|ty| sample_value(ty, &sampler))
            .collect::<Vec<_>>();
        let result = runtime
            .call_builtin(&name, &arguments)
            .unwrap_or_else(|error| {
                panic!(
                    "builtin {name}({}) -> {return_type}: {error}",
                    argument_types.join(", ")
                )
            });
        assert_type(
            &result,
            &return_type,
            &format!("{name}({})", argument_types.join(", ")),
        );
    }
}

#[test]
fn every_locked_ir_binary_signature_dispatches_with_its_annotated_type() {
    let mut signatures = BTreeSet::new();
    let mut unary = BTreeSet::new();
    let mut assignment = BTreeSet::new();
    let mut postfix = BTreeSet::new();
    for program in shader_bundle().unwrap().programs.values() {
        for function in &program.ir.functions {
            for statement in &function.body {
                statement.visit_expressions(&mut |expression| match expression {
                    Expression::Binary {
                        value_type,
                        operator,
                        left,
                        right,
                    } => {
                        signatures.insert((
                            operator.clone(),
                            expression_type(left).0.clone(),
                            expression_type(right).0.clone(),
                            value_type.0.clone(),
                        ));
                    }
                    Expression::Unary { operator, .. } => {
                        unary.insert(operator.clone());
                    }
                    Expression::Assignment { operator, .. } => {
                        assignment.insert(operator.clone());
                    }
                    Expression::Postfix { operator, .. } => {
                        postfix.insert(operator.clone());
                    }
                    _ => {}
                });
            }
        }
    }
    assert_eq!(
        unary,
        BTreeSet::from(["!".into(), "+".into(), "++".into(), "-".into(), "~".into()])
    );
    assert_eq!(
        assignment,
        BTreeSet::from([
            "*=".into(),
            "+=".into(),
            "-=".into(),
            "/=".into(),
            "=".into(),
            "^=".into()
        ])
    );
    assert_eq!(postfix, BTreeSet::from(["++".into(), "--".into()]));

    let runtime = Runtime::new();
    for (operator, left_type, right_type, return_type) in signatures {
        let left = sample_value(&left_type, &Value::Sampler(0));
        let right = sample_value(&right_type, &Value::Sampler(0));
        let result = runtime
            .binary(&operator, &left, &right)
            .unwrap_or_else(|error| {
                panic!("binary {left_type} {operator} {right_type} -> {return_type}: {error}")
            });
        assert_type(
            &result,
            &return_type,
            &format!("{left_type} {operator} {right_type}"),
        );
    }
}

fn expression_type(expression: &Expression) -> &Type {
    match expression {
        Expression::Literal { value_type, .. }
        | Expression::Identifier { value_type, .. }
        | Expression::Member { value_type, .. }
        | Expression::Index { value_type, .. }
        | Expression::Unary { value_type, .. }
        | Expression::Postfix { value_type, .. }
        | Expression::Conditional { value_type, .. }
        | Expression::Binary { value_type, .. }
        | Expression::Assignment { value_type, .. }
        | Expression::Construct { value_type, .. }
        | Expression::Call { value_type, .. } => value_type,
    }
}

fn sample_value(value_type: &str, sampler: &Value) -> Value {
    match value_type {
        "bool" => Value::Bool(true),
        "int" => Value::Int(1),
        "uint" => Value::Uint(1),
        "float" => Value::Float(0.5),
        "sampler2D" => sampler.clone(),
        name if name.starts_with("bvec") => Value::BVec(vec![true; name[4..].parse().unwrap()]),
        name if name.starts_with("ivec") => Value::IVec(vec![1; name[4..].parse().unwrap()]),
        name if name.starts_with("uvec") => Value::UVec(vec![1; name[4..].parse().unwrap()]),
        name if name.starts_with("vec") => Value::Vec(vec![0.5; name[3..].parse().unwrap()]),
        name if name.starts_with("mat") => {
            let dimension = name[3..].parse::<usize>().unwrap();
            let mut columns = vec![0.0; dimension * dimension];
            for index in 0..dimension {
                columns[index * dimension + index] = 1.0;
            }
            Value::Mat { dimension, columns }
        }
        _ => panic!("unhandled builtin sample type {value_type}"),
    }
}

fn assert_type(value: &Value, expected: &str, signature: &str) {
    let matches = match (value, expected) {
        (Value::Bool(_), "bool")
        | (Value::Int(_), "int")
        | (Value::Uint(_), "uint")
        | (Value::Float(_), "float") => true,
        (Value::BVec(v), ty) if ty.starts_with("bvec") => {
            v.len() == ty[4..].parse::<usize>().unwrap()
        }
        (Value::IVec(v), ty) if ty.starts_with("ivec") => {
            v.len() == ty[4..].parse::<usize>().unwrap()
        }
        (Value::UVec(v), ty) if ty.starts_with("uvec") => {
            v.len() == ty[4..].parse::<usize>().unwrap()
        }
        (Value::Vec(v), ty) if ty.starts_with("vec") => {
            v.len() == ty[3..].parse::<usize>().unwrap()
        }
        (Value::Mat { dimension, columns }, ty) if ty.starts_with("mat") => {
            *dimension == ty[3..].parse::<usize>().unwrap()
                && columns.len() == dimension * dimension
        }
        _ => false,
    };
    assert!(
        matches,
        "{signature} returned {}, expected {expected}",
        value.kind()
    );
}
