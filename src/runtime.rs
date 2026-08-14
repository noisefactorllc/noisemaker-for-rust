use std::collections::BTreeMap;

use crate::value::type_error;
use crate::{FilterMode, Surface, Value, VmError, sample_bilinear, sample_nearest};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DerivativeMode {
    #[default]
    Off,
    Record,
    Replay,
}

#[derive(Clone, Debug)]
pub struct DerivativeDiff {
    dx: Value,
    dy: Value,
    width: Value,
}

/// Stateful resources and GLSL operations used by [`crate::ShaderVm`].
#[derive(Clone, Debug, Default)]
pub struct Runtime {
    textures: Vec<Surface>,
    reduced_turn_sine: bool,
    derivative_mode: DerivativeMode,
    derivative_log: Vec<(String, Value)>,
    derivative_diffs: Vec<DerivativeDiff>,
    derivative_index: usize,
}

impl Runtime {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_texture(&mut self, surface: Surface) -> Value {
        let index = self.textures.len();
        self.textures.push(surface);
        Value::Sampler(index)
    }

    pub(crate) fn set_reduced_turn_sine(&mut self, enabled: bool) {
        self.reduced_turn_sine = enabled;
    }

    pub fn set_derivative_mode(&mut self, mode: DerivativeMode) {
        self.derivative_mode = mode;
        self.derivative_index = 0;
        self.derivative_log.clear();
        if mode != DerivativeMode::Replay {
            self.derivative_diffs.clear();
        }
    }
    pub fn set_derivative_replay(&mut self, diffs: Vec<DerivativeDiff>) {
        self.derivative_mode = DerivativeMode::Replay;
        self.derivative_diffs = diffs;
        self.derivative_index = 0;
        self.derivative_log.clear();
    }

    #[must_use]
    pub fn derivative_call_count(&self) -> usize {
        self.derivative_index
    }
    pub fn take_derivative_log(&mut self) -> Vec<(String, Value)> {
        std::mem::take(&mut self.derivative_log)
    }

    pub fn fine_derivatives(
        lanes: &[Vec<(String, Value)>],
        x_parity: usize,
        y_parity: usize,
    ) -> Result<Vec<DerivativeDiff>, VmError> {
        if lanes.len() != 4 {
            return Err(type_error(
                "fine derivatives require exactly four quad lanes",
            ));
        }
        let count = lanes[0].len();
        for (lane, log) in lanes.iter().enumerate().skip(1) {
            if log.len() != count {
                return Err(VmError::DivergentDerivatives {
                    lane,
                    call: log.len().min(count),
                });
            }
            for call in 0..count {
                if log[call].0 != lanes[0][call].0
                    || shape(&log[call].1) != shape(&lanes[0][call].1)
                {
                    return Err(VmError::DivergentDerivatives { lane, call });
                }
            }
        }
        let (left, right) = (&lanes[y_parity * 2], &lanes[y_parity * 2 + 1]);
        let (bottom, top) = (&lanes[x_parity], &lanes[x_parity + 2]);
        (0..count)
            .map(|call| {
                let dx = difference(&right[call].1, &left[call].1)?;
                let dy = difference(&top[call].1, &bottom[call].1)?;
                let width = add_abs(&dx, &dy)?;
                Ok(DerivativeDiff { dx, dy, width })
            })
            .collect()
    }

    pub fn construct(&self, value_type: &str, arguments: &[Value]) -> Result<Value, VmError> {
        if let Some(base) = value_type.strip_suffix("[]") {
            return arguments
                .iter()
                .map(|value| self.convert(base, value))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array);
        }
        match value_type {
            "bool" => unary_argument(value_type, arguments)
                .and_then(to_bool)
                .map(Value::Bool),
            "int" => unary_argument(value_type, arguments)
                .and_then(to_i32)
                .map(Value::Int),
            "uint" => unary_argument(value_type, arguments)
                .and_then(to_u32)
                .map(Value::Uint),
            "float" => unary_argument(value_type, arguments)
                .and_then(to_f32)
                .map(Value::Float),
            "sampler2D" => match unary_argument(value_type, arguments)? {
                Value::Sampler(index) => Ok(Value::Sampler(*index)),
                value => Err(type_error(format!(
                    "cannot construct sampler2D from {}",
                    value.kind()
                ))),
            },
            name if vector_spec(name).is_some() => {
                let (base, width) = vector_spec(name).unwrap();
                let flat = flatten(arguments);
                if flat.is_empty() {
                    return Err(type_error(format!("{name} constructor has no components")));
                }
                let chosen = if flat.len() == 1 && width > 1 {
                    vec![flat[0].clone(); width]
                } else {
                    flat
                };
                if chosen.len() < width {
                    return Err(type_error(format!(
                        "{name} constructor needs {width} components"
                    )));
                }
                build_vector(base, chosen.iter().take(width))
            }
            name if name.starts_with("mat") => {
                let dimension = name[3..]
                    .parse::<usize>()
                    .map_err(|_| type_error(format!("invalid matrix type {name}")))?;
                if !(2..=4).contains(&dimension) {
                    return Err(type_error(format!("invalid matrix type {name}")));
                }
                if arguments.len() == 1 {
                    if let Value::Mat {
                        dimension: source_dim,
                        columns,
                    } = &arguments[0]
                    {
                        if *source_dim == dimension {
                            return Ok(arguments[0].clone());
                        }
                        let mut output = vec![0.0; dimension * dimension];
                        for col in 0..dimension.min(*source_dim) {
                            for row in 0..dimension.min(*source_dim) {
                                output[col * dimension + row] = columns[col * source_dim + row];
                            }
                        }
                        for i in *source_dim..dimension {
                            output[i * dimension + i] = 1.0;
                        }
                        return Ok(Value::Mat {
                            dimension,
                            columns: output,
                        });
                    }
                    if let Ok(diagonal) = to_f32(&arguments[0]) {
                        let mut columns = vec![0.0; dimension * dimension];
                        for i in 0..dimension {
                            columns[i * dimension + i] = diagonal;
                        }
                        return Ok(Value::Mat { dimension, columns });
                    }
                }
                let flat = flatten(arguments);
                if flat.len() < dimension * dimension {
                    return Err(type_error(format!(
                        "{name} constructor needs {} components",
                        dimension * dimension
                    )));
                }
                Ok(Value::Mat {
                    dimension,
                    columns: flat
                        .iter()
                        .take(dimension * dimension)
                        .map(to_f32)
                        .collect::<Result<_, _>>()?,
                })
            }
            _ => Err(type_error(format!("unknown constructor type {value_type}"))),
        }
    }

    pub fn convert(&self, value_type: &str, value: &Value) -> Result<Value, VmError> {
        if let (Some(base), Value::Array(values)) = (value_type.strip_suffix("[]"), value) {
            return values
                .iter()
                .map(|element| self.convert(base, element))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array);
        }
        self.construct(value_type, std::slice::from_ref(value))
    }

    pub fn default_value(
        &self,
        value_type: &str,
        structs: &BTreeMap<String, Vec<(String, String)>>,
    ) -> Result<Value, VmError> {
        if let Some(base) = value_type.strip_suffix("[]") {
            let _ = base;
            return Ok(Value::Array(Vec::new()));
        }
        if let Some(fields) = structs.get(value_type) {
            return fields
                .iter()
                .map(|(name, field_type)| {
                    Ok((name.clone(), self.default_value(field_type, structs)?))
                })
                .collect::<Result<BTreeMap<_, _>, _>>()
                .map(Value::Struct);
        }
        match value_type {
            "void" => Ok(Value::Void),
            "bool" => Ok(Value::Bool(false)),
            "int" => Ok(Value::Int(0)),
            "uint" => Ok(Value::Uint(0)),
            "float" => Ok(Value::Float(0.0)),
            "sampler2D" => Ok(Value::Sampler(usize::MAX)),
            name if vector_spec(name).is_some() => {
                let (base, width) = vector_spec(name).unwrap();
                build_vector(base, std::iter::repeat_n(&Value::Int(0), width))
            }
            name if name.starts_with("mat") => {
                let n = name[3..]
                    .parse::<usize>()
                    .map_err(|_| type_error("bad matrix type"))?;
                Ok(Value::Mat {
                    dimension: n,
                    columns: vec![0.0; n * n],
                })
            }
            _ => Err(type_error(format!("unknown type {value_type}"))),
        }
    }

    pub fn unary(&self, operator: &str, value: &Value) -> Result<Value, VmError> {
        match (operator, value) {
            ("+", _) => Ok(value.clone()),
            ("!", Value::Bool(v)) => Ok(Value::Bool(!v)),
            ("~", Value::Int(v)) => Ok(Value::Int(!v)),
            ("~", Value::Uint(v)) => Ok(Value::Uint(!v)),
            ("~", Value::IVec(v)) => Ok(Value::IVec(v.iter().map(|x| !x).collect())),
            ("~", Value::UVec(v)) => Ok(Value::UVec(v.iter().map(|x| !x).collect())),
            ("-", Value::Int(v)) => Ok(Value::Int(v.wrapping_neg())),
            ("-", Value::Float(v)) => Ok(Value::Float(-v)),
            ("-", Value::IVec(v)) => Ok(Value::IVec(v.iter().map(|x| x.wrapping_neg()).collect())),
            ("-", Value::Vec(v)) => Ok(Value::Vec(v.iter().map(|x| -*x).collect())),
            _ => Err(type_error(format!(
                "unsupported unary {operator} for {}",
                value.kind()
            ))),
        }
    }

    pub fn binary(&self, operator: &str, left: &Value, right: &Value) -> Result<Value, VmError> {
        if matches!(
            operator,
            "==" | "!=" | "<" | ">" | "<=" | ">=" | "&&" | "||"
        ) {
            return logical(operator, left, right);
        }
        if operator == "*" {
            if let Some(value) = matrix_product(left, right)? {
                return Ok(value);
            }
        }
        match (left, right) {
            (Value::Float(a), Value::Float(b)) => float_binary(operator, *a, *b).map(Value::Float),
            (Value::Int(a), Value::Int(b)) => int_binary(operator, *a, *b).map(Value::Int),
            (Value::Uint(a), Value::Uint(b)) => uint_binary(operator, *a, *b).map(Value::Uint),
            (Value::Vec(a), Value::Vec(b)) => {
                zip_map(a, b, |a, b| float_binary(operator, a, b)).map(Value::Vec)
            }
            (Value::Vec(a), Value::Float(b)) => {
                map(a, |a| float_binary(operator, a, *b)).map(Value::Vec)
            }
            (Value::Float(a), Value::Vec(b)) => {
                map(b, |b| float_binary(operator, *a, b)).map(Value::Vec)
            }
            (Value::IVec(a), Value::IVec(b)) => {
                zip_map(a, b, |a, b| int_binary(operator, a, b)).map(Value::IVec)
            }
            (Value::IVec(a), Value::Int(b)) => {
                map(a, |a| int_binary(operator, a, *b)).map(Value::IVec)
            }
            (Value::Int(a), Value::IVec(b)) => {
                map(b, |b| int_binary(operator, *a, b)).map(Value::IVec)
            }
            (Value::UVec(a), Value::UVec(b)) => {
                zip_map(a, b, |a, b| uint_binary(operator, a, b)).map(Value::UVec)
            }
            (Value::UVec(a), Value::Uint(b)) => {
                map(a, |a| uint_binary(operator, a, *b)).map(Value::UVec)
            }
            (Value::Uint(a), Value::UVec(b)) => {
                map(b, |b| uint_binary(operator, *a, b)).map(Value::UVec)
            }
            (Value::Uint(a), Value::Int(b)) if operator == "<<" || operator == ">>" => {
                uint_binary(operator, *a, *b as u32).map(Value::Uint)
            }
            (Value::UVec(a), Value::Int(b)) if operator == "<<" || operator == ">>" => {
                map(a, |a| uint_binary(operator, a, *b as u32)).map(Value::UVec)
            }
            _ => Err(type_error(format!(
                "unsupported binary {operator} for {} and {}",
                left.kind(),
                right.kind()
            ))),
        }
    }

    pub fn call_builtin(&mut self, name: &str, arguments: &[Value]) -> Result<Value, VmError> {
        let arities: &[usize] = match name {
            "atan" => &[1, 2],
            "abs" | "acos" | "all" | "any" | "ceil" | "cos" | "dFdx" | "dFdy" | "degrees"
            | "exp" | "floatBitsToUint" | "floor" | "fract" | "fwidth" | "inversesqrt"
            | "isnan" | "length" | "log" | "log2" | "normalize" | "packHalf2x16" | "radians"
            | "round" | "sign" | "sin" | "sqrt" | "tanh" | "uintBitsToFloat" | "unpackHalf2x16" => {
                &[1]
            }
            "cross" | "distance" | "dot" | "equal" | "greaterThan" | "greaterThanEqual"
            | "lessThan" | "lessThanEqual" | "max" | "min" | "mod" | "notEqual" | "pow"
            | "reflect" | "step" | "texture" => &[2],
            "clamp" | "mix" | "refract" | "smoothstep" | "texelFetch" | "textureLod" => &[3],
            "textureSize" => &[2],
            _ => return Err(VmError::UnknownBuiltin { name: name.into() }),
        };
        if !arities.contains(&arguments.len()) {
            return Err(VmError::Arity {
                name: name.into(),
                expected: arities
                    .iter()
                    .map(usize::to_string)
                    .collect::<Vec<_>>()
                    .join(" or "),
                actual: arguments.len(),
            });
        }
        if let Some(result) = integer_component_builtin(name, arguments) {
            return result;
        }
        if name == "sin" && self.reduced_turn_sine {
            return reduced_turn_sine(&arguments[0]);
        }
        match name {
            "texture" => self.texture(&arguments[0], &arguments[1]),
            "textureLod" => self.texture(&arguments[0], &arguments[1]),
            "texelFetch" => self.texel_fetch(&arguments[0], &arguments[1]),
            "textureSize" => self.texture_size(&arguments[0]),
            "dFdx" | "dFdy" | "fwidth" => self.derivative(name, &arguments[0]),
            "dot" => dot(&arguments[0], &arguments[1]).map(Value::Float),
            "length" => length(&arguments[0]).map(Value::Float),
            "distance" => difference(&arguments[0], &arguments[1])
                .and_then(|v| length(&v))
                .map(Value::Float),
            "normalize" => normalize(&arguments[0]),
            "cross" => cross(&arguments[0], &arguments[1]),
            "reflect" => reflect(&arguments[0], &arguments[1]),
            "refract" => refract(&arguments[0], &arguments[1], &arguments[2]),
            "any" => bool_lanes(&arguments[0]).map(|v| Value::Bool(v.iter().any(|x| *x))),
            "all" => bool_lanes(&arguments[0]).map(|v| Value::Bool(v.iter().all(|x| *x))),
            "lessThan" | "lessThanEqual" | "greaterThan" | "greaterThanEqual" | "equal"
            | "notEqual" => relational(name, &arguments[0], &arguments[1]),
            "floatBitsToUint" => bitcast_float_uint(&arguments[0], true),
            "uintBitsToFloat" => bitcast_float_uint(&arguments[0], false),
            "packHalf2x16" => {
                let v = float_lanes(&arguments[0])?;
                if v.len() != 2 {
                    return Err(type_error("packHalf2x16 expects vec2"));
                }
                Ok(Value::Uint(
                    u32::from(f32_to_f16(v[0])) | (u32::from(f32_to_f16(v[1])) << 16),
                ))
            }
            "unpackHalf2x16" => {
                let u = to_u32(&arguments[0])?;
                Ok(Value::Vec(vec![
                    f16_to_f32(u as u16),
                    f16_to_f32((u >> 16) as u16),
                ]))
            }
            _ => component_builtin(name, arguments),
        }
    }

    pub fn pcg3d(&self, mut v: [u32; 3]) -> [u32; 3] {
        for x in &mut v {
            *x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        }
        v[0] = v[0].wrapping_add(v[1].wrapping_mul(v[2]));
        v[1] = v[1].wrapping_add(v[2].wrapping_mul(v[0]));
        v[2] = v[2].wrapping_add(v[0].wrapping_mul(v[1]));
        for x in &mut v {
            *x ^= *x >> 16;
        }
        v[0] = v[0].wrapping_add(v[1].wrapping_mul(v[2]));
        v[1] = v[1].wrapping_add(v[2].wrapping_mul(v[0]));
        v[2] = v[2].wrapping_add(v[0].wrapping_mul(v[1]));
        v
    }

    fn texture(&self, sampler: &Value, uv: &Value) -> Result<Value, VmError> {
        let surface = self.surface(sampler)?;
        let p = float_lanes(uv)?;
        if p.len() < 2 {
            return Err(type_error("texture coordinate must be vec2"));
        }
        let c = if surface.filter_mode() == FilterMode::Linear {
            sample_bilinear(surface, p[0], p[1])
        } else {
            sample_nearest(surface, p[0], p[1])
        };
        Ok(Value::Vec(c.to_vec()))
    }
    fn texel_fetch(&self, sampler: &Value, coord: &Value) -> Result<Value, VmError> {
        let surface = self.surface(sampler)?;
        let p = int_lanes(coord)?;
        if p.len() < 2 {
            return Err(type_error("texelFetch coordinate must be ivec2"));
        }
        let x = p[0].clamp(0, surface.width() as i32 - 1) as usize;
        let sy = p[1].clamp(0, surface.height() as i32 - 1) as usize;
        let y = surface.height() as usize - 1 - sy;
        let i = (y * surface.width() as usize + x) * 4;
        Ok(Value::Vec(surface.data()[i..i + 4].to_vec()))
    }
    fn texture_size(&self, sampler: &Value) -> Result<Value, VmError> {
        let s = self.surface(sampler)?;
        Ok(Value::IVec(vec![s.width() as i32, s.height() as i32]))
    }
    fn surface(&self, sampler: &Value) -> Result<&Surface, VmError> {
        let Value::Sampler(index) = sampler else {
            return Err(type_error("expected sampler2D"));
        };
        self.textures
            .get(*index)
            .ok_or_else(|| type_error(format!("invalid sampler handle {index}")))
    }
    fn derivative(&mut self, name: &str, value: &Value) -> Result<Value, VmError> {
        match self.derivative_mode {
            DerivativeMode::Off => zero_like(value),
            DerivativeMode::Record => {
                self.derivative_log.push((name.into(), value.clone()));
                self.derivative_index += 1;
                zero_like(value)
            }
            DerivativeMode::Replay => {
                let i = self.derivative_index;
                self.derivative_index += 1;
                let d = self
                    .derivative_diffs
                    .get(i)
                    .ok_or(VmError::DivergentDerivatives { lane: 4, call: i })?;
                Ok(match name {
                    "dFdx" => d.dx.clone(),
                    "dFdy" => d.dy.clone(),
                    _ => d.width.clone(),
                })
            }
        }
    }
}

fn reduced_turn_sine(value: &Value) -> Result<Value, VmError> {
    const TAU: f32 = std::f32::consts::TAU;
    const INV_TAU: f32 = 1.0 / std::f32::consts::TAU;
    let input = float_lanes(value)?;
    let output = input
        .iter()
        .map(|value| {
            let turns = *value * INV_TAU;
            let phase = turns - turns.floor();
            (f64::from(phase) * f64::from(TAU)).sin() as f32
        })
        .collect::<Vec<_>>();
    Ok(if matches!(value, Value::Float(_)) {
        Value::Float(output[0])
    } else {
        Value::Vec(output)
    })
}

fn unary_argument<'a>(name: &str, args: &'a [Value]) -> Result<&'a Value, VmError> {
    if args.len() == 1 {
        Ok(&args[0])
    } else {
        Err(VmError::Arity {
            name: name.into(),
            expected: "1".into(),
            actual: args.len(),
        })
    }
}
fn vector_spec(name: &str) -> Option<(&'static str, usize)> {
    for (prefix, base) in [
        ("bvec", "bool"),
        ("ivec", "int"),
        ("uvec", "uint"),
        ("vec", "float"),
    ] {
        if let Some(n) = name.strip_prefix(prefix).and_then(|v| v.parse().ok()) {
            return Some((base, n));
        }
    }
    None
}
fn flatten(args: &[Value]) -> Vec<Value> {
    let mut out = Vec::new();
    for arg in args {
        match arg {
            Value::Vec(v) => out.extend(v.iter().copied().map(Value::Float)),
            Value::IVec(v) => out.extend(v.iter().copied().map(Value::Int)),
            Value::UVec(v) => out.extend(v.iter().copied().map(Value::Uint)),
            Value::BVec(v) => out.extend(v.iter().copied().map(Value::Bool)),
            _ => out.push(arg.clone()),
        }
    }
    out
}
fn build_vector<'a>(base: &str, values: impl Iterator<Item = &'a Value>) -> Result<Value, VmError> {
    match base {
        "float" => values
            .map(to_f32)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Vec),
        "int" => values
            .map(to_i32)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::IVec),
        "uint" => values
            .map(to_u32)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::UVec),
        "bool" => values
            .map(to_bool)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::BVec),
        _ => Err(type_error("unknown vector base")),
    }
}
fn to_f32(v: &Value) -> Result<f32, VmError> {
    match v {
        Value::Float(x) => Ok(*x),
        Value::Int(x) => Ok(*x as f32),
        Value::Uint(x) => Ok(*x as f32),
        Value::Bool(x) => Ok(if *x { 1.0 } else { 0.0 }),
        _ => Err(type_error(format!("cannot convert {} to float", v.kind()))),
    }
}
fn to_i32(v: &Value) -> Result<i32, VmError> {
    match v {
        Value::Int(x) => Ok(*x),
        Value::Uint(x) => Ok(*x as i32),
        Value::Float(x) => Ok(*x as i32),
        Value::Bool(x) => Ok(i32::from(*x)),
        _ => Err(type_error(format!("cannot convert {} to int", v.kind()))),
    }
}
fn to_u32(v: &Value) -> Result<u32, VmError> {
    match v {
        Value::Uint(x) => Ok(*x),
        Value::Int(x) => Ok(*x as u32),
        Value::Float(x) => Ok(*x as u32),
        Value::Bool(x) => Ok(u32::from(*x)),
        _ => Err(type_error(format!("cannot convert {} to uint", v.kind()))),
    }
}
fn to_bool(v: &Value) -> Result<bool, VmError> {
    match v {
        Value::Bool(x) => Ok(*x),
        _ => Err(type_error(format!("cannot convert {} to bool", v.kind()))),
    }
}
fn float_binary(op: &str, a: f32, b: f32) -> Result<f32, VmError> {
    Ok(match op {
        "+" => a + b,
        "-" => a - b,
        "*" => a * b,
        "/" => a / b,
        "%" => a % b,
        _ => return Err(type_error(format!("unsupported float operator {op}"))),
    })
}
fn int_binary(op: &str, a: i32, b: i32) -> Result<i32, VmError> {
    Ok(match op {
        "+" => a.wrapping_add(b),
        "-" => a.wrapping_sub(b),
        "*" => a.wrapping_mul(b),
        "/" => a
            .checked_div(b)
            .unwrap_or(if b == 0 { 0 } else { i32::MIN }),
        "%" => {
            if b == 0 {
                0
            } else {
                a.wrapping_rem(b)
            }
        }
        "&" => a & b,
        "|" => a | b,
        "^" => a ^ b,
        "<<" => a.wrapping_shl(b as u32 & 31),
        ">>" => a.wrapping_shr(b as u32 & 31),
        _ => return Err(type_error(format!("unsupported int operator {op}"))),
    })
}
fn uint_binary(op: &str, a: u32, b: u32) -> Result<u32, VmError> {
    Ok(match op {
        "+" => a.wrapping_add(b),
        "-" => a.wrapping_sub(b),
        "*" => a.wrapping_mul(b),
        "/" => a.checked_div(b).unwrap_or(0),
        "%" => {
            if b == 0 {
                0
            } else {
                a % b
            }
        }
        "&" => a & b,
        "|" => a | b,
        "^" => a ^ b,
        "<<" => a.wrapping_shl(b & 31),
        ">>" => a.wrapping_shr(b & 31),
        _ => return Err(type_error(format!("unsupported uint operator {op}"))),
    })
}
fn map<T: Copy, U>(a: &[T], f: impl Fn(T) -> Result<U, VmError>) -> Result<Vec<U>, VmError> {
    a.iter().copied().map(f).collect()
}
fn zip_map<T: Copy, U: Copy, V>(
    a: &[T],
    b: &[U],
    f: impl Fn(T, U) -> Result<V, VmError>,
) -> Result<Vec<V>, VmError> {
    if a.len() != b.len() {
        return Err(type_error("vector width mismatch"));
    }
    a.iter()
        .copied()
        .zip(b.iter().copied())
        .map(|(x, y)| f(x, y))
        .collect()
}
fn logical(op: &str, a: &Value, b: &Value) -> Result<Value, VmError> {
    if op == "&&" || op == "||" {
        let (a, b) = (a.as_bool()?, b.as_bool()?);
        return Ok(Value::Bool(if op == "&&" { a && b } else { a || b }));
    }
    if op == "==" || op == "!=" {
        let equal = match (a, b) {
            (Value::Bool(left), Value::Bool(right)) => left == right,
            (Value::BVec(left), Value::BVec(right)) => left == right,
            (Value::Struct(left), Value::Struct(right)) => left == right,
            (Value::Array(left), Value::Array(right)) => left == right,
            _ => {
                let pairs = numeric_pairs(a, b)?;
                pairs.into_iter().all(|(left, right)| left == right)
            }
        };
        return Ok(Value::Bool(if op == "==" { equal } else { !equal }));
    }
    let pairs = numeric_pairs(a, b)?;
    let results: Vec<bool> = pairs
        .into_iter()
        .map(|(a, b)| match op {
            "==" => a == b,
            "!=" => a != b,
            "<" => a < b,
            ">" => a > b,
            "<=" => a <= b,
            ">=" => a >= b,
            _ => false,
        })
        .collect();
    Ok(Value::Bool(if op == "!=" {
        results.iter().any(|x| *x)
    } else {
        results.iter().all(|x| *x)
    }))
}
fn numeric_pairs(a: &Value, b: &Value) -> Result<Vec<(f64, f64)>, VmError> {
    let a = numeric_lanes(a)?;
    let b = numeric_lanes(b)?;
    if a.len() != b.len() {
        return Err(type_error("comparison width mismatch"));
    }
    Ok(a.into_iter().zip(b).collect())
}
fn numeric_lanes(v: &Value) -> Result<Vec<f64>, VmError> {
    match v {
        Value::Float(x) => Ok(vec![*x as f64]),
        Value::Int(x) => Ok(vec![*x as f64]),
        Value::Uint(x) => Ok(vec![*x as f64]),
        Value::Vec(x) => Ok(x.iter().map(|x| *x as f64).collect()),
        Value::IVec(x) => Ok(x.iter().map(|x| *x as f64).collect()),
        Value::UVec(x) => Ok(x.iter().map(|x| *x as f64).collect()),
        _ => Err(type_error("expected numeric value")),
    }
}
fn float_lanes(v: &Value) -> Result<Vec<f32>, VmError> {
    match v {
        Value::Float(x) => Ok(vec![*x]),
        Value::Vec(x) => Ok(x.clone()),
        _ => Err(type_error(format!(
            "expected float value, found {}",
            v.kind()
        ))),
    }
}
fn int_lanes(v: &Value) -> Result<Vec<i32>, VmError> {
    match v {
        Value::Int(x) => Ok(vec![*x]),
        Value::IVec(x) => Ok(x.clone()),
        _ => Err(type_error(format!(
            "expected int value, found {}",
            v.kind()
        ))),
    }
}
fn bool_lanes(v: &Value) -> Result<Vec<bool>, VmError> {
    match v {
        Value::Bool(x) => Ok(vec![*x]),
        Value::BVec(x) => Ok(x.clone()),
        _ => Err(type_error(format!(
            "expected bool value, found {}",
            v.kind()
        ))),
    }
}
fn shape(v: &Value) -> (&'static str, usize) {
    match v {
        Value::Vec(x) => ("float", x.len()),
        Value::IVec(x) => ("int", x.len()),
        Value::UVec(x) => ("uint", x.len()),
        Value::BVec(x) => ("bool", x.len()),
        _ => (v.kind(), 1),
    }
}
fn difference(a: &Value, b: &Value) -> Result<Value, VmError> {
    Runtime::new().binary("-", a, b)
}
fn add_abs(a: &Value, b: &Value) -> Result<Value, VmError> {
    let abs = |v: &Value| match v {
        Value::Float(x) => Ok(Value::Float(x.abs())),
        Value::Vec(x) => Ok(Value::Vec(x.iter().map(|x| x.abs()).collect())),
        _ => Err(type_error("derivatives require float values")),
    };
    Runtime::new().binary("+", &abs(a)?, &abs(b)?)
}
fn zero_like(v: &Value) -> Result<Value, VmError> {
    match v {
        Value::Float(_) => Ok(Value::Float(0.0)),
        Value::Vec(x) => Ok(Value::Vec(vec![0.0; x.len()])),
        _ => Err(type_error("derivatives require float scalar or vector")),
    }
}
fn matrix_product(a: &Value, b: &Value) -> Result<Option<Value>, VmError> {
    match (a, b) {
        (
            Value::Mat {
                dimension: n,
                columns: m,
            },
            Value::Vec(v),
        ) if v.len() == *n => {
            let mut o = vec![0.0; *n];
            for row in 0..*n {
                for col in 0..*n {
                    o[row] += m[col * n + row] * v[col];
                }
            }
            Ok(Some(Value::Vec(o)))
        }
        (
            Value::Vec(v),
            Value::Mat {
                dimension: n,
                columns: m,
            },
        ) if v.len() == *n => {
            let mut o = vec![0.0; *n];
            for col in 0..*n {
                for row in 0..*n {
                    o[col] += v[row] * m[col * n + row];
                }
            }
            Ok(Some(Value::Vec(o)))
        }
        (
            Value::Mat {
                dimension: n,
                columns: a,
            },
            Value::Mat {
                dimension: m,
                columns: b,
            },
        ) if n == m => {
            let mut o = vec![0.0; n * n];
            for c in 0..*n {
                for r in 0..*n {
                    for k in 0..*n {
                        o[c * n + r] += a[k * n + r] * b[c * n + k];
                    }
                }
            }
            Ok(Some(Value::Mat {
                dimension: *n,
                columns: o,
            }))
        }
        (Value::Mat { .. }, _) | (_, Value::Mat { .. }) => {
            Err(type_error("matrix dimensions mismatch"))
        }
        _ => Ok(None),
    }
}
fn dot(a: &Value, b: &Value) -> Result<f32, VmError> {
    let a = float_lanes(a)?;
    let b = float_lanes(b)?;
    if a.len() != b.len() {
        return Err(type_error("dot width mismatch"));
    }
    Ok(a.iter()
        .zip(b)
        .map(|(x, y)| f64::from(*x) * f64::from(y))
        .sum::<f64>() as f32)
}
fn length(v: &Value) -> Result<f32, VmError> {
    let v = float_lanes(v)?;
    Ok(v.iter()
        .map(|x| f64::from(*x) * f64::from(*x))
        .sum::<f64>()
        .sqrt() as f32)
}
fn normalize(v: &Value) -> Result<Value, VmError> {
    let lanes = float_lanes(v)?;
    let mag = length(v)?;
    if mag == 0.0 {
        return Ok(if matches!(v, Value::Float(_)) {
            Value::Float(0.0)
        } else {
            Value::Vec(vec![0.0; lanes.len()])
        });
    }
    let out: Vec<f32> = lanes.iter().map(|x| *x / mag).collect();
    Ok(if out.len() == 1 && matches!(v, Value::Float(_)) {
        Value::Float(out[0])
    } else {
        Value::Vec(out)
    })
}
fn cross(a: &Value, b: &Value) -> Result<Value, VmError> {
    let a = float_lanes(a)?;
    let b = float_lanes(b)?;
    if a.len() != 3 || b.len() != 3 {
        return Err(type_error("cross expects vec3"));
    }
    Ok(Value::Vec(vec![
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]))
}
fn reflect(i: &Value, n: &Value) -> Result<Value, VmError> {
    let scale = Value::Float(2.0 * dot(n, i)?);
    let r = Runtime::new();
    r.binary("-", i, &r.binary("*", n, &scale)?)
}
fn refract(i: &Value, n: &Value, eta: &Value) -> Result<Value, VmError> {
    let e = to_f32(eta)?;
    let d = dot(n, i)?;
    let k = 1.0 - e * e * (1.0 - d * d);
    if k < 0.0 {
        return zero_like(i);
    }
    let r = Runtime::new();
    r.binary(
        "-",
        &r.binary("*", i, &Value::Float(e))?,
        &r.binary("*", n, &Value::Float(e * d + k.sqrt()))?,
    )
}
fn relational(name: &str, a: &Value, b: &Value) -> Result<Value, VmError> {
    let pairs = numeric_pairs(a, b)?;
    let v = pairs
        .into_iter()
        .map(|(a, b)| match name {
            "lessThan" => a < b,
            "lessThanEqual" => a <= b,
            "greaterThan" => a > b,
            "greaterThanEqual" => a >= b,
            "equal" => a == b,
            "notEqual" => a != b,
            _ => false,
        })
        .collect();
    Ok(Value::BVec(v))
}
fn component_builtin(name: &str, args: &[Value]) -> Result<Value, VmError> {
    let widths: Vec<Vec<f32>> = args.iter().map(float_lanes).collect::<Result<_, _>>()?;
    let width = widths.iter().map(Vec::len).max().unwrap_or(1);
    if widths.iter().any(|v| v.len() != 1 && v.len() != width) {
        return Err(type_error(format!("{name} argument width mismatch")));
    }
    let mut out = Vec::with_capacity(width);
    for lane in 0..width {
        let get = |i: usize| widths[i][if widths[i].len() == 1 { 0 } else { lane }] as f64;
        let x = match name {
            "abs" => get(0).abs(),
            "acos" => get(0).acos(),
            "atan" => {
                if args.len() == 1 {
                    get(0).atan()
                } else {
                    get(0).atan2(get(1))
                }
            }
            "ceil" => get(0).ceil(),
            "clamp" => glsl_min(glsl_max(get(0), get(1)), get(2)),
            "cos" => get(0).cos(),
            "degrees" => get(0).to_degrees(),
            "exp" => get(0).exp(),
            "floor" => get(0).floor(),
            "fract" => get(0) - get(0).floor(),
            "inversesqrt" => 1.0 / get(0).sqrt(),
            "isnan" => return isnan_value(&args[0]),
            "log" => get(0).ln(),
            "log2" => get(0).log2(),
            "max" => glsl_max(get(0), get(1)),
            "min" => glsl_min(get(0), get(1)),
            "mix" => get(0) * (1.0 - get(2)) + get(1) * get(2),
            "mod" => get(0) - get(1) * (get(0) / get(1)).floor(),
            "pow" => get(0).powf(get(1)),
            "radians" => get(0).to_radians(),
            "round" => (get(0) + 0.5).floor(),
            "sign" => {
                let value = get(0);
                if value == 0.0 || value.is_nan() {
                    value
                } else if value < 0.0 {
                    -1.0
                } else {
                    1.0
                }
            }
            "sin" => get(0).sin(),
            "smoothstep" => {
                let t = ((get(2) - get(0)) / (get(1) - get(0))).clamp(0.0, 1.0);
                t * t * (3.0 - 2.0 * t)
            }
            "sqrt" => get(0).sqrt(),
            "step" => {
                if get(1) < get(0) {
                    0.0
                } else {
                    1.0
                }
            }
            "tanh" => get(0).tanh(),
            _ => return Err(VmError::UnknownBuiltin { name: name.into() }),
        };
        out.push(x as f32);
    }
    Ok(if width == 1 && matches!(args[0], Value::Float(_)) {
        Value::Float(out[0])
    } else {
        Value::Vec(out)
    })
}

fn glsl_min(left: f64, right: f64) -> f64 {
    if left.is_nan() || right.is_nan() {
        f64::NAN
    } else {
        left.min(right)
    }
}

fn glsl_max(left: f64, right: f64) -> f64 {
    if left.is_nan() || right.is_nan() {
        f64::NAN
    } else {
        left.max(right)
    }
}

fn integer_component_builtin(name: &str, args: &[Value]) -> Option<Result<Value, VmError>> {
    match (&args[0], name) {
        (Value::Int(value), "abs") => Some(Ok(Value::Int(value.wrapping_abs()))),
        (Value::IVec(values), "abs") => Some(Ok(Value::IVec(
            values.iter().map(|value| value.wrapping_abs()).collect(),
        ))),
        (Value::Int(_), "min" | "max" | "clamp") | (Value::IVec(_), "min" | "max" | "clamp") => {
            Some(integer_signed_component(name, args))
        }
        (Value::Uint(_), "min" | "max" | "clamp") | (Value::UVec(_), "min" | "max" | "clamp") => {
            Some(integer_unsigned_component(name, args))
        }
        _ => None,
    }
}

fn integer_signed_component(name: &str, args: &[Value]) -> Result<Value, VmError> {
    let lanes = args
        .iter()
        .map(signed_lanes)
        .collect::<Result<Vec<_>, _>>()?;
    let width = lanes.iter().map(Vec::len).max().unwrap_or(1);
    if lanes
        .iter()
        .any(|values| values.len() != 1 && values.len() != width)
    {
        return Err(type_error(format!("{name} argument width mismatch")));
    }
    let output = (0..width)
        .map(|lane| {
            let get = |index: usize| lanes[index][if lanes[index].len() == 1 { 0 } else { lane }];
            match name {
                "min" => get(0).min(get(1)),
                "max" => get(0).max(get(1)),
                "clamp" => get(0).max(get(1)).min(get(2)),
                _ => unreachable!(),
            }
        })
        .collect::<Vec<_>>();
    Ok(if width == 1 && matches!(args[0], Value::Int(_)) {
        Value::Int(output[0])
    } else {
        Value::IVec(output)
    })
}

fn integer_unsigned_component(name: &str, args: &[Value]) -> Result<Value, VmError> {
    let lanes = args
        .iter()
        .map(unsigned_lanes)
        .collect::<Result<Vec<_>, _>>()?;
    let width = lanes.iter().map(Vec::len).max().unwrap_or(1);
    if lanes
        .iter()
        .any(|values| values.len() != 1 && values.len() != width)
    {
        return Err(type_error(format!("{name} argument width mismatch")));
    }
    let output = (0..width)
        .map(|lane| {
            let get = |index: usize| lanes[index][if lanes[index].len() == 1 { 0 } else { lane }];
            match name {
                "min" => get(0).min(get(1)),
                "max" => get(0).max(get(1)),
                "clamp" => get(0).max(get(1)).min(get(2)),
                _ => unreachable!(),
            }
        })
        .collect::<Vec<_>>();
    Ok(if width == 1 && matches!(args[0], Value::Uint(_)) {
        Value::Uint(output[0])
    } else {
        Value::UVec(output)
    })
}

fn signed_lanes(value: &Value) -> Result<Vec<i32>, VmError> {
    match value {
        Value::Int(value) => Ok(vec![*value]),
        Value::IVec(values) => Ok(values.clone()),
        _ => Err(type_error(format!(
            "expected signed integer, found {}",
            value.kind()
        ))),
    }
}

fn unsigned_lanes(value: &Value) -> Result<Vec<u32>, VmError> {
    match value {
        Value::Uint(value) => Ok(vec![*value]),
        Value::UVec(values) => Ok(values.clone()),
        _ => Err(type_error(format!(
            "expected unsigned integer, found {}",
            value.kind()
        ))),
    }
}
fn isnan_value(v: &Value) -> Result<Value, VmError> {
    match v {
        Value::Float(x) => Ok(Value::Bool(x.is_nan())),
        Value::Vec(x) => Ok(Value::BVec(x.iter().map(|x| x.is_nan()).collect())),
        _ => Err(type_error("isnan expects float value")),
    }
}
fn bitcast_float_uint(v: &Value, to_uint: bool) -> Result<Value, VmError> {
    match (v, to_uint) {
        (Value::Float(x), true) => Ok(Value::Uint(x.to_bits())),
        (Value::Vec(x), true) => Ok(Value::UVec(x.iter().map(|x| x.to_bits()).collect())),
        (Value::Uint(x), false) => Ok(Value::Float(f32::from_bits(*x))),
        (Value::UVec(x), false) => Ok(Value::Vec(x.iter().map(|x| f32::from_bits(*x)).collect())),
        _ => Err(type_error("invalid bit cast")),
    }
}

// IEEE-754 binary16 conversion, round-to-nearest-even.
fn f32_to_f16(value: f32) -> u16 {
    let bits = value.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let exp = ((bits >> 23) & 0xff) as i32;
    let mant = bits & 0x7fffff;
    if exp == 255 {
        return sign | if mant == 0 { 0x7c00 } else { 0x7e00 };
    }
    let half_exp = exp - 127 + 15;
    if half_exp >= 31 {
        return sign | 0x7c00;
    }
    if half_exp <= 0 {
        if half_exp < -10 {
            return sign;
        }
        let m = mant | 0x800000;
        let shift = 14 - half_exp;
        let mut half = (m >> shift) as u16;
        let rem = m & ((1u32 << shift) - 1);
        let halfway = 1u32 << (shift - 1);
        if rem > halfway || (rem == halfway && (half & 1) != 0) {
            half = half.wrapping_add(1);
        }
        return sign | half;
    }
    let mut half = ((half_exp as u16) << 10) | (mant >> 13) as u16;
    let rem = mant & 0x1fff;
    if rem > 0x1000 || (rem == 0x1000 && (half & 1) != 0) {
        half = half.wrapping_add(1);
    }
    sign | half
}
fn f16_to_f32(value: u16) -> f32 {
    let sign = (u32::from(value & 0x8000)) << 16;
    let exp = (value >> 10) & 0x1f;
    let mant = value & 0x3ff;
    let bits = if exp == 0 {
        if mant == 0 {
            sign
        } else {
            let mut m = u32::from(mant);
            let mut e = -14i32;
            while m & 0x400 == 0 {
                m <<= 1;
                e -= 1;
            }
            m &= 0x3ff;
            sign | (((e + 127) as u32) << 23) | (m << 13)
        }
    } else if exp == 31 {
        sign | 0x7f800000 | (u32::from(mant) << 13)
    } else {
        sign | ((u32::from(exp) + 112) << 23) | (u32::from(mant) << 13)
    };
    f32::from_bits(bits)
}
