use std::collections::BTreeMap;

use noisemaker_cpu::catalog::{Expression, Statement, effect_catalog, shader_bundle};

#[test]
fn embedded_catalog_is_the_exact_cpu_inventory() {
    let catalog = effect_catalog().unwrap();
    assert_eq!(catalog.effects.len(), 210);
    assert_eq!(
        catalog.namespace_counts(),
        BTreeMap::from([
            ("classicNoisedeck", 20),
            ("filter", 113),
            ("filter3d", 2),
            ("mixer", 15),
            ("points", 11),
            ("render", 12),
            ("synth", 29),
            ("synth3d", 8),
        ])
    );
    // The formerly excluded reactive (MIDI/audio) and mesh (OBJ) effects are
    // imported into the catalog with host-fed external-input fixtures.
    for id in [
        "synth/roll",
        "synth/scope",
        "synth/spectrum",
        "render/meshLoader",
        "render/meshRender",
    ] {
        assert!(catalog.effects.contains_key(id));
    }
    assert!(catalog.excluded_effects.is_empty());
}

#[test]
fn catalog_retires_parameter_aliases_and_keeps_declaration_order() {
    let catalog = effect_catalog().unwrap();
    let noise = &catalog.effects["synth/noise"];
    assert_eq!(
        &noise.param_names[..5],
        &["type", "octaves", "scaleX", "scaleY", "seed"]
    );
    let aliased_effects = catalog
        .effects
        .iter()
        .filter_map(|(id, effect)| effect.param_aliases.as_ref().map(|_| id.as_str()))
        .collect::<Vec<&str>>();
    assert_eq!(
        aliased_effects,
        vec!["points/dla", "points/flow", "points/physarum"],
        "parameter aliases only appear on the three pinned points effects"
    );
    for id in ["points/dla", "points/flow", "points/physarum"] {
        assert_eq!(
            catalog.effects[id].param_aliases.as_ref(),
            Some(&BTreeMap::from([("fieldTex".into(), "heightTex".into())])),
            "{id} must carry the upstream heightTex alias"
        );
    }
}

#[test]
fn embedded_program_inventory_is_complete_and_structural() {
    let catalog = effect_catalog().unwrap();
    let shaders = shader_bundle().unwrap();
    let pass_keys = catalog
        .effects
        .values()
        .flat_map(|effect| effect.passes.iter())
        .filter_map(|pass| pass.key.as_deref())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(shaders.programs.len(), 302);
    assert_eq!(
        pass_keys,
        shaders.programs.keys().map(String::as_str).collect()
    );
    for program in shaders.programs.values() {
        assert!(!program.ir.functions.is_empty());
        assert!(
            program
                .ir
                .functions
                .iter()
                .any(|function| function.name == "main")
        );
    }
}

#[test]
fn typed_ir_deserializes_overloads_qualifiers_storage_and_lvalues() {
    let shaders = shader_bundle().unwrap();
    let mut saw_overload_target = false;
    let mut saw_out_qualifier = false;
    let mut saw_lvalue = false;
    for program in shaders.programs.values() {
        for function in &program.ir.functions {
            saw_out_qualifier |= function.parameters.iter().any(|parameter| {
                matches!(
                    parameter.qualifier,
                    noisemaker_cpu::catalog::ParameterQualifier::Out
                        | noisemaker_cpu::catalog::ParameterQualifier::InOut
                )
            });
            walk_statements(&function.body, &mut |expression| match expression {
                Expression::Call { target, .. } if !target.starts_with("builtin:") => {
                    saw_overload_target |= target.contains("__");
                }
                Expression::Assignment { lvalue, .. } => {
                    saw_lvalue |= matches!(
                        lvalue.as_ref(),
                        Expression::Identifier { .. }
                            | Expression::Member { .. }
                            | Expression::Index { .. }
                    );
                }
                _ => {}
            });
        }
    }
    assert!(saw_overload_target);
    assert!(saw_out_qualifier);
    assert!(saw_lvalue);
}

fn walk_statements(statements: &[Statement], visitor: &mut impl FnMut(&Expression)) {
    for statement in statements {
        statement.visit_expressions(visitor);
    }
}

#[test]
fn storage_targets_supply_missing_color_outputs_in_authored_order() {
    let mut pass = serde_json::json!({
        "name": "store",
        "program": "store",
        "storageTextures": { "zFirst": "left", "aSecond": "right" },
        "storageTextureOrder": ["zFirst", "aSecond"]
    });
    for outputs in [None, Some(serde_json::json!({}))] {
        if let Some(outputs) = outputs {
            pass["outputs"] = outputs;
        }
        let loaded: noisemaker_cpu::catalog::RenderPass =
            serde_json::from_value(pass.clone()).unwrap();
        assert_eq!(
            loaded.outputs.get("zFirst").map(String::as_str),
            Some("left")
        );
        assert_eq!(loaded.execution["outputOrder"], pass["storageTextureOrder"]);
    }
    pass["outputs"] = serde_json::json!({ "fragColor": "screen" });
    let loaded: noisemaker_cpu::catalog::RenderPass = serde_json::from_value(pass).unwrap();
    assert_eq!(
        loaded.outputs,
        BTreeMap::from([("fragColor".into(), "screen".into())])
    );
    assert!(!loaded.execution.contains_key("outputOrder"));
}

#[test]
fn storage_only_json_keeps_its_original_attachment_order() {
    let loaded: noisemaker_cpu::catalog::RenderPass = serde_json::from_str(
        r#"{"name":"raw","program":"raw","storageTextures":{"zFirst":"left","aSecond":"right"}}"#,
    )
    .unwrap();
    assert_eq!(
        loaded.execution["outputOrder"],
        serde_json::json!(["zFirst", "aSecond"])
    );
}

#[test]
fn published_resource_names_must_be_nonempty_strings_when_declared() {
    let base = serde_json::json!({
        "namespace": "synth", "func": "custom", "kind": "generator", "domain": "image",
        "paramNames": [], "passes": []
    });
    for field in [
        "outputTex",
        "outputTex3d",
        "outputGeo",
        "outputXyz",
        "outputVel",
        "outputRgba",
    ] {
        for invalid in [
            serde_json::Value::Null,
            serde_json::json!(7),
            serde_json::json!("  "),
        ] {
            let mut definition = base.clone();
            definition[field] = invalid;
            assert!(
                serde_json::from_value::<noisemaker_cpu::catalog::EffectDefinition>(definition)
                    .is_err(),
                "{field}"
            );
        }
    }
}
