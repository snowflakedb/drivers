//! Native REST client using reqwest.
//!
//! This module provides the native (non-WASM) implementation of the
//! `SnowflakeRestClient` trait using reqwest with rustls.

use super::client::{BindParameter, QueryResponse, RestClientError, SnowflakeRestClient};
use crate::auth::{Credentials, create_credentials};
use crate::config::rest_parameters::{ClientInfo, LoginParameters};
use crate::rest::snowflake::auth::{AuthRequest, AuthRequestClientEnvironment, AuthRequestData};
use crate::tls::client::create_tls_client_with_config;
use async_trait::async_trait;
use reqwest::header;
use snafu::Location;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Native REST client using reqwest.
pub struct NativeRestClient {
    client: Option<reqwest::Client>,
    session_token: Option<String>,
    base_url: String,
}

impl NativeRestClient {
    /// Create a new native REST client.
    pub fn new(base_url: &str) -> Self {
        Self {
            client: None,
            session_token: None,
            base_url: base_url.to_string(),
        }
    }

    /// Get or create the HTTP client.
    fn get_or_create_client(
        &mut self,
        client_info: &ClientInfo,
    ) -> Result<&reqwest::Client, RestClientError> {
        if self.client.is_none() {
            let client =
                create_tls_client_with_config(client_info.tls_config.clone()).map_err(|e| {
                    RestClientError::Http {
                        message: format!("Failed to create TLS client: {}", e),
                        location: Location::new(file!(), line!(), column!()),
                    }
                })?;
            self.client = Some(client);
        }
        Ok(self.client.as_ref().unwrap())
    }

    fn user_agent(client_info: &ClientInfo) -> String {
        format!(
            "{}/{} ({}) CPython/3.11.6",
            client_info.application, client_info.version, client_info.os
        )
    }
}

#[async_trait]
impl SnowflakeRestClient for NativeRestClient {
    async fn login(&mut self, params: &LoginParameters) -> Result<String, RestClientError> {
        // Clone base_url before borrowing self mutably
        let base_url = self.base_url.clone();
        let client = self.get_or_create_client(&params.client_info)?;

        // Build auth request data
        let credentials =
            create_credentials(params).map_err(|e| RestClientError::Authentication {
                message: e.to_string(),
                location: Location::new(file!(), line!(), column!()),
            })?;

        let mut data = AuthRequestData {
            account_name: params.account_name.clone(),
            client_app_id: params.client_info.application.clone(),
            client_app_version: params.client_info.version.clone(),
            client_environment: AuthRequestClientEnvironment {
                application: params.client_info.application.clone(),
                os: params.client_info.os.clone(),
                os_version: params.client_info.os_version.clone(),
                ocsp_mode: params.client_info.ocsp_mode.clone(),
                python_version: Some("3.11.6".to_string()),
                python_runtime: Some("CPython".to_string()),
                python_compiler: Some("Clang 13.0.0 (clang-1300.0.29.30)".to_string()),
            },
            ..Default::default()
        };

        match credentials {
            Credentials::Password { username, password } => {
                data.login_name = Some(username);
                data.password = Some(password);
                data.authenticator = Some("SNOWFLAKE".to_string());
            }
            Credentials::Jwt { username, token } => {
                data.login_name = Some(username);
                data.token = Some(token);
                data.authenticator = Some("SNOWFLAKE_JWT".to_string());
            }
            Credentials::Pat { username, token } => {
                data.login_name = Some(username);
                data.token = Some(token);
                data.authenticator = Some("PROGRAMMATIC_ACCESS_TOKEN".to_string());
            }
        }

        let login_request = AuthRequest { data };
        let login_url = format!("{}/session/v1/login-request", base_url);

        let request = client
            .post(&login_url)
            .query(&[
                (
                    "databaseName",
                    params.database.as_deref().unwrap_or_default(),
                ),
                ("schemaName", params.schema.as_deref().unwrap_or_default()),
                ("warehouse", params.warehouse.as_deref().unwrap_or_default()),
                ("roleName", params.role.as_deref().unwrap_or_default()),
            ])
            .json(&login_request)
            .header("accept", "application/snowflake")
            .header("User-Agent", Self::user_agent(&params.client_info))
            .header("Authorization", "Snowflake Token=\"None\"")
            .build()
            .map_err(|e| RestClientError::Http {
                message: format!("Failed to build login request: {}", e),
                location: Location::new(file!(), line!(), column!()),
            })?;

        let response = client
            .execute(request)
            .await
            .map_err(|e| RestClientError::Http {
                message: format!("Login request failed: {}", e),
                location: Location::new(file!(), line!(), column!()),
            })?;

        let status = response.status();
        let text = response.text().await.map_err(|e| RestClientError::Parse {
            message: format!("Failed to read response: {}", e),
            location: Location::new(file!(), line!(), column!()),
        })?;

        if !status.is_success() {
            if status == reqwest::StatusCode::UNAUTHORIZED {
                return Err(RestClientError::SessionExpired {
                    location: Location::new(file!(), line!(), column!()),
                });
            }
            return Err(RestClientError::LoginError {
                message: format!("HTTP {}: {}", status, text),
                code: status.as_u16() as i64,
                location: Location::new(file!(), line!(), column!()),
            });
        }

        let auth_response: AuthResponseNative =
            serde_json::from_str(&text).map_err(|e| RestClientError::Parse {
                message: format!("Failed to parse login response: {}", e),
                location: Location::new(file!(), line!(), column!()),
            })?;

        if !auth_response.success {
            let message = auth_response.message.unwrap_or_default();
            let code = auth_response
                .code
                .and_then(|c| c.parse::<i64>().ok())
                .unwrap_or(0);
            return Err(RestClientError::LoginError {
                message,
                code,
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
        let token = self
            .session_token
            .as_ref()
            .ok_or_else(|| RestClientError::NotLoggedIn {
                location: Location::new(file!(), line!(), column!()),
            })?;

        let client = self.client.as_ref().ok_or_else(|| RestClientError::Http {
            message: "HTTP client not initialized".to_string(),
            location: Location::new(file!(), line!(), column!()),
        })?;

        let query_submission_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let query_request = QueryRequestNative {
            sql_text: sql.to_string(),
            async_exec: false,
            sequence_id: 1,
            query_submission_time,
            is_internal: false,
            describe_only: None,
            parameters: None,
            bindings,
            bind_stage: None,
            query_context: QueryContextNative { entries: None },
        };

        let query_url = format!("{}/queries/v1/query-request", self.base_url);
        let request_id = uuid::Uuid::new_v4().to_string();

        let request = client
            .post(&query_url)
            .query(&[
                ("requestId", request_id.clone()),
                ("request_guid", request_id),
            ])
            .json(&query_request)
            .header(
                header::AUTHORIZATION,
                format!("Snowflake Token=\"{}\"", token),
            )
            .header(header::ACCEPT, "application/json")
            .header(header::CONTENT_TYPE, "application/json")
            .header("User-Agent", "PythonConnector/3.15.0 (Darwin) Rust/Native")
            .build()
            .map_err(|e| RestClientError::Http {
                message: format!("Failed to build query request: {}", e),
                location: Location::new(file!(), line!(), column!()),
            })?;

        let response = client
            .execute(request)
            .await
            .map_err(|e| RestClientError::Http {
                message: format!("Query request failed: {}", e),
                location: Location::new(file!(), line!(), column!()),
            })?;

        let status = response.status();
        let text = response.text().await.map_err(|e| RestClientError::Parse {
            message: format!("Failed to read response: {}", e),
            location: Location::new(file!(), line!(), column!()),
        })?;

        if !status.is_success() {
            if status == reqwest::StatusCode::UNAUTHORIZED {
                return Err(RestClientError::SessionExpired {
                    location: Location::new(file!(), line!(), column!()),
                });
            }
            return Err(RestClientError::Query {
                message: format!("HTTP {}: {}", status, text),
                location: Location::new(file!(), line!(), column!()),
            });
        }

        serde_json::from_str(&text).map_err(|e| RestClientError::Parse {
            message: format!("Failed to parse query response: {}", e),
            location: Location::new(file!(), line!(), column!()),
        })
    }

    async fn download_chunk(
        &self,
        url: &str,
        headers: &HashMap<String, String>,
    ) -> Result<Vec<u8>, RestClientError> {
        let client = self.client.as_ref().ok_or_else(|| RestClientError::Http {
            message: "HTTP client not initialized".to_string(),
            location: Location::new(file!(), line!(), column!()),
        })?;

        let mut request_builder = client.get(url);
        for (key, value) in headers {
            request_builder = request_builder.header(key, value);
        }

        let response = request_builder
            .send()
            .await
            .map_err(|e| RestClientError::Http {
                message: format!("Chunk download failed: {}", e),
                location: Location::new(file!(), line!(), column!()),
            })?;

        let status = response.status();
        if !status.is_success() {
            return Err(RestClientError::Http {
                message: format!("Chunk download returned HTTP {}", status),
                location: Location::new(file!(), line!(), column!()),
            });
        }

        // Check for gzip encoding
        let is_gzip = response
            .headers()
            .get(header::CONTENT_ENCODING)
            .map(|v| v.to_str().map(|s| s.contains("gzip")).unwrap_or(false))
            .unwrap_or(false);

        let bytes = response.bytes().await.map_err(|e| RestClientError::Http {
            message: format!("Failed to read chunk data: {}", e),
            location: Location::new(file!(), line!(), column!()),
        })?;

        let data = if is_gzip {
            use std::io::Read;
            let mut decoder = flate2::read::GzDecoder::new(&bytes[..]);
            let mut decompressed = Vec::new();
            decoder
                .read_to_end(&mut decompressed)
                .map_err(|e| RestClientError::Http {
                    message: format!("Failed to decompress chunk: {}", e),
                    location: Location::new(file!(), line!(), column!()),
                })?;
            decompressed
        } else {
            bytes.to_vec()
        };

        Ok(data)
    }

    fn session_token(&self) -> Option<&str> {
        self.session_token.as_deref()
    }
}

// Internal types for native client

#[derive(serde::Deserialize)]
struct AuthResponseNative {
    success: bool,
    message: Option<String>,
    #[serde(rename = "_code")]
    code: Option<String>,
    data: Option<AuthResponseDataNative>,
}

#[derive(serde::Deserialize)]
struct AuthResponseDataNative {
    token: Option<String>,
    #[serde(rename = "masterToken")]
    #[allow(dead_code)]
    master_token: Option<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct QueryRequestNative {
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
    query_context: QueryContextNative,
}

#[derive(serde::Serialize)]
struct QueryContextNative {
    #[serde(skip_serializing_if = "Option::is_none")]
    entries: Option<Vec<serde_json::Value>>,
}
