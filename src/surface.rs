use crate::SurfaceError;

pub const MAX_SURFACE_PIXELS: u64 = 16_777_216;
const CHANNELS: usize = 4;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FilterMode {
    #[default]
    Nearest,
    Linear,
}

/// Top-down, linear RGBA float storage shared by the CPU renderer.
#[derive(Clone, Debug, PartialEq)]
pub struct Surface {
    width: u32,
    height: u32,
    data: Vec<f32>,
    filter_mode: FilterMode,
}

impl Surface {
    pub fn new(width: u32, height: u32) -> Result<Self, SurfaceError> {
        let lane_count = validate_surface_dimensions(width, height)?;
        Ok(Self {
            width,
            height,
            data: vec![0.0; lane_count],
            filter_mode: FilterMode::Nearest,
        })
    }

    pub fn from_f32(width: u32, height: u32, data: Vec<f32>) -> Result<Self, SurfaceError> {
        let expected = validate_surface_dimensions(width, height)?;
        if data.len() != expected {
            return Err(SurfaceError::LengthMismatch {
                kind: "float data",
                expected,
                actual: data.len(),
            });
        }
        Ok(Self {
            width,
            height,
            data,
            filter_mode: FilterMode::Nearest,
        })
    }

    pub fn from_rgba8(width: u32, height: u32, bytes: &[u8]) -> Result<Self, SurfaceError> {
        let expected = validate_surface_dimensions(width, height)?;
        if bytes.len() != expected {
            return Err(SurfaceError::LengthMismatch {
                kind: "RGBA8 data",
                expected,
                actual: bytes.len(),
            });
        }
        let scale = 1.0_f32 / 255.0;
        let data = bytes.iter().map(|&byte| f32::from(byte) * scale).collect();
        Ok(Self {
            width,
            height,
            data,
            filter_mode: FilterMode::Nearest,
        })
    }

    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    #[must_use]
    pub fn data(&self) -> &[f32] {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut [f32] {
        &mut self.data
    }

    #[must_use]
    pub const fn filter_mode(&self) -> FilterMode {
        self.filter_mode
    }

    pub fn set_filter_mode(&mut self, filter_mode: FilterMode) {
        self.filter_mode = filter_mode;
    }

    pub fn clear(&mut self, color: [f32; CHANNELS]) -> &mut Self {
        for pixel in self.data.chunks_exact_mut(CHANNELS) {
            pixel.copy_from_slice(&color);
        }
        self
    }

    #[must_use]
    pub fn to_rgba8(&self) -> Vec<u8> {
        self.data.iter().copied().map(float_to_byte).collect()
    }
}

pub(crate) fn validate_surface_dimensions(width: u32, height: u32) -> Result<usize, SurfaceError> {
    if width == 0 {
        return Err(SurfaceError::ZeroDimension { dimension: "width" });
    }
    if height == 0 {
        return Err(SurfaceError::ZeroDimension {
            dimension: "height",
        });
    }

    let pixels = u64::from(width)
        .checked_mul(u64::from(height))
        .ok_or(SurfaceError::DimensionOverflow)?;
    if pixels > MAX_SURFACE_PIXELS {
        return Err(SurfaceError::PixelLimitExceeded {
            pixels,
            maximum: MAX_SURFACE_PIXELS,
        });
    }

    usize::try_from(pixels)
        .ok()
        .and_then(|count| count.checked_mul(CHANNELS))
        .ok_or(SurfaceError::DimensionOverflow)
}

fn float_to_byte(value: f32) -> u8 {
    if !value.is_finite() || value <= 0.0 {
        0
    } else if value >= 1.0 {
        255
    } else {
        (value * 255.0).round() as u8
    }
}
