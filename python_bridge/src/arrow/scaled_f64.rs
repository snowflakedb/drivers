const MAX_EXACT_POW10: u32 = 22;
const MAX_EXACT_INT: u128 = 1 << 53;

const POW10: [f64; MAX_EXACT_POW10 as usize + 1] = [
    1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15, 1e16,
    1e17, 1e18, 1e19, 1e20, 1e21, 1e22,
];

/// `coefficient * 10^exponent` as the nearest `f64`, rounding once.
///
/// Below `MAX_EXACT_INT` and `MAX_EXACT_POW10` both operands are exact, so the
/// multiply or divide is the only rounding step. Beyond either bound the
/// decimal string goes to the standard library's correctly-rounded parser.
/// `coefficient as f64 * 10f64.powi(exponent)` is wrong for both ranges: it
/// rounds the power of ten and then rounds the product, which lands up to 4
/// ULP away.
pub(super) fn scaled_f64(coefficient: i128, exponent: i32) -> f64 {
    if coefficient == 0 {
        return 0.0;
    }
    let magnitude = exponent.unsigned_abs();
    if coefficient.unsigned_abs() <= MAX_EXACT_INT && magnitude <= MAX_EXACT_POW10 {
        let pow = POW10[magnitude as usize];
        let value = coefficient as f64;
        return if exponent < 0 {
            value / pow
        } else {
            value * pow
        };
    }
    parse_decimal_f64(coefficient, exponent)
}

#[inline(never)]
fn parse_decimal_f64(coefficient: i128, exponent: i32) -> f64 {
    format!("{coefficient}e{exponent}")
        .parse()
        .unwrap_or(f64::NAN)
}

#[cfg(test)]
mod tests {
    use super::{MAX_EXACT_INT, MAX_EXACT_POW10, scaled_f64};

    #[test]
    fn rounds_once_for_values_inside_the_exact_range() {
        assert_eq!(scaled_f64(3, -1), 0.3);
        assert_eq!(scaled_f64(7, -1), 0.7);
        assert_eq!(scaled_f64(-7, -1), -0.7);
        assert_eq!(scaled_f64(1234, -4), 0.1234);
        assert_eq!(scaled_f64(999_999_999, -4), 99999.9999);
        assert_eq!(scaled_f64(123, -2), 1.23);
        assert_eq!(scaled_f64(-5000, -2), -50.0);
    }

    #[test]
    fn coefficients_beyond_the_exact_integer_range_round_once() {
        assert_eq!(scaled_f64(9_007_199_254_740_995, -1), 900_719_925_474_099.5);
        assert_eq!(
            scaled_f64(18_014_398_509_481_983, -1),
            1_801_439_850_948_198.2
        );
        assert_eq!(scaled_f64(9_007_199_254_740_993, -2), 90_071_992_547_409.94);
        assert_eq!(
            scaled_f64(-9_007_199_254_740_993, -2),
            -90_071_992_547_409.94
        );
        assert_eq!(scaled_f64(9_007_199_254_740_994, -5), 90_071_992_547.409_94);
        assert_eq!(scaled_f64(9_007_199_254_740_993, -8), 90_071_992.547_409_94);
    }

    /// Expected values computed independently of this module, from exact
    /// decimal arithmetic widened to 200 significant digits.
    #[test]
    fn matches_exact_decimal_arithmetic_outside_the_fast_path() {
        const EXPECTED: &[((i128, i32), f64)] = &[
            ((99999999999999999999999999999999999999, -38), 1.0),
            ((99999999999999999999999999999999999999, -19), 1e19),
            ((99999999999999999999999999999999999999, -10), 1e28),
            ((99999999999999999999999999999999999999, 0), 1e38),
            ((99999999999999999999999999999999999999, 10), 1e48),
            ((1, -400), 0.0),
            ((1, -330), 0.0),
            ((1, -310), 1e-310),
            ((1, -300), 1e-300),
            ((1, -100), 1e-100),
            ((1, -30), 1e-30),
            ((1, 100), 1e100),
            ((1, 200), 1e200),
            ((1, 300), 1e300),
            ((1, 308), 1e308),
            ((1, 309), f64::INFINITY),
            ((3, -400), 0.0),
            ((3, -330), 0.0),
            ((3, -310), 3e-310),
            ((3, -300), 3e-300),
            ((3, -100), 3e-100),
            ((3, -30), 3e-30),
            ((3, 100), 3e100),
            ((3, 200), 3e200),
            ((3, 300), 3e300),
            ((3, 308), f64::INFINITY),
            ((10000000000000000000000000000000000000, -400), 0.0),
            ((10000000000000000000000000000000000000, -330), 1e-293),
            ((10000000000000000000000000000000000000, -310), 1e-273),
            ((10000000000000000000000000000000000000, -300), 1e-263),
            ((10000000000000000000000000000000000000, -100), 1e-63),
            ((10000000000000000000000000000000000000, -30), 10000000.0),
            ((10000000000000000000000000000000000000, 100), 1e137),
            ((10000000000000000000000000000000000000, 200), 1e237),
            ((10000000000000000000000000000000000000, 300), f64::INFINITY),
        ];
        for &((coefficient, exponent), expected) in EXPECTED {
            assert_eq!(
                scaled_f64(coefficient, exponent),
                expected,
                "coefficient={coefficient} exponent={exponent}"
            );
        }
    }

    /// The exact-operand branch and the decimal-parser branch are independent
    /// implementations; they must agree everywhere the fast path applies.
    #[test]
    fn the_two_branches_agree_across_their_overlap() {
        let coefficients = [
            1_i128,
            3,
            7,
            9,
            11,
            123,
            1234,
            999_999_999,
            -3,
            -7,
            -1234,
            -999_999_999,
            MAX_EXACT_INT as i128,
            MAX_EXACT_INT as i128 - 1,
            -(MAX_EXACT_INT as i128),
        ];
        for exponent in -(MAX_EXACT_POW10 as i32)..=MAX_EXACT_POW10 as i32 {
            for coefficient in coefficients {
                let via_parser: f64 = format!("{coefficient}e{exponent}")
                    .parse()
                    .unwrap_or(f64::NAN);
                assert_eq!(
                    scaled_f64(coefficient, exponent),
                    via_parser,
                    "coefficient={coefficient} exponent={exponent}"
                );
            }
        }
    }

    #[test]
    fn saturates_and_underflows_like_the_decimal_parser() {
        assert_eq!(scaled_f64(0, -5), 0.0);
        assert_eq!(scaled_f64(0, 16_384), 0.0);
        assert_eq!(scaled_f64(0, i16::MAX as i32), 0.0);
        assert_eq!(scaled_f64(1, 16_384), f64::INFINITY);
        assert_eq!(scaled_f64(-1, 16_384), f64::NEG_INFINITY);
        assert_eq!(scaled_f64(1, i16::MAX as i32), f64::INFINITY);
        assert_eq!(scaled_f64(1, -16_383), 0.0);
        assert_eq!(scaled_f64(1, i16::MIN as i32), 0.0);
    }
}
