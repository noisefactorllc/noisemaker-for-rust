use thiserror::Error;

/// Failures raised while validating or allocating a [`crate::Surface`].
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum SurfaceError {
    #[error("surface {dimension} must be greater than zero")]
    ZeroDimension { dimension: &'static str },

    #[error("surface dimensions overflow the addressable element count")]
    DimensionOverflow,

    #[error("surface has {pixels} pixels; maximum is {maximum}")]
    PixelLimitExceeded { pixels: u64, maximum: u64 },

    #[error("{kind} has length {actual}; expected {expected}")]
    LengthMismatch {
        kind: &'static str,
        expected: usize,
        actual: usize,
    },
}

/// Failures raised at the bounded PNG byte boundary.
#[derive(Debug, Error)]
pub enum PngError {
    #[error(transparent)]
    Surface(#[from] SurfaceError),

    #[error("PNG input has {actual} bytes; maximum is {maximum}")]
    EncodedSizeExceeded { actual: usize, maximum: usize },

    #[error("PNG ancillary chunk {chunk} has {actual} bytes; maximum is {maximum}")]
    AncillarySizeExceeded {
        chunk: String,
        actual: usize,
        maximum: usize,
    },

    #[error("unsupported PNG bit depth {0}; expected 8")]
    UnsupportedBitDepth(u8),

    #[error("interlaced PNG images are not supported")]
    Interlaced,

    #[error("unsupported decoded PNG color type {0:?}")]
    UnsupportedColorType(png::ColorType),

    #[error("PNG decoded data has {actual} bytes; maximum expected is {maximum}")]
    DecodedSizeExceeded { actual: usize, maximum: usize },

    #[error("invalid PNG: {0}")]
    Decode(String),

    #[error("PNG decoded data exceeds the expected image size or is invalid: {0}")]
    DecodedData(String),

    #[error("failed to encode PNG: {0}")]
    Encode(String),
}

/// Deterministic failures produced while evaluating validated shader IR.
#[derive(Clone, Debug, Error, PartialEq)]
pub enum VmError {
    #[error("type error: {message}")]
    Type { message: String },
    #[error("index {index} is outside {length} elements")]
    Bounds { index: usize, length: usize },
    #[error("unknown field {field:?}")]
    UnknownField { field: String },
    #[error("unknown builtin {name:?}")]
    UnknownBuiltin { name: String },
    #[error("{name} expects {expected} argument(s), found {actual}")]
    Arity {
        name: String,
        expected: String,
        actual: usize,
    },
    #[error("unknown function target {target:?}")]
    UnknownFunction { target: String },
    #[error("unknown variable {name:?}")]
    UnknownVariable { name: String },
    #[error("variable {name:?} is read-only")]
    ReadOnly { name: String },
    #[error("derivative call shape diverged at lane {lane}, call {call}")]
    DivergentDerivatives { lane: usize, call: usize },
    #[error("shader exceeded the {limit} statement execution limit")]
    StatementLimit { limit: u64 },
    #[error("shader exceeded the {limit} function call depth limit")]
    CallDepthLimit { limit: usize },
    #[error("invalid shader IR: {message}")]
    InvalidIr { message: String },
    #[error("fragment discarded")]
    Discarded,
}
