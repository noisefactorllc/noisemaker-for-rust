// Expanded deviation bits from CPU authority commit 4aeb3e1, captured on Apple M4.
// The source capture's 2-bit table SHA-256 is checked below, with every mantissa.
const DEVIATIONS: &[u8; 1 << 20] = include_bytes!("metal-reciprocal.bin");

fn metal_reciprocal(value: f64) -> f32 {
    let rounded = value as f32;
    let word = rounded.to_bits();
    let exponent = (word >> 23) & 0xff;
    if exponent == 0 {
        return f32::INFINITY.copysign(rounded);
    }
    if exponent == 0xff {
        return (1.0 / value) as f32;
    }
    let exact = 1.0 / f64::from(rounded.abs());
    let mut reciprocal = exact as f32;
    let mantissa = (word & 0x7f_ffff) as usize;
    if (DEVIATIONS[mantissa >> 3] >> (mantissa & 7)) & 1 != 0 {
        let bits = reciprocal.to_bits();
        reciprocal = f32::from_bits(if f64::from(reciprocal) < exact {
            bits + 1
        } else {
            bits - 1
        });
    }
    if reciprocal < f32::MIN_POSITIVE {
        reciprocal = 0.0;
    }
    reciprocal.copysign(rounded)
}

fn flush(value: f64) -> f64 {
    let rounded = value as f32;
    f64::from(if rounded.is_subnormal() {
        0.0_f32.copysign(rounded)
    } else {
        rounded
    })
}

pub(crate) fn metal_divide(dividend: f64, divisor: f64) -> f64 {
    let product = flush(dividend) * f64::from(metal_reciprocal(divisor));
    if product != 0.0 && product.abs() < f64::from(f32::MIN_POSITIVE) {
        0.0_f64.copysign(product)
    } else {
        product
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    #[test]
    fn complete_reciprocal_capture_matches_its_measured_table() {
        let mut table = vec![0_u8; 1 << 21];
        let mut positive = 0;
        let mut negative = 0;
        for mantissa in 1..(1 << 23) {
            if (DEVIATIONS[mantissa >> 3] >> (mantissa & 7)) & 1 == 0 {
                continue;
            }
            let value = f32::from_bits(0x3f80_0000 | mantissa as u32);
            let exact = 1.0 / f64::from(value);
            let direction = if f64::from(exact as f32) < exact {
                positive += 1;
                1
            } else {
                negative += 1;
                2
            };
            table[mantissa >> 2] |= direction << ((mantissa & 3) * 2);
        }
        assert_eq!((positive, negative), (859_417, 8_757));
        assert_eq!(
            format!("{:x}", Sha256::digest(table)),
            "9e62e57b1413fcfbcf95d9809e463498a30a917fc6c8b351015bffd3e6855a79"
        );
    }

    #[test]
    fn division_preserves_raw_product_and_zero_sign() {
        let a = f64::from(1.3_f32);
        let product = metal_divide(a, 41.0);
        assert_eq!(product, a * f64::from(metal_reciprocal(41.0)));
        assert_ne!(product, f64::from(product as f32));
        for (a, b) in [
            (0.0, -41.0),
            (-0.0, 41.0),
            (1.0, f64::NEG_INFINITY),
            (-1.0, f64::INFINITY),
            (-f64::from(f32::MIN_POSITIVE), 100.0),
        ] {
            assert_eq!(metal_divide(a, b).to_bits(), (-0.0_f64).to_bits());
        }
    }

    #[test]
    fn measured_m4_division_pairs() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/cpu-sync/metal-division-m4.json"
        ))
        .unwrap();
        let mut count = 0;
        for group in ["runtime", "special"] {
            for pair in fixture[group].as_array().unwrap() {
                let bits = |i: usize| pair[i].as_u64().unwrap() as u32;
                let actual = metal_divide(
                    f64::from(f32::from_bits(bits(0))),
                    f64::from(f32::from_bits(bits(1))),
                ) as f32;
                let expected = f32::from_bits(bits(2));
                if expected.is_nan() {
                    assert!(actual.is_nan());
                } else {
                    assert_eq!(actual.to_bits(), expected.to_bits(), "{group}: {pair}");
                }
                count += 1;
            }
        }
        assert_eq!(count, 277);
    }
}
