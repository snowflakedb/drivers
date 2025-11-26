use reqwest::{Method, StatusCode};
use sf_core::config::retry::{BackoffConfig, HttpPolicy, Jitter, RetryPolicy};
use sf_core::http::retry::{HttpContext, HttpError, execute_bytes_with_retry};
use std::net::SocketAddr;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::test]
async fn should_retry_get_after_transient_failure() {
    // Given a server that fails once then succeeds
    let (addr, attempts, server) = spawn_test_server(2, |attempt| async move {
        if attempt == 1 {
            b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nRetry-After: 0\r\nConnection: close\r\n\r\n"
                .to_vec()
        } else {
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok".to_vec()
        }
    })
    .await;

    let client = reqwest::Client::new();
    let url = format!("http://{}", addr);
    let ctx = HttpContext::new(Method::GET, url.clone());

    // When the helper executes the request
    let body = execute_bytes_with_retry(|| client.get(&url), &ctx, &RetryPolicy::default())
        .await
        .expect("retry to succeed");

    // Then it should have retried once and returned the successful body
    assert_eq!(body, b"ok");
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    server.await.unwrap();
}

#[tokio::test]
async fn should_fail_when_retry_after_exceeds_deadline() {
    // Given a retry policy with a tight deadline and a server that responds with a Retry-After that is after the deadline
    let (addr, _, server) = spawn_test_server(1, |_| async move {
        b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nRetry-After: 5\r\nConnection: close\r\n\r\n"
            .to_vec()
    })
    .await;

    let client = reqwest::Client::new();
    let url = format!("http://{}", addr);
    let ctx = HttpContext::new(Method::GET, url.clone());
    let policy = RetryPolicy {
        http: HttpPolicy {
            retry_safe_reads: true,
            retry_idempotent_writes: true,
            retry_post_patch: false,
        },
        max_attempts: 3,
        backoff: BackoffConfig {
            base: Duration::from_millis(10),
            factor: 1.0,
            cap: Duration::from_millis(10),
            jitter: Jitter::None,
        },
        max_elapsed: Duration::from_millis(50),
    };

    // When the helper executes the request
    let err = execute_bytes_with_retry(|| client.get(&url), &ctx, &policy)
        .await
        .expect_err("should exceed retry-after budget");

    // Then it should return a Retry-After exceeded error
    match err {
        HttpError::RetryAfterExceeded { .. } => {}
        other => panic!("expected RetryAfterExceeded, got {other:?}"),
    }
    server.await.unwrap();
}

#[tokio::test]
async fn should_retry_idempotent_put_after_transient_failure() {
    // Given an idempotent PUT request that fails once then succeeds
    let (addr, attempts, server) = spawn_test_server(2, |attempt| async move {
        if attempt == 1 {
            b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nRetry-After: 0\r\nConnection: close\r\n\r\n"
                .to_vec()
        } else {
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok".to_vec()
        }
    })
    .await;

    let client = reqwest::Client::new();
    let url = format!("http://{}", addr);
    let ctx = HttpContext::new(Method::PUT, url.clone()).with_idempotent(true);

    // When the helper executes the request
    let body = execute_bytes_with_retry(
        || client.put(&url).body("payload"),
        &ctx,
        &RetryPolicy::default(),
    )
    .await
    .expect("retry to succeed");

    // Then it should have retried once and returned the successful body
    assert_eq!(body, b"ok");
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    server.await.unwrap();
}

#[tokio::test]
async fn should_fail_after_reaching_max_attempts() {
    // Given a server that always fails with a retryable status
    let (addr, attempts, server) = spawn_test_server(2, |_| async move {
        b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nRetry-After: 0\r\nConnection: close\r\n\r\n"
            .to_vec()
    })
    .await;

    let client = reqwest::Client::new();
    let url = format!("http://{}", addr);
    let ctx = HttpContext::new(Method::GET, url.clone()).with_idempotent(true);
    let policy = RetryPolicy {
        http: HttpPolicy {
            retry_safe_reads: true,
            retry_idempotent_writes: true,
            retry_post_patch: false,
        },
        max_attempts: 2,
        backoff: BackoffConfig {
            base: Duration::from_millis(10),
            factor: 1.0,
            cap: Duration::from_millis(10),
            jitter: Jitter::None,
        },
        max_elapsed: Duration::from_secs(5),
    };

    // When the helper executes the request
    let err = execute_bytes_with_retry(|| client.get(&url), &ctx, &policy)
        .await
        .expect_err("should stop after max attempts");

    // Then it should return a max attempts error
    match err {
        HttpError::MaxAttempts {
            attempts,
            last_status,
            ..
        } => {
            assert_eq!(attempts, 2);
            assert_eq!(last_status, StatusCode::SERVICE_UNAVAILABLE);
        }
        other => panic!("expected MaxAttempts, got {other:?}"),
    }
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    server.await.unwrap();
}

/// Demonstrates sync-style behavior: no retry, fails on first transient error
#[tokio::test]
async fn sync_style_fails_on_transient_error() {
    // Given a server that fails once then succeeds
    let (addr, attempts, server) = spawn_test_server(2, |attempt| async move {
        if attempt == 1 {
            b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                .to_vec()
        } else {
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok".to_vec()
        }
    })
    .await;

    let client = reqwest::Client::new();
    let url = format!("http://{}", addr);

    // When using direct execute (sync-style, no retry wrapper)
    let response = client.get(&url).send().await.expect("request to complete");

    // Then it fails with 503 - no retry attempted
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(attempts.load(Ordering::SeqCst), 1); // Only one attempt
    drop(server); // Clean up (server still waiting for second connection)
}

/// Demonstrates async-style behavior: retries transient errors, succeeds
#[tokio::test]
async fn async_style_retries_transient_error() {
    // Given a server that fails once then succeeds (same setup)
    let (addr, attempts, server) = spawn_test_server(2, |attempt| async move {
        if attempt == 1 {
            b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nRetry-After: 0\r\nConnection: close\r\n\r\n"
                .to_vec()
        } else {
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok".to_vec()
        }
    })
    .await;

    let client = reqwest::Client::new();
    let url = format!("http://{}", addr);
    let ctx = HttpContext::new(Method::GET, url.clone());

    // When using execute_with_retry (async-style)
    let body = execute_bytes_with_retry(|| client.get(&url), &ctx, &RetryPolicy::default())
        .await
        .expect("retry to succeed");

    // Then it retries and succeeds
    assert_eq!(body, b"ok");
    assert_eq!(attempts.load(Ordering::SeqCst), 2); // Two attempts
    server.await.unwrap();
}

#[tokio::test]
async fn should_retry_after_connection_reset() {
    // Given a server that resets the connection on first attempt, then succeeds
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let attempts = Arc::new(AtomicUsize::new(0));
    let attempts_clone = attempts.clone();

    let server = tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            let attempt = attempts_clone.fetch_add(1, Ordering::SeqCst) + 1;

            if attempt == 1 {
                // First attempt: accept connection then immediately close (connection reset)
                drop(stream);
            } else {
                // Second attempt: read request and respond successfully
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf).await;
                let response =
                    b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok";
                stream.write_all(response).await.unwrap();
                let _ = stream.shutdown().await;
                break;
            }
        }
    });

    let client = reqwest::Client::new();
    let url = format!("http://{}", addr);
    let ctx = HttpContext::new(Method::GET, url.clone());

    // When the helper executes the request
    let body = execute_bytes_with_retry(|| client.get(&url), &ctx, &RetryPolicy::default())
        .await
        .expect("should retry after connection reset");

    // Then it should have retried and succeeded
    assert_eq!(body, b"ok");
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    server.await.unwrap();
}

#[tokio::test]
async fn should_not_retry_401_unauthorized() {
    // Given a server that returns 401 Unauthorized
    let (addr, attempts, server) = spawn_test_server(1, |_| async move {
        b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 16\r\nConnection: close\r\n\r\nSession expired".to_vec()
    })
    .await;

    let client = reqwest::Client::new();
    let url = format!("http://{}", addr);
    let ctx = HttpContext::new(Method::GET, url.clone());

    // When the helper executes the request
    let result = execute_bytes_with_retry(|| client.get(&url), &ctx, &RetryPolicy::default()).await;

    // Then it should NOT retry (401 is not retryable) and return the response
    // The caller is responsible for handling 401 as session expired
    assert!(result.is_err()); // 401 is surfaced as Transport error
    assert_eq!(attempts.load(Ordering::SeqCst), 1); // No retry
    server.await.unwrap();
}

async fn spawn_test_server<F, Fut>(
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
            let mut buf = [0u8; 1024];
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
