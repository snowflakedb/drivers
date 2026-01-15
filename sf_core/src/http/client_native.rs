//! Native HTTP client implementation using reqwest.
//!
//! This module provides the reqwest-based HTTP client for native builds.

use super::client::{
    HttpClient, HttpClientConfig, HttpClientError, HttpRequest, HttpResponse, Method, StatusCode,
};
use std::time::Duration;

/// Native HTTP client using reqwest.
pub struct NativeHttpClient {
    client: reqwest::Client,
}

impl NativeHttpClient {
    /// Create a new HTTP client with default configuration.
    pub fn new() -> Result<Self, HttpClientError> {
        Self::with_config(HttpClientConfig::new())
    }

    /// Create a new HTTP client with custom configuration.
    pub fn with_config(config: HttpClientConfig) -> Result<Self, HttpClientError> {
        let mut builder = reqwest::Client::builder();

        if let Some(timeout) = config.connect_timeout {
            builder = builder.connect_timeout(timeout);
        }

        if let Some(timeout) = config.request_timeout {
            builder = builder.timeout(timeout);
        }

        if !config.verify_certificates {
            builder = builder
                .danger_accept_invalid_certs(true)
                .danger_accept_invalid_hostnames(true);
        }

        if config.pool_connections {
            builder = builder.pool_idle_timeout(Some(Duration::from_secs(30)));
            if let Some(max_idle) = config.pool_max_idle_per_host {
                builder = builder.pool_max_idle_per_host(max_idle);
            }
        }

        let client = builder.build().map_err(|e| HttpClientError::Connection {
            message: e.to_string(),
            location: snafu::Location::new(file!(), line!(), column!()),
        })?;

        Ok(Self { client })
    }

    /// Create from an existing reqwest client.
    pub fn from_reqwest(client: reqwest::Client) -> Self {
        Self { client }
    }

    /// Get a reference to the underlying reqwest client.
    pub fn inner(&self) -> &reqwest::Client {
        &self.client
    }
}

impl Default for NativeHttpClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default HTTP client")
    }
}

impl HttpClient for NativeHttpClient {
    async fn request(&self, request: HttpRequest) -> Result<HttpResponse, HttpClientError> {
        // Convert method
        let method = match request.method {
            Method::Get => reqwest::Method::GET,
            Method::Post => reqwest::Method::POST,
            Method::Put => reqwest::Method::PUT,
            Method::Delete => reqwest::Method::DELETE,
            Method::Head => reqwest::Method::HEAD,
            Method::Patch => reqwest::Method::PATCH,
            Method::Options => reqwest::Method::OPTIONS,
        };

        // Build the request
        let mut builder = self.client.request(method, &request.url);

        // Add query parameters
        if !request.query_params.is_empty() {
            builder = builder.query(&request.query_params);
        }

        // Add headers
        for (key, value) in &request.headers {
            builder = builder.header(key, value);
        }

        // Add body
        if let Some(body) = request.body {
            builder = builder.body(body);
        }

        // Set timeout
        if let Some(timeout) = request.timeout {
            builder = builder.timeout(timeout);
        }

        // Send the request
        let response = builder.send().await.map_err(|e| {
            if e.is_timeout() {
                HttpClientError::Timeout {
                    location: snafu::Location::new(file!(), line!(), column!()),
                }
            } else if e.is_connect() {
                HttpClientError::Connection {
                    message: e.to_string(),
                    location: snafu::Location::new(file!(), line!(), column!()),
                }
            } else if e.is_builder() {
                HttpClientError::RequestBuild {
                    message: e.to_string(),
                    location: snafu::Location::new(file!(), line!(), column!()),
                }
            } else {
                HttpClientError::Connection {
                    message: e.to_string(),
                    location: snafu::Location::new(file!(), line!(), column!()),
                }
            }
        })?;

        // Convert response
        let status = StatusCode(response.status().as_u16());

        // Extract headers
        let mut headers = std::collections::HashMap::new();
        for (key, value) in response.headers() {
            if let Ok(v) = value.to_str() {
                headers.insert(key.to_string(), v.to_string());
            }
        }

        // Read body
        let body = response.bytes().await.map_err(|e| HttpClientError::Io {
            message: e.to_string(),
            location: snafu::Location::new(file!(), line!(), column!()),
        })?;

        Ok(HttpResponse {
            status,
            headers,
            body: body.to_vec(),
        })
    }
}

/// Convert from our Method to reqwest::Method
impl From<Method> for reqwest::Method {
    fn from(method: Method) -> Self {
        match method {
            Method::Get => reqwest::Method::GET,
            Method::Post => reqwest::Method::POST,
            Method::Put => reqwest::Method::PUT,
            Method::Delete => reqwest::Method::DELETE,
            Method::Head => reqwest::Method::HEAD,
            Method::Patch => reqwest::Method::PATCH,
            Method::Options => reqwest::Method::OPTIONS,
        }
    }
}

/// Convert from reqwest::Method to our Method
impl From<reqwest::Method> for Method {
    fn from(method: reqwest::Method) -> Self {
        match method {
            reqwest::Method::GET => Method::Get,
            reqwest::Method::POST => Method::Post,
            reqwest::Method::PUT => Method::Put,
            reqwest::Method::DELETE => Method::Delete,
            reqwest::Method::HEAD => Method::Head,
            reqwest::Method::PATCH => Method::Patch,
            reqwest::Method::OPTIONS => Method::Options,
            _ => Method::Get, // Default fallback
        }
    }
}

/// Convert from reqwest::StatusCode to our StatusCode
impl From<reqwest::StatusCode> for StatusCode {
    fn from(status: reqwest::StatusCode) -> Self {
        StatusCode(status.as_u16())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = NativeHttpClient::new();
        assert!(client.is_ok());
    }

    #[test]
    fn test_client_with_config() {
        let config = HttpClientConfig::new()
            .with_timeout(Duration::from_secs(30))
            .with_connect_timeout(Duration::from_secs(10));
        let client = NativeHttpClient::with_config(config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_method_conversion() {
        assert_eq!(reqwest::Method::from(Method::Get), reqwest::Method::GET);
        assert_eq!(reqwest::Method::from(Method::Post), reqwest::Method::POST);
        assert_eq!(reqwest::Method::from(Method::Put), reqwest::Method::PUT);
        assert_eq!(
            reqwest::Method::from(Method::Delete),
            reqwest::Method::DELETE
        );
    }

    #[test]
    fn test_request_builder() {
        let request = HttpRequest::get("https://example.com")
            .header("Accept", "application/json")
            .query("key", "value")
            .timeout(Duration::from_secs(30));

        assert_eq!(request.method, Method::Get);
        assert_eq!(request.url, "https://example.com");
        assert_eq!(
            request.headers.get("Accept"),
            Some(&"application/json".to_string())
        );
        assert_eq!(request.query_params.len(), 1);
        assert_eq!(request.timeout, Some(Duration::from_secs(30)));
    }
}
