use convert_case::{Case, Casing};

/// Convert string to snake_case
pub fn to_snake_case(s: &str) -> String {
    s.to_case(Case::Snake)
}

/// Convert string to PascalCase
pub fn to_pascal_case(s: &str) -> String {
    s.to_case(Case::Pascal)
}

/// Strip common test-method prefixes (`test_`, `vpn_`) so the bare name
/// can be compared against the Gherkin scenario name.
pub fn clean_method_name(name: &str) -> &str {
    name.trim_start_matches("test_").trim_start_matches("vpn_")
}

/// Normalize a string for matching: lowercase, strip whitespace, underscores,
/// hyphens, angle brackets, and parentheses.
///
/// Angle-bracket stripping lets Scenario Outline names like
/// `"should throw <error_code> in strict"` match a test method named
/// `should_throw_error_code_in_strict` (single parametrized method).
fn normalize_for_matching(s: &str) -> String {
    s.to_lowercase()
        .replace(' ', "")
        .replace('_', "")
        .replace('-', "")
        .replace('<', "")
        .replace('>', "")
        .replace('(', "")
        .replace(')', "")
}

/// Check if two strings match when normalized (ignoring case, spaces,
/// underscores, hyphens, angle brackets, and parentheses).
pub fn strings_match_normalized(s1: &str, s2: &str) -> bool {
    normalize_for_matching(s1) == normalize_for_matching(s2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_for_matching_removes_spaces_and_underscores() {
        let result = normalize_for_matching("should throw after exhausted retries");
        assert_eq!(result, "shouldthrowafterexhaustedretries");

        let result = normalize_for_matching("should_throw_after_exhausted_retries");
        assert_eq!(result, "shouldthrowafterexhaustedretries");
    }

    #[test]
    fn test_normalize_for_matching_removes_hyphens() {
        let result = normalize_for_matching("should-throw-error");
        assert_eq!(result, "shouldthrowerror");
    }

    #[test]
    fn test_normalize_for_matching_removes_angle_brackets() {
        let result = normalize_for_matching("should throw <error_code> in strict");
        assert_eq!(result, "shouldthrowerrorcodeinstrict");

        let result = normalize_for_matching("should throw <max_attempts> retries");
        assert_eq!(result, "shouldthrowmaxattemptsretries");
    }

    #[test]
    fn test_normalize_for_matching_lowercases() {
        let result = normalize_for_matching("Should Throw Error");
        assert_eq!(result, "shouldthrowerror");
    }

    #[test]
    fn test_strings_match_normalized_exact_match() {
        assert!(strings_match_normalized(
            "should throw error",
            "should throw error"
        ));
    }

    #[test]
    fn test_strings_match_normalized_different_separators() {
        assert!(strings_match_normalized(
            "should throw error",
            "should_throw_error"
        ));
        assert!(strings_match_normalized(
            "should-throw-error",
            "should_throw_error"
        ));
    }

    #[test]
    fn test_strings_match_normalized_with_placeholders() {
        assert!(strings_match_normalized(
            "should_throw_<error_code>_in_strict",
            "should_throw_error_code_in_strict"
        ));
        assert!(strings_match_normalized(
            "should throw <max_attempts> retries",
            "should_throw_max_attempts_retries"
        ));
    }

    #[test]
    fn test_strings_match_normalized_case_insensitive() {
        assert!(strings_match_normalized(
            "Should Throw Error",
            "should_throw_error"
        ));
    }

    #[test]
    fn test_clean_method_name_strips_test_prefix() {
        assert_eq!(
            clean_method_name("test_should_authenticate"),
            "should_authenticate"
        );
    }

    #[test]
    fn test_clean_method_name_strips_vpn_prefix() {
        assert_eq!(clean_method_name("vpn_should_connect"), "should_connect");
    }

    #[test]
    fn test_clean_method_name_no_prefix() {
        assert_eq!(clean_method_name("should_work"), "should_work");
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("Should Throw Error"), "should_throw_error");
        assert_eq!(to_snake_case("shouldThrowError"), "should_throw_error");
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("should throw error"), "ShouldThrowError");
        assert_eq!(to_pascal_case("should_throw_error"), "ShouldThrowError");
    }
}
