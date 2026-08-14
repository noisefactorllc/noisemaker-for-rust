#![recursion_limit = "256"]

use noisemaker_cpu::catalog::{ProgramIr, validate_program_ir};
use serde_json::{Value, json};

fn fixture() -> Value {
    json!({
        "outputs": ["fragColor"],
        "varyings": [],
        "structs": [],
        "uniforms": [{"name": "gain", "type": "float"}],
        "globals": [{"name": "fragColor", "type": "vec4"}],
        "functions": [
            {
                "name": "identity",
                "mangledName": "identity__float",
                "returnType": "float",
                "parameters": [{"name": "value", "type": "float", "qualifier": "in"}],
                "body": [{
                    "kind": "return",
                    "value": {"kind": "identifier", "type": "float", "name": "value", "storage": "parameter"}
                }]
            },
            {
                "name": "sink",
                "mangledName": "sink__float",
                "returnType": "void",
                "parameters": [{"name": "destination", "type": "float", "qualifier": "out"}],
                "body": [{
                    "kind": "expression",
                    "expression": {
                        "kind": "assignment", "type": "float", "operator": "=",
                        "lvalue": {"kind": "identifier", "type": "float", "name": "destination", "storage": "parameter"},
                        "value": {"kind": "literal", "type": "float", "value": 1.0, "source": "1.0"}
                    }
                }]
            },
            {
                "name": "main",
                "mangledName": "main__void",
                "returnType": "void",
                "parameters": [],
                "body": [
                    {
                        "kind": "declaration",
                        "declarations": [{
                            "name": "local", "type": "float",
                            "initializer": {
                                "kind": "call", "type": "float", "name": "abs", "target": "builtin:abs",
                                "arguments": [{"kind": "identifier", "type": "float", "name": "gain", "storage": "uniform"}]
                            },
                            "arraySize": null
                        }]
                    },
                    {
                        "kind": "expression",
                        "expression": {
                            "kind": "call", "type": "void", "name": "sink", "target": "sink__float",
                            "arguments": [{"kind": "identifier", "type": "float", "name": "local", "storage": "local"}]
                        }
                    },
                    {
                        "kind": "expression",
                        "expression": {
                            "kind": "assignment", "type": "vec4", "operator": "=",
                            "lvalue": {"kind": "identifier", "type": "vec4", "name": "fragColor", "storage": "global"},
                            "value": {
                                "kind": "construct", "type": "vec4", "arguments": [
                                    {
                                        "kind": "call", "type": "float", "name": "identity", "target": "identity__float",
                                        "arguments": [{"kind": "identifier", "type": "float", "name": "local", "storage": "local"}]
                                    },
                                    {"kind": "literal", "type": "float", "value": 0.0, "source": "0.0"},
                                    {"kind": "literal", "type": "float", "value": 0.0, "source": "0.0"},
                                    {"kind": "literal", "type": "float", "value": 1.0, "source": "1.0"}
                                ]
                            }
                        }
                    }
                ]
            }
        ]
    })
}

fn assert_invalid(mutator: impl FnOnce(&mut Value), expected: &str) {
    let mut value = fixture();
    mutator(&mut value);
    let program: ProgramIr = serde_json::from_value(value).unwrap();
    let error = validate_program_ir(&program).unwrap_err().to_string();
    assert!(
        error.contains(expected),
        "expected {expected:?} in {error:?}"
    );
}

#[test]
fn rejects_unknown_types_and_duplicate_symbols() {
    assert_invalid(
        |value| {
            value
                .pointer_mut("/uniforms/0/type")
                .unwrap()
                .clone_from(&json!("mystery"))
        },
        "unknown type",
    );
    assert_invalid(
        |value| {
            let duplicate = value.pointer("/globals/0").unwrap().clone();
            value
                .pointer_mut("/uniforms")
                .unwrap()
                .as_array_mut()
                .unwrap()
                .push(duplicate);
        },
        "duplicate symbol",
    );
}

#[test]
fn rejects_invalid_operators_and_malformed_lvalues() {
    assert_invalid(
        |value| {
            value
                .pointer_mut("/functions/2/body/2/expression/operator")
                .unwrap()
                .clone_from(&json!("**="))
        },
        "assignment operator",
    );
    assert_invalid(
        |value| {
            let literal = json!({
                "kind": "construct", "type": "vec4",
                "arguments": [{"kind": "literal", "type": "float", "value": 0.0}]
            });
            value
                .pointer_mut("/functions/2/body/2/expression/lvalue")
                .unwrap()
                .clone_from(&literal);
        },
        "lvalue",
    );
    assert_invalid(
        |value| {
            let postfix = json!({
                "kind": "postfix", "type": "float", "operator": "++",
                "lvalue": {"kind": "literal", "type": "float", "value": 1.0}
            });
            value
                .pointer_mut("/functions/2/body/1/expression")
                .unwrap()
                .clone_from(&postfix);
        },
        "lvalue",
    );
}

#[test]
fn rejects_unresolved_user_calls_and_nested_identifiers() {
    assert_invalid(
        |value| {
            value
                .pointer_mut("/functions/2/body/2/expression/value/arguments/0/target")
                .unwrap()
                .clone_from(&json!("missing__float"))
        },
        "unknown function target",
    );
    assert_invalid(
        |value| {
            let argument = value
                .pointer_mut("/functions/2/body/2/expression/value/arguments/0/arguments/0")
                .unwrap();
            argument["name"] = json!("missing_local");
        },
        "unresolved identifier",
    );
}

#[test]
fn rejects_builtin_target_name_mismatch_and_unknown_builtins() {
    assert_invalid(
        |value| {
            value
                .pointer_mut("/functions/2/body/0/declarations/0/initializer/target")
                .unwrap()
                .clone_from(&json!("builtin:sin"))
        },
        "builtin target",
    );
    assert_invalid(
        |value| {
            value
                .pointer_mut("/functions/2/body/0/declarations/0/initializer/name")
                .unwrap()
                .clone_from(&json!("notABuiltin"));
            value
                .pointer_mut("/functions/2/body/0/declarations/0/initializer/target")
                .unwrap()
                .clone_from(&json!("builtin:notABuiltin"));
        },
        "unknown builtin",
    );
}

#[test]
fn rejects_storage_type_and_out_argument_inconsistencies() {
    assert_invalid(
        |value| {
            value
                .pointer_mut("/functions/2/body/0/declarations/0/initializer/arguments/0/storage")
                .unwrap()
                .clone_from(&json!("local"))
        },
        "storage",
    );
    assert_invalid(
        |value| {
            value
                .pointer_mut("/functions/2/body/2/expression/value/arguments/0/arguments/0/type")
                .unwrap()
                .clone_from(&json!("vec2"))
        },
        "identifier type",
    );
    assert_invalid(
        |value| {
            let literal = json!({"kind": "literal", "type": "float", "value": 1.0});
            value
                .pointer_mut("/functions/2/body/1/expression/arguments/0")
                .unwrap()
                .clone_from(&literal);
        },
        "out argument",
    );
}

#[test]
fn hoisted_declarations_deserialize_and_resolve_in_the_enclosing_scope() {
    let mut value = fixture();
    value.pointer_mut("/functions/2/body").unwrap().clone_from(&json!([
        {
            "kind": "if",
            "hoistedDeclarations": [{"name": "branchValue", "type": "float"}],
            "condition": {"kind": "literal", "type": "bool", "value": true},
            "then": {"kind": "block", "body": []},
            "else": null
        },
        {
            "kind": "expression",
            "expression": {
                "kind": "assignment", "type": "float", "operator": "=",
                "lvalue": {"kind": "identifier", "type": "float", "name": "branchValue", "storage": "local"},
                "value": {"kind": "literal", "type": "float", "value": 1.0}
            }
        }
    ]));
    let program: ProgramIr = serde_json::from_value(value).unwrap();
    validate_program_ir(&program).unwrap();
}
