//! Configuration parsing and environment variable handling

use crate::test_types::TestType;
use crate::types::ParametersJson;
use std::env;

type Result<T> = std::result::Result<T, String>;

/// Test configuration parsed from environment variables
pub struct TestConfig {
    pub sql_command: String,
    pub test_name: String,
    pub test_type: TestType,
    pub iterations: usize,
    pub warmup_iterations: usize,
    pub statement_async_override: Option<bool>,
    pub params: ParametersJson,
    pub setup_queries: Vec<String>,
}

impl TestConfig {
    /// Parse configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let sql_command = env::var("SQL_COMMAND")
            .map_err(|e| format!("SQL_COMMAND environment variable is required: {:?}", e))?;

        let test_name = env::var("TEST_NAME")
            .map_err(|e| format!("TEST_NAME environment variable is required: {:?}", e))?;

        let test_type_str = env::var("TEST_TYPE").unwrap_or_else(|_| "select".to_string());
        let test_type = test_type_str.parse::<TestType>().map_err(|e| {
            format!(
                "Invalid test type '{}'. Supported: select, put_get: {:?}",
                test_type_str, e
            )
        })?;

        let iterations: usize = env::var("PERF_ITERATIONS")
            .unwrap_or_else(|_| "1".to_string())
            .parse()
            .unwrap_or(1);

        let warmup_iterations: usize = env::var("PERF_WARMUP_ITERATIONS")
            .unwrap_or_else(|_| "0".to_string())
            .parse()
            .unwrap_or(0);

        let params_json = env::var("PARAMETERS_JSON")
            .map_err(|e| format!("PARAMETERS_JSON environment variable not set: {:?}", e))?;

        let params: ParametersJson = serde_json::from_str(&params_json)
            .map_err(|e| format!("Failed to parse PARAMETERS_JSON: {:?}", e))?;

        let setup_queries = if let Ok(setup_json) = env::var("SETUP_QUERIES") {
            serde_json::from_str::<Vec<String>>(&setup_json).unwrap_or_else(|_| Vec::new())
        } else {
            Vec::new()
        };

        let statement_async_override = match env::var("STATEMENT_ASYNC_EXECUTION") {
            Ok(value) => parse_async_override(&value)?,
            Err(_) => None,
        };

        Ok(Self {
            sql_command,
            test_name,
            test_type,
            iterations,
            warmup_iterations,
            statement_async_override,
            params,
            setup_queries,
        })
    }
}

/// Parse a boolean option from string value.
/// Returns `Some(true)` for truthy, `Some(false)` for falsy, `None` for auto/unset.
fn parse_bool_option(value: &str, field_name: &str) -> Result<Option<bool>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    match trimmed.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "y" => Ok(Some(true)),
        "false" | "0" | "no" | "n" => Ok(Some(false)),
        "auto" => Ok(None),
        other => Err(format!(
            "Invalid {} value: '{}' (expected true/false/yes/no/y/n/1/0/auto)",
            field_name, other
        )),
    }
}

fn parse_async_override(value: &str) -> Result<Option<bool>> {
    parse_bool_option(value, "STATEMENT_ASYNC_EXECUTION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bool_option_accepts_various_formats() {
        // Truthy values
        assert_eq!(parse_bool_option("true", "test").unwrap(), Some(true));
        assert_eq!(parse_bool_option("TRUE", "test").unwrap(), Some(true));
        assert_eq!(parse_bool_option("1", "test").unwrap(), Some(true));
        assert_eq!(parse_bool_option("yes", "test").unwrap(), Some(true));
        assert_eq!(parse_bool_option("y", "test").unwrap(), Some(true));
        assert_eq!(parse_bool_option("Y", "test").unwrap(), Some(true));

        // Falsy values
        assert_eq!(parse_bool_option("false", "test").unwrap(), Some(false));
        assert_eq!(parse_bool_option("FALSE", "test").unwrap(), Some(false));
        assert_eq!(parse_bool_option("0", "test").unwrap(), Some(false));
        assert_eq!(parse_bool_option("no", "test").unwrap(), Some(false));
        assert_eq!(parse_bool_option("n", "test").unwrap(), Some(false));
        assert_eq!(parse_bool_option("N", "test").unwrap(), Some(false));

        // Auto/none
        assert_eq!(parse_bool_option("auto", "test").unwrap(), None);
        assert_eq!(parse_bool_option("", "test").unwrap(), None);
        assert_eq!(parse_bool_option("  ", "test").unwrap(), None);

        // Invalid
        assert!(parse_bool_option("invalid", "test").is_err());
        assert!(parse_bool_option("maybe", "test").is_err());
    }

    #[test]
    fn parse_async_override_uses_bool_option() {
        assert_eq!(parse_async_override("true").unwrap(), Some(true));
        assert_eq!(parse_async_override("y").unwrap(), Some(true));
        assert_eq!(parse_async_override("false").unwrap(), Some(false));
        assert_eq!(parse_async_override("n").unwrap(), Some(false));
        assert_eq!(parse_async_override("auto").unwrap(), None);
    }
}
