//! Data structures for performance testing

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConnectionParams {
    #[serde(rename = "SNOWFLAKE_TEST_ACCOUNT")]
    pub account: String,
    #[serde(rename = "SNOWFLAKE_TEST_HOST")]
    pub host: String,
    #[serde(rename = "SNOWFLAKE_TEST_USER")]
    pub user: String,
    #[serde(rename = "SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS")]
    pub private_key_contents: Vec<String>,
    #[serde(rename = "SNOWFLAKE_TEST_DATABASE")]
    pub database: String,
    #[serde(rename = "SNOWFLAKE_TEST_SCHEMA")]
    pub schema: String,
    #[serde(rename = "SNOWFLAKE_TEST_WAREHOUSE")]
    pub warehouse: String,
    #[serde(rename = "SNOWFLAKE_TEST_ROLE")]
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParametersJson {
    pub testconnection: TestConnectionParams,
}

#[derive(Debug, Clone, Serialize)]
pub struct IterationResult {
    pub query_time_s: f64,
    pub fetch_time_s: f64,
}
