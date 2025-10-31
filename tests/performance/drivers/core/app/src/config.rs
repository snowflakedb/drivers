//! Configuration parsing and environment variable handling

use crate::types::ParametersJson;
use anyhow::Result;
use std::env;

/// Test configuration parsed from environment variables
pub struct TestConfig {
    pub sql_command: String,
    pub test_name: String,
    pub iterations: usize,
    pub warmup_iterations: usize,
    pub params: ParametersJson,
}

impl TestConfig {
    /// Parse configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let sql_command = env::var("SQL_COMMAND")
            .map_err(|_| anyhow::anyhow!("SQL_COMMAND environment variable is required"))?;

        let test_name = env::var("TEST_NAME")
            .map_err(|_| anyhow::anyhow!("TEST_NAME environment variable is required"))?;

        let iterations: usize = env::var("PERF_ITERATIONS")
            .unwrap_or_else(|_| "1".to_string())
            .parse()
            .unwrap_or(1);

        let warmup_iterations: usize = env::var("PERF_WARMUP_ITERATIONS")
            .unwrap_or_else(|_| "0".to_string())
            .parse()
            .unwrap_or(0);

        let params_json = env::var("PARAMETERS_JSON")
            .map_err(|_| anyhow::anyhow!("PARAMETERS_JSON environment variable not set"))?;

        let params: ParametersJson = serde_json::from_str(&params_json)
            .map_err(|e| anyhow::anyhow!("Failed to parse PARAMETERS_JSON: {}", e))?;

        Ok(Self {
            sql_command,
            test_name,
            iterations,
            warmup_iterations,
            params,
        })
    }
}
