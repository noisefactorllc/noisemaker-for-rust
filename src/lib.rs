//! Standalone CPU implementation of the Noisemaker rendering engine.

mod adapters;
pub mod catalog;
pub mod cli;
pub mod draw_ops;
pub mod dsl;
mod error;
mod generated;
pub mod iteration;
mod overlay;
mod param;
pub mod pass_runner;
mod png;
mod renderer;
mod runtime;
mod sampler;
mod surface;
mod texture_format;
mod value;
mod vm;

pub use adapters::{fragment_adapter_keys, historic_palette_dimensions, palette_dimensions};
pub use draw_ops::{draw_op_keys, execute_draw_pass, scatter_point_pixel};
pub use dsl::{
    DslError, DslValue, RenderPlan, RenderStep, SurfaceBinding, Token, TokenKind, compile_dsl,
    parse_dsl, tokenize_dsl,
};
pub use error::{PngError, SurfaceError, VmError};
pub use iteration::{
    ITERATION_DELTA_TIME, IterationError, IterationFrame, IterationGroup, IterationStep,
    compute_iteration_groups, is_particle_state_name, iteration_schedule, wrap01,
};
pub use overlay::render_overlay;
pub use param::ParamValue;
pub use png::{decode_png, encode_png};
pub use renderer::{
    CpuRenderer, CpuTextureCacheStats, OneShot, RenderError, RenderOptions, RenderResult,
    render_effect,
};
pub use runtime::{DerivativeDiff, DerivativeMode, Runtime};
pub use sampler::{sample_bilinear, sample_nearest};
pub use surface::{FilterMode, MAX_SURFACE_PIXELS, Surface};
pub use texture_format::{TextureFormat, quantize_texture};
pub use value::Value;
pub use vm::{PixelContext, ShaderOutputs, ShaderVm};
