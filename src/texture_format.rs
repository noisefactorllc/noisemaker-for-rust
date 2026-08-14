use crate::Surface;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextureFormat {
    Rgba32f,
    #[default]
    Rgba16f,
    Rgba8,
}

/// Apply the storage conversion performed by a render attachment after a pass.
pub fn quantize_texture(surface: &mut Surface, format: TextureFormat) {
    match format {
        TextureFormat::Rgba32f => {}
        TextureFormat::Rgba16f => {
            for value in surface.data_mut() {
                *value = truncate_to_binary16(*value);
            }
        }
        TextureFormat::Rgba8 => {
            for value in surface.data_mut() {
                *value = quantize_unorm8(*value);
            }
        }
    }
}

fn quantize_unorm8(value: f32) -> f32 {
    if value <= 0.0 {
        0.0
    } else if value >= 1.0 {
        1.0
    } else {
        ((f64::from(value) * 255.0).round() / 255.0) as f32
    }
}

/// Match the reference WebGL RGBA16F conversion, which truncates discarded
/// mantissa bits rather than rounding to the nearest binary16 value.
pub(crate) fn truncate_to_binary16(value: f32) -> f32 {
    let bits = value.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let source_exponent = (bits >> 23) & 0xff;
    let fraction = bits & 0x7f_ffff;

    if source_exponent == 0xff {
        return if fraction == 0 {
            if sign == 0 {
                f32::INFINITY
            } else {
                f32::NEG_INFINITY
            }
        } else {
            f32::NAN
        };
    }

    let exponent = source_exponent as i32 - 127 + 15;
    let half_bits = if exponent >= 0x1f {
        sign | 0x7bff
    } else if exponent <= 0 {
        if exponent < -10 {
            sign
        } else {
            sign | ((((fraction | 0x80_0000) >> (1 - exponent)) >> 13) as u16)
        }
    } else {
        sign | ((exponent as u16) << 10) | ((fraction >> 13) as u16)
    };
    decode_binary16(half_bits)
}

fn decode_binary16(bits: u16) -> f32 {
    let sign = u32::from(bits & 0x8000) << 16;
    let exponent = (bits >> 10) & 0x1f;
    let mut fraction = u32::from(bits & 0x03ff);

    let decoded = match exponent {
        0 if fraction == 0 => sign,
        0 => {
            let mut unbiased_exponent = -14_i32;
            while fraction & 0x0400 == 0 {
                fraction <<= 1;
                unbiased_exponent -= 1;
            }
            fraction &= 0x03ff;
            sign | (((unbiased_exponent + 127) as u32) << 23) | (fraction << 13)
        }
        0x1f => sign | 0x7f80_0000 | (fraction << 13),
        _ => sign | ((u32::from(exponent) + 112) << 23) | (fraction << 13),
    };
    f32::from_bits(decoded)
}
