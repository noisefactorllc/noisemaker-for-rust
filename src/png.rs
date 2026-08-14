use std::io::Cursor;

use png::{
    BitDepth, ColorType, Decoded, Decoder, Encoder, Limits, StreamingDecoder, Transformations,
    UnfilterRegion,
};

use crate::Surface;
use crate::error::PngError;
use crate::surface::validate_surface_dimensions;

const MAX_PNG_DECODED_BYTES: usize = 96 * 1024 * 1024;
const MAX_PNG_ENCODED_BYTES: usize = MAX_PNG_DECODED_BYTES;
const PNG_SIGNATURE_LENGTH: usize = 8;

/// Decode a bounded, non-interlaced 8-bit PNG into top-down RGBA float storage.
pub fn decode_png(input: &[u8]) -> Result<Surface, PngError> {
    if input.len() > MAX_PNG_ENCODED_BYTES {
        return Err(PngError::EncodedSizeExceeded {
            actual: input.len(),
            maximum: MAX_PNG_ENCODED_BYTES,
        });
    }
    validate_chunk_bounds(input)?;

    let limits = Limits {
        bytes: MAX_PNG_DECODED_BYTES,
    };
    let mut decoder = Decoder::new_with_limits(Cursor::new(input), limits);
    decoder.set_transformations(Transformations::EXPAND);

    let (width, height, source_color) = {
        let info = decoder
            .read_header_info()
            .map_err(|error| PngError::Decode(error.to_string()))?;
        validate_surface_dimensions(info.width, info.height)?;
        if info.bit_depth != BitDepth::Eight {
            return Err(PngError::UnsupportedBitDepth(info.bit_depth as u8));
        }
        if info.interlaced {
            return Err(PngError::Interlaced);
        }
        (info.width, info.height, info.color_type)
    };
    validate_decompressed_stream(input, width, height, source_color)?;

    let mut reader = decoder
        .read_info()
        .map_err(|error| PngError::Decode(error.to_string()))?;
    let output_size = reader
        .output_buffer_size()
        .ok_or(PngError::DecodedSizeExceeded {
            actual: usize::MAX,
            maximum: MAX_PNG_DECODED_BYTES,
        })?;
    if output_size > MAX_PNG_DECODED_BYTES {
        return Err(PngError::DecodedSizeExceeded {
            actual: output_size,
            maximum: MAX_PNG_DECODED_BYTES,
        });
    }

    let mut decoded = vec![0; output_size];
    let output = reader
        .next_frame(&mut decoded)
        .map_err(|error| PngError::DecodedData(error.to_string()))?;
    let decoded = &decoded[..output.buffer_size()];
    let rgba = normalize_rgba8(decoded, output.color_type, width, height)?;
    reader
        .finish()
        .map_err(|error| PngError::Decode(error.to_string()))?;
    Surface::from_rgba8(width, height, &rgba).map_err(PngError::from)
}

fn validate_chunk_bounds(input: &[u8]) -> Result<(), PngError> {
    if input.len() < PNG_SIGNATURE_LENGTH {
        return Ok(());
    }
    let mut offset = PNG_SIGNATURE_LENGTH;
    while offset < input.len() {
        if input.len() - offset < 8 {
            return Err(PngError::Decode(
                "PNG contains a truncated chunk".to_owned(),
            ));
        }
        let chunk_length = u32::from_be_bytes(
            input[offset..offset + 4]
                .try_into()
                .expect("four-byte chunk length"),
        ) as usize;
        let chunk_type = &input[offset + 4..offset + 8];
        if chunk_type[0] & 0x20 != 0 && chunk_length > MAX_PNG_DECODED_BYTES {
            return Err(PngError::AncillarySizeExceeded {
                chunk: String::from_utf8_lossy(chunk_type).into_owned(),
                actual: chunk_length,
                maximum: MAX_PNG_DECODED_BYTES,
            });
        }
        offset = offset
            .checked_add(12)
            .and_then(|end| end.checked_add(chunk_length))
            .filter(|&end| end <= input.len())
            .ok_or_else(|| PngError::Decode("PNG contains a truncated chunk".to_owned()))?;
    }
    Ok(())
}

/// Encode one complete 8-bit RGBA PNG in memory. Bytes are returned only after
/// the encoder has successfully written the image and terminal IEND chunk.
pub fn encode_png(surface: &Surface) -> Result<Vec<u8>, PngError> {
    let rgba = surface.to_rgba8();
    let mut output = Vec::new();
    {
        let mut encoder = Encoder::new(&mut output, surface.width(), surface.height());
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|error| PngError::Encode(error.to_string()))?;
        writer
            .write_image_data(&rgba)
            .map_err(|error| PngError::Encode(error.to_string()))?;
        writer
            .finish()
            .map_err(|error| PngError::Encode(error.to_string()))?;
    }
    Ok(output)
}

fn normalize_rgba8(
    decoded: &[u8],
    color_type: ColorType,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, PngError> {
    let lane_count = validate_surface_dimensions(width, height)?;
    let mut rgba = vec![0; lane_count];

    match color_type {
        ColorType::Grayscale => {
            for (gray, pixel) in decoded.iter().copied().zip(rgba.chunks_exact_mut(4)) {
                pixel.copy_from_slice(&[gray, gray, gray, 255]);
            }
        }
        ColorType::Rgb => {
            for (source, pixel) in decoded.chunks_exact(3).zip(rgba.chunks_exact_mut(4)) {
                pixel.copy_from_slice(&[source[0], source[1], source[2], 255]);
            }
        }
        ColorType::GrayscaleAlpha => {
            for (source, pixel) in decoded.chunks_exact(2).zip(rgba.chunks_exact_mut(4)) {
                pixel.copy_from_slice(&[source[0], source[0], source[0], source[1]]);
            }
        }
        ColorType::Rgba => rgba.copy_from_slice(decoded),
        ColorType::Indexed => return Err(PngError::UnsupportedColorType(color_type)),
    }
    Ok(rgba)
}

fn validate_decompressed_stream(
    input: &[u8],
    width: u32,
    height: u32,
    color_type: ColorType,
) -> Result<(), PngError> {
    let components = match color_type {
        ColorType::Grayscale | ColorType::Indexed => 1_usize,
        ColorType::Rgb => 3,
        ColorType::GrayscaleAlpha => 2,
        ColorType::Rgba => 4,
    };
    let row_bytes = usize::try_from(width)
        .ok()
        .and_then(|width| width.checked_mul(components))
        .and_then(|bytes| bytes.checked_add(1))
        .ok_or(PngError::DecodedSizeExceeded {
            actual: usize::MAX,
            maximum: MAX_PNG_DECODED_BYTES,
        })?;
    let expected = row_bytes
        .checked_mul(height as usize)
        .ok_or(PngError::DecodedSizeExceeded {
            actual: usize::MAX,
            maximum: MAX_PNG_DECODED_BYTES,
        })?;
    if expected > MAX_PNG_DECODED_BYTES {
        return Err(PngError::DecodedSizeExceeded {
            actual: expected,
            maximum: MAX_PNG_DECODED_BYTES,
        });
    }

    // The high-level decoder stops inflating after the declared rows are
    // satisfied. Drive png's low-level decoder into one extra byte of bounded
    // storage so an otherwise-valid zlib stream cannot hide a decompression
    // bomb behind a tiny IHDR.
    let buffer_len = expected
        .checked_add(1)
        .ok_or(PngError::DecodedSizeExceeded {
            actual: usize::MAX,
            maximum: expected,
        })?;
    let mut decoded = vec![0; buffer_len];
    let mut region = UnfilterRegion::default();
    let mut stream = StreamingDecoder::new();
    stream.set_ignore_adler32(false);
    let mut offset = 0;
    let mut no_progress_events = 0;

    drive_stream_piece(
        &mut stream,
        &input[..PNG_SIGNATURE_LENGTH],
        &mut decoded,
        &mut region,
        expected,
        &mut no_progress_events,
    )?;
    offset += PNG_SIGNATURE_LENGTH;

    while offset < input.len() {
        if input.len() - offset < 12 {
            return Err(PngError::Decode(
                "PNG contains a truncated chunk".to_owned(),
            ));
        }
        let chunk_length = u32::from_be_bytes(
            input[offset..offset + 4]
                .try_into()
                .expect("four-byte chunk length"),
        ) as usize;
        let chunk_type = &input[offset + 4..offset + 8];
        if chunk_type[0] & 0x20 != 0 && chunk_length > MAX_PNG_DECODED_BYTES {
            return Err(PngError::AncillarySizeExceeded {
                chunk: String::from_utf8_lossy(chunk_type).into_owned(),
                actual: chunk_length,
                maximum: MAX_PNG_DECODED_BYTES,
            });
        }
        let chunk_end = offset
            .checked_add(12)
            .and_then(|end| end.checked_add(chunk_length))
            .ok_or_else(|| PngError::Decode("PNG chunk length overflows input size".to_owned()))?;
        if chunk_end > input.len() {
            return Err(PngError::Decode(
                "PNG contains a truncated chunk".to_owned(),
            ));
        }

        // Feed only the structural and compressed-image chunks to the low-level
        // stream. Metadata is validated later by the bounded high-level decoder,
        // so this exact-size inflation check cannot allocate attacker-sized
        // ancillary payloads.
        if matches!(chunk_type, b"IHDR" | b"IDAT" | b"IEND")
            || (chunk_type == b"PLTE" && chunk_length <= 768)
        {
            drive_stream_piece(
                &mut stream,
                &input[offset..chunk_end],
                &mut decoded,
                &mut region,
                expected,
                &mut no_progress_events,
            )?;
        }
        offset = chunk_end;
    }

    if region.filled != expected {
        return Err(PngError::DecodedData(format!(
            "decompressed {actual} bytes; expected {expected}",
            actual = region.filled
        )));
    }
    Ok(())
}

fn drive_stream_piece(
    stream: &mut StreamingDecoder,
    mut input: &[u8],
    decoded: &mut Vec<u8>,
    region: &mut UnfilterRegion,
    expected: usize,
    no_progress_events: &mut usize,
) -> Result<(), PngError> {
    while !input.is_empty() {
        let (consumed, event) = stream
            .update(input, Some(&mut region.as_buf(decoded)))
            .map_err(|error| PngError::Decode(error.to_string()))?;
        if region.filled > expected {
            return Err(PngError::DecodedSizeExceeded {
                actual: region.filled,
                maximum: expected,
            });
        }
        if consumed == 0 {
            *no_progress_events += 1;
            if *no_progress_events > 16 || matches!(event, Decoded::Nothing) {
                return Err(PngError::DecodedData(
                    "decoder stopped before consuming the complete PNG stream".to_owned(),
                ));
            }
        } else {
            input = &input[consumed..];
            *no_progress_events = 0;
        }
    }
    Ok(())
}
