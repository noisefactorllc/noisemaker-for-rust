use std::collections::BTreeMap;

use crate::VmError;

/// A lossless dynamic representation of the value types admitted by the shader IR.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Void,
    Bool(bool),
    Int(i32),
    Uint(u32),
    Float(f32),
    BVec(Vec<bool>),
    IVec(Vec<i32>),
    UVec(Vec<u32>),
    Vec(Vec<f32>),
    /// Column-major square matrix.
    Mat {
        dimension: usize,
        columns: Vec<f32>,
    },
    Array(Vec<Value>),
    Struct(BTreeMap<String, Value>),
    /// Stable index into a [`crate::Runtime`] texture table.
    Sampler(usize),
}

impl Value {
    #[must_use]
    pub fn float_vector(values: impl IntoIterator<Item = f64>) -> Self {
        Self::Vec(values.into_iter().map(|value| value as f32).collect())
    }

    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Void => "void",
            Self::Bool(_) => "bool",
            Self::Int(_) => "int",
            Self::Uint(_) => "uint",
            Self::Float(_) => "float",
            Self::BVec(_) => "bvec",
            Self::IVec(_) => "ivec",
            Self::UVec(_) => "uvec",
            Self::Vec(_) => "vec",
            Self::Mat { .. } => "mat",
            Self::Array(_) => "array",
            Self::Struct(_) => "struct",
            Self::Sampler(_) => "sampler2D",
        }
    }

    pub fn float_lanes(&self) -> Result<&[f32], VmError> {
        match self {
            Self::Vec(values) => Ok(values),
            _ => Err(type_error(format!("{} is not a float vector", self.kind()))),
        }
    }

    pub fn read_swizzle(&self, swizzle: &str) -> Result<Self, VmError> {
        let indices = swizzle_indices(swizzle)?;
        match self {
            Self::Vec(values) => swizzle_read(values, &indices, Self::Float, Self::Vec),
            Self::IVec(values) => swizzle_read(values, &indices, Self::Int, Self::IVec),
            Self::UVec(values) => swizzle_read(values, &indices, Self::Uint, Self::UVec),
            Self::BVec(values) => swizzle_read(values, &indices, Self::Bool, Self::BVec),
            _ => Err(type_error(format!("cannot swizzle {}", self.kind()))),
        }
    }

    pub fn write_swizzle(&mut self, swizzle: &str, value: Self) -> Result<(), VmError> {
        let indices = swizzle_indices(swizzle)?;
        match (self, value) {
            (Self::Vec(base), Self::Float(value)) => swizzle_write(base, &indices, &[value]),
            (Self::Vec(base), Self::Vec(value)) => swizzle_write(base, &indices, &value),
            (Self::IVec(base), Self::Int(value)) => swizzle_write(base, &indices, &[value]),
            (Self::IVec(base), Self::IVec(value)) => swizzle_write(base, &indices, &value),
            (Self::UVec(base), Self::Uint(value)) => swizzle_write(base, &indices, &[value]),
            (Self::UVec(base), Self::UVec(value)) => swizzle_write(base, &indices, &value),
            (Self::BVec(base), Self::Bool(value)) => swizzle_write(base, &indices, &[value]),
            (Self::BVec(base), Self::BVec(value)) => swizzle_write(base, &indices, &value),
            (base, value) => Err(type_error(format!(
                "cannot assign {} to {} swizzle",
                value.kind(),
                base.kind()
            ))),
        }
    }

    pub fn index(&self, index: usize) -> Result<&Self, VmError> {
        match self {
            Self::Array(values) => values.get(index).ok_or(VmError::Bounds {
                index,
                length: values.len(),
            }),
            _ => Err(type_error(format!(
                "cannot index {} as an array",
                self.kind()
            ))),
        }
    }

    pub fn index_mut(&mut self, index: usize) -> Result<&mut Self, VmError> {
        match self {
            Self::Array(values) => {
                let length = values.len();
                values
                    .get_mut(index)
                    .ok_or(VmError::Bounds { index, length })
            }
            _ => Err(type_error(format!(
                "cannot index {} as an array",
                self.kind()
            ))),
        }
    }

    pub fn member(&self, field: &str) -> Result<&Self, VmError> {
        match self {
            Self::Struct(fields) => fields.get(field).ok_or_else(|| VmError::UnknownField {
                field: field.into(),
            }),
            _ => Err(type_error(format!(
                "cannot access a field of {}",
                self.kind()
            ))),
        }
    }

    pub fn member_mut(&mut self, field: &str) -> Result<&mut Self, VmError> {
        match self {
            Self::Struct(fields) => fields.get_mut(field).ok_or_else(|| VmError::UnknownField {
                field: field.into(),
            }),
            _ => Err(type_error(format!(
                "cannot access a field of {}",
                self.kind()
            ))),
        }
    }

    pub(crate) fn as_bool(&self) -> Result<bool, VmError> {
        match self {
            Self::Bool(value) => Ok(*value),
            _ => Err(type_error(format!("expected bool, found {}", self.kind()))),
        }
    }

    /// GLSL preprocessor feature flags are represented as integer uniforms in
    /// the locked IR, but remain valid scalar conditions (`if (FLAG)`).
    pub(crate) fn as_condition(&self) -> Result<bool, VmError> {
        match self {
            Self::Bool(value) => Ok(*value),
            Self::Int(value) => Ok(*value != 0),
            Self::Uint(value) => Ok(*value != 0),
            Self::Float(value) => Ok(*value != 0.0),
            _ => Err(type_error(format!(
                "expected scalar condition, found {}",
                self.kind()
            ))),
        }
    }

    pub(crate) fn as_index(&self) -> Result<usize, VmError> {
        match self {
            Self::Int(value) if *value >= 0 => Ok(*value as usize),
            Self::Uint(value) => Ok(*value as usize),
            _ => Err(type_error(format!(
                "expected non-negative integer index, found {}",
                self.kind()
            ))),
        }
    }
}

fn swizzle_indices(swizzle: &str) -> Result<Vec<usize>, VmError> {
    swizzle
        .chars()
        .map(|component| match component {
            'x' | 'r' | 's' => Ok(0),
            'y' | 'g' | 't' => Ok(1),
            'z' | 'b' | 'p' => Ok(2),
            'w' | 'a' | 'q' => Ok(3),
            _ => Err(type_error(format!(
                "invalid swizzle component {component:?}"
            ))),
        })
        .collect()
}

fn swizzle_read<T: Copy>(
    values: &[T],
    indices: &[usize],
    scalar: impl Fn(T) -> Value,
    vector: impl Fn(Vec<T>) -> Value,
) -> Result<Value, VmError> {
    for &index in indices {
        if index >= values.len() {
            return Err(VmError::Bounds {
                index,
                length: values.len(),
            });
        }
    }
    Ok(if indices.len() == 1 {
        scalar(values[indices[0]])
    } else {
        vector(indices.iter().map(|&i| values[i]).collect())
    })
}

fn swizzle_write<T: Copy>(base: &mut [T], indices: &[usize], values: &[T]) -> Result<(), VmError> {
    if values.len() != 1 && values.len() != indices.len() {
        return Err(type_error("swizzle assignment width mismatch"));
    }
    for &index in indices {
        if index >= base.len() {
            return Err(VmError::Bounds {
                index,
                length: base.len(),
            });
        }
    }
    let snapshot = values.to_vec();
    for (offset, &index) in indices.iter().enumerate() {
        base[index] = snapshot[if snapshot.len() == 1 { 0 } else { offset }];
    }
    Ok(())
}

pub(crate) fn type_error(message: impl Into<String>) -> VmError {
    VmError::Type {
        message: message.into(),
    }
}
