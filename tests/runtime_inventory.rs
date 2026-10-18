use std::collections::BTreeSet;

use noisemaker_cpu::catalog::{Expression, ProgramIr, Type, shader_bundle};
use noisemaker_cpu::{PixelContext, Runtime, ShaderOutputs, ShaderVm, Surface, Value};
use serde_json::json;

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
        54
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
    let mut cpu_compound = BTreeSet::new();
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
                    Expression::Assignment {
                        operator,
                        value_type,
                        lvalue,
                        value,
                    } => {
                        assignment.insert(operator.clone());
                        if matches!(operator.as_str(), "metal/=" | "trunc/=") {
                            assert!(program.ir.cpu_murmur_functions.is_some());
                            cpu_compound.insert((
                                operator.clone(),
                                expression_type(lvalue).0.clone(),
                                expression_type(value).0.clone(),
                                value_type.0.clone(),
                            ));
                        }
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
            "^=".into(),
            "metal/=".into(),
            "trunc/=".into()
        ])
    );
    assert_eq!(postfix, BTreeSet::from(["++".into(), "--".into()]));

    let runtime = Runtime::new();
    let mut cpu_binary_count = 0;
    for (operator, left_type, right_type, return_type) in signatures {
        if matches!(operator.as_str(), "metal/" | "trunc/") {
            assert_cpu_signature(&operator, &left_type, &right_type, &return_type, false);
            cpu_binary_count += 1;
            continue;
        }
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
    assert_eq!(cpu_binary_count, 8);
    assert_eq!(cpu_compound.len(), 2);
    for (operator, left_type, right_type, return_type) in cpu_compound {
        assert_cpu_signature(&operator, &left_type, &right_type, &return_type, true);
    }
}

fn assert_cpu_signature(
    operator: &str,
    left_type: &str,
    right_type: &str,
    return_type: &str,
    compound: bool,
) {
    let identifier = |ty: &str, name: &str, storage: &str| json!({"kind":"identifier","type":ty,"name":name,"storage":storage});
    let mut body = Vec::new();
    let expression = if compound {
        body.push(json!({"kind":"declaration","declarations":[{"name":"result","type":left_type,"arraySize":null,"initializer":identifier(left_type,"left","uniform")}]}));
        json!({"kind":"assignment","type":return_type,"operator":operator,"lvalue":identifier(left_type,"result","local"),"value":identifier(right_type,"right","uniform")})
    } else {
        json!({"kind":"binary","type":return_type,"operator":operator,"left":identifier(left_type,"left","uniform"),"right":identifier(right_type,"right","uniform")})
    };
    let width = match return_type {
        "float" | "int" | "uint" => 1,
        "vec2" => 2,
        "vec3" => 3,
        "vec4" => 4,
        _ => panic!("unhandled CPU result type {return_type}"),
    };
    let mut output_arguments = vec![expression];
    if width > 1 {
        for _ in width..4 {
            output_arguments
                .push(json!({"kind":"literal","type":"float","value":0.0,"source":"0.0"}));
        }
    }
    body.push(json!({"kind":"expression","expression":{"kind":"assignment","type":"vec4","operator":"=","lvalue":identifier("vec4","fragColor","global"),"value":{"kind":"construct","type":"vec4","arguments":output_arguments}}}));
    let ir: ProgramIr = serde_json::from_value(json!({
        "cpuMurmurFunctions":[],"outputs":["fragColor"],"varyings":[],"structs":[],
        "uniforms":[{"name":"left","type":left_type,"arraySize":null,"initializer":null},{"name":"right","type":right_type,"arraySize":null,"initializer":null}],
        "globals":[{"name":"fragColor","type":"vec4","arraySize":null,"initializer":null}],
        "functions":[{"name":"main","mangledName":"main__void","returnType":"void","parameters":[],"body":body}]
    })).unwrap();
    let truncated = operator.starts_with("trunc");
    let runtime = Runtime::new();
    let mut context = PixelContext::default();
    context.uniforms.insert(
        "left".into(),
        runtime
            .construct(
                left_type,
                &[Value::Float(if truncated { 7.0 } else { 1.0 })],
            )
            .unwrap(),
    );
    context.uniforms.insert(
        "right".into(),
        runtime
            .construct(
                right_type,
                &[Value::Float(if truncated { 4.0 } else { 41.0 })],
            )
            .unwrap(),
    );
    let signature = format!("CPU {left_type} {operator} {right_type} -> {return_type}");
    let mut vm = ShaderVm::new(&ir, runtime).unwrap_or_else(|error| panic!("{signature}: {error}"));
    let mut outputs = ShaderOutputs::new();
    vm.run_pixel(&context, &mut outputs)
        .unwrap_or_else(|error| panic!("{signature}: {error}"));
    assert_type(&outputs["fragColor"], "vec4", &signature);
    let Value::Vec(lanes) = &outputs["fragColor"] else {
        unreachable!()
    };
    let result_width = if width == 1 { 4 } else { width };
    let expected_bits = if truncated {
        1.0_f32.to_bits()
    } else {
        1019727373
    };
    assert_eq!(
        lanes
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>(),
        (0..4)
            .map(|index| if index < result_width {
                expected_bits
            } else {
                0
            })
            .collect::<Vec<_>>(),
        "{signature}"
    );
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
