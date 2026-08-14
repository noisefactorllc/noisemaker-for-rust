use std::collections::{BTreeMap, BTreeSet};

use crate::catalog::{
    Expression, Function, ParameterQualifier, ProgramIr, Statement, StorageClass,
    VariableDefinition, validate_program_ir,
};
use crate::value::type_error;
use crate::{Runtime, Value, VmError};

pub const STATEMENT_LIMIT: u64 = 1_048_576;
const CALL_DEPTH_LIMIT: usize = 64;

#[derive(Clone, Debug, Default)]
pub struct PixelContext {
    pub uniforms: BTreeMap<String, Value>,
    pub globals: BTreeMap<String, Value>,
    pub builtins: BTreeMap<String, Value>,
    pub varyings: BTreeMap<String, Value>,
}

pub type ShaderOutputs = BTreeMap<String, Value>;

fn expression_value_type(expression: &Expression) -> &crate::catalog::Type {
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

#[derive(Clone, Debug)]
struct Binding {
    value: Value,
    writable: bool,
    hoisted: bool,
}

#[derive(Clone, Debug)]
enum PathPart {
    Member(String),
    Index(usize),
    Swizzle(String),
}

#[derive(Clone, Debug)]
struct LValue {
    frame: usize,
    name: String,
    path: Vec<PathPart>,
}

#[derive(Clone, Debug)]
enum Flow {
    Next,
    Return(Value),
    Break,
    Continue,
    Discard,
}

/// Interpreter for the checked, typed structural shader IR.
#[derive(Clone, Debug)]
pub struct ShaderVm {
    program: ProgramIr,
    runtime: Runtime,
    structs: BTreeMap<String, Vec<(String, String)>>,
    function_indices: BTreeMap<String, usize>,
    temporal_aberration_factory_compatibility: bool,
    javascript_sine_hash_random_functions: BTreeSet<String>,
    javascript_float_atlas_z_program: bool,
    scopes: Vec<BTreeMap<String, Binding>>,
    root_frame: usize,
    statement_count: u64,
    call_depth: usize,
}

impl ShaderVm {
    pub fn new(program: &ProgramIr, runtime: Runtime) -> Result<Self, VmError> {
        validate_program_ir(program).map_err(|error| VmError::InvalidIr {
            message: error.to_string(),
        })?;
        let structs = program
            .structs
            .iter()
            .map(|s| {
                (
                    s.name.clone(),
                    s.fields
                        .iter()
                        .map(|f| (f.name.clone(), f.value_type.0.clone()))
                        .collect(),
                )
            })
            .collect();
        let function_indices = program
            .functions
            .iter()
            .enumerate()
            .map(|(index, function)| (function.mangled_name.clone(), index))
            .collect();
        let javascript_sine_hash_random_functions = program
            .functions
            .iter()
            .filter(|function| is_javascript_sine_hash_random(function))
            .map(|function| function.mangled_name.clone())
            .collect();
        let javascript_float_atlas_z_program = is_javascript_float_atlas_program(program);
        Ok(Self {
            program: program.clone(),
            runtime,
            structs,
            function_indices,
            temporal_aberration_factory_compatibility: false,
            javascript_sine_hash_random_functions,
            javascript_float_atlas_z_program,
            scopes: Vec::new(),
            root_frame: 0,
            statement_count: 0,
            call_depth: 0,
        })
    }

    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }
    pub fn runtime_mut(&mut self) -> &mut Runtime {
        &mut self.runtime
    }

    pub(crate) fn set_temporal_aberration_factory_compatibility(&mut self, enabled: bool) {
        self.temporal_aberration_factory_compatibility = enabled;
    }

    pub fn run_pixel(
        &mut self,
        context: &PixelContext,
        outputs: &mut ShaderOutputs,
    ) -> Result<(), VmError> {
        self.statement_count = 0;
        self.call_depth = 0;
        self.scopes.clear();
        self.scopes.push(BTreeMap::new());
        self.root_frame = 0;
        for uniform in self.program.uniforms.clone() {
            let value = context.uniforms.get(&uniform.name).cloned().unwrap_or(
                self.runtime
                    .default_value(&uniform.value_type.0, &self.structs)?,
            );
            self.define(&uniform.name, value, false)?;
        }
        for (name, value) in &context.varyings {
            self.define(name, value.clone(), false)?;
        }
        for varying in self.program.varyings.clone().into_iter().chain([
            "v_texCoord".into(),
            "vTexCoord".into(),
            "texCoord".into(),
        ]) {
            if self.resolve(&varying).is_none() {
                self.define(&varying, Value::Vec(vec![0.0, 0.0]), false)?;
            }
        }
        for (name, value) in &context.builtins {
            self.define(name, value.clone(), false)?;
        }
        for (name, value) in [
            ("gl_FragCoord", Value::Vec(vec![0.0; 4])),
            ("gl_PointCoord", Value::Vec(vec![0.0; 2])),
            ("gl_FragDepth", Value::Float(0.0)),
            ("gl_VertexID", Value::Int(0)),
            ("gl_InstanceID", Value::Int(0)),
            ("gl_PointSize", Value::Float(0.0)),
        ] {
            if self.resolve(name).is_none() {
                self.define(
                    name,
                    value,
                    name == "gl_FragDepth" || name == "gl_PointSize",
                )?;
            }
        }
        for global in self.program.globals.clone() {
            let value = if let Some(initializer) = &global.initializer {
                self.eval(initializer)?
            } else {
                self.variable_default(&global)?
            };
            self.define(&global.name, value, true)?;
        }
        for (name, value) in &context.globals {
            self.write_root(name, value.clone())?;
        }
        match self.call_function("main__void", &[])? {
            Flow::Discard => return Err(VmError::Discarded),
            Flow::Next | Flow::Return(_) => {}
            Flow::Break | Flow::Continue => {
                return Err(VmError::InvalidIr {
                    message: "loop flow escaped main".into(),
                });
            }
        }
        outputs.clear();
        for name in self.program.outputs.clone() {
            outputs.insert(name.clone(), self.read_root(&name)?.clone());
        }
        Ok(())
    }

    fn tick(&mut self) -> Result<(), VmError> {
        self.statement_count = self.statement_count.saturating_add(1);
        if self.statement_count > STATEMENT_LIMIT {
            Err(VmError::StatementLimit {
                limit: STATEMENT_LIMIT,
            })
        } else {
            Ok(())
        }
    }
    fn push(&mut self) {
        self.scopes.push(BTreeMap::new());
    }
    fn pop(&mut self) {
        self.scopes.pop();
    }
    fn define(&mut self, name: &str, value: Value, writable: bool) -> Result<(), VmError> {
        let scope = self.scopes.last_mut().unwrap();
        if scope
            .insert(
                name.into(),
                Binding {
                    value,
                    writable,
                    hoisted: false,
                },
            )
            .is_some()
        {
            return Err(VmError::InvalidIr {
                message: format!("duplicate runtime binding {name:?}"),
            });
        }
        Ok(())
    }
    fn define_hoisted(&mut self, name: &str, value: Value) -> Result<(), VmError> {
        self.define(name, value, true)?;
        self.scopes
            .last_mut()
            .unwrap()
            .get_mut(name)
            .unwrap()
            .hoisted = true;
        Ok(())
    }
    fn resolve(&self, name: &str) -> Option<usize> {
        (0..self.scopes.len())
            .rev()
            .find(|&i| self.scopes[i].contains_key(name))
    }
    fn read_root(&self, name: &str) -> Result<&Value, VmError> {
        self.scopes[self.root_frame]
            .get(name)
            .map(|b| &b.value)
            .ok_or_else(|| VmError::UnknownVariable { name: name.into() })
    }
    fn write_root(&mut self, name: &str, value: Value) -> Result<(), VmError> {
        let binding = self.scopes[self.root_frame]
            .get_mut(name)
            .ok_or_else(|| VmError::UnknownVariable { name: name.into() })?;
        binding.value = value;
        Ok(())
    }
    fn variable_default(&mut self, variable: &VariableDefinition) -> Result<Value, VmError> {
        if let Some(size) = &variable.array_size {
            let count = self.eval(size)?.as_index()?;
            let base = variable
                .value_type
                .0
                .strip_suffix("[]")
                .ok_or_else(|| type_error("array size on non-array"))?;
            let value = self.runtime.default_value(base, &self.structs)?;
            return Ok(Value::Array(vec![value; count]));
        }
        self.runtime
            .default_value(&variable.value_type.0, &self.structs)
    }

    fn execute_many(&mut self, statements: &[Statement]) -> Result<Flow, VmError> {
        for statement in statements {
            let flow = self.execute(statement)?;
            if !matches!(flow, Flow::Next) {
                return Ok(flow);
            }
        }
        Ok(Flow::Next)
    }
    fn execute(&mut self, statement: &Statement) -> Result<Flow, VmError> {
        self.tick()?;
        match statement {
            Statement::Block { body } => {
                self.push();
                let flow = self.execute_many(body);
                self.pop();
                flow
            }
            Statement::Declaration { declarations } => {
                for declaration in declarations {
                    let value = if let Some(initializer) = &declaration.initializer {
                        if self.javascript_float_atlas_z_program
                            && is_javascript_float_atlas_z_declaration(declaration)
                        {
                            let Expression::Binary { left, right, .. } = initializer else {
                                unreachable!("atlas z fingerprint requires a binary initializer");
                            };
                            let left = self.eval(left)?;
                            let right = self.eval(right)?;
                            let (Value::Int(left), Value::Int(right)) = (left, right) else {
                                return Err(type_error(
                                    "atlas z compatibility requires integer operands",
                                ));
                            };
                            // The canonical JS CPU compiler emits an untyped `var` here.
                            // Preserve its fractional atlas coordinate even though the source IR
                            // labels the declaration and division as `int`.
                            Value::Float((f64::from(left) / f64::from(right)) as f32)
                        } else {
                            self.eval(initializer)?
                        }
                    } else {
                        self.variable_default(declaration)?
                    };
                    if let Some(frame) = self.resolve(&declaration.name) {
                        let current_frame = self.scopes.len() - 1;
                        let binding = self.scopes[frame].get_mut(&declaration.name).unwrap();
                        if frame == current_frame || binding.hoisted {
                            binding.value = value;
                            binding.hoisted = false;
                            continue;
                        }
                    }
                    self.define(&declaration.name, value, true)?;
                }
                Ok(Flow::Next)
            }
            Statement::Expression { expression } => {
                self.eval(expression)?;
                Ok(Flow::Next)
            }
            Statement::If {
                hoisted_declarations,
                condition,
                then,
                otherwise,
            } => {
                for declaration in hoisted_declarations {
                    if self.resolve(&declaration.name).is_none() {
                        let value = self.variable_default(declaration)?;
                        self.define_hoisted(&declaration.name, value)?;
                    }
                }
                if self.eval(condition)?.as_condition()? {
                    self.execute(then)
                } else if let Some(otherwise) = otherwise {
                    self.execute(otherwise)
                } else {
                    Ok(Flow::Next)
                }
            }
            Statement::For {
                initializer,
                condition,
                update,
                body,
            } => {
                self.push();
                let result = (|| {
                    if let Some(initializer) = initializer {
                        let flow = self.execute(initializer)?;
                        if !matches!(flow, Flow::Next) {
                            return Ok(flow);
                        }
                    }
                    loop {
                        self.tick()?;
                        if let Some(condition) = condition {
                            if !self.eval(condition)?.as_condition()? {
                                break;
                            }
                        }
                        match self.execute(body)? {
                            Flow::Break => break,
                            Flow::Return(v) => return Ok(Flow::Return(v)),
                            Flow::Discard => return Ok(Flow::Discard),
                            Flow::Next | Flow::Continue => {}
                        }
                        if let Some(update) = update {
                            self.eval(update)?;
                        }
                    }
                    Ok(Flow::Next)
                })();
                self.pop();
                result
            }
            Statement::While { condition, body } => {
                loop {
                    self.tick()?;
                    if !self.eval(condition)?.as_condition()? {
                        break;
                    }
                    match self.execute(body)? {
                        Flow::Break => break,
                        Flow::Continue | Flow::Next => {}
                        flow @ Flow::Return(_) | flow @ Flow::Discard => return Ok(flow),
                    }
                }
                Ok(Flow::Next)
            }
            Statement::DoWhile { condition, body } => {
                loop {
                    self.tick()?;
                    match self.execute(body)? {
                        Flow::Break => break,
                        Flow::Continue | Flow::Next => {}
                        flow @ Flow::Return(_) | flow @ Flow::Discard => return Ok(flow),
                    }
                    if !self.eval(condition)?.as_condition()? {
                        break;
                    }
                }
                Ok(Flow::Next)
            }
            Statement::Return { value } => Ok(Flow::Return(if let Some(value) = value {
                self.eval(value)?
            } else {
                Value::Void
            })),
            Statement::Break => Ok(Flow::Break),
            Statement::Continue => Ok(Flow::Continue),
            Statement::Discard => Ok(Flow::Discard),
        }
    }

    fn eval(&mut self, expression: &Expression) -> Result<Value, VmError> {
        self.tick()?;
        match expression {
            Expression::Literal {
                value_type, value, ..
            } => literal(value_type.0.as_str(), value),
            Expression::Identifier { name, .. } => {
                let frame = self
                    .resolve(name)
                    .ok_or_else(|| VmError::UnknownVariable { name: name.clone() })?;
                Ok(self.scopes[frame][name].value.clone())
            }
            Expression::Member { object, field, .. } => {
                let value = self.eval(object)?;
                if matches!(value, Value::Struct(_)) {
                    Ok(value.member(field)?.clone())
                } else {
                    value.read_swizzle(field)
                }
            }
            Expression::Index { object, index, .. } => {
                let value = self.eval(object)?;
                let index = self.eval(index)?.as_index()?;
                read_index(&value, index)
            }
            Expression::Unary {
                operator, operand, ..
            } if operator == "++" || operator == "--" => {
                let lvalue = self.lvalue(operand)?;
                let old = self.read_lvalue(&lvalue)?;
                let one = match old {
                    Value::Uint(_) | Value::UVec(_) => Value::Uint(1),
                    Value::Float(_) | Value::Vec(_) => Value::Float(1.0),
                    _ => Value::Int(1),
                };
                let result =
                    self.runtime
                        .binary(if operator == "++" { "+" } else { "-" }, &old, &one)?;
                self.write_lvalue(&lvalue, result.clone())?;
                Ok(result)
            }
            Expression::Unary {
                operator, operand, ..
            } => {
                let value = self.eval(operand)?;
                self.runtime.unary(operator, &value)
            }
            Expression::Postfix {
                operator, lvalue, ..
            } => {
                let target = self.lvalue(lvalue)?;
                let old = self.read_lvalue(&target)?;
                let one = match old {
                    Value::Uint(_) | Value::UVec(_) => Value::Uint(1),
                    Value::Float(_) | Value::Vec(_) => Value::Float(1.0),
                    _ => Value::Int(1),
                };
                let result =
                    self.runtime
                        .binary(if operator == "++" { "+" } else { "-" }, &old, &one)?;
                self.write_lvalue(&target, result)?;
                Ok(old)
            }
            Expression::Conditional {
                value_type,
                condition,
                when_true,
                when_false,
            } => {
                let selected = if self.eval(condition)?.as_condition()? {
                    self.eval(when_true)
                } else {
                    self.eval(when_false)
                }?;
                self.runtime.convert(&value_type.0, &selected)
            }
            Expression::Binary {
                operator,
                left,
                right,
                ..
            } if operator == "&&" => {
                let left = self.eval(left)?;
                if !left.as_bool()? {
                    Ok(Value::Bool(false))
                } else {
                    Ok(Value::Bool(self.eval(right)?.as_bool()?))
                }
            }
            Expression::Binary {
                operator,
                left,
                right,
                ..
            } if operator == "||" => {
                let left = self.eval(left)?;
                if left.as_bool()? {
                    Ok(Value::Bool(true))
                } else {
                    Ok(Value::Bool(self.eval(right)?.as_bool()?))
                }
            }
            Expression::Binary {
                operator,
                left,
                right,
                ..
            } => {
                let left_value = self.eval(left)?;
                let left_value = self
                    .runtime
                    .convert(&expression_value_type(left).0, &left_value)?;
                let right_value = self.eval(right)?;
                let right_value = self
                    .runtime
                    .convert(&expression_value_type(right).0, &right_value)?;
                self.runtime.binary(operator, &left_value, &right_value)
            }
            Expression::Assignment {
                operator,
                lvalue,
                value,
                ..
            } => {
                if self.temporal_aberration_factory_compatibility && operator == "=" {
                    if let Expression::Conditional {
                        value_type,
                        condition,
                        when_true,
                        when_false,
                    } = value.as_ref()
                    {
                        let condition = self.eval(condition)?.as_condition()?;
                        let selected = if condition {
                            self.eval(when_true)?
                        } else {
                            self.eval(when_false)?
                        };
                        let selected = self.runtime.convert(&value_type.0, &selected)?;
                        if condition {
                            // Match the current JavaScript CPU factory: its generated ternary
                            // evaluates the true branch but omits the surrounding assignment.
                            return Ok(selected);
                        }
                        let target = self.lvalue(lvalue)?;
                        self.write_lvalue(&target, selected.clone())?;
                        return Ok(selected);
                    }
                }
                let target = self.lvalue(lvalue)?;
                let right = self.eval(value)?;
                let result = if operator == "=" {
                    right
                } else {
                    let left = self.read_lvalue(&target)?;
                    self.runtime
                        .binary(operator.strip_suffix('=').unwrap(), &left, &right)?
                };
                self.write_lvalue(&target, result.clone())?;
                Ok(result)
            }
            Expression::Construct {
                value_type,
                arguments,
            } => {
                if let Some(base) = value_type.0.strip_suffix("[]") {
                    if self.structs.contains_key(base) {
                        let values = arguments
                            .iter()
                            .map(|argument| self.eval(argument))
                            .collect::<Result<Vec<_>, _>>()?;
                        if values
                            .iter()
                            .any(|value| !matches!(value, Value::Struct(_)))
                        {
                            return Err(type_error(format!(
                                "{} array constructor requires struct elements",
                                value_type.0
                            )));
                        }
                        return Ok(Value::Array(values));
                    }
                }
                if self.structs.contains_key(&value_type.0) {
                    let fields = self.structs[&value_type.0].clone();
                    let mut result = BTreeMap::new();
                    if arguments.is_empty() {
                        for (name, ty) in fields {
                            result.insert(name, self.runtime.default_value(&ty, &self.structs)?);
                        }
                    } else if arguments.len() == 1 {
                        let value = self.eval(&arguments[0])?;
                        if matches!(value, Value::Struct(_)) {
                            return Ok(value);
                        }
                        if fields.len() == 1 {
                            let (name, ty) = fields.into_iter().next().unwrap();
                            result.insert(name, self.convert_value(&ty, &value)?);
                            return Ok(Value::Struct(result));
                        }
                        return Err(type_error(format!(
                            "{} copy constructor requires a struct value",
                            value_type.0
                        )));
                    } else {
                        if arguments.len() != fields.len() {
                            return Err(type_error(format!(
                                "{} constructor field count mismatch",
                                value_type.0
                            )));
                        }
                        for ((name, ty), arg) in fields.into_iter().zip(arguments) {
                            let value = self.eval(arg)?;
                            result.insert(name, self.convert_value(&ty, &value)?);
                        }
                    }
                    Ok(Value::Struct(result))
                } else {
                    let values = arguments
                        .iter()
                        .map(|arg| self.eval(arg))
                        .collect::<Result<Vec<_>, _>>()?;
                    self.runtime.construct(&value_type.0, &values)
                }
            }
            Expression::Call {
                target, arguments, ..
            } if target.starts_with("builtin:") => {
                let values = arguments
                    .iter()
                    .map(|arg| self.eval(arg))
                    .collect::<Result<Vec<_>, _>>()?;
                self.runtime.call_builtin(&target[8..], &values)
            }
            Expression::Call {
                target, arguments, ..
            } => {
                let values = arguments
                    .iter()
                    .map(|argument| self.eval(argument))
                    .collect::<Result<Vec<_>, _>>()?;
                let lvalues = arguments
                    .iter()
                    .map(|argument| self.lvalue(argument).ok())
                    .collect::<Vec<_>>();
                self.invoke(target, &values, &lvalues)
            }
        }
    }

    fn call_function(
        &mut self,
        target: &str,
        arguments: &[(Value, Option<LValue>)],
    ) -> Result<Flow, VmError> {
        let values = arguments.iter().map(|(v, _)| v.clone()).collect::<Vec<_>>();
        let paths = arguments.iter().map(|(_, p)| p.clone()).collect::<Vec<_>>();
        self.invoke_flow(target, &values, &paths)
    }

    fn convert_value(&self, value_type: &str, value: &Value) -> Result<Value, VmError> {
        if self.structs.contains_key(value_type) && matches!(value, Value::Struct(_)) {
            return Ok(value.clone());
        }
        self.runtime.convert(value_type, value)
    }
    fn invoke(
        &mut self,
        target: &str,
        values: &[Value],
        lvalues: &[Option<LValue>],
    ) -> Result<Value, VmError> {
        match self.invoke_flow(target, values, lvalues)? {
            Flow::Return(value) => Ok(value),
            Flow::Next => Ok(Value::Void),
            Flow::Discard => Err(VmError::Discarded),
            Flow::Break | Flow::Continue => Err(VmError::InvalidIr {
                message: "loop flow escaped function".into(),
            }),
        }
    }
    fn invoke_flow(
        &mut self,
        target: &str,
        values: &[Value],
        lvalues: &[Option<LValue>],
    ) -> Result<Flow, VmError> {
        if self.javascript_sine_hash_random_functions.contains(target) {
            let [Value::Vec(value)] = values else {
                return Err(VmError::Arity {
                    name: target.into(),
                    expected: "one vec2".into(),
                    actual: values.len(),
                });
            };
            if value.len() != 2 {
                return Err(type_error("random__vec2 requires a two-lane vector"));
            }
            return Ok(Flow::Return(Value::Float(javascript_sine_hash_random(
                value[0], value[1],
            ))));
        }
        if target == "hash_uint__uint" {
            let [Value::Uint(value)] = values else {
                return Err(VmError::Arity {
                    name: target.into(),
                    expected: "one uint".into(),
                    actual: values.len(),
                });
            };
            return Ok(Flow::Return(Value::Uint(canonical_hash_uint(*value))));
        }
        if self.call_depth >= CALL_DEPTH_LIMIT {
            return Err(VmError::CallDepthLimit {
                limit: CALL_DEPTH_LIMIT,
            });
        }
        let index = *self
            .function_indices
            .get(target)
            .ok_or_else(|| VmError::UnknownFunction {
                target: target.into(),
            })?;
        let function: Function = self.program.functions[index].clone();
        if values.len() != function.parameters.len() {
            return Err(VmError::Arity {
                name: target.into(),
                expected: function.parameters.len().to_string(),
                actual: values.len(),
            });
        }
        self.call_depth += 1;
        self.push();
        let mut copy_back = Vec::new();
        let setup = (|| {
            for (index, parameter) in function.parameters.iter().enumerate() {
                let value = match parameter.qualifier {
                    ParameterQualifier::In | ParameterQualifier::InOut => {
                        self.convert_value(&parameter.value_type.0, &values[index])?
                    }
                    ParameterQualifier::Out => self
                        .runtime
                        .default_value(&parameter.value_type.0, &self.structs)?,
                };
                self.define(&parameter.name, value, true)?;
                if parameter.qualifier != ParameterQualifier::In {
                    let target = lvalues[index].clone().ok_or_else(|| {
                        type_error(format!(
                            "{} parameter {} needs writable argument",
                            parameter.qualifier.qualifier_name(),
                            parameter.name
                        ))
                    })?;
                    copy_back.push((parameter.name.clone(), target));
                }
            }
            Ok::<_, VmError>(())
        })();
        if let Err(error) = setup {
            self.pop();
            self.call_depth -= 1;
            return Err(error);
        }
        let mut flow = self.execute_many(&function.body);
        if flow.is_ok() {
            for (name, target) in copy_back {
                let value = self.scopes.last().unwrap()[&name].value.clone();
                if let Err(error) = self.write_lvalue(&target, value) {
                    flow = Err(error);
                    break;
                }
            }
        }
        self.pop();
        self.call_depth -= 1;
        flow
    }

    fn lvalue(&mut self, expression: &Expression) -> Result<LValue, VmError> {
        match expression {
            Expression::Identifier { name, .. } => {
                let frame = self
                    .resolve(name)
                    .ok_or_else(|| VmError::UnknownVariable { name: name.clone() })?;
                Ok(LValue {
                    frame,
                    name: name.clone(),
                    path: Vec::new(),
                })
            }
            Expression::Member { object, field, .. } => {
                let mut path = self.lvalue(object)?;
                let value = self.read_lvalue(&path)?;
                path.path.push(if matches!(value, Value::Struct(_)) {
                    PathPart::Member(field.clone())
                } else {
                    PathPart::Swizzle(field.clone())
                });
                Ok(path)
            }
            Expression::Index { object, index, .. } => {
                let mut path = self.lvalue(object)?;
                let index = self.eval(index)?.as_index()?;
                path.path.push(PathPart::Index(index));
                Ok(path)
            }
            _ => Err(type_error("expression is not an lvalue")),
        }
    }
    fn read_lvalue(&self, lvalue: &LValue) -> Result<Value, VmError> {
        let binding = self
            .scopes
            .get(lvalue.frame)
            .and_then(|f| f.get(&lvalue.name))
            .ok_or_else(|| VmError::UnknownVariable {
                name: lvalue.name.clone(),
            })?;
        read_path(&binding.value, &lvalue.path)
    }
    fn write_lvalue(&mut self, lvalue: &LValue, value: Value) -> Result<(), VmError> {
        let binding = self
            .scopes
            .get_mut(lvalue.frame)
            .and_then(|f| f.get_mut(&lvalue.name))
            .ok_or_else(|| VmError::UnknownVariable {
                name: lvalue.name.clone(),
            })?;
        if !binding.writable {
            return Err(VmError::ReadOnly {
                name: lvalue.name.clone(),
            });
        }
        write_path(&mut binding.value, &lvalue.path, value)
    }
}

fn is_javascript_float_atlas_program(program: &ProgramIr) -> bool {
    let has_atlas_declaration = program.functions.iter().any(|function| {
        function.name == "main"
            && function.mangled_name == "main__void"
            && function.return_type.0 == "void"
            && function.parameters.is_empty()
            && function.body.iter().any(|statement| {
                matches!(
                    statement,
                    Statement::Declaration { declarations }
                        if declarations
                            .iter()
                            .any(is_javascript_float_atlas_z_declaration)
                )
            })
    });
    if !has_atlas_declaration {
        return false;
    }

    let reaction_diffusion_signature = program.outputs == ["fragColor"]
        && program.functions.iter().any(|function| {
            let [voxel, volume_size] = function.parameters.as_slice() else {
                return false;
            };
            function.name == "laplacian3D"
                && function.mangled_name == "laplacian3D__ivec3_int"
                && function.return_type.0 == "vec2"
                && voxel.name == "voxel"
                && voxel.value_type.0 == "ivec3"
                && voxel.qualifier == ParameterQualifier::In
                && volume_size.name == "volSize"
                && volume_size.value_type.0 == "int"
                && volume_size.qualifier == ParameterQualifier::In
        });
    let shape_signature = program.outputs == ["fragColor", "geoOut"]
        && program.functions.iter().any(|function| {
            let [point, loop_factor_1, loop_factor_2] = function.parameters.as_slice() else {
                return false;
            };
            function.name == "computeValue"
                && function.mangled_name == "computeValue__vec3_float_float"
                && function.return_type.0 == "float"
                && point.name == "p"
                && point.value_type.0 == "vec3"
                && point.qualifier == ParameterQualifier::In
                && loop_factor_1.name == "lf1"
                && loop_factor_1.value_type.0 == "float"
                && loop_factor_1.qualifier == ParameterQualifier::In
                && loop_factor_2.name == "lf2"
                && loop_factor_2.value_type.0 == "float"
                && loop_factor_2.qualifier == ParameterQualifier::In
        });
    reaction_diffusion_signature || shape_signature
}

fn is_javascript_float_atlas_z_declaration(declaration: &VariableDefinition) -> bool {
    if declaration.name != "z"
        || declaration.value_type.0 != "int"
        || declaration.array_size.is_some()
    {
        return false;
    }
    let Some(Expression::Binary {
        value_type,
        operator,
        left,
        right,
    }) = declaration.initializer.as_ref()
    else {
        return false;
    };
    if value_type.0 != "int" || operator != "/" {
        return false;
    }
    let Expression::Identifier {
        value_type: denominator_type,
        name: denominator_name,
        storage: denominator_storage,
    } = right.as_ref()
    else {
        return false;
    };
    if denominator_type.0 != "int"
        || denominator_name != "volSize"
        || *denominator_storage != StorageClass::Local
    {
        return false;
    }
    match left.as_ref() {
        Expression::Member {
            value_type,
            object,
            field,
        } => {
            let Expression::Identifier {
                value_type: object_type,
                name,
                storage,
            } = object.as_ref()
            else {
                return false;
            };
            value_type.0 == "int"
                && field == "y"
                && object_type.0 == "ivec2"
                && name == "pixelCoord"
                && *storage == StorageClass::Local
        }
        Expression::Identifier {
            value_type,
            name,
            storage,
        } => value_type.0 == "int" && name == "yAtlas" && *storage == StorageClass::Local,
        _ => false,
    }
}

fn canonical_hash_uint(value: u32) -> u32 {
    let mut value = value;
    value ^= value >> 16;
    value = value.wrapping_mul(0x7feb_352d);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846c_a68b);
    value ^ (value >> 16)
}

fn is_javascript_sine_hash_random(function: &Function) -> bool {
    let [parameter] = function.parameters.as_slice() else {
        return false;
    };
    if function.name != "random"
        || function.mangled_name != "random__vec2"
        || function.return_type.0 != "float"
        || parameter.name != "st"
        || parameter.value_type.0 != "vec2"
        || parameter.qualifier != ParameterQualifier::In
    {
        return false;
    }
    let [
        Statement::Return {
            value:
                Some(Expression::Call {
                    value_type: fract_type,
                    name: fract_name,
                    target: fract_target,
                    arguments: fract_arguments,
                }),
        },
    ] = function.body.as_slice()
    else {
        return false;
    };
    let [
        Expression::Binary {
            value_type: multiply_type,
            operator,
            left,
            right,
        },
    ] = fract_arguments.as_slice()
    else {
        return false;
    };
    let Expression::Call {
        value_type: sin_type,
        name: sin_name,
        target: sin_target,
        arguments: sin_arguments,
    } = left.as_ref()
    else {
        return false;
    };
    let [
        Expression::Call {
            value_type: dot_type,
            name: dot_name,
            target: dot_target,
            arguments: dot_arguments,
        },
    ] = sin_arguments.as_slice()
    else {
        return false;
    };
    let [
        Expression::Member {
            value_type: coordinate_type,
            object,
            field,
        },
        Expression::Construct {
            value_type: vector_type,
            arguments: vector_arguments,
        },
    ] = dot_arguments.as_slice()
    else {
        return false;
    };
    let Expression::Identifier {
        value_type: input_type,
        name: input_name,
        storage,
    } = object.as_ref()
    else {
        return false;
    };
    let [a, b] = vector_arguments.as_slice() else {
        return false;
    };
    fract_type.0 == "float"
        && fract_name == "fract"
        && fract_target == "builtin:fract"
        && multiply_type.0 == "float"
        && operator == "*"
        && sin_type.0 == "float"
        && sin_name == "sin"
        && sin_target == "builtin:sin"
        && dot_type.0 == "float"
        && dot_name == "dot"
        && dot_target == "builtin:dot"
        && coordinate_type.0 == "vec2"
        && field == "xy"
        && input_type.0 == "vec2"
        && input_name == "st"
        && *storage == StorageClass::Parameter
        && vector_type.0 == "vec2"
        && is_exact_float_literal(a, "12.9898", 12.9898)
        && is_exact_float_literal(b, "78.233", 78.233)
        && is_exact_float_literal(right, "43758.5453123", 43_758.545_312_3)
}

fn is_exact_float_literal(expression: &Expression, source: &str, value: f64) -> bool {
    matches!(
        expression,
        Expression::Literal {
            value_type,
            value: literal_value,
            source: Some(literal_source),
        } if value_type.0 == "float"
            && literal_source == source
            && literal_value.as_f64() == Some(value)
    )
}

fn javascript_sine_hash_random(x: f32, y: f32) -> f32 {
    const A: f32 = 12.9898_f32;
    const B: f32 = 78.233_f32;
    const SCALE: f32 = f32::from_bits(0x472a_ee8c);
    let dot = (f64::from(x) * f64::from(A) + f64::from(y) * f64::from(B)) as f32;
    let sine = f64::from(dot).sin() as f32;
    let product = f64::from(sine) * f64::from(SCALE);
    (product - product.floor()) as f32
}

trait QualifierName {
    fn qualifier_name(&self) -> &'static str;
}
impl QualifierName for ParameterQualifier {
    fn qualifier_name(&self) -> &'static str {
        match self {
            Self::In => "in",
            Self::Out => "out",
            Self::InOut => "inout",
        }
    }
}
fn literal(value_type: &str, value: &serde_json::Value) -> Result<Value, VmError> {
    match value_type {
        "bool" => value.as_bool().map(Value::Bool),
        "int" => value.as_i64().map(|v| Value::Int(v as i32)),
        "uint" => value.as_u64().map(|v| Value::Uint(v as u32)),
        "float" => value.as_f64().map(|v| Value::Float(v as f32)),
        _ => None,
    }
    .ok_or_else(|| type_error(format!("invalid {value_type} literal {value}")))
}
fn read_index(value: &Value, index: usize) -> Result<Value, VmError> {
    match value {
        Value::Array(v) => v.get(index).cloned().ok_or(VmError::Bounds {
            index,
            length: v.len(),
        }),
        Value::Vec(v) => v
            .get(index)
            .copied()
            .map(Value::Float)
            .ok_or(VmError::Bounds {
                index,
                length: v.len(),
            }),
        Value::IVec(v) => v
            .get(index)
            .copied()
            .map(Value::Int)
            .ok_or(VmError::Bounds {
                index,
                length: v.len(),
            }),
        Value::UVec(v) => v
            .get(index)
            .copied()
            .map(Value::Uint)
            .ok_or(VmError::Bounds {
                index,
                length: v.len(),
            }),
        Value::BVec(v) => v
            .get(index)
            .copied()
            .map(Value::Bool)
            .ok_or(VmError::Bounds {
                index,
                length: v.len(),
            }),
        Value::Mat { dimension, columns } => {
            if index >= *dimension {
                return Err(VmError::Bounds {
                    index,
                    length: *dimension,
                });
            }
            Ok(Value::Vec(
                columns[index * dimension..(index + 1) * dimension].to_vec(),
            ))
        }
        _ => Err(type_error(format!("cannot index {}", value.kind()))),
    }
}
fn read_path(value: &Value, path: &[PathPart]) -> Result<Value, VmError> {
    let mut value = value.clone();
    for part in path {
        match part {
            PathPart::Member(field) => value = value.member(field)?.clone(),
            PathPart::Index(index) => value = read_index(&value, *index)?,
            PathPart::Swizzle(swizzle) => value = value.read_swizzle(swizzle)?,
        }
    }
    Ok(value)
}
fn write_path(value: &mut Value, path: &[PathPart], replacement: Value) -> Result<(), VmError> {
    if path.is_empty() {
        *value = replacement;
        return Ok(());
    }
    match &path[0] {
        PathPart::Member(field) => write_path(value.member_mut(field)?, &path[1..], replacement),
        PathPart::Index(index) => {
            let mut selected = read_index(value, *index)?;
            write_path(&mut selected, &path[1..], replacement)?;
            write_index(value, *index, selected)
        }
        PathPart::Swizzle(swizzle) => {
            let mut selected = value.read_swizzle(swizzle)?;
            write_path(&mut selected, &path[1..], replacement)?;
            value.write_swizzle(swizzle, selected)
        }
    }
}

fn write_index(value: &mut Value, index: usize, replacement: Value) -> Result<(), VmError> {
    match (value, replacement) {
        (Value::Array(values), replacement) => {
            let length = values.len();
            *values
                .get_mut(index)
                .ok_or(VmError::Bounds { index, length })? = replacement;
        }
        (Value::Vec(values), Value::Float(replacement)) => write_lane(values, index, replacement)?,
        (Value::IVec(values), Value::Int(replacement)) => write_lane(values, index, replacement)?,
        (Value::UVec(values), Value::Uint(replacement)) => write_lane(values, index, replacement)?,
        (Value::BVec(values), Value::Bool(replacement)) => write_lane(values, index, replacement)?,
        (Value::Mat { dimension, columns }, Value::Vec(replacement)) => {
            if index >= *dimension {
                return Err(VmError::Bounds {
                    index,
                    length: *dimension,
                });
            }
            if replacement.len() != *dimension {
                return Err(type_error(format!(
                    "matrix column needs {} lanes, found {}",
                    dimension,
                    replacement.len()
                )));
            }
            columns[index * *dimension..(index + 1) * *dimension].copy_from_slice(&replacement);
        }
        (value, replacement) => {
            return Err(type_error(format!(
                "cannot assign {} through an index of {}",
                replacement.kind(),
                value.kind()
            )));
        }
    }
    Ok(())
}

fn write_lane<T>(values: &mut [T], index: usize, replacement: T) -> Result<(), VmError> {
    let length = values.len();
    *values
        .get_mut(index)
        .ok_or(VmError::Bounds { index, length })? = replacement;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{Type, shader_bundle};

    fn visit_declarations<'a>(
        statements: &'a [Statement],
        visitor: &mut impl FnMut(&'a VariableDefinition),
    ) {
        for statement in statements {
            match statement {
                Statement::Block { body } => visit_declarations(body, visitor),
                Statement::Declaration { declarations } => {
                    for declaration in declarations {
                        visitor(declaration);
                    }
                }
                Statement::If {
                    hoisted_declarations,
                    then,
                    otherwise,
                    ..
                } => {
                    for declaration in hoisted_declarations {
                        visitor(declaration);
                    }
                    visit_declarations(std::slice::from_ref(then.as_ref()), visitor);
                    if let Some(otherwise) = otherwise {
                        visit_declarations(std::slice::from_ref(otherwise.as_ref()), visitor);
                    }
                }
                Statement::For {
                    initializer, body, ..
                } => {
                    if let Some(initializer) = initializer {
                        visit_declarations(std::slice::from_ref(initializer.as_ref()), visitor);
                    }
                    visit_declarations(std::slice::from_ref(body.as_ref()), visitor);
                }
                Statement::While { body, .. } | Statement::DoWhile { body, .. } => {
                    visit_declarations(std::slice::from_ref(body.as_ref()), visitor);
                }
                Statement::Expression { .. }
                | Statement::Return { .. }
                | Statement::Break
                | Statement::Continue
                | Statement::Discard => {}
            }
        }
    }

    fn atlas_declaration(key: &str) -> VariableDefinition {
        let program = &shader_bundle().unwrap().programs[key].ir;
        let mut found = Vec::new();
        for function in &program.functions {
            visit_declarations(&function.body, &mut |declaration| {
                if is_javascript_float_atlas_z_declaration(declaration) {
                    found.push(declaration.clone());
                }
            });
        }
        assert_eq!(found.len(), 1, "expected one atlas z declaration in {key}");
        found.pop().unwrap()
    }

    fn count_typed_int_divisions(value: &serde_json::Value) -> usize {
        match value {
            serde_json::Value::Array(values) => values.iter().map(count_typed_int_divisions).sum(),
            serde_json::Value::Object(fields) => {
                usize::from(
                    fields.get("kind").and_then(serde_json::Value::as_str) == Some("binary")
                        && fields.get("operator").and_then(serde_json::Value::as_str) == Some("/")
                        && fields.get("type").and_then(serde_json::Value::as_str) == Some("int"),
                ) + fields
                    .values()
                    .map(count_typed_int_divisions)
                    .sum::<usize>()
            }
            _ => 0,
        }
    }

    #[test]
    fn javascript_float_atlas_z_fingerprint_matches_exactly_two_bundle_programs() {
        let matches = shader_bundle()
            .unwrap()
            .programs
            .iter()
            .filter(|(_, program)| is_javascript_float_atlas_program(&program.ir))
            .map(|(key, _)| key.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            matches,
            [
                "synth3d/reactionDiffusion3d:simulate",
                "synth3d/shape3d:precompute"
            ]
        );
    }

    #[test]
    fn atlas_program_integer_division_inventory_is_frozen() {
        let shaders: serde_json::Value =
            serde_json::from_str(include_str!("generated/shaders.json")).unwrap();
        let programs = &shaders["programs"];
        assert_eq!(
            count_typed_int_divisions(&programs["synth3d/reactionDiffusion3d:simulate"]["ir"]),
            2
        );
        assert_eq!(
            count_typed_int_divisions(&programs["synth3d/shape3d:precompute"]["ir"]),
            1
        );
    }

    #[test]
    fn javascript_float_atlas_z_fingerprint_rejects_near_misses() {
        let declaration = atlas_declaration("synth3d/reactionDiffusion3d:simulate");
        assert!(is_javascript_float_atlas_z_declaration(&declaration));

        let mut wrong_declaration = declaration.clone();
        wrong_declaration.name = "slice".into();
        assert!(!is_javascript_float_atlas_z_declaration(&wrong_declaration));

        let mut wrong_denominator = declaration.clone();
        let Some(Expression::Binary { right, .. }) = &mut wrong_denominator.initializer else {
            panic!("expected binary initializer");
        };
        let Expression::Identifier { name, .. } = right.as_mut() else {
            panic!("expected identifier denominator");
        };
        *name = "width".into();
        assert!(!is_javascript_float_atlas_z_declaration(&wrong_denominator));

        let mut wrong_numerator = declaration;
        let Some(Expression::Binary { left, .. }) = &mut wrong_numerator.initializer else {
            panic!("expected binary initializer");
        };
        let Expression::Member { field, .. } = left.as_mut() else {
            panic!("expected member numerator");
        };
        *field = "x".into();
        assert!(!is_javascript_float_atlas_z_declaration(&wrong_numerator));

        let mut wrong_program =
            shader_bundle().unwrap().programs["synth3d/reactionDiffusion3d:simulate"]
                .ir
                .clone();
        wrong_program
            .functions
            .iter_mut()
            .find(|function| function.mangled_name == "laplacian3D__ivec3_int")
            .unwrap()
            .return_type = Type("vec3".into());
        assert!(!is_javascript_float_atlas_program(&wrong_program));

        let already_exact = &shader_bundle().unwrap().programs["synth3d/noise3d:precompute"].ir;
        assert!(!is_javascript_float_atlas_program(already_exact));
    }

    #[test]
    fn atlas_z_initializer_is_fractional_but_same_program_integer_division_truncates() {
        let ir = &shader_bundle().unwrap().programs["synth3d/reactionDiffusion3d:simulate"].ir;
        let mut vm = ShaderVm::new(ir, Runtime::new()).unwrap();
        vm.push();
        vm.define("pixelCoord", Value::IVec(vec![0, 3]), true)
            .unwrap();
        vm.define("volSize", Value::Int(2), true).unwrap();
        let declaration = atlas_declaration("synth3d/reactionDiffusion3d:simulate");
        vm.execute(&Statement::Declaration {
            declarations: vec![declaration],
        })
        .unwrap();
        assert_eq!(vm.scopes.last().unwrap()["z"].value, Value::Float(1.5));

        vm.write_root("volSize", Value::Int(7)).unwrap();
        let unrelated = Expression::Binary {
            value_type: Type("int".into()),
            operator: "/".into(),
            left: Box::new(Expression::Identifier {
                value_type: Type("int".into()),
                name: "volSize".into(),
                storage: StorageClass::Local,
            }),
            right: Box::new(Expression::Literal {
                value_type: Type("int".into()),
                value: serde_json::json!(2),
                source: Some("2".into()),
            }),
        };
        assert_eq!(vm.eval(&unrelated).unwrap(), Value::Int(3));
    }

    fn random_functions() -> Vec<(&'static str, Function)> {
        shader_bundle()
            .unwrap()
            .programs
            .iter()
            .filter_map(|(key, program)| {
                program
                    .ir
                    .functions
                    .iter()
                    .find(|function| function.mangled_name == "random__vec2")
                    .cloned()
                    .map(|function| (key.as_str(), function))
            })
            .collect()
    }

    fn dot_vector_first_literal(function: &mut Function) -> &mut Expression {
        let [
            Statement::Return {
                value:
                    Some(Expression::Call {
                        arguments: fract_arguments,
                        ..
                    }),
            },
        ] = function.body.as_mut_slice()
        else {
            panic!("expected one return statement");
        };
        let [Expression::Binary { left, .. }] = fract_arguments.as_mut_slice() else {
            panic!("expected fract binary argument");
        };
        let Expression::Call {
            arguments: sin_arguments,
            ..
        } = left.as_mut()
        else {
            panic!("expected sin call");
        };
        let [
            Expression::Call {
                arguments: dot_arguments,
                ..
            },
        ] = sin_arguments.as_mut_slice()
        else {
            panic!("expected dot call");
        };
        let [_, Expression::Construct { arguments, .. }] = dot_arguments.as_mut_slice() else {
            panic!("expected dot constant vector");
        };
        &mut arguments[0]
    }

    #[test]
    fn javascript_sine_hash_fingerprint_matches_exactly_two_of_nine_random_functions() {
        let functions = random_functions();
        assert_eq!(functions.len(), 9);
        let matches = functions
            .iter()
            .filter(|(_, function)| is_javascript_sine_hash_random(function))
            .map(|(key, _)| *key)
            .collect::<Vec<_>>();
        assert_eq!(
            matches,
            ["synth/cellularAutomata:caFb", "synth/mnca:mncaFb"]
        );
    }

    #[test]
    fn javascript_sine_hash_fingerprint_rejects_signature_and_dot_constant_changes() {
        let (_, function) = random_functions()
            .into_iter()
            .find(|(key, _)| *key == "synth/cellularAutomata:caFb")
            .unwrap();
        assert!(is_javascript_sine_hash_random(&function));

        let mut wrong_result = function.clone();
        wrong_result.return_type = Type("vec2".into());
        assert!(!is_javascript_sine_hash_random(&wrong_result));

        let mut wrong_parameter = function.clone();
        wrong_parameter.parameters[0].value_type = Type("vec3".into());
        assert!(!is_javascript_sine_hash_random(&wrong_parameter));

        let mut wrong_constant = function;
        *dot_vector_first_literal(&mut wrong_constant) = Expression::Literal {
            value_type: Type("float".into()),
            value: serde_json::json!(13.0),
            source: Some("13.0".into()),
        };
        assert!(!is_javascript_sine_hash_random(&wrong_constant));
    }

    #[test]
    fn nonmatching_random_targets_execute_their_ir_and_return_scalar_literals() {
        for (key, _) in random_functions()
            .into_iter()
            .filter(|(_, function)| !is_javascript_sine_hash_random(function))
        {
            let mut ir = shader_bundle().unwrap().programs[key].ir.clone();
            let function = ir
                .functions
                .iter_mut()
                .find(|function| function.mangled_name == "random__vec2")
                .unwrap();
            function.body = vec![Statement::Return {
                value: Some(Expression::Literal {
                    value_type: Type("float".into()),
                    value: serde_json::json!(0.375),
                    source: Some("0.375".into()),
                }),
            }];

            let mut vm = ShaderVm::new(&ir, Runtime::new()).unwrap();
            assert_eq!(
                vm.invoke("random__vec2", &[Value::Vec(vec![4.25, -1.5])], &[None])
                    .unwrap(),
                Value::Float(0.375),
                "{key} should execute its IR body"
            );
        }
    }

    #[test]
    fn javascript_sine_hash_has_literal_js_scalar_result() {
        let expected_bits = 0x3e2c_a623;
        assert_eq!(
            javascript_sine_hash_random(1.5, 1.5).to_bits(),
            expected_bits
        );

        let ir = &shader_bundle().unwrap().programs["synth/cellularAutomata:caFb"].ir;
        let mut vm = ShaderVm::new(ir, Runtime::new()).unwrap();
        let Value::Float(result) = vm
            .invoke("random__vec2", &[Value::Vec(vec![1.5, 1.5])], &[None])
            .unwrap()
        else {
            panic!("random__vec2 should return a scalar float");
        };
        assert_eq!(result.to_bits(), expected_bits);
    }

    #[test]
    fn navier_hash_targets_remain_normal_ir_functions() {
        let ir = &shader_bundle().unwrap().programs["synth/navierStokes:nsSplat"].ir;
        let mut vm = ShaderVm::new(ir, Runtime::new()).unwrap();
        vm.program
            .functions
            .iter_mut()
            .find(|function| function.mangled_name == "hash11__float")
            .unwrap()
            .body = vec![Statement::Return {
            value: Some(Expression::Literal {
                value_type: Type("float".into()),
                value: serde_json::json!(0.125),
                source: Some("0.125".into()),
            }),
        }];
        vm.program
            .functions
            .iter_mut()
            .find(|function| function.mangled_name == "hash22__vec2")
            .unwrap()
            .body = vec![Statement::Return {
            value: Some(Expression::Construct {
                value_type: Type("vec2".into()),
                arguments: vec![
                    Expression::Literal {
                        value_type: Type("float".into()),
                        value: serde_json::json!(0.25),
                        source: Some("0.25".into()),
                    },
                    Expression::Literal {
                        value_type: Type("float".into()),
                        value: serde_json::json!(0.75),
                        source: Some("0.75".into()),
                    },
                ],
            }),
        }];
        assert_eq!(
            vm.invoke("hash11__float", &[Value::Float(1.0)], &[None])
                .unwrap(),
            Value::Float(0.125)
        );
        assert_eq!(
            vm.invoke("hash22__vec2", &[Value::Vec(vec![1.0, 1.0])], &[None])
                .unwrap(),
            Value::Vec(vec![0.25, 0.75])
        );
    }
}
