use noisemaker_cpu::{Surface, SurfaceError};

#[test]
fn surface_rejects_the_first_dimension_over_the_shared_pixel_cap() {
    let error = Surface::new(4097, 4096).unwrap_err();
    assert_eq!(
        error.to_string(),
        "surface has 16781312 pixels; maximum is 16777216"
    );
    assert!(matches!(
        error,
        SurfaceError::PixelLimitExceeded {
            pixels: 16_781_312,
            maximum: 16_777_216,
        }
    ));
}

#[test]
fn rgba8_conversion_is_top_down_clamped_rounded_and_finite() {
    let surface = Surface::from_f32(
        1,
        2,
        vec![1.2, -0.1, f32::NAN, 0.5, 0.0, 0.25, 0.75, f32::INFINITY],
    )
    .unwrap();
    assert_eq!(surface.to_rgba8(), vec![255, 0, 0, 128, 0, 64, 191, 0]);
}

#[test]
fn surface_rejects_zero_dimensions_with_stable_typed_errors() {
    assert_eq!(
        Surface::new(0, 1).unwrap_err(),
        SurfaceError::ZeroDimension { dimension: "width" }
    );
    assert_eq!(
        Surface::new(1, 0).unwrap_err(),
        SurfaceError::ZeroDimension {
            dimension: "height"
        }
    );
}

#[test]
fn surface_rejects_mismatched_float_and_byte_lengths() {
    assert_eq!(
        Surface::from_f32(1, 1, vec![0.0; 3]).unwrap_err(),
        SurfaceError::LengthMismatch {
            kind: "float data",
            expected: 4,
            actual: 3,
        }
    );
    assert_eq!(
        Surface::from_rgba8(1, 1, &[0; 3]).unwrap_err(),
        SurfaceError::LengthMismatch {
            kind: "RGBA8 data",
            expected: 4,
            actual: 3,
        }
    );
}

#[test]
fn rgba8_conversion_round_trips_and_surface_accessors_are_consistent() {
    let bytes = [0, 127, 128, 255, 255, 64, 32, 0];
    let mut surface = Surface::from_rgba8(2, 1, &bytes).unwrap();

    assert_eq!(surface.width(), 2);
    assert_eq!(surface.height(), 1);
    assert_eq!(surface.data().len(), 8);
    assert_eq!(surface.to_rgba8(), bytes);

    surface.data_mut()[0] = 0.5;
    assert_eq!(surface.to_rgba8()[0], 128);
}

#[test]
fn clone_and_clear_are_independent() {
    let original = Surface::from_rgba8(1, 1, &[255, 128, 0, 64]).unwrap();
    let mut copy = original.clone();
    copy.clear([0.25, 0.5, 0.75, 1.0]);

    assert_eq!(original.to_rgba8(), [255, 128, 0, 64]);
    assert_eq!(copy.to_rgba8(), [64, 128, 191, 255]);
}
