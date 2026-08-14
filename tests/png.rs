use noisemaker_cpu::{PngError, Surface, decode_png, encode_png};
use png::{BitDepth, ColorType, Encoder};

const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
const PNG_MEMORY_LIMIT: usize = 96 * 1024 * 1024;

#[test]
fn rgba8_encode_decode_is_top_down_and_complete() {
    let pixels = [
        255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 64, 255, 255, 255, 0,
    ];
    let surface = Surface::from_rgba8(2, 2, &pixels).unwrap();

    let encoded = encode_png(&surface).unwrap();
    assert_eq!(&encoded[..8], PNG_SIGNATURE);
    assert_eq!(
        &encoded[encoded.len() - 12..encoded.len() - 8],
        &[0, 0, 0, 0]
    );
    assert_eq!(&encoded[encoded.len() - 8..encoded.len() - 4], b"IEND");

    let decoded = decode_png(&encoded).unwrap();
    assert_eq!(decoded.width(), 2);
    assert_eq!(decoded.height(), 2);
    assert_eq!(decoded.to_rgba8(), pixels);
}

#[test]
fn decode_normalizes_every_supported_8_bit_color_type_to_rgba() {
    let cases = [
        (
            fixture(ColorType::Grayscale, &[7], None, None),
            [7, 7, 7, 255],
        ),
        (
            fixture(ColorType::Rgb, &[1, 2, 3], None, None),
            [1, 2, 3, 255],
        ),
        (
            fixture(
                ColorType::Indexed,
                &[1],
                Some(&[10, 20, 30, 40, 50, 60]),
                Some(&[255, 7]),
            ),
            [40, 50, 60, 7],
        ),
        (
            fixture(ColorType::GrayscaleAlpha, &[9, 128], None, None),
            [9, 9, 9, 128],
        ),
        (
            fixture(ColorType::Rgba, &[11, 22, 33, 44], None, None),
            [11, 22, 33, 44],
        ),
    ];

    for (encoded, expected) in cases {
        assert_eq!(decode_png(&encoded).unwrap().to_rgba8(), expected);
    }
}

#[test]
fn decode_rejects_crc_failures() {
    let mut encoded = fixture(ColorType::Rgba, &[1, 2, 3, 4], None, None);
    encoded[29] ^= 1;

    let error = decode_png(&encoded).unwrap_err();
    assert!(error.to_string().contains("CRC"), "{error}");
}

#[test]
fn decode_rejects_invalid_chunk_order() {
    let valid = fixture(ColorType::Rgba, &[1, 2, 3, 4], None, None);
    let chunks = png_chunks(&valid);
    let invalid = [
        PNG_SIGNATURE.as_slice(),
        chunks[0],
        chunks[0],
        chunks[1],
        chunks[2],
    ]
    .concat();

    let error = decode_png(&invalid).unwrap_err();
    assert!(error.to_string().contains("IHDR"), "{error}");
}

#[test]
fn decode_rejects_interlacing_before_allocating_output() {
    let mut encoded = fixture(ColorType::Rgba, &[1, 2, 3, 4], None, None);
    encoded[28] = 1;
    refresh_ihdr_crc(&mut encoded);

    assert_eq!(
        decode_png(&encoded).unwrap_err().to_string(),
        "interlaced PNG images are not supported"
    );
}

#[test]
fn decode_rejects_dimensions_over_the_shared_pixel_cap() {
    let mut encoded = fixture(ColorType::Rgba, &[1, 2, 3, 4], None, None);
    encoded[16..20].copy_from_slice(&16_777_217_u32.to_be_bytes());
    refresh_ihdr_crc(&mut encoded);

    assert_eq!(
        decode_png(&encoded).unwrap_err().to_string(),
        "surface has 16777217 pixels; maximum is 16777216"
    );
}

#[test]
fn decode_rejects_decompressed_data_beyond_the_header_size() {
    let raw = vec![0; 512 * 512];
    let mut encoded = fixture_with_size(512, 512, ColorType::Grayscale, &raw, None, None);
    encoded[16..20].copy_from_slice(&1_u32.to_be_bytes());
    encoded[20..24].copy_from_slice(&1_u32.to_be_bytes());
    refresh_ihdr_crc(&mut encoded);

    let error = decode_png(&encoded).unwrap_err();
    assert!(error.to_string().contains("decoded data"), "{error}");
}

#[test]
fn decode_rejects_non_8_bit_sources() {
    let mut output = Vec::new();
    let mut encoder = Encoder::new(&mut output, 1, 1);
    encoder.set_color(ColorType::Grayscale);
    encoder.set_depth(BitDepth::Sixteen);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&[0, 7]).unwrap();
    writer.finish().unwrap();

    assert_eq!(
        decode_png(&output).unwrap_err().to_string(),
        "unsupported PNG bit depth 16; expected 8"
    );
}

#[test]
fn decode_rejects_input_over_the_png_memory_boundary() {
    let input = vec![0; PNG_MEMORY_LIMIT + 1];

    assert!(matches!(
        decode_png(&input).unwrap_err(),
        PngError::EncodedSizeExceeded {
            actual,
            maximum: PNG_MEMORY_LIMIT,
        } if actual == PNG_MEMORY_LIMIT + 1
    ));
}

#[test]
fn decode_rejects_oversized_ancillary_chunks_without_reading_the_payload() {
    let valid = fixture(ColorType::Rgba, &[1, 2, 3, 4], None, None);
    let chunks = png_chunks(&valid);
    let declared_length = (PNG_MEMORY_LIMIT + 1) as u32;
    let oversized = [
        PNG_SIGNATURE.as_slice(),
        chunks[0],
        &declared_length.to_be_bytes(),
        b"eXIf",
    ]
    .concat();

    let error = decode_png(&oversized).unwrap_err();
    assert!(
        matches!(
            error,
            PngError::AncillarySizeExceeded {
                ref chunk,
                actual,
                maximum: PNG_MEMORY_LIMIT,
            } if chunk == "eXIf" && actual == PNG_MEMORY_LIMIT + 1
        ),
        "{error:?}"
    );
}

fn fixture(
    color: ColorType,
    data: &[u8],
    palette: Option<&[u8]>,
    transparency: Option<&[u8]>,
) -> Vec<u8> {
    fixture_with_size(1, 1, color, data, palette, transparency)
}

fn fixture_with_size(
    width: u32,
    height: u32,
    color: ColorType,
    data: &[u8],
    palette: Option<&[u8]>,
    transparency: Option<&[u8]>,
) -> Vec<u8> {
    let mut output = Vec::new();
    let mut encoder = Encoder::new(&mut output, width, height);
    encoder.set_color(color);
    encoder.set_depth(BitDepth::Eight);
    if let Some(palette) = palette {
        encoder.set_palette(palette);
    }
    if let Some(transparency) = transparency {
        encoder.set_trns(transparency);
    }
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(data).unwrap();
    writer.finish().unwrap();
    output
}

fn png_chunks(png: &[u8]) -> Vec<&[u8]> {
    let mut chunks = Vec::new();
    let mut offset = PNG_SIGNATURE.len();
    while offset < png.len() {
        let length = u32::from_be_bytes(png[offset..offset + 4].try_into().unwrap()) as usize;
        let end = offset + length + 12;
        chunks.push(&png[offset..end]);
        offset = end;
    }
    chunks
}

fn refresh_ihdr_crc(png: &mut [u8]) {
    let crc = crc32(&png[12..29]);
    png[29..33].copy_from_slice(&crc.to_be_bytes());
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff_u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                0xedb8_8320 ^ (crc >> 1)
            } else {
                crc >> 1
            };
        }
    }
    crc ^ 0xffff_ffff
}
