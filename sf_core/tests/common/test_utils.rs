extern crate sf_core;
extern crate tracing;
extern crate tracing_subscriber;

use arrow::array::{Array, ArrowPrimitiveType, PrimitiveArray, StructArray};
use flate2::read::GzDecoder;
use proto_utils::ProtoError;
use sf_core::protobuf_apis::database_driver_v1::DatabaseDriverClient;
use sf_core::protobuf_gen::database_driver_v1::*;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
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
    pub conn_handle: ConnectionHandle,
    pub db_handle: DatabaseHandle,
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

        let db_response = DatabaseDriverClient::database_new(DatabaseNewRequest {}).unwrap();
        let db_handle = db_response.db_handle.unwrap();

        DatabaseDriverClient::database_init(DatabaseInitRequest {
            db_handle: Some(db_handle),
        })
        .unwrap();

        let conn_response = DatabaseDriverClient::connection_new(ConnectionNewRequest {}).unwrap();
        let conn_handle = conn_response.conn_handle.unwrap();

        let client = Self {
            conn_handle,
            db_handle,
            parameters,
        };

        // Set connection options using the helper method
        client.set_connection_option("account", &client.parameters.account_name.clone().unwrap());
        client.set_connection_option("user", &client.parameters.user.clone().unwrap());

        // Set optional parameters if specified
        if let Some(database) = client.parameters.database.clone() {
            client.set_connection_option("database", &database);
        }

        if let Some(schema) = client.parameters.schema.clone() {
            client.set_connection_option("schema", &schema);
        }

        if let Some(warehouse) = client.parameters.warehouse.clone() {
            client.set_connection_option("warehouse", &warehouse);
        }

        if let Some(host) = client.parameters.host.clone() {
            client.set_connection_option("host", &host);
        }

        if let Some(role) = client.parameters.role.clone() {
            client.set_connection_option("role", &role);
        }

        if let Some(server_url) = client.parameters.server_url.clone() {
            client.set_connection_option("server_url", &server_url);
        }

        if let Some(port) = client.parameters.port {
            client.set_connection_option_int("port", port);
        }

        if let Some(protocol) = client.parameters.protocol.clone() {
            client.set_connection_option("protocol", &protocol);
        }

        client
    }

    /// Creates a test client with integration test parameters
    pub fn with_int_test_params() -> Self {
        setup_logging();

        // Create test parameters for integration tests
        let test_parameters = Parameters {
            account_name: Some("test_account".to_string()),
            user: Some("test_user".to_string()),
            password: Some("test_password".to_string()),
            database: Some("test_database".to_string()),
            schema: Some("test_schema".to_string()),
            warehouse: Some("test_warehouse".to_string()),
            host: Some("localhost".to_string()),
            role: Some("test_role".to_string()),
            server_url: Some("http://localhost:8090".to_string()),
            port: Some(8090),
            protocol: Some("http".to_string()),
            private_key_contents: None,
            private_key_password: None,
        };

        let db_response = DatabaseDriverClient::database_new(DatabaseNewRequest {}).unwrap();
        let db_handle = db_response.db_handle.unwrap();

        DatabaseDriverClient::database_init(DatabaseInitRequest {
            db_handle: Some(db_handle),
        })
        .unwrap();

        let conn_response = DatabaseDriverClient::connection_new(ConnectionNewRequest {}).unwrap();
        let conn_handle = conn_response.conn_handle.unwrap();

        let client = Self {
            conn_handle,
            db_handle,
            parameters: test_parameters,
        };

        // Set connection options using the helper method
        client.set_connection_option("account", &client.parameters.account_name.clone().unwrap());
        client.set_connection_option("user", &client.parameters.user.clone().unwrap());
        client.set_connection_option("database", &client.parameters.database.clone().unwrap());
        client.set_connection_option("schema", &client.parameters.schema.clone().unwrap());
        client.set_connection_option("warehouse", &client.parameters.warehouse.clone().unwrap());
        client.set_connection_option("host", &client.parameters.host.clone().unwrap());
        client.set_connection_option("role", &client.parameters.role.clone().unwrap());
        client.set_connection_option("server_url", &client.parameters.server_url.clone().unwrap());
        client.set_connection_option_int("port", client.parameters.port.unwrap());
        client.set_connection_option("protocol", &client.parameters.protocol.clone().unwrap());

        client
    }

    /// Creates a new test client with Snowflake connection established
    pub fn connect_with_default_auth() -> Self {
        setup_logging();
        let client = Self::with_default_params();

        DatabaseDriverClient::connection_set_option_string(ConnectionSetOptionStringRequest {
            conn_handle: Some(client.conn_handle),
            key: "password".to_string(),
            value: client.parameters.password.clone().unwrap(),
        })
        .unwrap();

        DatabaseDriverClient::connection_init(ConnectionInitRequest {
            conn_handle: Some(client.conn_handle),
            db_handle: Some(client.db_handle),
        })
        .unwrap();

        client
    }

    /// Creates a new statement handle
    pub fn new_statement(&self) -> StatementHandle {
        let response = DatabaseDriverClient::statement_new(StatementNewRequest {
            conn_handle: Some(self.conn_handle),
        })
        .unwrap();
        response.stmt_handle.unwrap()
    }

    /// Executes a SQL query and returns the result
    pub fn execute_query(&self, sql: &str) -> ExecuteResult {
        let stmt_handle = self.new_statement();

        DatabaseDriverClient::statement_set_sql_query(StatementSetSqlQueryRequest {
            stmt_handle: Some(stmt_handle),
            query: sql.to_string(),
        })
        .unwrap();

        let response =
            DatabaseDriverClient::statement_execute_query(StatementExecuteQueryRequest {
                stmt_handle: Some(stmt_handle),
            })
            .unwrap();

        response.result.unwrap()
    }

    pub fn execute_query_no_unwrap(&self, sql: &str) -> Result<ExecuteResult, String> {
        let stmt_handle = self.new_statement();

        if let Err(e) = DatabaseDriverClient::statement_set_sql_query(StatementSetSqlQueryRequest {
            stmt_handle: Some(stmt_handle),
            query: sql.to_string(),
        }) {
            return Err(format!("Failed to set SQL query: {e:?}"));
        }

        match DatabaseDriverClient::statement_execute_query(StatementExecuteQueryRequest {
            stmt_handle: Some(stmt_handle),
        }) {
            Ok(response) => {
                let proto_result = response.result.unwrap();
                Ok(proto_result)
            }
            Err(ProtoError::Application(e)) => Err(format!("Failed to execute query: {e:?}")),
            Err(ProtoError::Transport(e)) => Err(format!("Transport error: {e:?}")),
        }
    }

    pub fn create_temporary_stage(&self, stage_name: &str) {
        self.execute_query(&format!(
            "create temporary stage if not exists {stage_name}"
        ));
    }

    pub fn connect(&self) -> Result<(), String> {
        match DatabaseDriverClient::connection_init(ConnectionInitRequest {
            conn_handle: Some(self.conn_handle),
            db_handle: Some(self.db_handle),
        }) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Connection failed: {e:?}")),
        }
    }

    pub fn set_connection_option(&self, option_name: &str, option_value: &str) {
        DatabaseDriverClient::connection_set_option_string(ConnectionSetOptionStringRequest {
            conn_handle: Some(self.conn_handle),
            key: option_name.to_string(),
            value: option_value.to_string(),
        })
        .unwrap();
    }

    pub fn set_connection_option_int(&self, option_name: &str, option_value: i64) {
        DatabaseDriverClient::connection_set_option_int(ConnectionSetOptionIntRequest {
            conn_handle: Some(self.conn_handle),
            key: option_name.to_string(),
            value: option_value,
        })
        .unwrap();
    }

    pub fn verify_simple_query(&self, connection_result: Result<(), String>) {
        connection_result.expect("Login failed");
        let _result = self.execute_query("SELECT 1");
    }

    pub fn assert_login_error(&self, result: Result<(), String>) {
        let error_msg = result.expect_err("Expected error");

        // For protobuf errors, we check the string representation for now
        // TODO: Improve error handling to extract proper DriverException details
        assert!(
            error_msg.contains("login")
                || error_msg.contains("auth")
                || error_msg.contains("LoginError")
                || error_msg.contains("AuthError"),
            "Error message should contain login or auth related information: {error_msg}"
        );
        assert!(!error_msg.is_empty(), "Error message should not be empty");
    }

    pub fn assert_missing_parameter_error(&self, result: Result<(), String>) {
        let error_msg = result.expect_err("Expected error");

        // For protobuf errors, we check the string representation for now
        // TODO: Improve error handling to extract proper DriverException details
        assert!(
            error_msg.contains("MissingParameter")
                || error_msg.contains("missing")
                || error_msg.contains("parameter"),
            "Error message should contain missing parameter information: {error_msg}"
        );
        assert!(!error_msg.is_empty(), "Error message should not be empty");
    }
}

impl Drop for SnowflakeTestClient {
    fn drop(&mut self) {
        // Release the connection when the client is dropped
        if let Err(e) = DatabaseDriverClient::connection_release(ConnectionReleaseRequest {
            conn_handle: Some(self.conn_handle),
        }) {
            tracing::warn!("Failed to release connection in Drop: {e:?}");
        }
        // Release the database handle
        if let Err(e) = DatabaseDriverClient::database_release(DatabaseReleaseRequest {
            db_handle: Some(self.db_handle),
        }) {
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
    use sf_core::protobuf_gen::database_driver_v1::{ArrowArrayPtr, ArrowSchemaPtr};
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

/// Returns repository root path
pub fn repo_root() -> PathBuf {
    if let Ok(output) = Command::new("git")
        .arg("rev-parse")
        .arg("--show-toplevel")
        .output()
        && output.status.success()
        && let Ok(stdout) = String::from_utf8(output.stdout)
    {
        let root = stdout.trim();
        if !root.is_empty() {
            return PathBuf::from(root);
        }
    }
    panic!("Failed to determine repository root");
}

/// Path to shared test data directory: repo_root/tests/test_data
pub fn shared_test_data_dir() -> PathBuf {
    repo_root()
        .join("tests")
        .join("test_data")
        .join("generated_test_data")
}
