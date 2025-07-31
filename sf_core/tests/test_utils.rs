extern crate sf_core;
extern crate tracing;
extern crate tracing_subscriber;

use arrow::array::{Array, Int8Array};
use arrow::ffi_stream::ArrowArrayStreamReader;
use arrow::ffi_stream::FFI_ArrowArrayStream;
use flate2::read::GzDecoder;
use sf_core::api_client::new_database_driver_v1_client;
use std::fs;
use tracing::Level;
use tracing_subscriber::EnvFilter;

// Use serde to parse parameters.json
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct ParametersFile {
    pub testconnection: Parameters,
}

#[derive(Deserialize, Serialize)]
pub struct Parameters {
    #[serde(rename = "SNOWFLAKE_TEST_ACCOUNT")]
    pub account_name: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_USER")]
    pub user: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_PASSWORD")]
    pub password: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_DATABASE")]
    pub database: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_SCHEMA")]
    pub schema: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_WAREHOUSE")]
    pub warehouse: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_HOST")]
    pub host: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_ROLE")]
    pub role: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_SERVER_URL")]
    pub server_url: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_PORT")]
    pub port: Option<i64>,
    #[serde(rename = "SNOWFLAKE_TEST_PROTOCOL")]
    pub protocol: Option<String>,
}

/// Parses and returns the test parameters from the configured parameter file
pub fn get_parameters() -> Parameters {
    let parameter_path = std::env::var("PARAMETER_PATH").unwrap();
    println!("Parameter path: {parameter_path}");
    let parameters = fs::read_to_string(parameter_path).unwrap();
    let parameters: ParametersFile = serde_json::from_str(&parameters).unwrap();
    println!(
        "Parameters: {:?}",
        serde_json::to_string_pretty(&parameters).unwrap()
    );
    parameters.testconnection
}

/// Sets up logging for tests
pub fn setup_logging() {
    let env_filter = EnvFilter::builder()
        .with_default_directive(Level::DEBUG.into())
        .from_env()
        .unwrap();
    let _ = tracing_subscriber::fmt::fmt()
        .with_env_filter(env_filter)
        .try_init();
}

/// Creates a connected Snowflake client with database and connection initialized
pub struct SnowflakeTestClient {
    pub driver: Box<dyn sf_core::thrift_gen::database_driver_v1::TDatabaseDriverSyncClient + Send>,
    pub conn_handle: sf_core::thrift_gen::database_driver_v1::ConnectionHandle,
    pub db_handle: sf_core::thrift_gen::database_driver_v1::DatabaseHandle,
}

impl Default for SnowflakeTestClient {
    fn default() -> Self {
        Self::new()
    }
}

impl SnowflakeTestClient {
    /// Creates a new test client with Snowflake connection established
    pub fn new() -> Self {
        setup_logging();
        let parameters = get_parameters();
        let mut driver = new_database_driver_v1_client();
        let db_handle = driver.database_new().unwrap();
        driver.database_init(db_handle.clone()).unwrap();

        let conn_handle = driver.connection_new().unwrap();
        driver
            .connection_set_option_string(
                conn_handle.clone(),
                "account".to_string(),
                parameters.account_name.clone().unwrap(),
            )
            .unwrap();
        driver
            .connection_set_option_string(
                conn_handle.clone(),
                "user".to_string(),
                parameters.user.clone().unwrap(),
            )
            .unwrap();
        driver
            .connection_set_option_string(
                conn_handle.clone(),
                "password".to_string(),
                parameters.password.clone().unwrap(),
            )
            .unwrap();

        // Set optional parameters if specified
        if let Some(database) = parameters.database.clone() {
            driver
                .connection_set_option_string(conn_handle.clone(), "database".to_string(), database)
                .unwrap();
        }

        if let Some(schema) = parameters.schema.clone() {
            driver
                .connection_set_option_string(conn_handle.clone(), "schema".to_string(), schema)
                .unwrap();
        }

        if let Some(warehouse) = parameters.warehouse.clone() {
            driver
                .connection_set_option_string(
                    conn_handle.clone(),
                    "warehouse".to_string(),
                    warehouse,
                )
                .unwrap();
        }

        if let Some(host) = parameters.host.clone() {
            driver
                .connection_set_option_string(conn_handle.clone(), "host".to_string(), host)
                .unwrap();
        }

        if let Some(role) = parameters.role.clone() {
            driver
                .connection_set_option_string(conn_handle.clone(), "role".to_string(), role)
                .unwrap();
        }

        if let Some(server_url) = parameters.server_url.clone() {
            driver
                .connection_set_option_string(
                    conn_handle.clone(),
                    "server_url".to_string(),
                    server_url,
                )
                .unwrap();
        }

        if let Some(port) = parameters.port {
            driver
                .connection_set_option_int(conn_handle.clone(), "port".to_string(), port)
                .unwrap();
        }

        if let Some(protocol) = parameters.protocol.clone() {
            driver
                .connection_set_option_string(conn_handle.clone(), "protocol".to_string(), protocol)
                .unwrap();
        }

        driver
            .connection_init(conn_handle.clone(), db_handle.clone())
            .unwrap();

        Self {
            driver,
            conn_handle,
            db_handle,
        }
    }

    /// Creates a new statement handle
    pub fn new_statement(&mut self) -> sf_core::thrift_gen::database_driver_v1::StatementHandle {
        self.driver.statement_new(self.conn_handle.clone()).unwrap()
    }

    /// Executes a SQL query and returns the result
    pub fn execute_query(
        &mut self,
        sql: &str,
    ) -> sf_core::thrift_gen::database_driver_v1::ExecuteResult {
        let stmt_handle = self.new_statement();
        self.driver
            .statement_set_sql_query(stmt_handle.clone(), sql.to_string())
            .unwrap();
        self.driver
            .statement_execute_query(stmt_handle.clone())
            .unwrap()
    }

    /// Executes a SQL query and expects it to fail with a specific error message
    pub fn _execute_query_expect_error(&mut self, sql: &str, expected_error: &str) {
        let stmt_handle = self.new_statement();
        self.driver
            .statement_set_sql_query(stmt_handle.clone(), sql.to_string())
            .unwrap();

        let result = self.driver.statement_execute_query(stmt_handle.clone());
        match result {
            Err(err) => {
                let error_msg = format!("{err:?}");
                assert!(
                    error_msg.contains(expected_error),
                    "Expected error to contain '{expected_error}', got: {error_msg}"
                );
            }
            Ok(_) => {
                panic!("Expected query to fail with '{expected_error}' error, but it succeeded");
            }
        }
    }
}

impl Drop for SnowflakeTestClient {
    fn drop(&mut self) {
        // Release the connection when the client is dropped
        if let Err(e) = self.driver.connection_release(self.conn_handle.clone()) {
            tracing::warn!("Failed to release connection in Drop: {e:?}");
        }
        // Release the database handle
        if let Err(e) = self.driver.database_release(self.db_handle.clone()) {
            tracing::warn!("Failed to release database handle in Drop: {e:?}");
        }
    }
}

/// Helper for processing Arrow stream results
pub struct ArrowResultHelper {
    reader: ArrowArrayStreamReader,
}

impl ArrowResultHelper {
    /// Creates a new Arrow result helper from an ExecuteResult
    pub fn from_result(result: sf_core::thrift_gen::database_driver_v1::ExecuteResult) -> Self {
        let stream_ptr: *mut FFI_ArrowArrayStream = result.stream.into();
        let stream: FFI_ArrowArrayStream = unsafe { FFI_ArrowArrayStream::from_raw(stream_ptr) };
        let reader = ArrowArrayStreamReader::try_new(stream).unwrap();
        Self { reader }
    }

    /// Gets the next record batch
    pub fn next_batch(&mut self) -> Option<arrow::record_batch::RecordBatch> {
        match self.reader.next() {
            Some(Ok(batch)) => Some(batch),
            Some(Err(e)) => {
                tracing::error!("Error reading record batch: {e}");
                None
            }
            None => None,
        }
    }

    /// Gets the first record batch (convenience method)
    pub fn first_batch(&mut self) -> arrow::record_batch::RecordBatch {
        self.next_batch()
            .expect("Expected at least one record batch")
    }

    /// Extracts an integer value from the first column of the first row
    pub fn first_int_value(&mut self) -> i8 {
        let batch = self.first_batch();
        let array_ref = batch.column(0);
        let int_array = array_ref
            .as_any()
            .downcast_ref::<Int8Array>()
            .expect("Expected int8 array");
        int_array.value(0)
    }

    /// Validates that exactly one row is returned
    pub fn assert_single_row(&mut self) -> arrow::record_batch::RecordBatch {
        let batch = self
            .next_batch()
            .expect("Expected at least one record batch");
        assert_eq!(batch.num_rows(), 1, "Expected exactly one row");
        assert!(
            self.next_batch().is_none(),
            "Expected no more record batches"
        );
        batch
    }
}

/// Decompresses a gzipped file and returns its content as a string
pub fn decompress_gzipped_file<P: AsRef<std::path::Path>>(file_path: P) -> std::io::Result<String> {
    use std::io::Read;

    let gz_file = fs::File::open(file_path)?;
    let mut decoder = GzDecoder::new(gz_file);
    let mut decompressed_content = String::new();
    decoder.read_to_string(&mut decompressed_content)?;
    Ok(decompressed_content)
}
