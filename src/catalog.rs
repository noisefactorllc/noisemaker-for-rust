use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

use crate::generated::{BUNDLE_LOCK_JSON, CATALOG_JSON, SHADERS_JSON};

mod validation;
pub use validation::validate_program_ir;

const EXPECTED_EFFECTS: usize = 205;
const EXPECTED_PROGRAMS: usize = 288;
const EXPECTED_EXCLUSIONS: [&str; 5] = [
    "render/meshLoader",
    "render/meshRender",
    "synth/roll",
    "synth/scope",
    "synth/spectrum",
];

#[derive(Debug, Error)]
pub enum CatalogError {
    #[error("embedded {artifact} JSON is invalid: {source}")]
    Json {
        artifact: &'static str,
        #[source]
        source: serde_json::Error,
    },
    #[error("embedded bundle is inconsistent: {0}")]
    Invalid(String),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectCatalog {
    pub provenance: Provenance,
    pub excluded_effects: Vec<String>,
    pub effects: BTreeMap<String, EffectDefinition>,
}

impl EffectCatalog {
    #[must_use]
    pub fn namespace_counts(&self) -> BTreeMap<&str, usize> {
        let mut counts = BTreeMap::new();
        for effect in self.effects.values() {
            *counts.entry(effect.namespace.as_str()).or_default() += 1;
        }
        counts
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Provenance {
    pub source: String,
    pub version: String,
    pub base: String,
    pub lock_source: String,
    pub lock_version: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectDefinition {
    pub namespace: String,
    pub func: String,
    pub kind: String,
    pub domain: String,
    #[serde(default)]
    pub params: BTreeMap<String, ParameterSpec>,
    pub param_names: Vec<String>,
    #[serde(default)]
    pub param_aliases: BTreeMap<String, String>,
    #[serde(default)]
    pub textures: BTreeMap<String, Value>,
    pub passes: Vec<RenderPass>,
    #[serde(default)]
    pub iterated: bool,
    pub external_texture: Option<String>,
    pub output_tex3d: Option<String>,
    pub output_geo: Option<String>,
    pub loop_role: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ParameterSpec {
    #[serde(rename = "type")]
    pub parameter_type: String,
    #[serde(default)]
    pub default: Value,
    pub uniform: Option<String>,
    pub define: Option<String>,
    #[serde(default, flatten)]
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderPass {
    pub name: String,
    pub program: String,
    pub key: Option<String>,
    #[serde(default)]
    pub inputs: BTreeMap<String, String>,
    #[serde(default)]
    pub outputs: BTreeMap<String, String>,
    #[serde(default, flatten)]
    pub execution: BTreeMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct ShaderBundle {
    pub provenance: Provenance,
    pub programs: BTreeMap<String, ShaderProgram>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShaderProgram {
    pub effect_id: String,
    pub program: String,
    pub source_sha256: String,
    pub ir: ProgramIr,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgramIr {
    pub outputs: Vec<String>,
    pub varyings: Vec<String>,
    pub structs: Vec<StructDefinition>,
    pub uniforms: Vec<VariableDefinition>,
    pub globals: Vec<VariableDefinition>,
    pub functions: Vec<Function>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct StructDefinition {
    pub name: String,
    pub fields: Vec<VariableDefinition>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableDefinition {
    pub name: String,
    #[serde(rename = "type")]
    pub value_type: Type,
    #[serde(default)]
    pub initializer: Option<Expression>,
    #[serde(default)]
    pub array_size: Option<Expression>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(transparent)]
pub struct Type(pub String);

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Function {
    pub name: String,
    pub mangled_name: String,
    pub return_type: Type,
    pub parameters: Vec<FunctionParameter>,
    pub body: Vec<Statement>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FunctionParameter {
    pub name: String,
    #[serde(rename = "type")]
    pub value_type: Type,
    pub qualifier: ParameterQualifier,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ParameterQualifier {
    In,
    Out,
    InOut,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StorageClass {
    Uniform,
    Global,
    Local,
    Varying,
    Parameter,
    Builtin,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Statement {
    Block {
        body: Vec<Statement>,
    },
    Declaration {
        declarations: Vec<VariableDefinition>,
    },
    Expression {
        expression: Expression,
    },
    If {
        #[serde(rename = "hoistedDeclarations", default)]
        hoisted_declarations: Vec<VariableDefinition>,
        condition: Expression,
        then: Box<Statement>,
        #[serde(rename = "else")]
        otherwise: Option<Box<Statement>>,
    },
    For {
        initializer: Option<Box<Statement>>,
        condition: Option<Expression>,
        update: Option<Expression>,
        body: Box<Statement>,
    },
    While {
        condition: Expression,
        body: Box<Statement>,
    },
    DoWhile {
        condition: Expression,
        body: Box<Statement>,
    },
    Return {
        value: Option<Expression>,
    },
    Break,
    Continue,
    Discard,
}

impl Statement {
    pub fn visit_expressions(&self, visitor: &mut impl FnMut(&Expression)) {
        match self {
            Self::Block { body } => visit_statements(body, visitor),
            Self::Declaration { declarations } => {
                for declaration in declarations {
                    if let Some(expression) = &declaration.initializer {
                        expression.visit(visitor);
                    }
                    if let Some(expression) = &declaration.array_size {
                        expression.visit(visitor);
                    }
                }
            }
            Self::Expression { expression } => expression.visit(visitor),
            Self::If {
                condition,
                then,
                otherwise,
                ..
            } => {
                condition.visit(visitor);
                then.visit_expressions(visitor);
                if let Some(otherwise) = otherwise {
                    otherwise.visit_expressions(visitor);
                }
            }
            Self::For {
                initializer,
                condition,
                update,
                body,
            } => {
                if let Some(initializer) = initializer {
                    initializer.visit_expressions(visitor);
                }
                if let Some(condition) = condition {
                    condition.visit(visitor);
                }
                if let Some(update) = update {
                    update.visit(visitor);
                }
                body.visit_expressions(visitor);
            }
            Self::While { condition, body } | Self::DoWhile { condition, body } => {
                condition.visit(visitor);
                body.visit_expressions(visitor);
            }
            Self::Return { value } => {
                if let Some(value) = value {
                    value.visit(visitor);
                }
            }
            Self::Break | Self::Continue | Self::Discard => {}
        }
    }
}

fn visit_statements(statements: &[Statement], visitor: &mut impl FnMut(&Expression)) {
    for statement in statements {
        statement.visit_expressions(visitor);
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Expression {
    Literal {
        #[serde(rename = "type")]
        value_type: Type,
        value: Value,
        source: Option<String>,
    },
    Identifier {
        #[serde(rename = "type")]
        value_type: Type,
        name: String,
        storage: StorageClass,
    },
    Member {
        #[serde(rename = "type")]
        value_type: Type,
        object: Box<Expression>,
        field: String,
    },
    Index {
        #[serde(rename = "type")]
        value_type: Type,
        object: Box<Expression>,
        index: Box<Expression>,
    },
    Unary {
        #[serde(rename = "type")]
        value_type: Type,
        operator: String,
        operand: Box<Expression>,
    },
    Postfix {
        #[serde(rename = "type")]
        value_type: Type,
        operator: String,
        lvalue: Box<Expression>,
    },
    Conditional {
        #[serde(rename = "type")]
        value_type: Type,
        condition: Box<Expression>,
        #[serde(rename = "whenTrue")]
        when_true: Box<Expression>,
        #[serde(rename = "whenFalse")]
        when_false: Box<Expression>,
    },
    Binary {
        #[serde(rename = "type")]
        value_type: Type,
        operator: String,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Assignment {
        #[serde(rename = "type")]
        value_type: Type,
        operator: String,
        lvalue: Box<Expression>,
        value: Box<Expression>,
    },
    Construct {
        #[serde(rename = "type")]
        value_type: Type,
        arguments: Vec<Expression>,
    },
    Call {
        #[serde(rename = "type")]
        value_type: Type,
        name: String,
        target: String,
        arguments: Vec<Expression>,
    },
}

impl Expression {
    fn visit(&self, visitor: &mut impl FnMut(&Expression)) {
        visitor(self);
        match self {
            Self::Member { object, .. }
            | Self::Unary {
                operand: object, ..
            } => object.visit(visitor),
            Self::Index { object, index, .. } => {
                object.visit(visitor);
                index.visit(visitor);
            }
            Self::Postfix { lvalue, .. } => lvalue.visit(visitor),
            Self::Conditional {
                condition,
                when_true,
                when_false,
                ..
            } => {
                condition.visit(visitor);
                when_true.visit(visitor);
                when_false.visit(visitor);
            }
            Self::Binary { left, right, .. } => {
                left.visit(visitor);
                right.visit(visitor);
            }
            Self::Assignment { lvalue, value, .. } => {
                lvalue.visit(visitor);
                value.visit(visitor);
            }
            Self::Construct { arguments, .. } | Self::Call { arguments, .. } => {
                for argument in arguments {
                    argument.visit(visitor);
                }
            }
            Self::Literal { .. } | Self::Identifier { .. } => {}
        }
    }
}

#[derive(Debug, Deserialize)]
struct BundleLock {
    source: String,
    version: String,
    hashes: BTreeMap<String, String>,
}

static CATALOG: OnceLock<Result<EffectCatalog, CatalogError>> = OnceLock::new();
static SHADERS: OnceLock<Result<ShaderBundle, CatalogError>> = OnceLock::new();

pub fn effect_catalog() -> Result<&'static EffectCatalog, CatalogError> {
    CATALOG
        .get_or_init(|| {
            let catalog: EffectCatalog = parse_json("catalog", CATALOG_JSON)?;
            validate_catalog(&catalog)?;
            Ok(catalog)
        })
        .as_ref()
        .map_err(|error| CatalogError::Invalid(error.to_string()))
}

pub fn shader_bundle() -> Result<&'static ShaderBundle, CatalogError> {
    SHADERS
        .get_or_init(|| {
            let shaders: ShaderBundle = parse_json("shaders", SHADERS_JSON)?;
            let lock: BundleLock = parse_json("bundle lock", BUNDLE_LOCK_JSON)?;
            let catalog = effect_catalog()?;
            validate_shaders(catalog, &shaders, &lock)?;
            Ok(shaders)
        })
        .as_ref()
        .map_err(|error| CatalogError::Invalid(error.to_string()))
}

fn parse_json<T: for<'de> Deserialize<'de>>(
    artifact: &'static str,
    source: &str,
) -> Result<T, CatalogError> {
    serde_json::from_str(source).map_err(|source| CatalogError::Json { artifact, source })
}

fn validate_catalog(catalog: &EffectCatalog) -> Result<(), CatalogError> {
    if catalog.effects.len() != EXPECTED_EFFECTS {
        return Err(CatalogError::Invalid(format!(
            "expected {EXPECTED_EFFECTS} effects, found {}",
            catalog.effects.len()
        )));
    }
    let exclusions = catalog
        .excluded_effects
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if exclusions != BTreeSet::from(EXPECTED_EXCLUSIONS) {
        return Err(CatalogError::Invalid(
            "the five-effect exclusion set changed".into(),
        ));
    }
    for (effect_id, effect) in &catalog.effects {
        if effect.namespace != effect_id.split('/').next().unwrap_or_default() {
            return Err(CatalogError::Invalid(format!(
                "effect {effect_id} has mismatched namespace {}",
                effect.namespace
            )));
        }
        let ordered = effect.param_names.iter().collect::<BTreeSet<_>>();
        let declared = effect.params.keys().collect::<BTreeSet<_>>();
        if ordered != declared || effect.param_names.len() != effect.params.len() {
            return Err(CatalogError::Invalid(format!(
                "effect {effect_id} paramNames do not exactly order its parameters"
            )));
        }
        for (alias, target) in &effect.param_aliases {
            if effect.params.contains_key(alias) {
                return Err(CatalogError::Invalid(format!(
                    "effect {effect_id} parameter alias {alias:?} collides with a canonical parameter"
                )));
            }
            if !effect.params.contains_key(target) {
                return Err(CatalogError::Invalid(format!(
                    "effect {effect_id} parameter alias {alias:?} targets unknown parameter {target:?}"
                )));
            }
        }
        match effect.domain.as_str() {
            "image" | "loop-begin" | "loop-end" | "volume-filter" | "volume-generator"
            | "volume-renderer" => {}
            domain => {
                return Err(CatalogError::Invalid(format!(
                    "effect {effect_id} has invalid domain {domain}"
                )));
            }
        }
        match (effect_id.as_str(), effect.loop_role.as_deref()) {
            ("render/loopBegin", Some("begin")) | ("render/loopEnd", Some("end")) => {}
            ("render/loopBegin" | "render/loopEnd", _) => {
                return Err(CatalogError::Invalid(format!(
                    "effect {effect_id} has invalid loop role"
                )));
            }
            (_, Some(_)) => {
                return Err(CatalogError::Invalid(format!(
                    "effect {effect_id} has unexpected loop role"
                )));
            }
            (_, None) => {}
        }
    }
    Ok(())
}

fn validate_shaders(
    catalog: &EffectCatalog,
    shaders: &ShaderBundle,
    lock: &BundleLock,
) -> Result<(), CatalogError> {
    if shaders.programs.len() != EXPECTED_PROGRAMS {
        return Err(CatalogError::Invalid(format!(
            "expected {EXPECTED_PROGRAMS} programs, found {}",
            shaders.programs.len()
        )));
    }
    let pass_keys = catalog
        .effects
        .values()
        .flat_map(|effect| effect.passes.iter())
        .filter_map(|render_pass| render_pass.key.clone())
        .collect::<BTreeSet<_>>();
    let shader_keys = shaders.programs.keys().cloned().collect::<BTreeSet<_>>();
    let lock_keys = lock.hashes.keys().cloned().collect::<BTreeSet<_>>();
    if pass_keys != shader_keys || pass_keys != lock_keys {
        return Err(CatalogError::Invalid(
            "pass, program, and lock inventories differ".into(),
        ));
    }
    if shaders.provenance.lock_source != lock.source
        || shaders.provenance.lock_version != lock.version
    {
        return Err(CatalogError::Invalid(
            "shader and lock provenance differ".into(),
        ));
    }
    for (key, program) in &shaders.programs {
        if lock.hashes.get(key) != Some(&program.source_sha256) {
            return Err(CatalogError::Invalid(format!(
                "program {key} hash differs from lock"
            )));
        }
        if !catalog.effects.contains_key(&program.effect_id) {
            return Err(CatalogError::Invalid(format!(
                "program {key} references an unknown effect"
            )));
        }
        if !program
            .ir
            .functions
            .iter()
            .any(|function| function.name == "main")
        {
            return Err(CatalogError::Invalid(format!(
                "program {key} has no main function"
            )));
        }
        validate_program_ir(&program.ir).map_err(|error| {
            CatalogError::Invalid(format!("program {key} has invalid typed IR: {error}"))
        })?;
    }
    Ok(())
}
