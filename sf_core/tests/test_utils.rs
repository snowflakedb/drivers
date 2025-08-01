extern crate sf_core;
extern crate tracing;
extern crate tracing_subscriber;

use arrow::array::{Array, Float64Array, Int8Array, Int64Array, StringArray};
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

    pub fn create_temporary_stage(&mut self, stage_name: &str) {
        self.execute_query(&format!("create temporary stage {stage_name}"));
    }

    pub fn _put_file_with_options(
        &mut self,
        file_path: &std::path::Path,
        stage_name: &str,
        options: &str,
    ) {
        let put_sql = format!(
            "PUT 'file://{}' @{stage_name} {options}",
            file_path.to_str().unwrap().replace("\\", "/")
        );
        self.execute_query(&put_sql);
    }

    pub fn _get_file(&mut self, stage_file_path: &str, download_dir: &std::path::Path) {
        let get_sql = format!(
            "GET @{stage_file_path} file://{}/",
            download_dir.to_str().unwrap().replace("\\", "/")
        );
        self.execute_query(&get_sql);
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

    /// Converts all result data to a 2D array of strings for easy comparison
    pub fn transform_into_string_array(&mut self) -> Vec<Vec<String>> {
        let mut all_rows = Vec::new();

        while let Some(batch) = self.next_batch() {
            for row_idx in 0..batch.num_rows() {
                let mut row = Vec::new();
                for col_idx in 0..batch.num_columns() {
                    let column = batch.column(col_idx);
                    let value_str = extract_string_value(column, row_idx);
                    row.push(value_str);
                }
                all_rows.push(row);
            }
        }

        all_rows
    }

    /// Asserts that the result equals the expected 2D array
    pub fn assert_equals_array(&mut self, expected: Vec<Vec<&str>>) {
        let actual = self.transform_into_string_array();
        let expected_strings: Vec<Vec<String>> = expected
            .iter()
            .map(|row| row.iter().map(|s| s.to_string()).collect())
            .collect();

        assert_eq!(
            actual, expected_strings,
            "Arrow result does not match expected array"
        );
    }

    /// Convenience method for single row assertions
    pub fn assert_equals_single_row(&mut self, expected: Vec<&str>) {
        self.assert_equals_array(vec![expected]);
    }

    /// Convenience method for single value assertions
    pub fn assert_equals_single_value(&mut self, expected: &str) {
        self.assert_equals_array(vec![vec![expected]]);
    }
}

/// Extracts a string representation of a value from an Arrow array at the given index
fn extract_string_value(column: &dyn Array, row_idx: usize) -> String {
    use arrow::datatypes::DataType;

    if column.is_null(row_idx) {
        return "NULL".to_string();
    }

    match column.data_type() {
        DataType::Utf8 => {
            let string_array = column
                .as_any()
                .downcast_ref::<StringArray>()
                .expect("Expected string array");
            string_array.value(row_idx).to_string()
        }
        DataType::Int8 => {
            let int_array = column
                .as_any()
                .downcast_ref::<Int8Array>()
                .expect("Expected int8 array");
            int_array.value(row_idx).to_string()
        }
        DataType::Int64 => {
            let int_array = column
                .as_any()
                .downcast_ref::<Int64Array>()
                .expect("Expected int64 array");
            int_array.value(row_idx).to_string()
        }
        DataType::Float64 => {
            let float_array = column
                .as_any()
                .downcast_ref::<Float64Array>()
                .expect("Expected float64 array");
            float_array.value(row_idx).to_string()
        }
        _ => format!("UNSUPPORTED_TYPE({:?})", column.data_type()),
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

pub fn create_test_file(
    temp_dir: &std::path::Path,
    file_name: &str,
    content: &str,
) -> std::path::PathBuf {
    let file_path = temp_dir.join(file_name);
    fs::write(&file_path, content).unwrap();
    file_path
}
