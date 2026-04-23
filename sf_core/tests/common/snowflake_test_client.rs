use proto_utils::ProtoError;
use sf_core::config::logout::ErrorStrategy;
use sf_core::config::param_names;
use sf_core::protobuf::apis::database_driver_v1::{
    DatabaseDriverClient, DatabaseDriverClientBlockingExt, DriverProviders, database_driver_client,
    database_driver_client_with,
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
                ..Default::default()
            })
            .unwrap();

        test_client.private_key_file = Some(temp_key_file);
        test_client
    }

    pub fn with_int_tests_params(server_url: Option<&str>) -> Self {
        Self::with_int_tests_params_and_client(server_url, database_driver_client())
    }

    /// Variant of [`Self::with_int_tests_params`] that installs test
    /// providers (e.g. a mocked filesystem) on the underlying driver
    /// before the client starts issuing requests. Add new providers by
    /// extending [`DriverProviders`]; no new constructor is required.
    pub fn with_int_tests_params_using(
        server_url: Option<&str>,
        providers: DriverProviders,
    ) -> Self {
        Self::with_int_tests_params_and_client(server_url, database_driver_client_with(providers))
    }

    fn with_int_tests_params_and_client(
        server_url: Option<&str>,
        client: DatabaseDriverClient,
    ) -> Self {
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
                ..Default::default()
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

    pub fn execute_statement_query(
        &self,
        stmt: &StatementHandle,
    ) -> execute_query_response::Result {
        self.execute_statement_query_with_bindings(stmt, None)
    }

    pub fn execute_statement_query_with_bindings(
        &self,
        stmt: &StatementHandle,
        json_bindings: Option<&str>,
    ) -> execute_query_response::Result {
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

    pub fn result_chunks(&self, stmt: &StatementHandle, query_id: &str) -> ResultChunksResult {
        self.client
            .statement_result_chunks_blocking(StatementResultChunksRequest {
                stmt_handle: Some(*stmt),
                query_id: query_id.to_string(),
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

    /// Executes a SQL statement, ignoring the result. Use for DDL/side-effect queries.
    pub fn execute_sql(&self, sql: &str) {
        let stmt_handle = self.new_statement();

        self.client
            .statement_set_sql_query_blocking(StatementSetSqlQueryRequest {
                stmt_handle: Some(stmt_handle),
                query: sql.to_string(),
            })
            .unwrap();

        self.client
            .statement_execute_query_blocking(StatementExecuteQueryRequest {
                stmt_handle: Some(stmt_handle),
                bindings: None,
            })
            .unwrap();

        self.release_statement(&stmt_handle);
    }

    pub fn execute_query_no_unwrap(
        &self,
        sql: &str,
    ) -> Result<execute_query_response::Result, String> {
        let stmt_handle = self.new_statement();

        if let Err(e) = self
            .client
            .statement_set_sql_query_blocking(StatementSetSqlQueryRequest {
                stmt_handle: Some(stmt_handle),
                query: sql.to_string(),
            })
        {
            self.release_statement(&stmt_handle);
            return Err(format!("Failed to set SQL query: {e:?}"));
        }

        let result =
            match self
                .client
                .statement_execute_query_blocking(StatementExecuteQueryRequest {
                    stmt_handle: Some(stmt_handle),
                    bindings: None,
                }) {
                Ok(response) => Ok(response.result.unwrap()),
                Err(e) => match *e {
                    ProtoError::Application(e) => Err(format!("Failed to execute query: {e:?}")),
                    ProtoError::Transport(e) => Err(format!("Transport error: {e:?}")),
                },
            };
        self.release_statement(&stmt_handle);
        result
    }

    /// Execute a single query and get the result set (Arrow stream) directly.
    pub fn execute_query(&self, sql: &str) -> ResultSetResponse {
        let stmt_handle = self.new_statement();
        self.set_sql_query(&stmt_handle, sql);
        let result = self.execute_statement_query(&stmt_handle);
        let query_id = unwrap_single_query_id(&result);
        let rs = self.get_result_set(&stmt_handle, &query_id);
        self.release_statement(&stmt_handle);
        rs
    }

    /// Get a result set by query_id using the statement handle.
    pub fn get_result_set(&self, stmt: &StatementHandle, query_id: &str) -> ResultSetResponse {
        self.client
            .statement_get_result_set_blocking(StatementGetResultSetRequest {
                stmt_handle: Some(*stmt),
                query_id: query_id.to_string(),
            })
            .unwrap()
    }

    /// Get a result set by query_id using the connection handle.
    pub fn connection_get_result_set(&self, query_id: &str) -> ResultSetResponse {
        self.client
            .connection_get_result_set_blocking(ConnectionGetResultSetRequest {
                conn_handle: Some(self.conn_handle),
                query_id: query_id.to_string(),
            })
            .unwrap()
    }

    /// Execute a multistatement query.
    pub fn execute_multistatement(&self, sql: &str, count: i64) -> execute_query_response::Result {
        let stmt_handle = self.new_statement();
        self.set_sql_query(&stmt_handle, sql);
        self.client
            .statement_set_options_blocking(StatementSetOptionsRequest {
                stmt_handle: Some(stmt_handle),
                options: [(
                    "multi_statement_count".to_string(),
                    ConfigSetting::from(count),
                )]
                .into_iter()
                .collect(),
            })
            .unwrap();
        let result = self.execute_statement_query(&stmt_handle);
        self.release_statement(&stmt_handle);
        result
    }

    /// Execute a multistatement query, returning error on failure.
    pub fn execute_multistatement_no_unwrap(
        &self,
        sql: &str,
        count: i64,
    ) -> Result<execute_query_response::Result, String> {
        let stmt_handle = self.new_statement();
        self.set_sql_query(&stmt_handle, sql);
        self.client
            .statement_set_options_blocking(StatementSetOptionsRequest {
                stmt_handle: Some(stmt_handle),
                options: [(
                    "multi_statement_count".to_string(),
                    ConfigSetting::from(count),
                )]
                .into_iter()
                .collect(),
            })
            .unwrap();
        let result =
            match self
                .client
                .statement_execute_query_blocking(StatementExecuteQueryRequest {
                    stmt_handle: Some(stmt_handle),
                    bindings: None,
                }) {
                Ok(response) => Ok(response.result.unwrap()),
                Err(e) => match *e {
                    ProtoError::Application(e) => Err(format!("{e:?}")),
                    ProtoError::Transport(e) => Err(format!("{e:?}")),
                },
            };
        self.release_statement(&stmt_handle);
        result
    }

    pub fn create_temporary_stage(&self, stage_name: &str) {
        self.execute_sql(&format!(
            "create temporary stage if not exists {stage_name}"
        ));
    }

    pub fn connect(&self) -> Result<(), String> {
        match self.client.connection_init_blocking(ConnectionInitRequest {
            conn_handle: Some(self.conn_handle),
            db_handle: Some(self.db_handle),
            ..Default::default()
        }) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Connection failed: {e:?}")),
        }
    }

    pub fn set_connection_option(&self, option_name: &str, option_value: &str) {
        self.set_connection_config_setting(option_name, option_value.to_string().into());
    }

    pub fn set_connection_option_int(&self, option_name: &str, option_value: i64) {
        self.set_connection_config_setting(option_name, option_value.into());
    }

    pub fn set_connection_option_bool(&self, option_name: &str, option_value: bool) {
        self.set_connection_config_setting(option_name, option_value.into());
    }

    pub fn set_connection_option_bytes(&self, option_name: &str, option_value: &[u8]) {
        self.client
            .connection_set_options_blocking(ConnectionSetOptionsRequest {
                conn_handle: Some(self.conn_handle),
                options: [(option_name.to_string(), option_value.to_vec().into())]
                    .into_iter()
                    .collect(),
            })
            .unwrap();
    }

    pub fn set_statement_async_execution(&self, stmt: &StatementHandle, enabled: bool) {
        self.client
            .statement_set_options_blocking(StatementSetOptionsRequest {
                stmt_handle: Some(*stmt),
                options: [(
                    param_names::ASYNC_EXECUTION.as_str().to_string(),
                    ConfigSetting::from(enabled),
                )]
                .into_iter()
                .collect(),
            })
            .unwrap();
    }

    fn set_connection_config_setting(&self, key: &str, setting: ConfigSetting) {
        self.client
            .connection_set_options_blocking(ConnectionSetOptionsRequest {
                conn_handle: Some(self.conn_handle),
                options: [(key.to_string(), setting)].into_iter().collect(),
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

    /// Initialize this connection (call after configuring options, before queries).
    #[allow(clippy::result_large_err)]
    pub fn connection_init_blocking(
        &self,
    ) -> Result<ConnectionInitResponse, Box<ProtoError<DriverException>>> {
        self.client.connection_init_blocking(ConnectionInitRequest {
            conn_handle: Some(self.conn_handle),
            db_handle: Some(self.db_handle),
            ..Default::default()
        })
    }

    pub fn set_logout_error_strategy(&self, strategy: ErrorStrategy) {
        self.set_connection_option("logout_error_strategy", strategy.as_str());
    }

    /// Close this connection (logout + release).
    #[allow(clippy::result_large_err)]
    pub fn connection_close_blocking(
        &self,
    ) -> Result<ConnectionCloseResponse, Box<ProtoError<DriverException>>> {
        self.client
            .connection_close_blocking(ConnectionCloseRequest {
                conn_handle: Some(self.conn_handle),
                ..Default::default()
            })
    }

    /// Check whether this connection has been closed.
    #[allow(clippy::result_large_err)]
    pub fn connection_is_closed_blocking(&self) -> Result<bool, Box<ProtoError<DriverException>>> {
        self.client
            .connection_is_closed_blocking(ConnectionIsClosedRequest {
                conn_handle: Some(self.conn_handle),
            })
            .map(|r| r.is_closed)
    }

    /// Get connection info (tokens, host, etc.) for inspection.
    #[allow(clippy::result_large_err)]
    pub fn connection_get_info_blocking(
        &self,
        include_master_token: bool,
    ) -> Result<ConnectionGetInfoResponse, Box<ProtoError<DriverException>>> {
        self.client
            .connection_get_info_blocking(ConnectionGetInfoRequest {
                conn_handle: Some(self.conn_handle),
                include_master_token,
                ..Default::default()
            })
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

/// Extract the query_id from a single-statement execute result.
/// Panics if the result is a multi-statement result.
pub fn unwrap_single_query_id(result: &execute_query_response::Result) -> String {
    match result {
        execute_query_response::Result::Single(d) => d.query_id.clone(),
        execute_query_response::Result::Multi(_) => {
            panic!("Expected single-statement result, got multi-statement")
        }
    }
}
