use std::collections::BTreeMap;

use noisemaker_cpu::catalog::{Expression, Statement, effect_catalog, shader_bundle};

#[test]
fn embedded_catalog_is_the_exact_cpu_inventory() {
    let catalog = effect_catalog().unwrap();
    assert_eq!(catalog.effects.len(), 205);
    assert_eq!(
        catalog.namespace_counts(),
        BTreeMap::from([
            ("classicNoisedeck", 20),
            ("filter", 116),
            ("filter3d", 2),
            ("mixer", 15),
            ("points", 10),
            ("render", 9),
            ("synth", 26),
            ("synth3d", 7),
        ])
    );
    for id in [
        "synth/roll",
        "synth/scope",
        "synth/spectrum",
        "render/meshLoader",
        "render/meshRender",
    ] {
        assert!(!catalog.effects.contains_key(id));
        assert!(
            catalog
                .excluded_effects
                .iter()
                .any(|excluded| excluded == id)
        );
    }
}

#[test]
fn catalog_preserves_canonical_parameter_aliases_and_declaration_order() {
    let catalog = effect_catalog().unwrap();
    assert_eq!(
        catalog
            .effects
            .values()
            .filter(|effect| !effect.param_aliases.is_empty())
            .count(),
        44
    );
    let noise = &catalog.effects["synth/noise"];
    assert_eq!(noise.param_aliases["xScale"], "scaleX");
    assert_eq!(noise.param_aliases["yScale"], "scaleY");
    assert_eq!(noise.param_aliases["noiseType"], "type");
    assert_eq!(
        &noise.param_names[..5],
        &["type", "octaves", "scaleX", "scaleY", "seed"]
    );
    assert_eq!(
        catalog.effects["synth/osc2d"].param_aliases["frequency"],
        "freq"
    );
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
    assert_eq!(shaders.programs.len(), 288);
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
