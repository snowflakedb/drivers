//! Small, dependency-free helpers shared across `sf_core`.

pub mod sync;

pub(crate) fn env_flag(name: &str) -> bool {
    std::env::var(name).is_ok_and(|value| value.eq_ignore_ascii_case("true") || value == "1")
}

pub(crate) fn parse_bool_token(value: &str) -> Option<bool> {
    match value.to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

pub(crate) fn parse_setting_bool_token(value: &str) -> Option<bool> {
    match value.to_lowercase().as_str() {
        "true" | "1" | "on" => Some(true),
        "false" | "0" | "off" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_flag_accepts_only_true_and_1() {
        for value in ["true", "TRUE", "True", "1"] {
            temp_env::with_var("SF_TEST_ENV_FLAG", Some(value), || {
                assert!(env_flag("SF_TEST_ENV_FLAG"), "{value}");
            });
        }
        for value in ["yes", "on", "false", "0", "no", "off", " true ", ""] {
            temp_env::with_var("SF_TEST_ENV_FLAG", Some(value), || {
                assert!(!env_flag("SF_TEST_ENV_FLAG"), "{value}");
            });
        }
        temp_env::with_var_unset("SF_TEST_ENV_FLAG", || {
            assert!(!env_flag("SF_TEST_ENV_FLAG"));
        });
    }

    #[test]
    fn parse_bool_token_accepts_yes_and_on() {
        for value in ["true", "TRUE", "1", "yes", "Yes", "on", "ON"] {
            assert_eq!(parse_bool_token(value), Some(true), "{value}");
        }
        for value in ["false", "FALSE", "0", "no", "No", "off", "OFF"] {
            assert_eq!(parse_bool_token(value), Some(false), "{value}");
        }
        for value in ["", " true ", "trueish", "y", "n", "auto"] {
            assert_eq!(parse_bool_token(value), None, "{value}");
        }
    }

    #[test]
    fn parse_setting_bool_token_rejects_yes_and_no() {
        for value in ["true", "TRUE", "1", "on", "ON"] {
            assert_eq!(parse_setting_bool_token(value), Some(true), "{value}");
        }
        for value in ["false", "FALSE", "0", "off", "OFF"] {
            assert_eq!(parse_setting_bool_token(value), Some(false), "{value}");
        }
        for value in ["yes", "no", "", " true ", "trueish"] {
            assert_eq!(parse_setting_bool_token(value), None, "{value}");
        }
    }
}
