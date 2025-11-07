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

        Ok(Self {
            sql_command,
            test_name,
            test_type,
            iterations,
            warmup_iterations,
            params,
            setup_queries,
        })
    }
}
