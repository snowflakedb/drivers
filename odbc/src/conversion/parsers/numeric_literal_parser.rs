use snafu::Snafu;
use std::fmt;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum NumericParsingError {
    #[snafu(display("Invalid character: {character}, expected: {expected}"))]
    InvalidCharacter {
        character: char,
        expected: &'static str,
    },
    #[snafu(display("Invalid exponent part: {exponent_part}"))]
    InvalidExponentPart { exponent_part: String },
}

#[derive(Eq, PartialEq, Clone)]
pub enum Sign {
    Positive,
    Negative,
}

impl fmt::Display for Sign {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Sign::Positive => write!(f, "+"),
            Sign::Negative => write!(f, "-"),
        }
    }
}

impl NumericLiteral {
    pub fn normalize(self) -> Result<NumericLiteral, NumericParsingError> {
        let mut normalized = self.clone();
        let exponent = if normalized.exponent_part.is_empty() {
            0
        } else {
            normalized.exponent_part.parse::<i16>().map_err(|_| {
                InvalidExponentPartSnafu {
                    exponent_part: normalized.exponent_part.clone(),
                }
                .build()
            })?
        };
        if normalized.exponent_sign == Sign::Negative {
            // Negative exponent: move digits from whole_part to fractional_part
            // e.g., 123e-2 -> 1.23 (move 2 digits from end of whole to start of fractional)
            for _ in 0..exponent {
                match normalized.whole_part.pop() {
                    Some(digit) => {
                        // Insert at the beginning of fractional_part
                        normalized.fractional_part.insert(0, digit);
                    }
                    None => {
                        // No more digits in whole, prepend '0' to fractional
                        normalized.fractional_part.insert(0, '0');
                    }
                }
            }
        } else {
            // Positive exponent: move digits from fractional_part to whole_part
            // e.g., 1.5e2 -> 150 (move fractional digits to whole, pad with zeros)
            let mut fractional_chars: Vec<char> = normalized.fractional_part.chars().collect();
            for _ in 0..exponent {
                if !fractional_chars.is_empty() {
                    // Take first char from fractional and append to whole
                    normalized.whole_part.push(fractional_chars.remove(0));
                } else {
                    // No more fractional digits, append '0'
                    normalized.whole_part.push('0');
                }
            }
            normalized.fractional_part = fractional_chars.into_iter().collect();
        }
        // Truncate leading zeros from whole_part
        normalized.whole_part = normalized.whole_part.trim_start_matches('0').to_string();
        if normalized.whole_part.is_empty() {
            normalized.whole_part = "0".to_string();
        }
        // Truncate trailing zeros from fractional_part
        normalized.fractional_part = normalized.fractional_part.trim_end_matches('0').to_string();
        if normalized.fractional_part.is_empty() {
            normalized.fractional_part = "0".to_string();
        }
        normalized.exponent_part = "0".to_string();
        normalized.exponent_sign = Sign::Positive;
        Ok(normalized)
    }

    pub fn whole_part_with_sign(&self) -> String {
        self.sign.to_string() + &self.whole_part
    }

    pub fn float_with_sign(&self) -> String {
        self.sign.to_string()
            + &self.whole_part
            + "."
            + &self.fractional_part
            + "e"
            + &self.exponent_sign.to_string()
            + &self.exponent_part
    }

    pub fn has_fractional_part(&self) -> bool {
        !self.fractional_part.is_empty() && self.fractional_part != "0"
    }
}

#[derive(Clone)]
pub struct NumericLiteral {
    pub sign: Sign,
    pub whole_part: String,
    pub fractional_part: String,
    pub exponent_sign: Sign,
    pub exponent_part: String,
}

enum ParseState {
    Initial,
    WholeDigits,
    FractionalDigits,
    ExponentSign,
    ExponentDigits,
}

pub fn parse_numeric_literal(value: &str) -> Result<NumericLiteral, NumericParsingError> {
    let value = value.trim();
    let mut numeric_literal = NumericLiteral {
        sign: Sign::Positive,
        whole_part: String::new(),
        fractional_part: String::new(),
        exponent_sign: Sign::Positive,
        exponent_part: String::new(),
    };
    let mut state = ParseState::Initial;
    let mut pointer = 0;
    let mut chars = value.chars().collect::<Vec<_>>();
    chars.push('\0');
    while pointer < chars.len() {
        let ch = chars[pointer];
        state = match state {
            ParseState::Initial => match ch {
                '-' => {
                    numeric_literal.sign = Sign::Negative;
                    pointer += 1;
                    ParseState::WholeDigits
                }
                '+' => {
                    numeric_literal.sign = Sign::Positive;
                    pointer += 1;
                    ParseState::WholeDigits
                }
                _ => ParseState::WholeDigits,
            },
            ParseState::WholeDigits => match ch {
                '0'..='9' => {
                    pointer += 1;
                    numeric_literal.whole_part.push(ch);
                    ParseState::WholeDigits
                }
                '.' => {
                    pointer += 1;
                    ParseState::FractionalDigits
                }
                'e' | 'E' => {
                    pointer += 1;
                    ParseState::ExponentSign
                }
                '\0' => {
                    break;
                }
                _ => {
                    return Err(NumericParsingError::InvalidCharacter {
                        character: ch,
                        expected: "0-9|.|e|E",
                    });
                }
            },
            ParseState::FractionalDigits => match ch {
                '0'..='9' => {
                    pointer += 1;
                    numeric_literal.fractional_part.push(ch);
                    ParseState::FractionalDigits
                }
                'e' | 'E' => {
                    pointer += 1;
                    ParseState::ExponentSign
                }
                '\0' => {
                    break;
                }
                _ => {
                    return Err(NumericParsingError::InvalidCharacter {
                        character: ch,
                        expected: "0-9|e|E",
                    });
                }
            },
            ParseState::ExponentSign => match ch {
                '+' => {
                    pointer += 1;
                    numeric_literal.exponent_sign = Sign::Positive;
                    ParseState::ExponentDigits
                }
                '-' => {
                    pointer += 1;
                    numeric_literal.exponent_sign = Sign::Negative;
                    ParseState::ExponentDigits
                }
                _ => ParseState::ExponentDigits,
            },
            ParseState::ExponentDigits => match ch {
                '0'..='9' => {
                    pointer += 1;
                    numeric_literal.exponent_part.push(ch);
                    ParseState::ExponentDigits
                }
                '\0' => {
                    break;
                }
                _ => {
                    return Err(NumericParsingError::InvalidCharacter {
                        character: ch,
                        expected: "0-9",
                    });
                }
            },
        };
    }
    Ok(numeric_literal)
}

// numeric-literal ::= signed-numeric-literal | unsigned-numeric-literal

// signed-numeric-literal ::= [sign] unsigned-numeric-literal

// unsigned-numeric-literal ::= exact-numeric-literal | approximate-numeric-literal

// exact-numeric-literal ::= unsigned-integer [period[unsigned-integer]] |period unsigned-integer

// sign ::= plus-sign | minus-sign

// approximate-numeric-literal ::= mantissa E exponent

// mantissa ::= exact-numeric-literal

// exponent ::= signed-integer

// signed-integer ::= [sign] unsigned-integer

// unsigned-integer ::= digit...

// plus-sign ::= +

// minus-sign ::= -

// digit ::= 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 0

// period ::= .

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to check sign equality
    fn is_positive(sign: &Sign) -> bool {
        matches!(sign, Sign::Positive)
    }

    fn is_negative(sign: &Sign) -> bool {
        matches!(sign, Sign::Negative)
    }

    // ==================== Unsigned Integer Tests ====================

    #[test]
    fn test_single_digit() {
        let result = parse_numeric_literal("5").unwrap();
        assert!(is_positive(&result.sign));
        assert_eq!(result.whole_part, "5");
        assert_eq!(result.fractional_part, "");
        assert_eq!(result.exponent_part, "");
    }

    #[test]
    fn test_multiple_digits() {
        let result = parse_numeric_literal("12345").unwrap();
        assert!(is_positive(&result.sign));
        assert_eq!(result.whole_part, "12345");
        assert_eq!(result.fractional_part, "");
        assert_eq!(result.exponent_part, "");
    }

    #[test]
    fn test_zero() {
        let result = parse_numeric_literal("0").unwrap();
        assert!(is_positive(&result.sign));
        assert_eq!(result.whole_part, "0");
        assert_eq!(result.fractional_part, "");
        assert_eq!(result.exponent_part, "");
    }

    #[test]
    fn test_leading_zeros() {
        let result = parse_numeric_literal("007").unwrap();
        assert!(is_positive(&result.sign));
        assert_eq!(result.whole_part, "007");
        assert_eq!(result.fractional_part, "");
        assert_eq!(result.exponent_part, "");
    }

    // ==================== Signed Integer Tests ====================

    #[test]
    fn test_positive_sign() {
        let result = parse_numeric_literal("+42").unwrap();
        assert!(is_positive(&result.sign));
        assert_eq!(result.whole_part, "42");
        assert_eq!(result.fractional_part, "");
        assert_eq!(result.exponent_part, "");
    }

    #[test]
    fn test_negative_sign() {
        let result = parse_numeric_literal("-42").unwrap();
        assert!(is_negative(&result.sign));
        assert_eq!(result.whole_part, "42");
        assert_eq!(result.fractional_part, "");
        assert_eq!(result.exponent_part, "");
    }

    #[test]
    fn test_negative_zero() {
        let result = parse_numeric_literal("-0").unwrap();
        assert!(is_negative(&result.sign));
        assert_eq!(result.whole_part, "0");
        assert_eq!(result.fractional_part, "");
        assert_eq!(result.exponent_part, "");
    }

    // ==================== Exact Numeric Literal (Decimal) Tests ====================

    #[test]
    fn test_integer_with_decimal_point() {
        let result = parse_numeric_literal("123.").unwrap();
        assert!(is_positive(&result.sign));
        assert_eq!(result.whole_part, "123");
        assert_eq!(result.fractional_part, "");
        assert_eq!(result.exponent_part, "");
    }

    #[test]
    fn test_decimal_with_fractional_part() {
        let result = parse_numeric_literal("123.456").unwrap();
        assert!(is_positive(&result.sign));
        assert_eq!(result.whole_part, "123");
        assert_eq!(result.fractional_part, "456");
        assert_eq!(result.exponent_part, "");
    }

    #[test]
    fn test_negative_decimal() {
        let result = parse_numeric_literal("-99.99").unwrap();
        assert!(is_negative(&result.sign));
        assert_eq!(result.whole_part, "99");
        assert_eq!(result.fractional_part, "99");
        assert_eq!(result.exponent_part, "");
    }

    #[test]
    fn test_zero_point_zero() {
        let result = parse_numeric_literal("0.0").unwrap();
        assert!(is_positive(&result.sign));
        assert_eq!(result.whole_part, "0");
        assert_eq!(result.fractional_part, "0");
        assert_eq!(result.exponent_part, "");
    }

    #[test]
    fn test_fractional_with_trailing_zeros() {
        let result = parse_numeric_literal("1.500").unwrap();
        assert!(is_positive(&result.sign));
        assert_eq!(result.whole_part, "1");
        assert_eq!(result.fractional_part, "500");
        assert_eq!(result.exponent_part, "");
    }

    // ==================== Approximate Numeric Literal (Scientific Notation) Tests ====================

    #[test]
    fn test_scientific_notation_lowercase_e() {
        let result = parse_numeric_literal("1e10").unwrap();
        assert!(is_positive(&result.sign));
        assert_eq!(result.whole_part, "1");
        assert_eq!(result.fractional_part, "");
        assert!(is_positive(&result.exponent_sign));
        assert_eq!(result.exponent_part, "10");
    }

    #[test]
    fn test_scientific_notation_uppercase_e() {
        let result = parse_numeric_literal("1E10").unwrap();
        assert!(is_positive(&result.sign));
        assert_eq!(result.whole_part, "1");
        assert_eq!(result.fractional_part, "");
        assert!(is_positive(&result.exponent_sign));
        assert_eq!(result.exponent_part, "10");
    }

    #[test]
    fn test_scientific_notation_positive_exponent() {
        let result = parse_numeric_literal("5e+3").unwrap();
        assert!(is_positive(&result.sign));
        assert_eq!(result.whole_part, "5");
        assert!(is_positive(&result.exponent_sign));
        assert_eq!(result.exponent_part, "3");
    }

    #[test]
    fn test_scientific_notation_negative_exponent() {
        let result = parse_numeric_literal("5e-3").unwrap();
        assert!(is_positive(&result.sign));
        assert_eq!(result.whole_part, "5");
        assert!(is_negative(&result.exponent_sign));
        assert_eq!(result.exponent_part, "3");
    }

    #[test]
    fn test_scientific_notation_with_decimal() {
        let result = parse_numeric_literal("1.5e10").unwrap();
        assert!(is_positive(&result.sign));
        assert_eq!(result.whole_part, "1");
        assert_eq!(result.fractional_part, "5");
        assert!(is_positive(&result.exponent_sign));
        assert_eq!(result.exponent_part, "10");
    }

    #[test]
    fn test_scientific_notation_negative_mantissa() {
        let result = parse_numeric_literal("-2.5e10").unwrap();
        assert!(is_negative(&result.sign));
        assert_eq!(result.whole_part, "2");
        assert_eq!(result.fractional_part, "5");
        assert!(is_positive(&result.exponent_sign));
        assert_eq!(result.exponent_part, "10");
    }

    #[test]
    fn test_scientific_notation_negative_both() {
        let result = parse_numeric_literal("-2.5e-10").unwrap();
        assert!(is_negative(&result.sign));
        assert_eq!(result.whole_part, "2");
        assert_eq!(result.fractional_part, "5");
        assert!(is_negative(&result.exponent_sign));
        assert_eq!(result.exponent_part, "10");
    }

    #[test]
    fn test_scientific_notation_large_exponent() {
        let result = parse_numeric_literal("1e308").unwrap();
        assert!(is_positive(&result.sign));
        assert_eq!(result.whole_part, "1");
        assert!(is_positive(&result.exponent_sign));
        assert_eq!(result.exponent_part, "308");
    }

    #[test]
    fn test_scientific_notation_with_trailing_decimal() {
        let result = parse_numeric_literal("123.e5").unwrap();
        assert!(is_positive(&result.sign));
        assert_eq!(result.whole_part, "123");
        assert_eq!(result.fractional_part, "");
        assert!(is_positive(&result.exponent_sign));
        assert_eq!(result.exponent_part, "5");
    }

    // ==================== Whitespace Handling Tests ====================

    #[test]
    fn test_leading_whitespace() {
        let result = parse_numeric_literal("  42").unwrap();
        assert!(is_positive(&result.sign));
        assert_eq!(result.whole_part, "42");
    }

    #[test]
    fn test_trailing_whitespace() {
        let result = parse_numeric_literal("42  ").unwrap();
        assert!(is_positive(&result.sign));
        assert_eq!(result.whole_part, "42");
    }

    #[test]
    fn test_surrounding_whitespace() {
        let result = parse_numeric_literal("  -123.456e+7  ").unwrap();
        assert!(is_negative(&result.sign));
        assert_eq!(result.whole_part, "123");
        assert_eq!(result.fractional_part, "456");
        assert!(is_positive(&result.exponent_sign));
        assert_eq!(result.exponent_part, "7");
    }

    #[test]
    fn test_tab_whitespace() {
        let result = parse_numeric_literal("\t42\t").unwrap();
        assert!(is_positive(&result.sign));
        assert_eq!(result.whole_part, "42");
    }

    // ==================== Error Cases ====================

    #[test]
    fn test_error_invalid_character_in_whole_part() {
        let result = parse_numeric_literal("12a34");
        assert!(result.is_err());
        match result {
            Err(NumericParsingError::InvalidCharacter { character, .. }) => {
                assert_eq!(character, 'a');
            }
            _ => panic!("Expected InvalidCharacter error"),
        }
    }

    #[test]
    fn test_error_invalid_character_in_fractional_part() {
        let result = parse_numeric_literal("12.3x4");
        assert!(result.is_err());
        match result {
            Err(NumericParsingError::InvalidCharacter { character, .. }) => {
                assert_eq!(character, 'x');
            }
            _ => panic!("Expected InvalidCharacter error"),
        }
    }

    #[test]
    fn test_error_invalid_character_in_exponent() {
        let result = parse_numeric_literal("1e1y2");
        assert!(result.is_err());
        match result {
            Err(NumericParsingError::InvalidCharacter { character, .. }) => {
                assert_eq!(character, 'y');
            }
            _ => panic!("Expected InvalidCharacter error"),
        }
    }

    #[test]
    fn test_error_space_in_middle() {
        let result = parse_numeric_literal("12 34");
        assert!(result.is_err());
    }

    #[test]
    fn test_error_multiple_decimal_points() {
        let result = parse_numeric_literal("12.34.56");
        assert!(result.is_err());
    }

    #[test]
    fn test_error_multiple_exponents() {
        let result = parse_numeric_literal("1e2e3");
        assert!(result.is_err());
    }

    #[test]
    fn test_error_letter_instead_of_digit() {
        let result = parse_numeric_literal("abc");
        assert!(result.is_err());
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_large_number() {
        let result = parse_numeric_literal("99999999999999999999").unwrap();
        assert!(is_positive(&result.sign));
        assert_eq!(result.whole_part, "99999999999999999999");
    }

    #[test]
    fn test_many_fractional_digits() {
        let result = parse_numeric_literal("1.12345678901234567890").unwrap();
        assert!(is_positive(&result.sign));
        assert_eq!(result.whole_part, "1");
        assert_eq!(result.fractional_part, "12345678901234567890");
    }

    #[test]
    fn test_all_zeros() {
        let result = parse_numeric_literal("000.000e000").unwrap();
        assert!(is_positive(&result.sign));
        assert_eq!(result.whole_part, "000");
        assert_eq!(result.fractional_part, "000");
        assert_eq!(result.exponent_part, "000");
    }

    // ==================== Normalize Tests ====================

    #[test]
    fn test_normalize_positive_exponent_moves_fractional_to_whole() {
        // 1.5e2 -> 150 (moves 5 to whole, adds one 0)
        let result = parse_numeric_literal("1.5e2").unwrap().normalize().unwrap();
        assert_eq!(result.whole_part, "150");
        assert_eq!(result.fractional_part, "");
    }

    #[test]
    fn test_normalize_positive_exponent_partial_fractional() {
        // 1.234e2 -> 123.4 (moves 23 to whole, leaves 4 in fractional)
        let result = parse_numeric_literal("1.234e2")
            .unwrap()
            .normalize()
            .unwrap();
        assert_eq!(result.whole_part, "123");
        assert_eq!(result.fractional_part, "4");
    }

    #[test]
    fn test_normalize_positive_exponent_no_fractional() {
        // 5e3 -> 5000 (adds three 0s to whole)
        let result = parse_numeric_literal("5e3").unwrap().normalize().unwrap();
        assert_eq!(result.whole_part, "5000");
        assert_eq!(result.fractional_part, "");
    }

    #[test]
    fn test_normalize_negative_exponent_moves_whole_to_fractional() {
        // 123e-2 -> 1.23 (moves 23 from whole to fractional)
        let result = parse_numeric_literal("123e-2")
            .unwrap()
            .normalize()
            .unwrap();
        assert_eq!(result.whole_part, "1");
        assert_eq!(result.fractional_part, "23");
    }

    #[test]
    fn test_normalize_negative_exponent_adds_zeros() {
        // 5e-3 -> 0.005 (whole becomes empty, fractional gets 005)
        let result = parse_numeric_literal("5e-3").unwrap().normalize().unwrap();
        assert_eq!(result.whole_part, "");
        assert_eq!(result.fractional_part, "005");
    }

    #[test]
    fn test_normalize_negative_exponent_partial() {
        // 12e-1 -> 1.2 (moves 2 from whole to fractional)
        let result = parse_numeric_literal("12e-1").unwrap().normalize().unwrap();
        assert_eq!(result.whole_part, "1");
        assert_eq!(result.fractional_part, "2");
    }

    #[test]
    fn test_normalize_zero_exponent() {
        // 123.45e0 -> 123.45 (no change)
        let result = parse_numeric_literal("0123.45e0")
            .unwrap()
            .normalize()
            .unwrap();
        assert_eq!(result.whole_part, "123");
        assert_eq!(result.fractional_part, "45");
    }

    #[test]
    fn test_normalize_preserves_sign() {
        // -1.5e2 -> -150
        let result = parse_numeric_literal("-01.5e2")
            .unwrap()
            .normalize()
            .unwrap();
        assert!(is_negative(&result.sign));
        assert_eq!(result.whole_part, "150");
        assert_eq!(result.fractional_part, "");
    }

    #[test]
    fn test_normalize_no_exponent() {
        // 123.45 with no exponent (empty exponent_part)
        let result = parse_numeric_literal("123.45")
            .unwrap()
            .normalize()
            .unwrap();
        assert_eq!(result.whole_part, "123");
        assert_eq!(result.fractional_part, "45");
    }

    #[test]
    fn test_normalize_large_positive_exponent() {
        // 1.2e5 -> 120000
        let result = parse_numeric_literal("1.2e5").unwrap().normalize().unwrap();
        assert_eq!(result.whole_part, "120000");
        assert_eq!(result.fractional_part, "");
    }

    #[test]
    fn test_normalize_large_negative_exponent() {
        // 1e-5 -> 0.00001
        let result = parse_numeric_literal("1e-5").unwrap().normalize().unwrap();
        assert_eq!(result.whole_part, "");
        assert_eq!(result.fractional_part, "00001");
    }
}
