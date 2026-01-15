//! HTTP client abstraction for platform-independent HTTP operations.
//!
//! This module provides traits and types for HTTP requests that can be implemented
//! by different backends (reqwest for native builds, wasi-http for WASM).

use snafu::{Location, Snafu};
use std::collections::HashMap;
use std::time::Duration;

/// HTTP methods supported by the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
    Head,
    Patch,
    Options,
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Method::Get => write!(f, "GET"),
            Method::Post => write!(f, "POST"),
            Method::Put => write!(f, "PUT"),
            Method::Delete => write!(f, "DELETE"),
            Method::Head => write!(f, "HEAD"),
            Method::Patch => write!(f, "PATCH"),
            Method::Options => write!(f, "OPTIONS"),
        }
    }
}

/// HTTP status code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusCode(pub u16);

impl StatusCode {
    pub const OK: StatusCode = StatusCode(200);
    pub const CREATED: StatusCode = StatusCode(201);
    pub const NO_CONTENT: StatusCode = StatusCode(204);
    pub const BAD_REQUEST: StatusCode = StatusCode(400);
    pub const UNAUTHORIZED: StatusCode = StatusCode(401);
    pub const FORBIDDEN: StatusCode = StatusCode(403);
    pub const NOT_FOUND: StatusCode = StatusCode(404);
    pub const REQUEST_TIMEOUT: StatusCode = StatusCode(408);
    pub const TOO_MANY_REQUESTS: StatusCode = StatusCode(429);
    pub const INTERNAL_SERVER_ERROR: StatusCode = StatusCode(500);
    pub const BAD_GATEWAY: StatusCode = StatusCode(502);
    pub const SERVICE_UNAVAILABLE: StatusCode = StatusCode(503);
    pub const GATEWAY_TIMEOUT: StatusCode = StatusCode(504);

    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.0)
    }

    pub fn is_server_error(&self) -> bool {
        (500..600).contains(&self.0)
    }

    pub fn is_client_error(&self) -> bool {
        (400..500).contains(&self.0)
    }

    pub fn as_u16(&self) -> u16 {
        self.0
    }
}

impl std::fmt::Display for StatusCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// HTTP headers as key-value pairs.
pub type Headers = HashMap<String, String>;

/// An HTTP request.
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: Method,
    pub url: String,
    pub headers: Headers,
    pub query_params: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
    pub timeout: Option<Duration>,
}

impl HttpRequest {
    /// Create a new GET request.
    pub fn get(url: impl Into<String>) -> Self {
        Self {
            method: Method::Get,
            url: url.into(),
            headers: HashMap::new(),
            query_params: Vec::new(),
            body: None,
            timeout: None,
        }
    }

    /// Create a new POST request.
    pub fn post(url: impl Into<String>) -> Self {
        Self {
            method: Method::Post,
            url: url.into(),
            headers: HashMap::new(),
            query_params: Vec::new(),
            body: None,
            timeout: None,
        }
    }

    /// Create a new PUT request.
    pub fn put(url: impl Into<String>) -> Self {
        Self {
            method: Method::Put,
            url: url.into(),
            headers: HashMap::new(),
            query_params: Vec::new(),
            body: None,
            timeout: None,
        }
    }

    /// Create a new DELETE request.
    pub fn delete(url: impl Into<String>) -> Self {
        Self {
            method: Method::Delete,
            url: url.into(),
            headers: HashMap::new(),
            query_params: Vec::new(),
            body: None,
            timeout: None,
        }
    }

    /// Create a new HEAD request.
    pub fn head(url: impl Into<String>) -> Self {
        Self {
            method: Method::Head,
            url: url.into(),
            headers: HashMap::new(),
            query_params: Vec::new(),
            body: None,
            timeout: None,
        }
    }

    /// Add a header to the request.
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Add a query parameter to the request.
    pub fn query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.query_params.push((key.into(), value.into()));
        self
    }

    /// Set the request body.
    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Set the request body as JSON.
    pub fn json<T: serde::Serialize>(mut self, value: &T) -> Result<Self, HttpClientError> {
        let json = serde_json::to_vec(value).map_err(|e| HttpClientError::Serialization {
            message: e.to_string(),
            location: snafu::Location::new(file!(), line!(), column!()),
        })?;
        self.body = Some(json);
        self.headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        Ok(self)
    }

    /// Set request timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

/// An HTTP response.
#[derive(Debug)]
pub struct HttpResponse {
    pub status: StatusCode,
    pub headers: Headers,
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// Get the response body as text.
    pub fn text(&self) -> Result<String, HttpClientError> {
        String::from_utf8(self.body.clone()).map_err(|e| HttpClientError::ResponseParse {
            message: e.to_string(),
            location: snafu::Location::new(file!(), line!(), column!()),
        })
    }

    /// Parse the response body as JSON.
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T, HttpClientError> {
        serde_json::from_slice(&self.body).map_err(|e| HttpClientError::ResponseParse {
            message: e.to_string(),
            location: snafu::Location::new(file!(), line!(), column!()),
        })
    }

    /// Get a header value.
    pub fn header(&self, key: &str) -> Option<&String> {
        // Header lookup is case-insensitive
        let key_lower = key.to_lowercase();
        self.headers
            .iter()
            .find(|(k, _)| k.to_lowercase() == key_lower)
            .map(|(_, v)| v)
    }
}

/// Errors that can occur during HTTP operations.
#[derive(Debug, Snafu)]
pub enum HttpClientError {
    #[snafu(display("Connection failed: {message}"))]
    Connection {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Request timed out"))]
    Timeout {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to build request: {message}"))]
    RequestBuild {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to serialize request body: {message}"))]
    Serialization {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to parse response: {message}"))]
    ResponseParse {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("TLS error: {message}"))]
    Tls {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("HTTP error {status}: {message}"))]
    HttpStatus {
        status: StatusCode,
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("I/O error: {message}"))]
    Io {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },
}

/// Trait for HTTP clients.
///
/// This trait abstracts over different HTTP client implementations,
/// allowing the same code to work with reqwest (native) or wasi-http (WASM).
pub trait HttpClient: Send + Sync {
    /// Send an HTTP request and return the response.
    fn request(
        &self,
        request: HttpRequest,
    ) -> impl std::future::Future<Output = Result<HttpResponse, HttpClientError>> + Send;
}

/// Configuration for creating an HTTP client.
#[derive(Debug, Clone, Default)]
pub struct HttpClientConfig {
    /// Connection timeout.
    pub connect_timeout: Option<Duration>,
    /// Request timeout.
    pub request_timeout: Option<Duration>,
    /// Whether to verify TLS certificates.
    pub verify_certificates: bool,
    /// Custom CA certificate (PEM format).
    pub ca_cert: Option<Vec<u8>>,
    /// Enable connection pooling.
    pub pool_connections: bool,
    /// Maximum idle connections per host.
    pub pool_max_idle_per_host: Option<usize>,
}

impl HttpClientConfig {
    pub fn new() -> Self {
        Self {
            verify_certificates: true,
            pool_connections: true,
            ..Default::default()
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = Some(timeout);
        self
    }

    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = Some(timeout);
        self
    }

    pub fn insecure(mut self) -> Self {
        self.verify_certificates = false;
        self
    }
}
