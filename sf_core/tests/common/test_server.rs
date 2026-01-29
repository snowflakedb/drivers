//! Common test server helpers for integration tests.
//!
//! This module provides reusable mock HTTP server implementations for testing
//! HTTP retry behavior, request verification, and response simulation.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Spawn a test server that responds to HTTP requests.
///
/// The responder receives the attempt number (1-based) and returns the raw HTTP response bytes.
/// Useful for testing retry behavior.
///
/// # Arguments
/// * `max_attempts` - Maximum number of requests to handle before the server stops
/// * `responder` - Async closure that takes the attempt number and returns response bytes
///
/// # Returns
/// * `SocketAddr` - The address the server is listening on
/// * `Arc<AtomicUsize>` - Counter for number of attempts made
/// * `JoinHandle` - Handle to the server task
///
/// # Example
/// ```ignore
/// let (addr, attempts, server) = spawn_test_server(2, |attempt| async move {
///     if attempt == 1 {
///         // First attempt: return 503
///         b"HTTP/1.1 503 Service Unavailable\r\n...".to_vec()
///     } else {
///         // Second attempt: success
///         b"HTTP/1.1 200 OK\r\n...".to_vec()
///     }
/// }).await;
/// ```
pub async fn spawn_test_server<F, Fut>(
    max_attempts: usize,
    responder: F,
) -> (SocketAddr, Arc<AtomicUsize>, tokio::task::JoinHandle<()>)
where
    F: Fn(usize) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Vec<u8>> + Send + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let attempts = Arc::new(AtomicUsize::new(0));
    let attempts_clone = attempts.clone();
    let responder = Arc::new(responder);

    let handle = tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            let attempt = attempts_clone.fetch_add(1, Ordering::SeqCst) + 1;
            let responder = responder.clone();

            // Read the request (we discard it, responder only cares about attempt number)
            let mut buf = [0u8; 2048];
            let _ = stream.read(&mut buf).await;

            let response = responder(attempt).await;
            stream.write_all(&response).await.unwrap();
            let _ = stream.shutdown().await;

            if attempt >= max_attempts {
                break;
            }
        }
    });

    (addr, attempts, handle)
}

/// Spawn a server that captures the full HTTP request for verification.
///
/// The server reads the full request and returns it, allowing tests to verify
/// request format, headers, body, etc.
///
/// # Arguments
/// * `response` - The raw HTTP response bytes to return for all requests
///
/// # Returns
/// * `SocketAddr` - The address the server is listening on
/// * `Arc<AtomicUsize>` - Counter for number of attempts made
/// * `JoinHandle<Vec<u8>>` - Handle that resolves to the captured request bytes
///
/// # Example
/// ```ignore
/// let (addr, attempts, server) = spawn_capture_server().await;
/// // ... make request to addr ...
/// let captured_request = server.await.unwrap();
/// assert!(captured_request.starts_with(b"POST /session"));
/// ```
pub async fn spawn_capture_server() -> (SocketAddr, Arc<AtomicUsize>, tokio::task::JoinHandle<Vec<u8>>)
{
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let attempts = Arc::new(AtomicUsize::new(0));
    let attempts_clone = attempts.clone();

    let handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        attempts_clone.fetch_add(1, Ordering::SeqCst);

        // Read the request
        let mut buf = vec![0u8; 4096];
        let n = stream.read(&mut buf).await.unwrap();
        buf.truncate(n);

        // Send success response
        let body = r#"{"success":true}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).await.unwrap();
        let _ = stream.shutdown().await;

        buf
    });

    (addr, attempts, handle)
}

/// Spawn a capture server with custom response.
///
/// Like `spawn_capture_server`, but allows specifying the response.
///
/// # Arguments
/// * `max_attempts` - Maximum number of requests to handle
/// * `responder` - Closure that takes the request string and returns response bytes
///
/// # Returns
/// * `SocketAddr` - The address the server is listening on
/// * `JoinHandle` - Handle to the server task
pub async fn spawn_capture_server_with_response<F>(
    max_attempts: usize,
    responder: F,
) -> (SocketAddr, tokio::task::JoinHandle<()>)
where
    F: Fn(String) -> Vec<u8> + Send + Sync + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let responder = Arc::new(responder);
    let attempt = Arc::new(AtomicUsize::new(0));

    let handle = tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            let current = attempt.fetch_add(1, Ordering::SeqCst) + 1;
            let responder = responder.clone();

            // Read request
            let mut buf = vec![0u8; 4096];
            let n = stream.read(&mut buf).await.unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]).to_string();

            let response = responder(request);
            let _ = stream.write_all(&response).await;
            let _ = stream.shutdown().await;

            if current >= max_attempts {
                break;
            }
        }
    });

    (addr, handle)
}

/// Helper to create a JSON HTTP 200 response.
pub fn json_response(body: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    )
    .into_bytes()
}

/// Helper to create a JSON HTTP error response.
pub fn json_error_response(status: u16, status_text: &str, body: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        status_text,
        body.len(),
        body
    )
    .into_bytes()
}

/// Helper to create a 503 Service Unavailable response with Retry-After header.
pub fn service_unavailable_response(body: &str, retry_after: u32) -> Vec<u8> {
    format!(
        "HTTP/1.1 503 Service Unavailable\r\nContent-Type: application/json\r\nContent-Length: {}\r\nRetry-After: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        retry_after,
        body
    )
    .into_bytes()
}

/// Extract a query parameter from an HTTP request string.
pub fn extract_query_param<'a>(request: &'a str, param: &str) -> Option<&'a str> {
    let search = format!("{}=", param);
    if let Some(start) = request.find(&search) {
        let value_start = start + search.len();
        let remaining = &request[value_start..];
        let end = remaining
            .find(['&', ' ', '\r', '\n'])
            .unwrap_or(remaining.len());
        Some(&remaining[..end])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_query_param() {
        let request = "GET /session?delete=true&requestId=abc123 HTTP/1.1\r\n";
        assert_eq!(extract_query_param(request, "delete"), Some("true"));
        assert_eq!(extract_query_param(request, "requestId"), Some("abc123"));
        assert_eq!(extract_query_param(request, "missing"), None);
    }

    #[test]
    fn test_json_response() {
        let response = json_response(r#"{"success":true}"#);
        let response_str = String::from_utf8_lossy(&response);
        assert!(response_str.contains("HTTP/1.1 200 OK"));
        assert!(response_str.contains("Content-Type: application/json"));
        assert!(response_str.contains(r#"{"success":true}"#));
    }

    #[test]
    fn test_service_unavailable_response() {
        let response = service_unavailable_response(r#"{"error":"busy"}"#, 5);
        let response_str = String::from_utf8_lossy(&response);
        assert!(response_str.contains("HTTP/1.1 503 Service Unavailable"));
        assert!(response_str.contains("Retry-After: 5"));
    }
}
