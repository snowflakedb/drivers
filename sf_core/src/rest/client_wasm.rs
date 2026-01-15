//! WASM REST client using portable HTTP.
//!
//! This module provides the WASM implementation of the `SnowflakeRestClient`
//! trait using the portable HTTP client abstraction.

use super::client::{
    BindParameter, QueryExecutionMode, QueryResponse, RestClientError, SnowflakeRestClient,
};
use crate::auth::{Credentials, create_credentials};
use crate::config::rest_parameters::LoginParameters;
use crate::http::{DefaultHttpClient, HttpClient, HttpRequest};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use snafu::Location;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// WASM REST client using portable HTTP.
pub struct WasmRestClient {
    http_client: DefaultHttpClient,
    session_token: Option<String>,
    base_url: String,
}

impl WasmRestClient {
    /// Create a new WASM REST client.
    pub fn new(base_url: &str) -> Self {
        Self {
            http_client: DefaultHttpClient::default(),
            session_token: None,
            base_url: base_url.to_string(),
        }
    }

    fn user_agent(&self, params: &LoginParameters) -> String {
        format!(
            "{}/{} ({}) Rust/WASM",
            params.client_info.application, params.client_info.version, params.client_info.os
        )
    }
}

#[async_trait]
impl SnowflakeRestClient for WasmRestClient {
    async fn login(&mut self, params: &LoginParameters) -> Result<String, RestClientError> {
        let credentials =
            create_credentials(params).map_err(|e| RestClientError::Authentication {
                message: e.to_string(),
                location: Location::new(file!(), line!(), column!()),
            })?;

        let auth_request = build_auth_request(params, &credentials);
        let auth_json =
            serde_json::to_string(&auth_request).map_err(|e| RestClientError::Parse {
                message: format!("Failed to serialize auth request: {}", e),
                location: Location::new(file!(), line!(), column!()),
            })?;

        let url = format!("{}/session/v1/login-request", self.base_url);
        let mut request = HttpRequest::post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/snowflake")
            .header("User-Agent", self.user_agent(params))
            .header("Authorization", "Snowflake Token=\"None\"")
            .body(auth_json.into_bytes());

        // Add query parameters for database context
        if let Some(ref db) = params.database {
            request = request.query("databaseName", db);
        }
        if let Some(ref schema) = params.schema {
            request = request.query("schemaName", schema);
        }
        if let Some(ref warehouse) = params.warehouse {
            request = request.query("warehouse", warehouse);
        }
        if let Some(ref role) = params.role {
            request = request.query("roleName", role);
        }

        let response =
            self.http_client
                .request(request)
                .await
                .map_err(|e| RestClientError::Http {
                    message: format!("Login request failed: {}", e),
                    location: Location::new(file!(), line!(), column!()),
                })?;

        if !response.status.is_success() {
            let body = String::from_utf8_lossy(&response.body);
            return Err(RestClientError::LoginError {
                message: format!("HTTP error: {} - {}", response.status, body),
                code: response.status.as_u16() as i64,
                location: Location::new(file!(), line!(), column!()),
            });
        }

        let auth_response: AuthResponseWasm =
            response.json().map_err(|e| RestClientError::Parse {
                message: format!("Failed to parse login response: {}", e),
                location: Location::new(file!(), line!(), column!()),
            })?;

        if !auth_response.success {
            return Err(RestClientError::LoginError {
                message: auth_response.message.unwrap_or_default(),
                code: auth_response.code.unwrap_or(0),
                location: Location::new(file!(), line!(), column!()),
            });
        }

        let token = auth_response.data.and_then(|d| d.token).ok_or_else(|| {
            RestClientError::LoginError {
                message: "No token in response".to_string(),
                code: 0,
                location: Location::new(file!(), line!(), column!()),
            }
        })?;

        self.session_token = Some(token.clone());
        Ok(token)
    }

    async fn query(
        &self,
        sql: &str,
        bindings: Option<HashMap<String, BindParameter>>,
    ) -> Result<QueryResponse, RestClientError> {
        self.execute_query_internal(sql, bindings, false).await
    }

    async fn query_with_mode(
        &self,
        sql: &str,
        bindings: Option<HashMap<String, BindParameter>>,
        mode: QueryExecutionMode,
    ) -> Result<QueryResponse, RestClientError> {
        match mode {
            QueryExecutionMode::Blocking => self.execute_query_internal(sql, bindings, false).await,
            QueryExecutionMode::Async => self.execute_async_query(sql, bindings).await,
        }
    }

    async fn download_chunk(
        &self,
        url: &str,
        headers: &HashMap<String, String>,
    ) -> Result<Vec<u8>, RestClientError> {
        let mut request = HttpRequest::get(url);
        for (key, value) in headers {
            request = request.header(key, value);
        }

        let response =
            self.http_client
                .request(request)
                .await
                .map_err(|e| RestClientError::Http {
                    message: format!("Chunk download failed: {}", e),
                    location: Location::new(file!(), line!(), column!()),
                })?;

        if !response.status.is_success() {
            return Err(RestClientError::Http {
                message: format!("Chunk download returned HTTP {}", response.status),
                location: Location::new(file!(), line!(), column!()),
            });
        }

        // Check for gzip encoding
        let is_gzip = response
            .headers
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("content-encoding") && v.contains("gzip"));

        let data = if is_gzip {
            use std::io::Read;
            let mut decoder = flate2::read::GzDecoder::new(&response.body[..]);
            let mut decompressed = Vec::new();
            decoder
                .read_to_end(&mut decompressed)
                .map_err(|e| RestClientError::Http {
                    message: format!("Failed to decompress chunk: {}", e),
                    location: Location::new(file!(), line!(), column!()),
                })?;
            decompressed
        } else {
            response.body
        };

        Ok(data)
    }

    fn session_token(&self) -> Option<&str> {
        self.session_token.as_deref()
    }
}

impl WasmRestClient {
    /// Execute a query internally with optional async mode.
    async fn execute_query_internal(
        &self,
        sql: &str,
        bindings: Option<HashMap<String, BindParameter>>,
        async_exec: bool,
    ) -> Result<QueryResponse, RestClientError> {
        let token = self
            .session_token
            .as_ref()
            .ok_or_else(|| RestClientError::NotLoggedIn {
                location: Location::new(file!(), line!(), column!()),
            })?;

        let query_submission_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let query_request = QueryRequestWasm {
            sql_text: sql.to_string(),
            async_exec,
            sequence_id: 1,
            query_submission_time,
            is_internal: false,
            describe_only: None,
            parameters: None,
            bindings,
            bind_stage: None,
            query_context: QueryContextWasm::default(),
        };

        let request_json =
            serde_json::to_string(&query_request).map_err(|e| RestClientError::Parse {
                message: format!("Failed to serialize query request: {}", e),
                location: Location::new(file!(), line!(), column!()),
            })?;

        // Generate unique request IDs
        let request_id = format!(
            "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
            rand::random::<u32>(),
            rand::random::<u16>(),
            rand::random::<u16>(),
            rand::random::<u16>(),
            rand::random::<u64>() & 0xffffffffffff
        );

        let url = format!("{}/queries/v1/query-request", self.base_url);
        let request = HttpRequest::post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/snowflake")
            .header("User-Agent", "PythonConnector/3.15.0 (Darwin) Rust/WASM")
            .header("Authorization", format!("Snowflake Token=\"{}\"", token))
            .query("requestId", &request_id)
            .query("request_guid", &request_id)
            .body(request_json.into_bytes());

        let response =
            self.http_client
                .request(request)
                .await
                .map_err(|e| RestClientError::Http {
                    message: format!("Query request failed: {}", e),
                    location: Location::new(file!(), line!(), column!()),
                })?;

        if !response.status.is_success() {
            return Err(RestClientError::Query {
                message: format!("HTTP error: {}", response.status),
                location: Location::new(file!(), line!(), column!()),
            });
        }

        self.parse_query_response(response.body, &response.headers)
            .await
    }

    /// Execute an async query with polling.
    async fn execute_async_query(
        &self,
        sql: &str,
        bindings: Option<HashMap<String, BindParameter>>,
    ) -> Result<QueryResponse, RestClientError> {
        // Submit the query with async_exec=true
        let response = self.execute_query_internal(sql, bindings, true).await?;

        // Check if we need to poll for results
        if let Some(ref data) = response.data
            && let Some(ref get_result_url) = data.get_result_url
        {
            // Poll until complete
            return self.poll_for_result(get_result_url).await;
        }

        // Query completed immediately
        Ok(response)
    }

    /// Poll for async query results.
    async fn poll_for_result(&self, result_url: &str) -> Result<QueryResponse, RestClientError> {
        let token = self
            .session_token
            .as_ref()
            .ok_or_else(|| RestClientError::NotLoggedIn {
                location: Location::new(file!(), line!(), column!()),
            })?;

        // Polling delays: start short, then exponentially increase
        let delays = [
            5, 10, 20, 40, 80, 160, 320, 640, 1000, 2000, 3000, 4000, 5000,
        ];
        let mut attempt = 0;
        const MAX_ATTEMPTS: usize = 100;

        loop {
            if attempt >= MAX_ATTEMPTS {
                return Err(RestClientError::Query {
                    message: "Async query polling timeout".to_string(),
                    location: Location::new(file!(), line!(), column!()),
                });
            }

            // Wait before polling (except for first attempt)
            if attempt > 0 {
                let delay_ms = delays
                    .get(attempt.min(delays.len() - 1))
                    .copied()
                    .unwrap_or(5000);
                // Simple delay using a busy loop with time check
                let start = SystemTime::now();
                let delay_duration = std::time::Duration::from_millis(delay_ms as u64);
                loop {
                    if SystemTime::now()
                        .duration_since(start)
                        .map(|d| d >= delay_duration)
                        .unwrap_or(true)
                    {
                        break;
                    }
                }
            }

            let url = format!("{}{}", self.base_url, result_url);
            let request = HttpRequest::get(&url)
                .header("Accept", "application/snowflake")
                .header("User-Agent", "PythonConnector/3.15.0 (Darwin) Rust/WASM")
                .header("Authorization", format!("Snowflake Token=\"{}\"", token));

            let response =
                self.http_client
                    .request(request)
                    .await
                    .map_err(|e| RestClientError::Http {
                        message: format!("Poll request failed: {}", e),
                        location: Location::new(file!(), line!(), column!()),
                    })?;

            // Status code 202 means still running
            if response.status.as_u16() == 202 {
                attempt += 1;
                continue;
            }

            if !response.status.is_success() {
                return Err(RestClientError::Query {
                    message: format!("Poll HTTP error: {}", response.status),
                    location: Location::new(file!(), line!(), column!()),
                });
            }

            return self
                .parse_query_response(response.body, &response.headers)
                .await;
        }
    }

    /// Parse a query response body.
    async fn parse_query_response(
        &self,
        body: Vec<u8>,
        headers: &HashMap<String, String>,
    ) -> Result<QueryResponse, RestClientError> {
        // Handle chunked transfer encoding
        let is_chunked = headers
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("transfer-encoding") && v.contains("chunked"));

        let body = if is_chunked {
            dechunk_body(&body)?
        } else {
            body
        };

        // Handle gzip encoding
        let is_gzip = headers
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("content-encoding") && v.contains("gzip"));

        let body = if is_gzip {
            use std::io::Read;
            let mut decoder = flate2::read::GzDecoder::new(&body[..]);
            let mut decompressed = Vec::new();
            decoder
                .read_to_end(&mut decompressed)
                .map_err(|e| RestClientError::Parse {
                    message: format!("Failed to decompress response: {}", e),
                    location: Location::new(file!(), line!(), column!()),
                })?;
            decompressed
        } else {
            body
        };

        serde_json::from_slice(&body).map_err(|e| RestClientError::Parse {
            message: format!("Failed to parse query response: {}", e),
            location: Location::new(file!(), line!(), column!()),
        })
    }
}

fn build_auth_request(params: &LoginParameters, credentials: &Credentials) -> AuthRequestWasm {
    let mut data = AuthRequestDataWasm {
        // Use full account name (same as native client)
        account_name: params.account_name.clone(),
        client_app_id: params.client_info.application.clone(),
        client_app_version: params.client_info.version.clone(),
        client_environment: ClientEnvironmentWasm {
            application: params.client_info.application.clone(),
            os: params.client_info.os.clone(),
            os_version: params.client_info.os_version.clone(),
        },
        ..Default::default()
    };

    match credentials {
        Credentials::Password { username, password } => {
            data.login_name = Some(username.clone());
            data.password = Some(password.clone());
            data.authenticator = Some("SNOWFLAKE".to_string());
        }
        Credentials::Jwt { username, token } => {
            data.login_name = Some(username.clone());
            data.token = Some(token.clone());
            data.authenticator = Some("SNOWFLAKE_JWT".to_string());
        }
        Credentials::Pat { username, token } => {
            data.login_name = Some(username.clone());
            data.token = Some(token.clone());
            data.authenticator = Some("PROGRAMMATIC_ACCESS_TOKEN".to_string());
        }
    }

    AuthRequestWasm { data }
}

// Internal types for WASM client

#[derive(Serialize)]
struct AuthRequestWasm {
    data: AuthRequestDataWasm,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
struct AuthRequestDataWasm {
    account_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    login_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    authenticator: Option<String>,
    client_app_id: String,
    client_app_version: String,
    client_environment: ClientEnvironmentWasm,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
struct ClientEnvironmentWasm {
    application: String,
    os: String,
    os_version: String,
}

// Helper for deserializing codes that may be strings or numbers
fn deserialize_code<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de;

    struct CodeVisitor;

    impl<'de> de::Visitor<'de> for CodeVisitor {
        type Value = Option<i64>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a number or string representing a number")
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> {
            Ok(Some(v))
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
            Ok(Some(v as i64))
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            v.parse::<i64>().map(Some).map_err(de::Error::custom)
        }

        fn visit_none<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
    }

    deserializer.deserialize_any(CodeVisitor)
}

#[derive(Deserialize)]
struct AuthResponseWasm {
    success: bool,
    message: Option<String>,
    #[serde(default, deserialize_with = "deserialize_code")]
    code: Option<i64>,
    data: Option<AuthResponseDataWasm>,
}

#[derive(Deserialize)]
struct AuthResponseDataWasm {
    token: Option<String>,
    #[serde(rename = "masterToken")]
    #[allow(dead_code)]
    master_token: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct QueryRequestWasm {
    sql_text: String,
    async_exec: bool,
    sequence_id: i64,
    query_submission_time: i64,
    is_internal: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    describe_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parameters: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bindings: Option<HashMap<String, BindParameter>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bind_stage: Option<String>,
    #[serde(rename = "queryContextDTO")]
    query_context: QueryContextWasm,
}

#[derive(Serialize, Default)]
struct QueryContextWasm {
    #[serde(skip_serializing_if = "Option::is_none")]
    entries: Option<Vec<serde_json::Value>>,
}

/// Decode chunked transfer encoding.
fn dechunk_body(data: &[u8]) -> Result<Vec<u8>, RestClientError> {
    let mut result = Vec::new();
    let mut pos = 0;

    while pos < data.len() {
        // Find the chunk size line (ends with \r\n)
        let line_end = data[pos..]
            .windows(2)
            .position(|w| w == b"\r\n")
            .ok_or_else(|| RestClientError::Parse {
                message: "Invalid chunked encoding: missing chunk size".to_string(),
                location: Location::new(file!(), line!(), column!()),
            })?;

        let size_str = std::str::from_utf8(&data[pos..pos + line_end]).map_err(|_| {
            RestClientError::Parse {
                message: "Invalid chunked encoding: non-UTF8 size".to_string(),
                location: Location::new(file!(), line!(), column!()),
            }
        })?;

        // Parse chunk size (hex)
        let chunk_size =
            usize::from_str_radix(size_str.trim(), 16).map_err(|_| RestClientError::Parse {
                message: format!("Invalid chunked encoding: bad size '{}'", size_str),
                location: Location::new(file!(), line!(), column!()),
            })?;

        // Move past the size line and \r\n
        pos += line_end + 2;

        if chunk_size == 0 {
            // End of chunks
            break;
        }

        // Read chunk data
        if pos + chunk_size > data.len() {
            return Err(RestClientError::Parse {
                message: "Invalid chunked encoding: truncated chunk".to_string(),
                location: Location::new(file!(), line!(), column!()),
            });
        }

        result.extend_from_slice(&data[pos..pos + chunk_size]);
        pos += chunk_size;

        // Skip trailing \r\n after chunk
        if pos + 2 <= data.len() && &data[pos..pos + 2] == b"\r\n" {
            pos += 2;
        }
    }

    Ok(result)
}
