use std::collections::BTreeMap;

use noisemaker_cpu::catalog::ProgramIr;
use noisemaker_cpu::{
    DerivativeMode, PixelContext, Runtime, ShaderOutputs, ShaderVm, Surface, Value, VmError,
};
use serde_json::{Value as Json, json};

fn literal(value_type: &str, value: impl Into<Json>) -> Json {
    json!({"kind":"literal","type":value_type,"value":value.into(),"source":null})
}

#[test]
fn vm_branch_hoisted_declaration_is_initialized_by_the_selected_nested_block() {
    let body = vec![
        json!({
            "kind":"if",
            "hoistedDeclarations":[{"name":"base","type":"float","initializer":null,"arraySize":null}],
            "condition":literal("bool", true),
            "then":{"kind":"block","body":[{"kind":"declaration","declarations":[
                {"name":"base","type":"float","initializer":literal("float",3.0),"arraySize":null}
            ]}]},
            "else":null
        }),
        expr(assign(
            "vec4",
            "=",
            id("vec4", "fragColor", "global"),
            json!({"kind":"construct","type":"vec4","arguments":[id("float","base","local")]}),
        )),
    ];
    let ir = program(
        vec![main_function(body)],
        &["fragColor"],
        vec![json!({"name":"fragColor","type":"vec4","initializer":null,"arraySize":null})],
        vec![],
        vec![],
    );
    let mut vm = ShaderVm::new(&ir, Runtime::new()).unwrap();
    let mut outputs = ShaderOutputs::new();
    vm.run_pixel(&PixelContext::default(), &mut outputs)
        .unwrap();
    assert_eq!(outputs["fragColor"], Value::Vec(vec![3.0; 4]));
}
fn id(value_type: &str, name: &str, storage: &str) -> Json {
    json!({"kind":"identifier","type":value_type,"name":name,"storage":storage})
}
fn binary(value_type: &str, operator: &str, left: Json, right: Json) -> Json {
    json!({"kind":"binary","type":value_type,"operator":operator,"left":left,"right":right})
}
fn assign(value_type: &str, operator: &str, lvalue: Json, value: Json) -> Json {
    json!({"kind":"assignment","type":value_type,"operator":operator,"lvalue":lvalue,"value":value})
}
fn expr(expression: Json) -> Json {
    json!({"kind":"expression","expression":expression})
}

fn program(
    functions: Vec<Json>,
    outputs: &[&str],
    globals: Vec<Json>,
    uniforms: Vec<Json>,
    structs: Vec<Json>,
) -> ProgramIr {
    serde_json::from_value(json!({"outputs":outputs,"varyings":[],"structs":structs,"uniforms":uniforms,"globals":globals,"functions":functions})).unwrap()
}

fn main_function(body: Vec<Json>) -> Json {
    json!({"name":"main","mangledName":"main__void","returnType":"void","parameters":[],"body":body})
}

#[test]
fn conditional_expression_converts_the_selected_branch_to_its_declared_type() {
    let conditional = json!({
        "kind":"conditional",
        "type":"float",
        "condition":literal("bool", true),
        "whenTrue":literal("int", 3),
        "whenFalse":literal("float", 1.5)
    });
    let ir = program(
        vec![main_function(vec![expr(assign(
            "vec4",
            "=",
            id("vec4", "fragColor", "global"),
            json!({"kind":"construct","type":"vec4","arguments":[
                binary("float", "+", conditional, literal("float", 0.5))
            ]}),
        ))])],
        &["fragColor"],
        vec![json!({"name":"fragColor","type":"vec4","initializer":null,"arraySize":null})],
        vec![],
        vec![],
    );
    let mut vm = ShaderVm::new(&ir, Runtime::new()).unwrap();
    let mut outputs = BTreeMap::new();
    vm.run_pixel(&PixelContext::default(), &mut outputs)
        .unwrap();
    assert_eq!(outputs["fragColor"], Value::Vec(vec![3.5; 4]));
}

#[test]
fn ordinary_direct_conditional_assignment_writes_the_true_branch() {
    let ir = program(
        vec![main_function(vec![expr(assign(
            "vec4",
            "=",
            id("vec4", "fragColor", "global"),
            json!({
                "kind":"conditional",
                "type":"vec4",
                "condition":literal("bool", true),
                "whenTrue":{"kind":"construct","type":"vec4","arguments":[
                    literal("float",1.0),literal("float",2.0),literal("float",3.0),literal("float",4.0)
                ]},
                "whenFalse":{"kind":"construct","type":"vec4","arguments":[
                    literal("float",5.0),literal("float",6.0),literal("float",7.0),literal("float",8.0)
                ]}
            }),
        ))])],
        &["fragColor"],
        vec![json!({"name":"fragColor","type":"vec4","initializer":null,"arraySize":null})],
        vec![],
        vec![],
    );
    let mut vm = ShaderVm::new(&ir, Runtime::new()).unwrap();
    let mut outputs = BTreeMap::new();
    vm.run_pixel(&PixelContext::default(), &mut outputs)
        .unwrap();
    assert_eq!(outputs["fragColor"], Value::Vec(vec![1.0, 2.0, 3.0, 4.0]));
}

#[test]
fn canonical_hash_uint_call_matches_javascript_cpu_semantics() {
    let canonical_hash = json!({
        "name":"hash_uint",
        "mangledName":"hash_uint__uint",
        "returnType":"uint",
        "parameters":[{"name":"value","type":"uint","qualifier":"in"}],
        "body":[{"kind":"return","value":literal("uint",99)}]
    });
    let user_hash = json!({
        "name":"user_hash",
        "mangledName":"user_hash__uint",
        "returnType":"uint",
        "parameters":[{"name":"value","type":"uint","qualifier":"in"}],
        "body":[{"kind":"return","value":literal("uint",99)}]
    });
    let call = |name: &str, target: &str, value: u64| {
        json!({
            "kind":"call", "type":"uint", "name":name, "target":target,
            "arguments":[literal("uint",value)]
        })
    };
    let ir = program(
        vec![
            canonical_hash,
            user_hash,
            main_function(vec![expr(assign(
                "vec4",
                "=",
                id("vec4", "result", "global"),
                json!({"kind":"construct","type":"vec4","arguments":[
                    call("hash_uint", "hash_uint__uint", 0),
                    call("hash_uint", "hash_uint__uint", 1),
                    call("hash_uint", "hash_uint__uint", u32::MAX.into()),
                    call("user_hash", "user_hash__uint", 1)
                ]}),
            ))]),
        ],
        &["result"],
        vec![json!({"name":"result","type":"vec4","initializer":null,"arraySize":null})],
        vec![],
        vec![],
    );
    let mut vm = ShaderVm::new(&ir, Runtime::new()).unwrap();
    let mut outputs = ShaderOutputs::new();
    vm.run_pixel(&PixelContext::default(), &mut outputs)
        .unwrap();
    assert_eq!(
        outputs["result"],
        Value::Vec(vec![
            0.0,
            1_753_845_952_u32 as f32,
            1_734_902_346_u32 as f32,
            99.0,
        ])
    );
}

#[test]
fn binary_expression_converts_operands_to_their_typed_ir_declarations() {
    let selected_noise_helper = json!({
        "name":"selectedNoiseHelper",
        "mangledName":"selectedNoiseHelper__void",
        "returnType":"float",
        "parameters":[],
        "body":[{"kind":"return","value":literal("int", 3)}]
    });
    let helper_call = json!({
        "kind":"call",
        "type":"float",
        "name":"selectedNoiseHelper",
        "target":"selectedNoiseHelper__void",
        "arguments":[]
    });
    let ir = program(
        vec![
            selected_noise_helper,
            main_function(vec![expr(assign(
                "vec4",
                "=",
                id("vec4", "fragColor", "global"),
                json!({"kind":"construct","type":"vec4","arguments":[
                    binary("float", "+", helper_call, literal("float", 0.5))
                ]}),
            ))]),
        ],
        &["fragColor"],
        vec![json!({"name":"fragColor","type":"vec4","initializer":null,"arraySize":null})],
        vec![],
        vec![],
    );
    let mut vm = ShaderVm::new(&ir, Runtime::new()).unwrap();
    let mut outputs = BTreeMap::new();
    vm.run_pixel(&PixelContext::default(), &mut outputs)
        .unwrap();
    assert_eq!(outputs["fragColor"], Value::Vec(vec![3.5; 4]));
}

#[test]
fn vm_executes_scopes_loops_lvalues_assignment_expressions_and_copy_back() {
    let mutate = json!({
        "name":"mutate","mangledName":"mutate__int_int","returnType":"void",
        "parameters":[
            {"name":"a","type":"int","qualifier":"inout"},
            {"name":"b","type":"int","qualifier":"out"}
        ],
        "body":[
            expr(assign("int","+=",id("int","a","parameter"),literal("int",2))),
            expr(assign("int","=",id("int","b","parameter"),id("int","a","parameter"))),
            {"kind":"return","value":null}
        ]
    });
    let body = vec![
        json!({"kind":"declaration","declarations":[
            {"name":"a","type":"int","initializer":literal("int",1),"arraySize":null},
            {"name":"b","type":"int","initializer":literal("int",0),"arraySize":null},
            {"name":"i","type":"int","initializer":literal("int",0),"arraySize":null},
            {"name":"v","type":"vec2","initializer":{"kind":"construct","type":"vec2","arguments":[literal("float",1.0),literal("float",2.0)]},"arraySize":null},
            {"name":"arr","type":"float[]","initializer":{"kind":"construct","type":"float[]","arguments":[literal("float",5.0),literal("float",6.0)]},"arraySize":literal("int",2)}
        ]}),
        json!({"kind":"for",
            "initializer":expr(assign("int","=",id("int","i","local"),literal("int",0))),
            "condition":binary("bool","<",id("int","i","local"),literal("int",4)),
            "update":{"kind":"postfix","type":"int","operator":"++","lvalue":id("int","i","local")},
            "body":{"kind":"block","body":[
                {"kind":"if","hoistedDeclarations":[],"condition":binary("bool","==",id("int","i","local"),literal("int",1)),"then":{"kind":"continue"},"else":null},
                expr(assign("int","+=",id("int","a","local"),id("int","i","local"))),
                {"kind":"if","hoistedDeclarations":[],"condition":binary("bool",">",id("int","a","local"),literal("int",5)),"then":{"kind":"break"},"else":null}
            ]}
        }),
        expr(
            json!({"kind":"call","type":"void","name":"mutate","target":"mutate__int_int","arguments":[id("int","a","local"),id("int","b","local")]}),
        ),
        expr(assign(
            "vec2",
            "=",
            json!({"kind":"member","type":"vec2","object":id("vec2","v","local"),"field":"yx"}),
            json!({"kind":"member","type":"vec2","object":id("vec2","v","local"),"field":"xy"}),
        )),
        expr(assign(
            "float",
            "+=",
            json!({"kind":"index","type":"float","object":id("vec2","v","local"),"index":literal("int",0)}),
            literal("float", 1.0),
        )),
        expr(assign(
            "float",
            "=",
            json!({"kind":"index","type":"float","object":id("float[]","arr","local"),"index":literal("int",1)}),
            literal("float", 9.0),
        )),
        expr(assign(
            "vec4",
            "=",
            id("vec4", "fragColor", "global"),
            json!({"kind":"construct","type":"vec4","arguments":[
                {"kind":"construct","type":"float","arguments":[id("int","a","local")]},
                {"kind":"construct","type":"float","arguments":[id("int","b","local")]},
                {"kind":"member","type":"float","object":id("vec2","v","local"),"field":"x"},
                {"kind":"index","type":"float","object":id("float[]","arr","local"),"index":literal("int",1)}
            ]}),
        )),
    ];
    let ir = program(
        vec![mutate, main_function(body)],
        &["fragColor"],
        vec![json!({"name":"fragColor","type":"vec4","initializer":null,"arraySize":null})],
        vec![],
        vec![],
    );
    let mut vm = ShaderVm::new(&ir, Runtime::new()).unwrap();
    let mut outputs = ShaderOutputs::new();
    vm.run_pixel(&PixelContext::default(), &mut outputs)
        .unwrap();
    assert_eq!(outputs["fragColor"], Value::Vec(vec![8.0, 8.0, 3.0, 9.0]));
}

#[test]
fn vm_supports_struct_members_prefix_ternary_while_and_do_while() {
    let body = vec![
        json!({"kind":"declaration","declarations":[
            {"name":"state","type":"State","initializer":{"kind":"construct","type":"State","arguments":[literal("float",0.0)]},"arraySize":null},
            {"name":"i","type":"int","initializer":literal("int",0),"arraySize":null}
        ]}),
        expr(assign(
            "float",
            "=",
            json!({"kind":"member","type":"float","object":id("State","state","local"),"field":"x"}),
            literal("float", 3.0),
        )),
        json!({"kind":"while","condition":binary("bool","<",id("int","i","local"),literal("int",2)),"body":{"kind":"block","body":[expr(json!({"kind":"unary","type":"int","operator":"++","operand":id("int","i","local")}))]}}),
        json!({"kind":"doWhile","condition":literal("bool",false),"body":{"kind":"block","body":[expr(assign("float","+=",json!({"kind":"member","type":"float","object":id("State","state","local"),"field":"x"}),literal("float",1.0)))]}}),
        expr(assign(
            "vec4",
            "=",
            id("vec4", "fragColor", "global"),
            json!({"kind":"construct","type":"vec4","arguments":[
                {"kind":"conditional","type":"float","condition":binary("bool","==",id("int","i","local"),literal("int",2)),"whenTrue":json!({"kind":"member","type":"float","object":id("State","state","local"),"field":"x"}),"whenFalse":literal("float",0.0)}, literal("float",0.0),literal("float",0.0),literal("float",1.0)
            ]}),
        )),
    ];
    let ir = program(
        vec![main_function(body)],
        &["fragColor"],
        vec![json!({"name":"fragColor","type":"vec4","initializer":null,"arraySize":null})],
        vec![],
        vec![
            json!({"name":"State","fields":[{"name":"x","type":"float","initializer":null,"arraySize":null}]}),
        ],
    );
    let mut vm = ShaderVm::new(&ir, Runtime::new()).unwrap();
    let mut outputs = BTreeMap::new();
    vm.run_pixel(&PixelContext::default(), &mut outputs)
        .unwrap();
    assert_eq!(outputs["fragColor"], Value::Vec(vec![4.0, 0.0, 0.0, 1.0]));
}

#[test]
fn vm_rejects_unbounded_execution_recursion_and_discard_deterministically() {
    let infinite = program(
        vec![main_function(vec![
            json!({"kind":"while","condition":literal("bool",true),"body":{"kind":"block","body":[]}}),
        ])],
        &[],
        vec![],
        vec![],
        vec![],
    );
    let mut vm = ShaderVm::new(&infinite, Runtime::new()).unwrap();
    assert_eq!(
        vm.run_pixel(&PixelContext::default(), &mut BTreeMap::new()),
        Err(VmError::StatementLimit { limit: 1_048_576 })
    );

    let discard = program(
        vec![main_function(vec![json!({"kind":"discard"})])],
        &[],
        vec![],
        vec![],
        vec![],
    );
    let mut vm = ShaderVm::new(&discard, Runtime::new()).unwrap();
    assert_eq!(
        vm.run_pixel(&PixelContext::default(), &mut BTreeMap::new()),
        Err(VmError::Discarded)
    );

    let recurse = json!({"name":"recurse","mangledName":"recurse__void","returnType":"void","parameters":[],"body":[
        expr(json!({"kind":"call","type":"void","name":"recurse","target":"recurse__void","arguments":[]}))
    ]});
    let recursive = program(
        vec![
            recurse,
            main_function(vec![expr(
                json!({"kind":"call","type":"void","name":"recurse","target":"recurse__void","arguments":[]}),
            )]),
        ],
        &[],
        vec![],
        vec![],
        vec![],
    );
    let mut vm = ShaderVm::new(&recursive, Runtime::new()).unwrap();
    assert_eq!(
        vm.run_pixel(&PixelContext::default(), &mut BTreeMap::new()),
        Err(VmError::CallDepthLimit { limit: 64 })
    );
}

#[test]
fn vm_records_and_replays_all_fine_derivative_builtins() {
    let derivative = |name: &str| json!({"kind":"call","type":"float","name":name,"target":format!("builtin:{name}"),"arguments":[id("float","x","uniform")]});
    let body = vec![expr(assign(
        "vec4",
        "=",
        id("vec4", "fragColor", "global"),
        json!({"kind":"construct","type":"vec4","arguments":[derivative("dFdx"),derivative("dFdy"),derivative("fwidth"),literal("float",1.0)]}),
    ))];
    let ir = program(
        vec![main_function(body)],
        &["fragColor"],
        vec![json!({"name":"fragColor","type":"vec4","initializer":null,"arraySize":null})],
        vec![json!({"name":"x","type":"float","initializer":null,"arraySize":null})],
        vec![],
    );

    let mut lane_logs = Vec::new();
    for x in [1.0, 4.0, 11.0, 20.0] {
        let mut runtime = Runtime::new();
        runtime.set_derivative_mode(DerivativeMode::Record);
        let mut vm = ShaderVm::new(&ir, runtime).unwrap();
        vm.run_pixel(
            &PixelContext {
                uniforms: BTreeMap::from([("x".into(), Value::Float(x))]),
                ..Default::default()
            },
            &mut BTreeMap::new(),
        )
        .unwrap();
        lane_logs.push(vm.runtime_mut().take_derivative_log());
    }
    let diffs = Runtime::fine_derivatives(&lane_logs, 0, 0).unwrap();
    let mut runtime = Runtime::new();
    runtime.set_derivative_replay(diffs);
    let mut vm = ShaderVm::new(&ir, runtime).unwrap();
    let mut outputs = BTreeMap::new();
    vm.run_pixel(
        &PixelContext {
            uniforms: BTreeMap::from([("x".into(), Value::Float(1.0))]),
            ..Default::default()
        },
        &mut outputs,
    )
    .unwrap();
    assert_eq!(outputs["fragColor"], Value::Vec(vec![3.0, 10.0, 13.0, 1.0]));
    assert_eq!(vm.runtime().derivative_call_count(), 3);

    lane_logs[3].pop();
    assert!(matches!(
        Runtime::fine_derivatives(&lane_logs, 0, 0),
        Err(VmError::DivergentDerivatives { lane: 3, call: 2 })
    ));
}

#[test]
fn vm_writes_all_indexed_types_and_nested_lvalue_compositions() {
    let index = |value_type: &str, object: Json, offset: i64| json!({"kind":"index","type":value_type,"object":object,"index":literal("int",offset)});
    let member = |value_type: &str, object: Json, field: &str| json!({"kind":"member","type":value_type,"object":object,"field":field});
    let output = |name: &str, arguments: Vec<Json>| {
        expr(assign(
            "vec4",
            "=",
            id("vec4", name, "global"),
            json!({"kind":"construct","type":"vec4","arguments":arguments}),
        ))
    };
    let declarations = json!({"kind":"declaration","declarations":[
        {"name":"bv","type":"bvec2","initializer":{"kind":"construct","type":"bvec2","arguments":[literal("bool",true),literal("bool",false)]},"arraySize":null},
        {"name":"iv","type":"ivec3","initializer":{"kind":"construct","type":"ivec3","arguments":[literal("int",1),literal("int",2),literal("int",3)]},"arraySize":null},
        {"name":"uv","type":"uvec3","initializer":{"kind":"construct","type":"uvec3","arguments":[literal("uint",1),literal("uint",2),literal("uint",3)]},"arraySize":null},
        {"name":"fv","type":"vec3","initializer":{"kind":"construct","type":"vec3","arguments":[literal("float",1.0),literal("float",2.0),literal("float",3.0)]},"arraySize":null},
        {"name":"m","type":"mat2","initializer":{"kind":"construct","type":"mat2","arguments":[literal("float",1.0),literal("float",2.0),literal("float",3.0),literal("float",4.0)]},"arraySize":null},
        {"name":"arr","type":"vec3[]","initializer":{"kind":"construct","type":"vec3[]","arguments":[{"kind":"construct","type":"vec3","arguments":[literal("float",1.0),literal("float",2.0),literal("float",3.0)]}]},"arraySize":literal("int",1)},
        {"name":"state","type":"State","initializer":{"kind":"construct","type":"State","arguments":[{"kind":"construct","type":"vec3","arguments":[literal("float",1.0),literal("float",2.0),literal("float",3.0)]}]},"arraySize":null}
    ]});
    let bv1 = index("bool", id("bvec2", "bv", "local"), 1);
    let iv1 = index("int", id("ivec3", "iv", "local"), 1);
    let uv2 = index("uint", id("uvec3", "uv", "local"), 2);
    let fv2 = index("float", id("vec3", "fv", "local"), 2);
    let m0 = index("vec2", id("mat2", "m", "local"), 0);
    let m1 = index("vec2", id("mat2", "m", "local"), 1);
    let m01 = index("float", m0.clone(), 1);
    let arr0 = index("vec3", id("vec3[]", "arr", "local"), 0);
    let state_v = member("vec3", id("State", "state", "local"), "v");
    let fv_yx = member("vec2", id("vec3", "fv", "local"), "yx");
    let body = vec![
        declarations,
        expr(assign("bool", "=", bv1.clone(), literal("bool", true))),
        expr(assign("int", "+=", iv1.clone(), literal("int", 5))),
        expr(assign("uint", "*=", uv2.clone(), literal("uint", 4))),
        expr(assign("float", "+=", fv2.clone(), literal("float", 4.0))),
        expr(assign(
            "vec2",
            "=",
            m1.clone(),
            json!({"kind":"construct","type":"vec2","arguments":[literal("float",8.0),literal("float",9.0)]}),
        )),
        expr(assign("float", "=", m01.clone(), literal("float", 7.0))),
        expr(assign(
            "vec2",
            "=",
            member("vec2", arr0.clone(), "yz"),
            json!({"kind":"construct","type":"vec2","arguments":[literal("float",8.0),literal("float",9.0)]}),
        )),
        expr(assign(
            "vec2",
            "=",
            member("vec2", state_v.clone(), "yx"),
            json!({"kind":"construct","type":"vec2","arguments":[literal("float",5.0),literal("float",6.0)]}),
        )),
        expr(assign(
            "float",
            "=",
            index("float", fv_yx, 0),
            literal("float", 9.0),
        )),
        output(
            "o0",
            vec![
                json!({"kind":"conditional","type":"float","condition":bv1,"whenTrue":literal("float",1.0),"whenFalse":literal("float",0.0)}),
                json!({"kind":"construct","type":"float","arguments":[iv1]}),
                json!({"kind":"construct","type":"float","arguments":[uv2]}),
                fv2,
            ],
        ),
        output(
            "o1",
            vec![
                index("float", m0.clone(), 0),
                m01,
                index("float", m1.clone(), 0),
                index("float", m1, 1),
            ],
        ),
        output("o2", vec![arr0, literal("float", 1.0)]),
        output("o3", vec![state_v, literal("float", 1.0)]),
        output("o4", vec![id("vec3", "fv", "local"), literal("float", 1.0)]),
    ];
    let globals = ["o0", "o1", "o2", "o3", "o4"]
        .into_iter()
        .map(|name| json!({"name":name,"type":"vec4","initializer":null,"arraySize":null}))
        .collect();
    let ir = program(
        vec![main_function(body)],
        &["o0", "o1", "o2", "o3", "o4"],
        globals,
        vec![],
        vec![
            json!({"name":"State","fields":[{"name":"v","type":"vec3","initializer":null,"arraySize":null}]}),
        ],
    );
    let mut vm = ShaderVm::new(&ir, Runtime::new()).unwrap();
    let mut outputs = BTreeMap::new();
    vm.run_pixel(&PixelContext::default(), &mut outputs)
        .unwrap();
    assert_eq!(outputs["o0"], Value::Vec(vec![1.0, 7.0, 12.0, 7.0]));
    assert_eq!(outputs["o1"], Value::Vec(vec![1.0, 7.0, 8.0, 9.0]));
    assert_eq!(outputs["o2"], Value::Vec(vec![1.0, 8.0, 9.0, 1.0]));
    assert_eq!(outputs["o3"], Value::Vec(vec![6.0, 5.0, 3.0, 1.0]));
    assert_eq!(outputs["o4"], Value::Vec(vec![1.0, 9.0, 7.0, 1.0]));
}

#[test]
fn vm_validation_rejects_duplicate_component_swizzle_writes() {
    let body = vec![
        json!({"kind":"declaration","declarations":[{"name":"v","type":"vec2","initializer":{"kind":"construct","type":"vec2","arguments":[literal("float",1.0),literal("float",2.0)]},"arraySize":null}]}),
        expr(assign(
            "vec2",
            "=",
            json!({"kind":"member","type":"vec2","object":id("vec2","v","local"),"field":"xx"}),
            json!({"kind":"construct","type":"vec2","arguments":[literal("float",3.0),literal("float",4.0)]}),
        )),
    ];
    let ir = program(vec![main_function(body)], &[], vec![], vec![], vec![]);
    assert!(
        matches!(ShaderVm::new(&ir,Runtime::new()),Err(VmError::InvalidIr { message }) if message.contains("not a writable lvalue"))
    );
}

#[test]
fn vm_wires_texture_and_derivative_builtins_through_runtime() {
    let body = vec![expr(assign(
        "vec4",
        "=",
        id("vec4", "fragColor", "global"),
        json!({"kind":"call","type":"vec4","name":"texture","target":"builtin:texture","arguments":[id("sampler2D","tex","uniform"),id("vec2","uv","varying")]}),
    ))];
    let mut ir = program(
        vec![main_function(body)],
        &["fragColor"],
        vec![json!({"name":"fragColor","type":"vec4","initializer":null,"arraySize":null})],
        vec![json!({"name":"tex","type":"sampler2D","initializer":null,"arraySize":null})],
        vec![],
    );
    ir.varyings.push("uv".into());
    let mut runtime = Runtime::new();
    let handle = runtime.add_texture(Surface::from_f32(1, 1, vec![0.25, 0.5, 0.75, 1.0]).unwrap());
    runtime.set_derivative_mode(DerivativeMode::Off);
    let mut vm = ShaderVm::new(&ir, runtime).unwrap();
    let context = PixelContext {
        uniforms: BTreeMap::from([("tex".into(), handle)]),
        varyings: BTreeMap::from([("uv".into(), Value::Vec(vec![0.5, 0.5]))]),
        ..Default::default()
    };
    let mut outputs = BTreeMap::new();
    vm.run_pixel(&context, &mut outputs).unwrap();
    assert_eq!(outputs["fragColor"], Value::Vec(vec![0.25, 0.5, 0.75, 1.0]));
}
