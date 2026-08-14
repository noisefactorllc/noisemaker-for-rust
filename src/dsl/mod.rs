mod compile;
mod error;
mod parse;
mod tokenize;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Location {
    pub source_name: String,
    pub line: usize,
    pub column: usize,
    /// UTF-8 byte offset from the start of the DSL source.
    pub index: usize,
}

pub use compile::{CompiledChain, RenderPlan, RenderStep, SurfaceBinding, compile_dsl};
pub use error::DslError;
pub use parse::{
    DslArgument, DslBinding, DslCall, DslChain, DslProgram, DslValue, SurfaceRef, parse_dsl,
};
pub use tokenize::{Token, TokenKind, tokenize_dsl};
