use proto_utils::ProtoError;
use sf_core::protobuf::apis::database_driver_v1::{
    DatabaseDriverClient, DatabaseDriverClientBlockingExt, database_driver_client,
};
use sf_core::protobuf::generated::database_driver_v1::*;

use super::config::{Parameters, get_parameters, setup_logging};
use super::private_key_helper::{self, PrivateKeyFile};

/// Creates a connected Snowflake client with database and connection initialized
pub struct SnowflakeTestClient {
    pub conn_handle: ConnectionHandle,
    pub db_handle: DatabaseHandle,
    pub parameters: Parameters,
    private_key_file: Option<PrivateKeyFile>,
    client: DatabaseDriverClient,
}

impl SnowflakeTestClient {
    /// Creates a client with default parameters (no authentication parameters set)
    pub fn with_default_params() -> Self {
        setup_logging();
        let parameters = get_parameters();
        let client = database_driver_client();
        let db_response = client.database_new_blocking(DatabaseNewRequest {}).unwrap();
        let db_handle = db_response.db_handle.unwrap();

        client
            .database_init_blocking(DatabaseInitRequest {
                db_handle: Some(db_handle),
            })
            .unwrap();

        let conn_response = client
            .connection_new_blocking(ConnectionNewRequest {})
            .unwrap();
        let conn_handle = conn_response.conn_handle.unwrap();

        let test_client = Self {
            conn_handle,
            db_handle,
            parameters,
            private_key_file: None,
            client,
        };

        test_client.set_options_from_parameters();
        test_client
    }

    /// Creates a client with default parameters and JWT authentication configured
    pub fn with_default_jwt_auth_params() -> Self {
        setup_logging();
        let mut client = Self::with_default_params();

        let temp_key_file = client.setup_jwt_auth();
        client.private_key_file = Some(temp_key_file);
        client
    }

    pub fn connect_with_default_auth() -> Self {
        setup_logging();
        let mut test_client = Self::with_default_params();

        let temp_key_file = test_client.setup_jwt_auth();

        test_client
            .client
            .connection_init_blocking(ConnectionInitRequest {
                conn_handle: Some(test_client.conn_handle),
                db_handle: Some(test_client.db_handle),
            })
            .unwrap();

        test_client.private_key_file = Some(temp_key_file);
        test_client
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
            protocol: Some("http".to_string()),
            ..Default::default()
        };

        let client = database_driver_client();

        let db_response = client.database_new_blocking(DatabaseNewRequest {}).unwrap();
        let db_handle = db_response.db_handle.unwrap();

        client
            .database_init_blocking(DatabaseInitRequest {
                db_handle: Some(db_handle),
            })
            .unwrap();

        let conn_response = client
            .connection_new_blocking(ConnectionNewRequest {})
            .unwrap();
        let conn_handle = conn_response.conn_handle.unwrap();

        let test_client = Self {
            conn_handle,
            db_handle,
            parameters: test_parameters,
            private_key_file: None,
            client,
        };

        test_client.set_options_from_parameters();
        test_client
    }

    pub fn connect_integration_test(server_url: Option<&str>) -> Self {
        let mut test_client = Self::with_int_tests_params(server_url);

        test_client.set_connection_option("authenticator", "SNOWFLAKE_JWT");
        let temp_key_file = private_key_helper::get_test_private_key_file()
            .expect("Failed to create test private key file");
        test_client
            .set_connection_option("private_key_file", temp_key_file.path().to_str().unwrap());

        test_client
            .client
            .connection_init_blocking(ConnectionInitRequest {
                conn_handle: Some(test_client.conn_handle),
                db_handle: Some(test_client.db_handle),
            })
            .unwrap();

        test_client.private_key_file = Some(temp_key_file);
        test_client
    }

    /// Creates a new statement handle
    pub fn new_statement(&self) -> StatementHandle {
        let response = self
            .client
            .statement_new_blocking(StatementNewRequest {
                conn_handle: Some(self.conn_handle),
            })
            .unwrap();
        response.stmt_handle.unwrap()
    }

    pub fn execute_statement_query(&self, stmt: &StatementHandle) -> ExecuteResult {
        self.execute_statement_query_with_bindings(stmt, None)
    }

    pub fn execute_statement_query_with_bindings(
        &self,
        stmt: &StatementHandle,
        json_bindings: Option<&str>,
    ) -> ExecuteResult {
        let bindings = json_bindings.map(|json| {
            let ptr = json.as_bytes().as_ptr() as u64;
            QueryBindings {
                binding_type: Some(query_bindings::BindingType::Json(BinaryDataPtr {
                    value: ptr.to_le_bytes().to_vec(),
                    length: json.len() as i64,
                })),
            }
        });
        self.client
            .statement_execute_query_blocking(StatementExecuteQueryRequest {
                stmt_handle: Some(*stmt),
                bindings,
            })
            .unwrap()
            .result
            .unwrap()
    }

    pub fn set_sql_query(&self, stmt: &StatementHandle, query: &str) {
        self.client
            .statement_set_sql_query_blocking(StatementSetSqlQueryRequest {
                stmt_handle: Some(*stmt),
                query: query.to_string(),
            })
            .unwrap();
    }

    /// Builds a JSON bindings string for integer parameters.
    pub fn bind_int_parameters_json(&self, params: &[i32]) -> String {
        let mut bindings = serde_json::Map::new();
        for (i, value) in params.iter().enumerate() {
            let mut entry = serde_json::Map::new();
            entry.insert(
                "type".to_string(),
                serde_json::Value::String("FIXED".to_string()),
            );
            entry.insert(
                "value".to_string(),
                serde_json::Value::String(value.to_string()),
            );
            bindings.insert((i + 1).to_string(), serde_json::Value::Object(entry));
        }
        serde_json::to_string(&bindings).unwrap()
    }

    pub fn result_chunks(&self, stmt: &StatementHandle) -> ResultChunksResult {
        self.client
            .statement_result_chunks_blocking(StatementResultChunksRequest {
                stmt_handle: Some(*stmt),
            })
            .unwrap()
            .result
            .unwrap()
    }

    pub fn fetch_chunk(&self, chunk: ResultChunk) -> DatabaseFetchChunkResponse {
        self.client
            .database_fetch_chunk_blocking(DatabaseFetchChunkRequest {
                db_handle: Some(self.db_handle),
                chunk: Some(chunk),
            })
            .unwrap()
    }

    pub fn release_statement(&self, stmt: &StatementHandle) {
        self.client
            .statement_release_blocking(StatementReleaseRequest {
                stmt_handle: Some(*stmt),
            })
            .unwrap();
    }

    /// Executes a SQL query and returns the result
    pub fn execute_query(&self, sql: &str) -> ExecuteResult {
        let stmt_handle = self.new_statement();

        self.client
            .statement_set_sql_query_blocking(StatementSetSqlQueryRequest {
                stmt_handle: Some(stmt_handle),
                query: sql.to_string(),
            })
            .unwrap();

        let response = self
            .client
            .statement_execute_query_blocking(StatementExecuteQueryRequest {
                stmt_handle: Some(stmt_handle),
                bindings: None,
            })
            .unwrap();

        response.result.unwrap()
    }

    pub fn execute_query_no_unwrap(&self, sql: &str) -> Result<ExecuteResult, String> {
        let stmt_handle = self.new_statement();

        if let Err(e) = self
            .client
            .statement_set_sql_query_blocking(StatementSetSqlQueryRequest {
                stmt_handle: Some(stmt_handle),
                query: sql.to_string(),
            })
        {
            return Err(format!("Failed to set SQL query: {e:?}"));
        }

        match self
            .client
            .statement_execute_query_blocking(StatementExecuteQueryRequest {
                stmt_handle: Some(stmt_handle),
                bindings: None,
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
        match self.client.connection_init_blocking(ConnectionInitRequest {
            conn_handle: Some(self.conn_handle),
            db_handle: Some(self.db_handle),
        }) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Connection failed: {e:?}")),
        }
    }

    pub fn set_connection_option(&self, option_name: &str, option_value: &str) {
        self.client
            .connection_set_option_string_blocking(ConnectionSetOptionStringRequest {
                conn_handle: Some(self.conn_handle),
                key: option_name.to_string(),
                value: option_value.to_string(),
            })
            .unwrap();
    }

    pub fn set_connection_option_int(&self, option_name: &str, option_value: i64) {
        self.client
            .connection_set_option_int_blocking(ConnectionSetOptionIntRequest {
                conn_handle: Some(self.conn_handle),
                key: option_name.to_string(),
                value: option_value,
            })
            .unwrap();
    }

    pub fn set_connection_option_bytes(&self, option_name: &str, option_value: &[u8]) {
        self.client
            .connection_set_option_bytes_blocking(ConnectionSetOptionBytesRequest {
                conn_handle: Some(self.conn_handle),
                key: option_name.to_string(),
                value: option_value.to_vec(),
            })
            .unwrap();
    }

    pub fn set_statement_async_execution(&self, stmt: &StatementHandle, enabled: bool) {
        self.client
            .statement_set_option_bool_blocking(StatementSetOptionBoolRequest {
                stmt_handle: Some(*stmt),
                key: "async_execution".to_string(),
                value: enabled,
            })
            .unwrap();
    }

    /// Stores a temporary private key file to keep it alive for the duration of the test.
    pub fn set_temp_key_file(&mut self, temp_key_file: PrivateKeyFile) {
        self.private_key_file = Some(temp_key_file);
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
    fn setup_jwt_auth(&mut self) -> PrivateKeyFile {
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

        if let Some(warehouse) = &self.parameters.warehouse() {
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
        if let Err(e) = self
            .client
            .connection_release_blocking(ConnectionReleaseRequest {
                conn_handle: Some(self.conn_handle),
            })
        {
            tracing::warn!("Failed to release connection in Drop: {e:?}");
        }
        // Release the database handle
        if let Err(e) = self
            .client
            .database_release_blocking(DatabaseReleaseRequest {
                db_handle: Some(self.db_handle),
            })
        {
            tracing::warn!("Failed to release database handle in Drop: {e:?}");
        }
    }
}
