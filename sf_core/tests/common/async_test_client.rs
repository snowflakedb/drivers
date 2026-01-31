//! Async test client for Snowflake integration tests.
//!
//! This client uses the async APIs directly, making it suitable for use
//! in `#[tokio::test]` without runtime nesting issues.

use sf_core::config::rest_parameters::{ClientInfo, LoginMethod, LoginParameters, QueryParameters};
use sf_core::config::retry::RetryPolicy;
use sf_core::crl::config::CrlConfig;
use sf_core::rest::snowflake::{
    QueryExecutionMode, RestError, SessionTokens, snowflake_login_with_client,
    snowflake_query_with_client,
};
use sf_core::tls::client::create_tls_client_with_config;
use sf_core::tls::config::TlsConfig;

use super::private_key_helper::{self, TempPrivateKeyFile};

/// Async Snowflake test client for use in `#[tokio::test]` tests.
pub struct AsyncSnowflakeTestClient {
    pub http_client: reqwest::Client,
    pub session_tokens: SessionTokens,
    pub query_parameters: QueryParameters,
    pub retry_policy: RetryPolicy,
    _temp_key_file: Option<TempPrivateKeyFile>,
}

impl AsyncSnowflakeTestClient {
    /// Connect to a mock server for integration testing.
    pub async fn connect_integration_test(server_url: &str) -> Result<Self, RestError> {
        let temp_key_file = private_key_helper::get_test_private_key_file()
            .expect("Failed to create test private key file");

        let private_key =
            std::fs::read_to_string(temp_key_file.path()).expect("Failed to read private key");

        let tls_config = TlsConfig {
            verify_certificates: false,
            ..Default::default()
        };

        let http_client = create_tls_client_with_config(tls_config.clone())
            .expect("Failed to create HTTP client");

        let client_info = ClientInfo {
            application: "UniversalDriver".to_string(),
            version: "0.1.0".to_string(),
            os: std::env::consts::OS.to_string(),
            os_version: "1.0".to_string(),
            ocsp_mode: None,
            crl_config: CrlConfig::default(),
            tls_config: tls_config.clone(),
        };

        let login_parameters = LoginParameters {
            server_url: server_url.to_string(),
            account_name: "test_account".to_string(),
            login_method: LoginMethod::PrivateKey {
                username: "test_user".to_string(),
                private_key,
                passphrase: None,
            },
            database: Some("test_database".to_string()),
            schema: Some("test_schema".to_string()),
            warehouse: Some("test_warehouse".to_string()),
            role: Some("test_role".to_string()),
            client_info,
        };

        let session_tokens = snowflake_login_with_client(&http_client, &login_parameters).await?;

        let query_parameters = QueryParameters {
            server_url: server_url.to_string(),
            client_info: login_parameters.client_info.clone(),
        };

        Ok(Self {
            http_client,
            session_tokens,
            query_parameters,
            retry_policy: RetryPolicy::default(),
            _temp_key_file: Some(temp_key_file),
        })
    }

    /// Execute a SQL query asynchronously.
    pub async fn execute_query(&self, sql: &str) -> Result<String, RestError> {
        let response = snowflake_query_with_client(
            &self.http_client,
            self.query_parameters.clone(),
            self.session_tokens.session_token.clone(),
            sql.to_string(),
            None,
            &self.retry_policy,
            QueryExecutionMode::Blocking,
        )
        .await?;

        Ok(response.data.query_id.unwrap_or_default())
    }

    /// Execute a SQL query and return the result or error message.
    pub async fn execute_query_result(&self, sql: &str) -> Result<(), String> {
        match snowflake_query_with_client(
            &self.http_client,
            self.query_parameters.clone(),
            self.session_tokens.session_token.clone(),
            sql.to_string(),
            None,
            &self.retry_policy,
            QueryExecutionMode::Blocking,
        )
        .await
        {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("{e:?}")),
        }
    }
}
