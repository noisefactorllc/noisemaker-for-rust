use std::collections::{BTreeMap, BTreeSet};

use super::{
    CatalogError, Expression, Function, ParameterQualifier, ProgramIr, Statement, StorageClass,
    StructDefinition, Type, VariableDefinition,
};

const BUILTINS: &[(&str, &[usize])] = &[
    ("abs", &[1]),
    ("acos", &[1]),
    ("all", &[1]),
    ("any", &[1]),
    ("atan", &[1, 2]),
    ("ceil", &[1]),
    ("clamp", &[3]),
    ("cos", &[1]),
    ("cross", &[2]),
    ("dFdx", &[1]),
    ("dFdy", &[1]),
    ("degrees", &[1]),
    ("distance", &[2]),
    ("dot", &[2]),
    ("equal", &[2]),
    ("exp", &[1]),
    ("floatBitsToUint", &[1]),
    ("floor", &[1]),
    ("fract", &[1]),
    ("fwidth", &[1]),
    ("greaterThan", &[2]),
    ("greaterThanEqual", &[2]),
    ("inversesqrt", &[1]),
    ("isnan", &[1]),
    ("length", &[1]),
    ("lessThan", &[2]),
    ("lessThanEqual", &[2]),
    ("log", &[1]),
    ("log2", &[1]),
    ("max", &[2]),
    ("min", &[2]),
    ("mix", &[3]),
    ("mod", &[2]),
    ("normalize", &[1]),
    ("notEqual", &[2]),
    ("packHalf2x16", &[1]),
    ("pow", &[2]),
    ("radians", &[1]),
    ("reflect", &[2]),
    ("refract", &[3]),
    ("round", &[1]),
    ("sign", &[1]),
    ("sin", &[1]),
    ("smoothstep", &[3]),
    ("sqrt", &[1]),
    ("step", &[2]),
    ("tanh", &[1]),
    ("texelFetch", &[3]),
    ("texture", &[2]),
    ("textureLod", &[3]),
    ("textureSize", &[2]),
    ("uintBitsToFloat", &[1]),
    ("unpackHalf2x16", &[1]),
];

#[derive(Clone)]
struct Symbol {
    value_type: String,
    storage: StorageClass,
    writable: bool,
}

struct Validator<'a> {
    ir: &'a ProgramIr,
    structs: BTreeMap<&'a str, &'a StructDefinition>,
    functions: BTreeMap<&'a str, &'a Function>,
    scopes: Vec<BTreeMap<String, Symbol>>,
    hoisted: Vec<BTreeSet<String>>,
}

pub fn validate_program_ir(ir: &ProgramIr) -> Result<(), CatalogError> {
    Validator::new(ir).validate().map_err(CatalogError::Invalid)
}

impl<'a> Validator<'a> {
    fn new(ir: &'a ProgramIr) -> Self {
        Self {
            ir,
            structs: BTreeMap::new(),
            functions: BTreeMap::new(),
            scopes: vec![BTreeMap::new()],
            hoisted: vec![BTreeSet::new()],
        }
    }

    fn validate(mut self) -> Result<(), String> {
        self.collect_structs()?;
        self.collect_functions()?;
        self.collect_root_symbols()?;
        self.validate_structs()?;

        for variable in &self.ir.globals {
            self.validate_variable(variable, false)?;
        }
        for variable in &self.ir.uniforms {
            self.validate_variable(variable, true)?;
        }
        for output in &self.ir.outputs {
            let symbol = self
                .resolve(output)
                .ok_or_else(|| format!("output {output:?} is not declared"))?;
            if symbol.storage != StorageClass::Global || symbol.value_type != "vec4" {
                return Err(format!("output {output:?} must name a global vec4"));
            }
        }

        let main = self
            .functions
            .get("main__void")
            .ok_or("missing main__void function")?;
        if main.name != "main" || main.return_type.0 != "void" || !main.parameters.is_empty() {
            return Err("main__void has an invalid signature".into());
        }
        for function in &self.ir.functions {
            self.validate_function(function)
                .map_err(|error| format!("function {}: {error}", function.mangled_name))?;
        }
        Ok(())
    }

    fn collect_structs(&mut self) -> Result<(), String> {
        for structure in &self.ir.structs {
            if is_builtin_type(&structure.name)
                || self.structs.insert(&structure.name, structure).is_some()
            {
                return Err(format!(
                    "duplicate or reserved struct type {:?}",
                    structure.name
                ));
            }
        }
        Ok(())
    }

    fn collect_functions(&mut self) -> Result<(), String> {
        for function in &self.ir.functions {
            self.require_type(&function.return_type, true)?;
            let mut parameter_names = BTreeSet::new();
            let mut parameter_types = Vec::new();
            for parameter in &function.parameters {
                self.require_type(&parameter.value_type, false)?;
                if !parameter_names.insert(parameter.name.as_str()) {
                    return Err(format!(
                        "duplicate parameter {:?} in {}",
                        parameter.name, function.mangled_name
                    ));
                }
                parameter_types.push(parameter.value_type.0.as_str());
            }
            let expected = mangle(&function.name, &parameter_types);
            if function.mangled_name != expected {
                return Err(format!(
                    "function {} has invalid mangled name; expected {expected}",
                    function.mangled_name
                ));
            }
            if self
                .functions
                .insert(&function.mangled_name, function)
                .is_some()
            {
                return Err(format!(
                    "duplicate function target {}",
                    function.mangled_name
                ));
            }
        }
        Ok(())
    }

    fn collect_root_symbols(&mut self) -> Result<(), String> {
        for (name, value_type) in [
            ("gl_FragCoord", "vec4"),
            ("gl_PointCoord", "vec2"),
            ("gl_FragDepth", "float"),
            ("gl_VertexID", "int"),
            ("gl_InstanceID", "int"),
            ("gl_PointSize", "float"),
        ] {
            self.define(name, value_type, StorageClass::Builtin, false)?;
        }
        let mut varyings = BTreeSet::new();
        for varying in self.ir.varyings.iter().map(String::as_str).chain([
            "v_texCoord",
            "vTexCoord",
            "texCoord",
        ]) {
            if varyings.insert(varying) {
                self.define(varying, "vec2", StorageClass::Varying, false)?;
            }
        }
        for uniform in &self.ir.uniforms {
            self.require_type(&uniform.value_type, false)?;
            self.define(
                &uniform.name,
                &uniform.value_type.0,
                StorageClass::Uniform,
                false,
            )?;
        }
        for global in &self.ir.globals {
            self.require_type(&global.value_type, false)?;
            self.define(
                &global.name,
                &global.value_type.0,
                StorageClass::Global,
                true,
            )?;
        }
        Ok(())
    }

    fn validate_structs(&self) -> Result<(), String> {
        for structure in &self.ir.structs {
            let mut names = BTreeSet::new();
            for field in &structure.fields {
                self.require_type(&field.value_type, false)?;
                if !names.insert(field.name.as_str()) {
                    return Err(format!(
                        "duplicate field {:?} in struct {}",
                        field.name, structure.name
                    ));
                }
                if base_type(&field.value_type.0) == structure.name {
                    return Err(format!(
                        "recursive struct {} is unsupported",
                        structure.name
                    ));
                }
            }
        }
        Ok(())
    }

    fn validate_function(&mut self, function: &Function) -> Result<(), String> {
        self.push_scope();
        for parameter in &function.parameters {
            self.define(
                &parameter.name,
                &parameter.value_type.0,
                StorageClass::Parameter,
                true,
            )?;
        }
        self.validate_statements(&function.body, &function.return_type, 0)?;
        self.pop_scope();
        Ok(())
    }

    fn validate_statements(
        &mut self,
        statements: &[Statement],
        return_type: &Type,
        loop_depth: usize,
    ) -> Result<(), String> {
        for (index, statement) in statements.iter().enumerate() {
            self.validate_statement(statement, return_type, loop_depth)
                .map_err(|error| format!("statement {index}: {error}"))?;
        }
        Ok(())
    }

    fn validate_statement(
        &mut self,
        statement: &Statement,
        return_type: &Type,
        loop_depth: usize,
    ) -> Result<(), String> {
        match statement {
            Statement::Block { body } => {
                self.push_scope();
                self.validate_statements(body, return_type, loop_depth)?;
                self.pop_scope();
            }
            Statement::Declaration { declarations } => {
                for declaration in declarations {
                    self.validate_variable(declaration, false)?;
                    match self
                        .scopes
                        .last()
                        .and_then(|scope| scope.get(&declaration.name))
                    {
                        Some(existing)
                            if existing.storage == StorageClass::Local
                                && existing.value_type == declaration.value_type.0
                                && self.hoisted.last_mut().unwrap().remove(&declaration.name) => {}
                        Some(_) => return Err(format!("duplicate symbol {:?}", declaration.name)),
                        None => self.define(
                            &declaration.name,
                            &declaration.value_type.0,
                            StorageClass::Local,
                            true,
                        )?,
                    }
                }
            }
            Statement::Expression { expression } => {
                self.validate_expression(expression)?;
            }
            Statement::If {
                hoisted_declarations,
                condition,
                then,
                otherwise,
            } => {
                for declaration in hoisted_declarations {
                    self.validate_variable(declaration, false)?;
                    if self.resolve(&declaration.name).is_none() {
                        self.define(
                            &declaration.name,
                            &declaration.value_type.0,
                            StorageClass::Local,
                            true,
                        )?;
                        self.hoisted
                            .last_mut()
                            .unwrap()
                            .insert(declaration.name.clone());
                    }
                }
                let condition_type = self.validate_expression(condition)?;
                self.require_condition(&condition_type, "if condition")?;
                self.validate_statement(then, return_type, loop_depth)?;
                if let Some(otherwise) = otherwise {
                    self.validate_statement(otherwise, return_type, loop_depth)?;
                }
            }
            Statement::For {
                initializer,
                condition,
                update,
                body,
            } => {
                self.push_scope();
                if let Some(initializer) = initializer {
                    self.validate_statement(initializer, return_type, loop_depth)?;
                }
                if let Some(condition) = condition {
                    let condition_type = self.validate_expression(condition)?;
                    self.require_condition(&condition_type, "for condition")?;
                }
                if let Some(update) = update {
                    self.validate_expression(update)?;
                }
                self.validate_statement(body, return_type, loop_depth + 1)?;
                self.pop_scope();
            }
            Statement::While { condition, body } | Statement::DoWhile { condition, body } => {
                let condition_type = self.validate_expression(condition)?;
                self.require_condition(&condition_type, "loop condition")?;
                self.validate_statement(body, return_type, loop_depth + 1)?;
            }
            Statement::Return { value } => match (return_type.0.as_str(), value) {
                ("void", None) => {}
                ("void", Some(_)) => return Err("void function returns a value".into()),
                (_, None) => {
                    return Err(format!(
                        "function returning {} has an empty return",
                        return_type.0
                    ));
                }
                (_, Some(value)) => {
                    let actual = self.validate_expression(value)?;
                    self.require_compatible(&return_type.0, &actual, "return type")?;
                }
            },
            Statement::Break | Statement::Continue if loop_depth == 0 => {
                return Err("break/continue outside a loop".into());
            }
            Statement::Break | Statement::Continue | Statement::Discard => {}
        }
        Ok(())
    }

    fn validate_variable(
        &mut self,
        variable: &VariableDefinition,
        uniform: bool,
    ) -> Result<(), String> {
        self.require_type(&variable.value_type, false)?;
        if uniform && variable.initializer.is_some() {
            return Err(format!("uniform {:?} has an initializer", variable.name));
        }
        if let Some(size) = &variable.array_size {
            let size_type = self.validate_expression(size)?;
            if !is_integer_scalar(&size_type) {
                return Err(format!(
                    "array size for {:?} is not an integer scalar",
                    variable.name
                ));
            }
        }
        if let Some(initializer) = &variable.initializer {
            let actual = self.validate_expression(initializer)?;
            self.require_compatible(&variable.value_type.0, &actual, "initializer type")?;
        }
        Ok(())
    }

    fn validate_expression(&mut self, expression: &Expression) -> Result<String, String> {
        let annotated = expression_type(expression);
        self.require_type_name(annotated, matches!(expression, Expression::Call { .. }))?;
        let inferred = match expression {
            Expression::Literal {
                value_type, value, ..
            } => {
                if !literal_matches(&value_type.0, value) {
                    return Err(format!(
                        "literal value is incompatible with type {}",
                        value_type.0
                    ));
                }
                value_type.0.clone()
            }
            Expression::Identifier {
                value_type,
                name,
                storage,
            } => {
                let symbol = self
                    .resolve(name)
                    .ok_or_else(|| format!("unresolved identifier {name:?}"))?;
                if symbol.storage != *storage {
                    return Err(format!(
                        "identifier {name:?} has storage {:?}, annotated {storage:?}",
                        symbol.storage
                    ));
                }
                self.require_compatible(&symbol.value_type, &value_type.0, "identifier type")?;
                value_type.0.clone()
            }
            Expression::Member { object, field, .. } => self.validate_member(object, field)?,
            Expression::Index { object, index, .. } => self.validate_index(object, index)?,
            Expression::Unary {
                operator, operand, ..
            } => self.validate_unary(operator, operand)?,
            Expression::Postfix {
                operator, lvalue, ..
            } => {
                if !matches!(operator.as_str(), "++" | "--") {
                    return Err(format!("invalid postfix operator {operator:?}"));
                }
                let value_type = self.validate_expression(lvalue)?;
                self.require_numeric(&value_type, "postfix operand")?;
                self.validate_lvalue(lvalue)?;
                value_type
            }
            Expression::Conditional {
                condition,
                when_true,
                when_false,
                ..
            } => {
                let condition_type = self.validate_expression(condition)?;
                self.require_bool(&condition_type, "conditional condition")?;
                let left = self.validate_expression(when_true)?;
                let right = self.validate_expression(when_false)?;
                common_type(&left, &right).ok_or_else(|| {
                    format!("conditional branch types {left} and {right} are incompatible")
                })?
            }
            Expression::Binary {
                operator,
                left,
                right,
                ..
            } => self.validate_binary(operator, left, right)?,
            Expression::Assignment {
                operator,
                lvalue,
                value,
                ..
            } => {
                if !matches!(
                    operator.as_str(),
                    "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^=" | "<<=" | ">>="
                ) {
                    return Err(format!("invalid assignment operator {operator:?}"));
                }
                let target_type = self.validate_expression(lvalue)?;
                self.validate_lvalue(lvalue)?;
                let value_type = self.validate_expression(value)?;
                if operator == "=" {
                    self.require_compatible(&target_type, &value_type, "assignment value")?;
                } else {
                    self.require_numeric(&target_type, "compound assignment")?;
                    self.require_numeric(&value_type, "compound assignment")?;
                    let matrix_multiply = operator == "*="
                        && (matrix_shape(&target_type).is_some()
                            || matrix_shape(&value_type).is_some());
                    if !matrix_multiply && common_type(&target_type, &value_type).is_none() {
                        return Err(format!(
                            "compound assignment types {target_type} and {value_type} are incompatible"
                        ));
                    }
                }
                target_type
            }
            Expression::Construct {
                value_type,
                arguments,
            } => self.validate_construct(value_type, arguments)?,
            Expression::Call {
                value_type,
                name,
                target,
                arguments,
            } => self.validate_call(value_type, name, target, arguments)?,
        };
        self.require_compatible(annotated, &inferred, "expression annotation")?;
        Ok(annotated.to_string())
    }

    fn validate_member(&mut self, object: &Expression, field: &str) -> Result<String, String> {
        let object_type = self.validate_expression(object)?;
        let bare = base_type(&object_type);
        if let Some(structure) = self.structs.get(bare) {
            let member = structure
                .fields
                .iter()
                .find(|member| member.name == field)
                .ok_or_else(|| format!("unknown member {field:?} on struct {bare}"))?;
            return Ok(member.value_type.0.clone());
        }
        let (base, width) = numeric_shape(&object_type)
            .or_else(|| bool_shape(&object_type).map(|width| ("bool", width)))
            .ok_or_else(|| format!("member access on non-vector type {object_type}"))?;
        let families = ["xyzw", "rgba", "stpq"];
        let family = families
            .iter()
            .find(|family| field.chars().all(|character| family.contains(character)))
            .ok_or_else(|| format!("invalid swizzle {field:?}"))?;
        if field.is_empty() || field.len() > 4 {
            return Err(format!("invalid swizzle {field:?}"));
        }
        if field
            .chars()
            .any(|character| family.find(character).unwrap_or(4) >= width)
        {
            return Err(format!("swizzle {field:?} exceeds {object_type}"));
        }
        Ok(type_from_shape(base, field.len()))
    }

    fn validate_index(
        &mut self,
        object: &Expression,
        index: &Expression,
    ) -> Result<String, String> {
        let object_type = self.validate_expression(object)?;
        let index_type = self.validate_expression(index)?;
        if !is_integer_scalar(&index_type) {
            return Err(format!(
                "index expression has non-integer type {index_type}"
            ));
        }
        if let Some(element) = object_type.strip_suffix("[]") {
            return Ok(element.to_string());
        }
        if let Some((base, width)) = numeric_shape(&object_type) {
            if width > 1 {
                return Ok(type_from_shape(base, 1));
            }
        }
        if let Some(width) = bool_shape(&object_type) {
            if width > 1 {
                return Ok("bool".into());
            }
        }
        if let Some((_, rows)) = matrix_shape(&object_type) {
            return Ok(format!("vec{rows}"));
        }
        Err(format!("cannot index value of type {object_type}"))
    }

    fn validate_unary(&mut self, operator: &str, operand: &Expression) -> Result<String, String> {
        let operand_type = self.validate_expression(operand)?;
        match operator {
            "+" | "-" => self.require_numeric(&operand_type, "unary operand")?,
            "!" => {
                self.require_bool(&operand_type, "logical-not operand")?;
                return Ok("bool".into());
            }
            "~" => {
                if !is_integer_type(&operand_type) {
                    return Err(format!("bitwise-not operand has type {operand_type}"));
                }
            }
            "++" | "--" => {
                self.require_numeric(&operand_type, "increment operand")?;
                self.validate_lvalue(operand)?;
            }
            _ => return Err(format!("invalid unary operator {operator:?}")),
        }
        Ok(operand_type)
    }

    fn validate_binary(
        &mut self,
        operator: &str,
        left: &Expression,
        right: &Expression,
    ) -> Result<String, String> {
        let left_type = self.validate_expression(left)?;
        let right_type = self.validate_expression(right)?;
        match operator {
            "&&" | "||" => {
                self.require_bool(&left_type, "logical operand")?;
                self.require_bool(&right_type, "logical operand")?;
                Ok("bool".into())
            }
            "==" | "!=" | "<" | ">" | "<=" | ">=" => {
                self.require_compatible(&left_type, &right_type, "comparison operands")?;
                Ok("bool".into())
            }
            "&" | "|" | "^" | "<<" | ">>" => {
                if !is_integer_type(&left_type) || !is_integer_type(&right_type) {
                    return Err(format!(
                        "bitwise operands have types {left_type} and {right_type}"
                    ));
                }
                common_type(&left_type, &right_type).ok_or_else(|| {
                    format!("bitwise operands {left_type} and {right_type} are incompatible")
                })
            }
            "+" | "-" | "*" | "/" | "%" => {
                self.require_numeric(&left_type, "arithmetic operand")?;
                self.require_numeric(&right_type, "arithmetic operand")?;
                if operator == "*" {
                    if let Some((_columns, rows)) = matrix_shape(&left_type) {
                        return if matrix_shape(&right_type).is_some() {
                            Ok(left_type)
                        } else {
                            Ok(format!("vec{rows}"))
                        };
                    }
                    if let Some((columns, _)) = matrix_shape(&right_type) {
                        return Ok(format!("vec{columns}"));
                    }
                }
                common_type(&left_type, &right_type).ok_or_else(|| {
                    format!("arithmetic operands {left_type} and {right_type} are incompatible")
                })
            }
            _ => Err(format!("invalid binary operator {operator:?}")),
        }
    }

    fn validate_construct(
        &mut self,
        value_type: &Type,
        arguments: &[Expression],
    ) -> Result<String, String> {
        let target = value_type.0.as_str();
        self.require_type(value_type, false)?;
        let argument_types = arguments
            .iter()
            .map(|argument| self.validate_expression(argument))
            .collect::<Result<Vec<_>, _>>()?;
        if let Some(element) = target.strip_suffix("[]") {
            for argument in &argument_types {
                self.require_compatible(element, argument, "array constructor argument")?;
            }
            return Ok(target.into());
        }
        if let Some(structure) = self.structs.get(target) {
            if structure.fields.len() != argument_types.len() {
                return Err(format!(
                    "struct {target} constructor has {} arguments, expected {}",
                    argument_types.len(),
                    structure.fields.len()
                ));
            }
            for (field, argument) in structure.fields.iter().zip(&argument_types) {
                self.require_compatible(
                    &field.value_type.0,
                    argument,
                    "struct constructor argument",
                )?;
            }
            return Ok(target.into());
        }
        let required = numeric_shape(target)
            .map(|(_, width)| width)
            .or_else(|| bool_shape(target))
            .or_else(|| matrix_shape(target).map(|(columns, rows)| columns * rows))
            .ok_or_else(|| format!("cannot construct type {target}"))?;
        if arguments.is_empty() {
            return Err(format!("constructor for {target} has no arguments"));
        }
        let provided = argument_types
            .iter()
            .map(|argument| type_components(argument).unwrap_or(0))
            .sum::<usize>();
        let scalar_splat =
            argument_types.len() == 1 && type_components(&argument_types[0]) == Some(1);
        if !scalar_splat && provided < required {
            return Err(format!(
                "constructor for {target} provides {provided} components, expected {required}"
            ));
        }
        Ok(target.into())
    }

    fn validate_call(
        &mut self,
        value_type: &Type,
        name: &str,
        target: &str,
        arguments: &[Expression],
    ) -> Result<String, String> {
        let argument_types = arguments
            .iter()
            .map(|argument| self.validate_expression(argument))
            .collect::<Result<Vec<_>, _>>()?;
        if let Some(builtin_name) = target.strip_prefix("builtin:") {
            if builtin_name != name {
                return Err(format!(
                    "builtin target {target:?} does not match call name {name:?}"
                ));
            }
            let arities = BUILTINS
                .iter()
                .find_map(|(known, arities)| (*known == name).then_some(*arities))
                .ok_or_else(|| format!("unknown builtin {name:?}"))?;
            if !arities.contains(&arguments.len()) {
                return Err(format!(
                    "builtin {name} has {} arguments; expected {arities:?}",
                    arguments.len()
                ));
            }
            let inferred = builtin_return_type(name, &argument_types)?;
            self.require_compatible(
                &value_type.0,
                &inferred,
                &format!("builtin {name} return type"),
            )?;
            return Ok(inferred);
        }
        let function = self
            .functions
            .get(target)
            .copied()
            .ok_or_else(|| format!("unknown function target {target:?}"))?;
        if function.name != name {
            return Err(format!(
                "function target {target:?} does not match call name {name:?}"
            ));
        }
        if function.parameters.len() != arguments.len() {
            return Err(format!(
                "call to {target} has {} arguments, expected {}",
                arguments.len(),
                function.parameters.len()
            ));
        }
        for ((parameter, argument), argument_type) in function
            .parameters
            .iter()
            .zip(arguments)
            .zip(&argument_types)
        {
            self.require_compatible(&parameter.value_type.0, argument_type, "argument type")?;
            if matches!(
                parameter.qualifier,
                ParameterQualifier::Out | ParameterQualifier::InOut
            ) {
                self.validate_lvalue(argument).map_err(|_| {
                    format!(
                        "out argument for {} is not a writable lvalue",
                        parameter.name
                    )
                })?;
            }
        }
        self.require_compatible(
            &value_type.0,
            &function.return_type.0,
            "function return annotation",
        )?;
        Ok(function.return_type.0.clone())
    }

    fn validate_lvalue(&self, expression: &Expression) -> Result<(), String> {
        match expression {
            Expression::Identifier { name, .. } => {
                let symbol = self
                    .resolve(name)
                    .ok_or_else(|| format!("unresolved identifier {name:?}"))?;
                if !symbol.writable {
                    return Err(format!("identifier {name:?} is not a writable lvalue"));
                }
                Ok(())
            }
            Expression::Member { object, field, .. } => {
                self.validate_lvalue(object)?;
                let object_type = expression_type(object);
                let is_swizzle = numeric_shape(object_type).is_some();
                if is_swizzle
                    && field.len() > 1
                    && field.chars().collect::<BTreeSet<_>>().len() != field.len()
                {
                    return Err(format!("swizzle {field:?} is not a writable lvalue"));
                }
                Ok(())
            }
            Expression::Index { object, .. } => self.validate_lvalue(object),
            _ => Err("expression is not an lvalue".into()),
        }
    }

    fn require_type(&self, value_type: &Type, allow_void: bool) -> Result<(), String> {
        self.require_type_name(&value_type.0, allow_void)
    }

    fn require_type_name(&self, value_type: &str, allow_void: bool) -> Result<(), String> {
        let bare = base_type(value_type);
        if (!allow_void && bare == "void")
            || (!is_builtin_type(bare) && !self.structs.contains_key(bare))
        {
            return Err(format!("unknown type {value_type:?}"));
        }
        if value_type.matches("[]").count() > 1
            || (value_type.contains("[]") && !value_type.ends_with("[]"))
        {
            return Err(format!("malformed array type {value_type:?}"));
        }
        Ok(())
    }

    fn require_compatible(
        &self,
        expected: &str,
        actual: &str,
        context: &str,
    ) -> Result<(), String> {
        if compatible(expected, actual) {
            Ok(())
        } else {
            Err(format!("{context}: expected {expected}, found {actual}"))
        }
    }

    fn require_bool(&self, value_type: &str, context: &str) -> Result<(), String> {
        if value_type == "bool" {
            Ok(())
        } else {
            Err(format!("{context} has type {value_type}, expected bool"))
        }
    }

    fn require_condition(&self, value_type: &str, context: &str) -> Result<(), String> {
        if value_type == "bool" || matches!(numeric_shape(value_type), Some((_, 1))) {
            Ok(())
        } else {
            Err(format!(
                "{context} has type {value_type}, expected a scalar condition"
            ))
        }
    }

    fn require_numeric(&self, value_type: &str, context: &str) -> Result<(), String> {
        if numeric_shape(value_type).is_some() || matrix_shape(value_type).is_some() {
            Ok(())
        } else {
            Err(format!("{context} has non-numeric type {value_type}"))
        }
    }

    fn define(
        &mut self,
        name: &str,
        value_type: &str,
        storage: StorageClass,
        writable: bool,
    ) -> Result<(), String> {
        let scope = self
            .scopes
            .last_mut()
            .expect("validator always has a root scope");
        if scope.contains_key(name) {
            return Err(format!("duplicate symbol {name:?}"));
        }
        scope.insert(
            name.into(),
            Symbol {
                value_type: value_type.into(),
                storage,
                writable,
            },
        );
        Ok(())
    }

    fn resolve(&self, name: &str) -> Option<&Symbol> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }

    fn push_scope(&mut self) {
        self.scopes.push(BTreeMap::new());
        self.hoisted.push(BTreeSet::new());
    }
    fn pop_scope(&mut self) {
        self.scopes.pop();
        self.hoisted.pop();
    }
}

fn expression_type(expression: &Expression) -> &str {
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
        | Expression::Call { value_type, .. } => &value_type.0,
    }
}

fn base_type(value_type: &str) -> &str {
    value_type.strip_suffix("[]").unwrap_or(value_type)
}

fn is_builtin_type(value_type: &str) -> bool {
    matches!(
        value_type,
        "void"
            | "bool"
            | "int"
            | "uint"
            | "float"
            | "vec2"
            | "vec3"
            | "vec4"
            | "ivec2"
            | "ivec3"
            | "ivec4"
            | "uvec2"
            | "uvec3"
            | "uvec4"
            | "bvec2"
            | "bvec3"
            | "bvec4"
            | "mat2"
            | "mat3"
            | "mat4"
            | "mat2x2"
            | "mat2x3"
            | "mat2x4"
            | "mat3x2"
            | "mat3x3"
            | "mat3x4"
            | "mat4x2"
            | "mat4x3"
            | "mat4x4"
            | "sampler2D"
            | "sampler3D"
            | "samplerCube"
            | "sampler2DArray"
    )
}

fn numeric_shape(value_type: &str) -> Option<(&'static str, usize)> {
    match value_type {
        "float" => Some(("float", 1)),
        "int" => Some(("int", 1)),
        "uint" => Some(("uint", 1)),
        "vec2" => Some(("float", 2)),
        "vec3" => Some(("float", 3)),
        "vec4" => Some(("float", 4)),
        "ivec2" => Some(("int", 2)),
        "ivec3" => Some(("int", 3)),
        "ivec4" => Some(("int", 4)),
        "uvec2" => Some(("uint", 2)),
        "uvec3" => Some(("uint", 3)),
        "uvec4" => Some(("uint", 4)),
        _ => None,
    }
}

fn bool_shape(value_type: &str) -> Option<usize> {
    match value_type {
        "bool" => Some(1),
        "bvec2" => Some(2),
        "bvec3" => Some(3),
        "bvec4" => Some(4),
        _ => None,
    }
}

fn matrix_shape(value_type: &str) -> Option<(usize, usize)> {
    let body = value_type.strip_prefix("mat")?;
    let mut parts = body.split('x');
    let columns = parts.next()?.parse().ok()?;
    let rows = match parts.next() {
        Some(value) => value.parse().ok()?,
        None => columns,
    };
    if parts.next().is_some() || !(2..=4).contains(&columns) || !(2..=4).contains(&rows) {
        None
    } else {
        Some((columns, rows))
    }
}

fn type_components(value_type: &str) -> Option<usize> {
    numeric_shape(value_type)
        .map(|(_, width)| width)
        .or_else(|| bool_shape(value_type))
        .or_else(|| matrix_shape(value_type).map(|(columns, rows)| columns * rows))
}

fn type_from_shape(base: &str, width: usize) -> String {
    match (base, width) {
        ("float", 1) => "float".into(),
        ("int", 1) => "int".into(),
        ("uint", 1) => "uint".into(),
        ("bool", 1) => "bool".into(),
        ("float", width) => format!("vec{width}"),
        ("int", width) => format!("ivec{width}"),
        ("uint", width) => format!("uvec{width}"),
        ("bool", width) => format!("bvec{width}"),
        _ => base.into(),
    }
}

fn compatible(expected: &str, actual: &str) -> bool {
    if expected == actual {
        return true;
    }
    match (numeric_shape(expected), numeric_shape(actual)) {
        (Some((_, expected_width)), Some((_, actual_width))) => expected_width == actual_width,
        _ => false,
    }
}

fn common_type(left: &str, right: &str) -> Option<String> {
    if left == right {
        return Some(left.into());
    }
    let ((left_base, left_width), (right_base, right_width)) =
        (numeric_shape(left)?, numeric_shape(right)?);
    let width = left_width.max(right_width);
    let base = if left_base == "uint" || right_base == "uint" {
        "uint"
    } else if left_base == "int" && right_base == "int" {
        "int"
    } else {
        "float"
    };
    Some(type_from_shape(base, width))
}

fn is_integer_scalar(value_type: &str) -> bool {
    matches!(value_type, "int" | "uint")
}
fn is_integer_type(value_type: &str) -> bool {
    matches!(numeric_shape(value_type), Some(("int" | "uint", _)))
}

fn literal_matches(value_type: &str, value: &serde_json::Value) -> bool {
    match value_type {
        "bool" => value.is_boolean(),
        "int" | "uint" => value.as_i64().is_some() || value.as_u64().is_some(),
        "float" => value.is_number(),
        _ => false,
    }
}

fn mangle(name: &str, parameter_types: &[&str]) -> String {
    format!(
        "{name}__{}",
        if parameter_types.is_empty() {
            "void".into()
        } else {
            parameter_types.join("_")
        }
    )
}

fn builtin_return_type(name: &str, arguments: &[String]) -> Result<String, String> {
    let first = arguments
        .first()
        .ok_or_else(|| format!("builtin {name} has no arguments"))?;
    match name {
        "all" | "any" => Ok("bool".into()),
        "distance" | "dot" | "length" => Ok("float".into()),
        "equal" | "greaterThan" | "greaterThanEqual" | "lessThan" | "lessThanEqual"
        | "notEqual" | "isnan" => {
            let width = type_components(first)
                .ok_or_else(|| format!("builtin {name} argument type {first} is invalid"))?;
            Ok(type_from_shape("bool", width))
        }
        "texture" | "textureLod" | "texelFetch" => Ok("vec4".into()),
        "textureSize" => Ok(match first.as_str() {
            "sampler3D" | "samplerCube" | "sampler2DArray" => "ivec3",
            _ => "ivec2",
        }
        .into()),
        "packHalf2x16" => Ok("uint".into()),
        "floatBitsToUint" => {
            let width = type_components(first).unwrap_or(1);
            Ok(type_from_shape("uint", width))
        }
        "unpackHalf2x16" => Ok("vec2".into()),
        "uintBitsToFloat" => {
            let width = type_components(first).unwrap_or(1);
            Ok(type_from_shape("float", width))
        }
        "cross" | "normalize" | "reflect" | "refract" | "abs" | "acos" | "ceil" | "cos"
        | "dFdx" | "dFdy" | "degrees" | "exp" | "floor" | "fract" | "fwidth" | "inversesqrt"
        | "log" | "log2" | "radians" | "round" | "sign" | "sin" | "sqrt" | "tanh" => {
            Ok(first.clone())
        }
        "step" => Ok(arguments[1].clone()),
        "smoothstep" => Ok(arguments[2].clone()),
        "atan" | "clamp" | "max" | "min" | "mix" | "mod" | "pow" => {
            let mut result = first.clone();
            for argument in &arguments[1..] {
                result = common_type(&result, argument)
                    .ok_or_else(|| format!("builtin {name} has incompatible argument types"))?;
            }
            Ok(result)
        }
        _ => Err(format!("unknown builtin {name:?}")),
    }
}
