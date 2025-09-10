extern crate sf_core;
extern crate tracing;
extern crate tracing_subscriber;

use arrow::array::{Array, ArrowPrimitiveType, PrimitiveArray, StructArray};
use flate2::read::GzDecoder;
use sf_core::thrift_apis::DatabaseDriverV1;
use sf_core::thrift_apis::client::create_client;
use sf_core::thrift_gen::database_driver_v1::{ArrowArrayPtr, ArrowSchemaPtr, ExecuteResult};
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
    #[serde(rename = "SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS")]
    pub private_key_contents: Option<Vec<String>>,
    #[serde(rename = "SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD")]
    pub private_key_password: Option<String>,
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
        .with_default_directive(Level::INFO.into())
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
    pub parameters: Parameters,
}

impl Default for SnowflakeTestClient {
    fn default() -> Self {
        Self::connect_with_default_auth()
    }
}

impl SnowflakeTestClient {
    pub fn with_default_params() -> Self {
        setup_logging();
        let parameters = get_parameters();
        let mut driver = create_client::<DatabaseDriverV1>();
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

        Self {
            driver,
            conn_handle,
            db_handle,
            parameters,
        }
    }
    /// Creates a new test client with Snowflake connection established
    pub fn connect_with_default_auth() -> Self {
        setup_logging();
        let mut client = Self::with_default_params();

        client
            .driver
            .connection_set_option_string(
                client.conn_handle.clone(),
                "password".to_string(),
                client.parameters.password.clone().unwrap(),
            )
            .unwrap();

        client
            .driver
            .connection_init(client.conn_handle.clone(), client.db_handle.clone())
            .unwrap();

        client
    }

    /// Creates a new statement handle
    pub fn new_statement(&mut self) -> sf_core::thrift_gen::database_driver_v1::StatementHandle {
        self.driver.statement_new(self.conn_handle.clone()).unwrap()
    }

    /// Executes a SQL query and returns the result
    pub fn execute_query(&mut self, sql: &str) -> ExecuteResult {
        let stmt_handle = self.new_statement();
        self.driver
            .statement_set_sql_query(stmt_handle.clone(), sql.to_string())
            .unwrap();
        self.driver
            .statement_execute_query(stmt_handle.clone())
            .unwrap()
    }

    pub fn execute_query_no_unwrap(&mut self, sql: &str) -> thrift::Result<ExecuteResult> {
        let stmt_handle = self.new_statement();
        self.driver
            .statement_set_sql_query(stmt_handle.clone(), sql.to_string())?;
        self.driver.statement_execute_query(stmt_handle.clone())
    }

    pub fn create_temporary_stage(&mut self, stage_name: &str) {
        self.execute_query(&format!("create temporary stage {stage_name}"));
    }

    pub fn connect(&mut self) -> thrift::Result<()> {
        self.driver
            .connection_init(self.conn_handle.clone(), self.db_handle.clone())
    }

    pub fn verify_simple_query(&mut self) {
        let _result = self.execute_query("SELECT 1");
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
    filename: &str,
    content: &str,
) -> std::path::PathBuf {
    let file_path = temp_dir.join(filename);
    fs::write(&file_path, content).unwrap();
    file_path
}

pub fn create_param_bindings<T: ArrowPrimitiveType>(
    params: &[T::Native],
) -> (ArrowSchemaPtr, ArrowArrayPtr)
where
    PrimitiveArray<T>: From<Vec<T::Native>>,
{
    use arrow::array::{ArrayRef, PrimitiveArray};
    use arrow::datatypes::{Field, Schema};
    use arrow::ffi::{FFI_ArrowArray, FFI_ArrowSchema};
    use sf_core::thrift_gen::database_driver_v1::{ArrowArrayPtr, ArrowSchemaPtr};
    use std::sync::Arc;

    let schema_fields = params
        .iter()
        .enumerate()
        .map(|(i, _)| Field::new(format!("param_{}", i + 1), T::DATA_TYPE, false))
        .collect::<Vec<_>>();

    let arrays = params
        .iter()
        .map(|p| Arc::new(PrimitiveArray::<T>::from(vec![*p])) as ArrayRef)
        .collect::<Vec<_>>();
    let array = StructArray::from(
        arrays
            .iter()
            .enumerate()
            .map(|(i, array)| {
                (
                    Arc::new(Field::new(format!("param_{}", i + 1), T::DATA_TYPE, false)),
                    array.clone(),
                )
            })
            .collect::<Vec<_>>(),
    );
    let array_data = array.to_data();
    let schema = Schema::new(schema_fields);

    let schema_box = Box::new(FFI_ArrowSchema::try_from(&schema).unwrap());
    let array_box = Box::new(FFI_ArrowArray::new(&array_data));
    let raw_array = Box::into_raw(array_box);
    let raw_schema = Box::into_raw(schema_box);

    let schema = ArrowSchemaPtr {
        value: unsafe {
            let len = size_of::<*mut FFI_ArrowSchema>();
            let buf_ptr = std::ptr::addr_of!(raw_schema) as *const u8;
            std::slice::from_raw_parts(buf_ptr, len).to_vec()
        },
    };

    let array = ArrowArrayPtr {
        value: unsafe {
            let len = size_of::<*mut FFI_ArrowArray>();
            let buf_ptr = std::ptr::addr_of!(raw_array) as *const u8;
            std::slice::from_raw_parts(buf_ptr, len).to_vec()
        },
    };
    (schema, array)
}
