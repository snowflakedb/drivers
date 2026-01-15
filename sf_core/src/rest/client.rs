//! Unified REST client trait for Snowflake API.
//!
//! This module provides a platform-agnostic trait for interacting with the
//! Snowflake REST API. Both native (reqwest) and WASM (portable HTTP) clients
//! implement this trait.

use crate::config::rest_parameters::LoginParameters;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use snafu::{Location, Snafu};
use std::collections::HashMap;

/// Unified error type for REST client operations.
#[derive(Debug, Snafu)]
pub enum RestClientError {
    #[snafu(display("Authentication failed: {message}"))]
    Authentication {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("HTTP error: {message}"))]
    Http {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to parse response: {message}"))]
    Parse {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Login failed: {message} (code: {code})"))]
    LoginError {
        message: String,
        code: i64,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Query failed: {message}"))]
    Query {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Session expired"))]
    SessionExpired {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Not logged in"))]
    NotLoggedIn {
        #[snafu(implicit)]
        location: Location,
    },
}

/// Query execution mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum QueryExecutionMode {
    #[default]
    Blocking,
    Async,
}

/// Trait for Snowflake REST client implementations.
///
/// This trait abstracts the platform-specific details of making HTTP requests
/// to the Snowflake REST API, allowing the same API code to work with both
/// native (reqwest) and WASM (portable HTTP) clients.
#[async_trait]
pub trait SnowflakeRestClient: Send + Sync {
    /// Login to Snowflake and establish a session.
    ///
    /// Returns the session token on success.
    async fn login(&mut self, params: &LoginParameters) -> Result<String, RestClientError>;

    /// Execute a SQL query (synchronous mode).
    ///
    /// Returns the query response containing results or metadata.
    async fn query(
        &self,
        sql: &str,
        bindings: Option<HashMap<String, BindParameter>>,
    ) -> Result<QueryResponse, RestClientError>;

    /// Execute a SQL query with specified execution mode.
    ///
    /// For async mode, submits the query and polls until complete.
    async fn query_with_mode(
        &self,
        sql: &str,
        bindings: Option<HashMap<String, BindParameter>>,
        mode: QueryExecutionMode,
    ) -> Result<QueryResponse, RestClientError> {
        // Default implementation just uses sync query
        let _ = mode;
        self.query(sql, bindings).await
    }

    /// Download a result chunk from a presigned S3 URL.
    ///
    /// Returns the raw (decompressed) Arrow IPC data.
    async fn download_chunk(
        &self,
        url: &str,
        headers: &HashMap<String, String>,
    ) -> Result<Vec<u8>, RestClientError>;

    /// Get the session token, if logged in.
    fn session_token(&self) -> Option<&str>;

    /// Check if the client has an active session.
    fn is_logged_in(&self) -> bool {
        self.session_token().is_some()
    }
}

/// Parameter binding for prepared statements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindParameter {
    #[serde(rename = "type")]
    pub data_type: String,
    pub value: serde_json::Value,
}

/// Response from a query request.
#[derive(Debug, Deserialize, Clone)]
pub struct QueryResponse {
    pub success: bool,
    pub message: Option<String>,
    pub code: Option<String>,
    pub data: Option<QueryResponseData>,
}

/// Query response data.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QueryResponseData {
    pub parameters: Option<serde_json::Value>,
    pub rowtype: Option<Vec<RowType>>,
    pub rowset: Option<Vec<Vec<serde_json::Value>>>,
    #[serde(rename = "rowsetBase64")]
    pub rowset_base64: Option<String>,
    pub total: Option<i64>,
    pub returned: Option<i64>,
    pub query_id: Option<String>,
    pub database_provider: Option<String>,
    pub final_database_name: Option<String>,
    pub final_schema_name: Option<String>,
    pub final_warehouse_name: Option<String>,
    pub final_role_name: Option<String>,
    pub statement_type_id: Option<i64>,
    pub query_result_format: Option<String>,
    /// Result chunks for large results
    pub chunks: Option<Vec<ChunkInfo>>,
    /// Headers required to download chunks (S3 presigned URL auth)
    #[serde(rename = "chunkHeaders")]
    pub chunk_headers: Option<std::collections::HashMap<String, String>>,
    /// Command type (e.g., "SELECT", "UPLOAD", "DOWNLOAD")
    pub command: Option<String>,
    /// URL for async query results
    #[serde(rename = "getResultUrl")]
    pub get_result_url: Option<String>,
}

/// Chunk information for large result sets.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChunkInfo {
    pub url: String,
    pub row_count: i64,
    pub uncompressed_size: Option<i64>,
    pub compressed_size: Option<i64>,
}

/// Row type metadata.
#[derive(Debug, Deserialize, Clone)]
pub struct RowType {
    pub name: String,
    #[serde(rename = "type")]
    pub data_type: String,
    pub nullable: bool,
    pub precision: Option<i64>,
    pub scale: Option<i64>,
    pub length: Option<i64>,
    #[serde(rename = "byteLength")]
    pub byte_length: Option<i64>,
}
