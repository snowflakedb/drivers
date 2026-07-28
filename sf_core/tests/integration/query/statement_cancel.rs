//! Driver-API tests for cross-thread `statement_cancel` — where server-side
//! cancel is actually *verified*.
//!
//! These drive the protobuf driver API against a tiny in-test async mock
//! Snowflake server so we can assert the `POST /queries/v1/abort-request` fired
//! with the same `requestId` the execute used (the crux of the feature — the
//! pre-existing local cancel-token can't prove that).
//!
//! Coordination is deterministic (a `tokio::sync::Notify` handshake, no
//! `sleep`s — flakiness requirement F3). The mock is fully async: the
//! query-request handler *awaits* (never blocks a thread) until the
//! abort-request lands, so the two requests are genuinely handled
//! concurrently.

use crate::common::snowflake_test_client::SnowflakeTestClient;
use proto_utils::ProtoError;
use sf_core::protobuf::generated::database_driver_v1::AbortQueryOutcome;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Notify;

const LOGIN_OK: &str = r#"{"success":true,"data":{"token":"mock_token","masterToken":"mock_master_token","sessionId":12345}}"#;
const ABORT_OK: &str = r#"{"success":true,"data":{}}"#;
// What Snowflake returns on a query-request once it has been aborted: gsCode 604.
const CANCELED_QUERY: &str = r#"{"success":false,"code":"000604","message":"SQL execution canceled","data":{"sqlState":"57014"}}"#;
const CATCH_ALL_OK: &str = r#"{"success":true,"data":{}}"#;

#[derive(Default)]
struct MockState {
    query_request_id: Mutex<Option<String>>,
    abort_bodies: Mutex<Vec<String>>,
    /// Signaled by the query-request handler once it has received the query.
    query_received: Notify,
    /// Signaled by the abort-request handler to release the blocked query.
    release_query: Notify,
    /// Whether the query-request handler should block awaiting an abort.
    /// (Set for the crux test; left false for the no-op test.)
    block_query_until_abort: bool,
}

/// Spawn the mock server's acceptor loop. Returns the bound address and the
/// acceptor's [`JoinHandle`] so each test can `.abort()` it on teardown rather
/// than leaking the task.
async fn spawn_mock(state: Arc<MockState>) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(pair) => pair,
                Err(_) => break,
            };
            let state = state.clone();
            // One task per connection: the query-request task can await the
            // release while the abort-request task is accepted and handled.
            tokio::spawn(handle_connection(stream, state));
        }
    });
    (addr, handle)
}

async fn handle_connection(mut stream: TcpStream, state: Arc<MockState>) {
    let (head, body) = read_http_request(&mut stream).await;
    let path = request_target(&head);

    let response_body = if path.contains("/session/v1/login-request") {
        LOGIN_OK.to_string()
    } else if path.contains("/queries/v1/query-request") {
        let request_id = query_param(&path, "requestId").unwrap_or_default();
        *state.query_request_id.lock().unwrap() = Some(request_id);
        state.query_received.notify_one();
        if state.block_query_until_abort {
            // Await (non-blocking) until the abort-request releases us, then
            // return the canceled response as Snowflake would.
            state.release_query.notified().await;
        }
        CANCELED_QUERY.to_string()
    } else if path.contains("/queries/v1/abort-request") {
        state.abort_bodies.lock().unwrap().push(body);
        state.release_query.notify_one();
        ABORT_OK.to_string()
    } else {
        CATCH_ALL_OK.to_string()
    };

    write_json_response(&mut stream, &response_body).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn statement_cancel_aborts_running_query_by_request_id() {
    let state = Arc::new(MockState {
        block_query_until_abort: true,
        ..Default::default()
    });
    let (addr, server) = spawn_mock(state.clone()).await;
    let server_url = format!("http://{addr}");

    let state_for_task = state.clone();
    let query_request_id = tokio::task::spawn_blocking(move || {
        let client = Arc::new(SnowflakeTestClient::connect_integration_test(Some(
            &server_url,
        )));
        let stmt = client.new_statement();
        client.set_sql_query(&stmt, "SELECT COUNT(*) FROM huge_table");

        // Execute on its own thread — it blocks server-side in the query
        // handler (which awaits the abort before responding).
        let exec_client = client.clone();
        let exec_stmt = stmt;
        let executor =
            std::thread::spawn(move || exec_client.execute_statement_query_raw(&exec_stmt));

        // Deterministically wait until the query-request has been received
        // (the in-flight slot is now populated), THEN cancel from this thread.
        wait_for(&state_for_task.query_received);
        let request_id = state_for_task
            .query_request_id
            .lock()
            .unwrap()
            .clone()
            .expect("query-request should have been received");

        let cancel = client
            .statement_cancel_blocking(&stmt)
            .expect("statement_cancel RPC should not transport-fail");
        assert_eq!(
            AbortQueryOutcome::try_from(cancel.outcome),
            Ok(AbortQueryOutcome::Aborted),
            "abort-request returned success:true, so cancel should report ABORTED"
        );

        // The executing thread now unblocks with the canceled (604) response.
        let exec_result = executor.join().expect("executor thread panicked");
        let err = exec_result.expect_err("canceled query must surface an error");
        match *err {
            ProtoError::Application(driver_exception) => assert_eq!(
                driver_exception.vendor_code,
                Some(604),
                "canceled query must surface gsCode 604 so ODBC can map it to HY008"
            ),
            ProtoError::Transport(e) => panic!("unexpected transport error: {e:?}"),
        }

        client.release_statement(&stmt);
        request_id
    })
    .await
    .expect("cancel task panicked");

    // The crux: exactly one abort-request fired, carrying the SAME requestId the
    // query-request used (proves a real server abort, not just a local unblock).
    let abort_bodies = state.abort_bodies.lock().unwrap();
    assert_eq!(
        abort_bodies.len(),
        1,
        "exactly one abort-request must fire (idempotent cancel); got {abort_bodies:?}"
    );
    assert!(
        abort_bodies[0].contains(&format!(r#""requestId":"{query_request_id}""#)),
        "abort body must target the running query's requestId {query_request_id}; body: {}",
        abort_bodies[0]
    );
    assert!(
        abort_bodies[0].contains(r#""sqlText":"SELECT COUNT(*) FROM huge_table""#),
        "abort body must echo the running query's sqlText; body: {}",
        abort_bodies[0]
    );

    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn statement_cancel_is_a_no_op_when_no_query_in_flight() {
    let state = Arc::new(MockState::default());
    let (addr, server) = spawn_mock(state.clone()).await;
    let server_url = format!("http://{addr}");

    tokio::task::spawn_blocking(move || {
        let client = SnowflakeTestClient::connect_integration_test(Some(&server_url));
        let stmt = client.new_statement();
        // No execute → in-flight slot is empty → cancel is a no-op.
        let cancel = client
            .statement_cancel_blocking(&stmt)
            .expect("statement_cancel RPC should not transport-fail");
        assert_eq!(
            AbortQueryOutcome::try_from(cancel.outcome),
            Ok(AbortQueryOutcome::NotRunning),
            "no-op cancel reports NOT_RUNNING"
        );
        client.release_statement(&stmt);
    })
    .await
    .unwrap();

    assert!(
        state.abort_bodies.lock().unwrap().is_empty(),
        "no abort-request may be sent when nothing is in flight"
    );

    server.abort();
}

/// Block the calling (sync) thread until `notify` fires. The permit-storing
/// semantics of `Notify::notify_one` make this race-free even if the signal
/// arrives first.
fn wait_for(notify: &Notify) {
    // We're on a blocking thread; bridge to the notify via a fresh current-thread
    // runtime so we don't touch the shared blocking runtime.
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(notify.notified());
}

async fn write_json_response(stream: &mut TcpStream, body: &str) {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.shutdown().await;
}

/// Read a full HTTP/1.1 request: headers up to the blank line, then the body
/// as sized by `Content-Length`.
async fn read_http_request(stream: &mut TcpStream) -> (String, String) {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];

    let header_end = loop {
        if let Some(pos) = find_crlf_crlf(&buf) {
            break pos + 4;
        }
        let n = stream.read(&mut tmp).await.unwrap_or(0);
        if n == 0 {
            return (String::from_utf8_lossy(&buf).into_owned(), String::new());
        }
        buf.extend_from_slice(&tmp[..n]);
    };

    let head = String::from_utf8_lossy(&buf[..header_end]).into_owned();
    let content_length = content_length(&head);
    let mut body = buf[header_end..].to_vec();
    while body.len() < content_length {
        let n = stream.read(&mut tmp).await.unwrap_or(0);
        if n == 0 {
            break;
        }
        body.extend_from_slice(&tmp[..n]);
    }

    (head, String::from_utf8_lossy(&body).into_owned())
}

fn find_crlf_crlf(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

fn content_length(head: &str) -> usize {
    for line in head.lines() {
        if let Some((name, value)) = line.split_once(':')
            && name.trim().eq_ignore_ascii_case("content-length")
        {
            return value.trim().parse().unwrap_or(0);
        }
    }
    0
}

/// Extract the request target (path + query) from the request line.
fn request_target(head: &str) -> String {
    head.lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("")
        .to_string()
}

fn query_param(target: &str, key: &str) -> Option<String> {
    let query = target.split_once('?')?.1;
    query.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        (k == key).then(|| v.to_string())
    })
}
