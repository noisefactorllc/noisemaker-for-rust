use noisemaker_cpu::{
    FilterMode, Surface, TextureFormat, quantize_texture, sample_bilinear, sample_nearest,
};

const TOP_LEFT: [f32; 4] = [0.0, 0.25, 0.5, 1.0];
const TOP_RIGHT: [f32; 4] = [0.25, 0.5, 1.0, 0.0];
const BOTTOM_LEFT: [f32; 4] = [0.5, 1.0, 0.0, 0.25];
const BOTTOM_RIGHT: [f32; 4] = [1.0, 0.0, 0.25, 0.5];

fn corners() -> Surface {
    Surface::from_f32(
        2,
        2,
        [TOP_LEFT, TOP_RIGHT, BOTTOM_LEFT, BOTTOM_RIGHT].concat(),
    )
    .unwrap()
}

#[test]
fn nearest_reads_top_down_storage_with_glsl_bottom_left_coordinates() {
    let surface = corners();

    assert_eq!(sample_nearest(&surface, 0.25, 0.25), BOTTOM_LEFT);
    assert_eq!(sample_nearest(&surface, 0.75, 0.25), BOTTOM_RIGHT);
    assert_eq!(sample_nearest(&surface, 0.25, 0.75), TOP_LEFT);
    assert_eq!(sample_nearest(&surface, 0.75, 0.75), TOP_RIGHT);
}

#[test]
fn nearest_clamps_out_of_range_uvs_without_wrapping() {
    let surface = corners();

    assert_eq!(sample_nearest(&surface, -100.0, -100.0), BOTTOM_LEFT);
    assert_eq!(sample_nearest(&surface, 100.0, -100.0), BOTTOM_RIGHT);
    assert_eq!(sample_nearest(&surface, -100.0, 100.0), TOP_LEFT);
    assert_eq!(sample_nearest(&surface, 100.0, 100.0), TOP_RIGHT);
}

#[test]
fn bilinear_uses_bottom_left_texel_centers_and_averages_the_center() {
    let surface = corners();

    assert_eq!(sample_bilinear(&surface, 0.25, 0.25), BOTTOM_LEFT);
    assert_eq!(sample_bilinear(&surface, 0.75, 0.25), BOTTOM_RIGHT);
    assert_eq!(sample_bilinear(&surface, 0.25, 0.75), TOP_LEFT);
    assert_eq!(sample_bilinear(&surface, 0.75, 0.75), TOP_RIGHT);
    assert_eq!(
        sample_bilinear(&surface, 0.5, 0.5),
        [0.4375, 0.4375, 0.4375, 0.4375]
    );
}

#[test]
fn bilinear_clamps_to_edge_texels() {
    let surface = corners();

    assert_eq!(sample_bilinear(&surface, -10.0, -10.0), BOTTOM_LEFT);
    assert_eq!(sample_bilinear(&surface, 10.0, 10.0), TOP_RIGHT);
}

#[test]
fn samplers_map_non_finite_uvs_to_stable_clamped_edges() {
    let surface = corners();

    assert_eq!(sample_nearest(&surface, f32::NAN, f32::NAN), BOTTOM_LEFT);
    assert_eq!(
        sample_nearest(&surface, f32::INFINITY, f32::INFINITY),
        TOP_RIGHT
    );
    assert_eq!(
        sample_nearest(&surface, f32::NEG_INFINITY, f32::NEG_INFINITY),
        BOTTOM_LEFT
    );
    assert_eq!(sample_bilinear(&surface, f32::NAN, f32::NAN), BOTTOM_LEFT);
    assert_eq!(
        sample_bilinear(&surface, f32::INFINITY, f32::INFINITY),
        TOP_RIGHT
    );
    assert_eq!(
        sample_bilinear(&surface, f32::NEG_INFINITY, f32::NEG_INFINITY),
        BOTTOM_LEFT
    );
}

#[test]
fn surfaces_default_to_nearest_filtering_and_can_request_linear_filtering() {
    let mut surface = corners();
    assert_eq!(surface.filter_mode(), FilterMode::Nearest);
    surface.set_filter_mode(FilterMode::Linear);
    assert_eq!(surface.filter_mode(), FilterMode::Linear);
}

#[test]
#[allow(clippy::excessive_precision)] // Preserve the sibling's exact binary16 fixture literals.
fn rgba16f_quantization_matches_reference_webgl_attachment_truncation() {
    let mut surface = Surface::from_f32(1, 1, vec![0.1, 0.3333, 1.5, -0.25]).unwrap();
    quantize_texture(&mut surface, TextureFormat::Rgba16f);
    assert_eq!(
        surface.data(),
        [0.0999755859375, 0.333251953125, 1.5, -0.25]
    );

    let mut signed = Surface::from_f32(1, 1, vec![-0.1, 65_504.0, 70_000.0, 0.0]).unwrap();
    quantize_texture(&mut signed, TextureFormat::Rgba16f);
    assert_eq!(signed.data(), [-0.0999755859375, 65_504.0, 65_504.0, 0.0]);
}

#[test]
fn rgba16f_quantization_preserves_special_values_and_truncates_subnormals() {
    let mut special =
        Surface::from_f32(1, 1, vec![f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.0]).unwrap();
    quantize_texture(&mut special, TextureFormat::Rgba16f);
    assert!(special.data()[0].is_nan());
    assert_eq!(special.data()[1], f32::INFINITY);
    assert_eq!(special.data()[2], f32::NEG_INFINITY);
    assert_eq!(special.data()[3].to_bits(), (-0.0_f32).to_bits());

    let half_subnormal = 2.0_f32.powi(-20);
    let below_half_subnormal = 2.0_f32.powi(-25);
    let mut subnormal = Surface::from_f32(
        1,
        1,
        vec![
            half_subnormal,
            -half_subnormal,
            below_half_subnormal,
            -below_half_subnormal,
        ],
    )
    .unwrap();
    quantize_texture(&mut subnormal, TextureFormat::Rgba16f);
    assert_eq!(subnormal.data()[0], half_subnormal);
    assert_eq!(subnormal.data()[1], -half_subnormal);
    assert_eq!(subnormal.data()[2].to_bits(), 0.0_f32.to_bits());
    assert_eq!(subnormal.data()[3].to_bits(), (-0.0_f32).to_bits());
}

#[test]
fn rgba8_quantization_clamps_rounds_and_round_trips_as_bytes() {
    let mut surface = Surface::from_f32(1, 1, vec![0.1, 0.5, 2.0, -1.0]).unwrap();
    quantize_texture(&mut surface, TextureFormat::Rgba8);
    assert_eq!(surface.to_rgba8(), [26, 128, 255, 0]);
}

#[test]
fn rgba32f_quantization_is_an_explicit_no_op() {
    let mut surface = Surface::from_f32(1, 1, vec![0.00001, 0.3333, 1.5, -0.25]).unwrap();
    let before = surface.clone();
    quantize_texture(&mut surface, TextureFormat::Rgba32f);
    assert_eq!(surface, before);
}
