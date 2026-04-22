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
    #[serde(rename = "SNOWFLAKE_TEST_PRIVATE_KEY_FILE")]
    pub private_key_file: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS")]
    pub private_key_contents: Option<Vec<String>>,
    #[serde(rename = "SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD")]
    pub private_key_password: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_DATABASE")]
    pub database: String,
    #[serde(rename = "SNOWFLAKE_TEST_SCHEMA")]
    pub schema: String,
    #[serde(rename = "SNOWFLAKE_TEST_WAREHOUSE")]
    pub warehouse: String,
    #[serde(rename = "SNOWFLAKE_TEST_ROLE")]
    pub role: String,
    // Optional TLS settings for WireMock support
    #[serde(rename = "custom_root_store_path", default)]
    pub custom_root_store_path: Option<String>,
    #[serde(rename = "verify_certificates", default)]
    pub verify_certificates: Option<String>,
    #[serde(rename = "verify_hostname", default)]
    pub verify_hostname: Option<String>,
    #[serde(rename = "crl_check_mode", default)]
    pub crl_check_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParametersJson {
    pub testconnection: TestConnectionParams,
}

#[derive(Debug, Clone, Serialize)]
pub struct IterationResult {
    pub timestamp: i64,
    pub query_time_s: f64,
    pub fetch_time_s: f64,
    pub core_batch_wait_s: f64,
    pub core_chunk_download_s: f64,
    pub core_arrow_decode_s: f64,
    pub row_count: usize,
    pub cpu_time_s: f64,
    pub peak_rss_mb: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PutGetResult {
    pub timestamp: i64,
    pub query_time_s: f64,
    pub cpu_time_s: f64,
    pub peak_rss_mb: f64,
}
