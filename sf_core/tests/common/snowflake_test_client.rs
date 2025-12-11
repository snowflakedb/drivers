use arrow::array::{Array, ArrayRef, ArrowPrimitiveType, PrimitiveArray, StructArray};
use arrow::datatypes::{Field, Schema};
use arrow::ffi::{FFI_ArrowArray, FFI_ArrowSchema};
use proto_utils::ProtoError;
use sf_core::protobuf_apis::database_driver_v1::DatabaseDriverClient;
use sf_core::protobuf_gen::database_driver_v1::*;
use sf_core::rest::snowflake::STATEMENT_ASYNC_EXECUTION_OPTION;
use std::mem::size_of;
use std::sync::Arc;

use super::config::{Parameters, get_parameters, setup_logging};
use super::private_key_helper::{self, TempPrivateKeyFile};

/// Creates a connected Snowflake client with database and connection initialized
pub struct SnowflakeTestClient {
    pub conn_handle: ConnectionHandle,
    pub db_handle: DatabaseHandle,
    pub parameters: Parameters,
    temp_key_file: Option<TempPrivateKeyFile>,
}

impl SnowflakeTestClient {
    /// Creates a client with default parameters (no authentication parameters set)
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
            temp_key_file: None,
        };

        client.set_options_from_parameters();
        client
    }

    /// Creates a client with default parameters and JWT authentication configured
    pub fn with_default_jwt_auth_params() -> Self {
        setup_logging();
        let mut client = Self::with_default_params();

        let temp_key_file = client.setup_jwt_auth();
        client.temp_key_file = Some(temp_key_file);
        client
    }

    pub fn connect_with_default_auth() -> Self {
        setup_logging();
        let mut client = Self::with_default_params();

        let temp_key_file = client.setup_jwt_auth();

        DatabaseDriverClient::connection_init(ConnectionInitRequest {
            conn_handle: Some(client.conn_handle),
            db_handle: Some(client.db_handle),
        })
        .unwrap();

        client.temp_key_file = Some(temp_key_file);
        client
    }

    pub fn with_int_tests_params(server_url: Option<&str>) -> Self {
        setup_logging();

        let server_url = server_url.unwrap_or("http://localhost:8090");

        let test_parameters = Parameters {
            account_name: Some("test_account".to_string()),
            user: Some("test_user".to_string()),
            password: Some("test_password".to_string()),
            database: Some("test_database".to_string()),
            schema: Some("test_schema".to_string()),
            warehouse: Some("test_warehouse".to_string()),
            host: Some("localhost".to_string()),
            role: Some("test_role".to_string()),
            server_url: Some(server_url.to_string()),
            port: None,
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
            temp_key_file: None,
        };

        client.set_options_from_parameters();
        client
    }

    pub fn connect_integration_test(server_url: Option<&str>) -> Self {
        let mut client = Self::with_int_tests_params(server_url);

        client.set_connection_option("authenticator", "SNOWFLAKE_JWT");
        let temp_key_file = private_key_helper::get_test_private_key_file()
            .expect("Failed to create test private key file");
        client.set_connection_option("private_key_file", temp_key_file.path().to_str().unwrap());

        DatabaseDriverClient::connection_init(ConnectionInitRequest {
            conn_handle: Some(client.conn_handle),
            db_handle: Some(client.db_handle),
        })
        .unwrap();

        client.temp_key_file = Some(temp_key_file);
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

    pub fn execute_statement_query(&self, stmt: &StatementHandle) -> ExecuteResult {
        DatabaseDriverClient::statement_execute_query(StatementExecuteQueryRequest {
            stmt_handle: Some(*stmt),
        })
        .unwrap()
        .result
        .unwrap()
    }

    pub fn set_sql_query(&self, stmt: &StatementHandle, query: &str) {
        DatabaseDriverClient::statement_set_sql_query(StatementSetSqlQueryRequest {
            stmt_handle: Some(*stmt),
            query: query.to_string(),
        })
        .unwrap();
    }

    pub fn bind_parameters<T: ArrowPrimitiveType>(
        &self,
        stmt: &StatementHandle,
        params: &[T::Native],
    ) where
        PrimitiveArray<T>: From<Vec<T::Native>>,
    {
        let (schema, array) = create_param_bindings::<T>(params);

        DatabaseDriverClient::statement_bind(StatementBindRequest {
            stmt_handle: Some(*stmt),
            schema: Some(schema),
            array: Some(array),
        })
        .unwrap();
    }

    pub fn release_statement(&self, stmt: &StatementHandle) {
        DatabaseDriverClient::statement_release(StatementReleaseRequest {
            stmt_handle: Some(*stmt),
        })
        .unwrap();
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

    pub fn set_statement_async_execution(&self, stmt: &StatementHandle, enabled: bool) {
        DatabaseDriverClient::statement_set_option_string(StatementSetOptionStringRequest {
            stmt_handle: Some(*stmt),
            key: STATEMENT_ASYNC_EXECUTION_OPTION.to_string(),
            value: if enabled { "true" } else { "false" }.to_string(),
        })
        .unwrap();
    }

    /// Stores a temporary private key file to keep it alive for the duration of the test.
    pub fn set_temp_key_file(&mut self, temp_key_file: TempPrivateKeyFile) {
        self.temp_key_file = Some(temp_key_file);
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

    /// Sets up JWT authentication configuration and returns a private key file
    fn setup_jwt_auth(&mut self) -> TempPrivateKeyFile {
        self.set_connection_option("authenticator", "SNOWFLAKE_JWT");
        let temp_key_file = private_key_helper::get_private_key_from_parameters(&self.parameters)
            .expect("Failed to create private key file");
        self.set_connection_option("private_key_file", temp_key_file.path().to_str().unwrap());
        if let Some(password) = &self.parameters.private_key_password {
            self.set_connection_option("private_key_password", password);
        }
        temp_key_file
    }

    fn set_options_from_parameters(&self) {
        self.set_connection_option("account", &self.parameters.account_name.clone().unwrap());
        self.set_connection_option("user", &self.parameters.user.clone().unwrap());

        // Set optional parameters if specified
        if let Some(database) = &self.parameters.database {
            self.set_connection_option("database", database);
        }

        if let Some(schema) = &self.parameters.schema {
            self.set_connection_option("schema", schema);
        }

        if let Some(warehouse) = &self.parameters.warehouse {
            self.set_connection_option("warehouse", warehouse);
        }

        if let Some(host) = &self.parameters.host {
            self.set_connection_option("host", host);
        }

        if let Some(role) = &self.parameters.role {
            self.set_connection_option("role", role);
        }

        if let Some(server_url) = &self.parameters.server_url {
            self.set_connection_option("server_url", server_url);
        }

        if let Some(port) = self.parameters.port {
            self.set_connection_option_int("port", port);
        }

        if let Some(protocol) = &self.parameters.protocol {
            self.set_connection_option("protocol", protocol);
        }
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

/// Creates Arrow schema and array for parameter binding
pub fn create_param_bindings<T: ArrowPrimitiveType>(
    params: &[T::Native],
) -> (ArrowSchemaPtr, ArrowArrayPtr)
where
    PrimitiveArray<T>: From<Vec<T::Native>>,
{
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
