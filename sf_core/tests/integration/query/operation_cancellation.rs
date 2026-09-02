//! End-to-end proof that an operation handle cancels an in-flight RPC across
//! threads, now that cancellation is observed inside the per-RPC dispatch rather
//! than raced at the protobuf transport.
//!
//! What this pins that the unit tests cannot:
//!
//! * the cancel arrives from a **plain OS thread** with no tokio runtime, the
//!   way `SQLCancel` / `Statement.cancel()` / a signal handler would;
//! * it reaches an RPC genuinely parked on network I/O (`login-request`), not a
//!   ready-made future;
//! * the caller gets `ERROR_KIND_CANCELLED` **without** waiting for the server,
//!   which is the whole point of cancelling.
//!
//! Every wait here is bounded: the mock deliberately never answers, so an
//! unbounded wait would hang the suite rather than fail it.

use prost::Message as _;
use proto_utils::{CancellableTransport, ProtoError, Transport};
use sf_core::protobuf::apis::RustTransport;
use sf_core::protobuf::generated::database_driver_v1::{
    CancellationAbortOutcome, ConfigSetting, ConnectionGetQueryResultRequest,
    ConnectionGetQueryStatusRequest, ConnectionGetResultSetRequest, ConnectionHandle,
    ConnectionInitRequest, ConnectionInitResponse, ConnectionNewRequest, ConnectionNewResponse,
    ConnectionSendHttpRequest, ConnectionSetOptionsRequest, ConnectionSetOptionsResponse,
    ConnectionTokenRequest, DatabaseInitRequest, DatabaseInitResponse, DatabaseNewRequest,
    DatabaseNewResponse, DriverException, ErrorKind, StatementExecuteAsyncRequest,
    StatementExecuteQueryRequest, StatementHandle, StatementNewRequest, StatementNewResponse,
    StatementPrepareRequest, StatementSetSqlQueryRequest, StatementSetSqlQueryResponse,
    TokenRequestType, config_setting,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// Upper bound on any wait in this file. Generous enough not to be timing
/// sensitive, small enough that a genuine hang fails the test instead of the
/// suite.
const BOUND: Duration = Duration::from_secs(20);

/// A mock that accepts the login request and then **never answers**, so the only
/// way the call can finish is cancellation.
struct HangingLogin {
    /// Signalled once a request has been read off the wire, so the canceller
    /// knows the operation is genuinely in flight. A plain `std` channel, not a
    /// tokio primitive: the canceller is a bare OS thread with no runtime, which
    /// is exactly the situation being tested.
    request_received: Mutex<SyncSender<()>>,
    /// Every per-connection task spawned by the accept loop, so [`MockServer`]
    /// can abort them too. Each one parks on `pending()` forever and would
    /// otherwise only be reaped when the test's runtime is dropped.
    connections: Mutex<Vec<tokio::task::JoinHandle<()>>>,
}

/// Owns the mock's tasks and aborts **all** of them on drop: the accept loop and
/// every per-connection task it spawned.
///
/// Drop rather than an explicit `abort()` call at the end of each test, so
/// teardown does not depend on the assertions passing — a panicking assertion
/// still unwinds through this. Aborting the accept-loop handle alone would leave
/// the per-connection tasks parked on `pending()`.
struct MockServer {
    listener: tokio::task::JoinHandle<()>,
    state: Arc<HangingLogin>,
}

impl Drop for MockServer {
    fn drop(&mut self) {
        self.listener.abort();
        for conn in self.state.connections.lock().unwrap().drain(..) {
            conn.abort();
        }
    }
}

async fn spawn_hanging_login(state: Arc<HangingLogin>) -> (SocketAddr, MockServer) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let accept_state = state.clone();
    let handle = tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            let state = accept_state.clone();
            let conn = tokio::spawn({
                let state = state.clone();
                async move {
                    read_request_head(&mut stream).await;
                    // `try_send` on a rendezvous-free buffered channel never blocks
                    // the runtime; a full/closed channel just means the canceller
                    // already got its signal.
                    let _ = state.request_received.lock().unwrap().try_send(());
                    // Hold the connection open, writing nothing.
                    std::future::pending::<()>().await;
                }
            });
            state.connections.lock().unwrap().push(conn);
        }
    });
    (
        addr,
        MockServer {
            listener: handle,
            state,
        },
    )
}

const SLOW_QUERY: &str = "SELECT COUNT(*) FROM huge_table";
const LOGIN_OK: &str = r#"{"success":true,"data":{"token":"mock_token","masterToken":"mock_master_token","sessionId":12345}}"#;
const ABORT_OK: &str = r#"{"success":true,"data":{}}"#;
const EMPTY_RESULT: &str = r#"{"success":true,"data":{"rowtype":[],"rowset":[],"total":0,"queryId":"01b2-cancel-test","queryResultFormat":"json"}}"#;
/// A query-request answer that GS accepted but which carries no `queryId`, so the
/// caller has nothing to name the query it created with.
const RESULT_WITHOUT_QUERY_ID: &str =
    r#"{"success":true,"data":{"rowtype":[],"rowset":[],"total":0,"queryResultFormat":"json"}}"#;
const CATCH_ALL_OK: &str = r#"{"success":true,"data":{}}"#;
/// Query id for the finished-query lookups (`connection_get_result_set`,
/// `connection_get_query_status`, `connection_get_query_result`). Must be a
/// syntactically valid UUID: `snowflake_get_query_result` parses it with
/// `Uuid::parse_str(..).expect(..)`, so a placeholder like `"not-a-uuid"` would
/// panic in core rather than reach the network.
const FINISHED_QUERY_ID: &str = "01b2c3d4-0000-4000-8000-000000000001";

/// Relative path the `connection_send_http` test asks for. That RPC forwards
/// whatever path the caller passes, so this is an arbitrary one chosen not to
/// collide with any endpoint the mock matches above it.
const SEND_HTTP_PROBE_PATH: &str = "/api/cancellation-probe";

/// Which request the mock parks on, so the operation under test is genuinely
/// blocked on the network when the cancel arrives.
#[derive(Clone, Copy, PartialEq, Eq)]
enum HangOn {
    /// `POST /queries/v1/query-request` — the execute and prepare paths.
    QueryRequest,
    /// A lookup of an **already-finished** query: `GET /queries/{id}/result`
    /// (`connection_get_result_set`, `connection_get_query_result`) or
    /// `GET /monitoring/queries/{id}` (`connection_get_query_status`). One
    /// variant for all three because each test wants the same thing: park so
    /// only cancellation can end the call.
    FinishedQueryLookup,
    /// [`SEND_HTTP_PROBE_PATH`] — the caller-directed request `connection_send_http`
    /// forwards.
    SendHttpRequest,
    /// `POST /session/token-request` — `connection_request_token`.
    TokenRequest,
    /// [`SEND_HTTP_PROBE_PATH`], parked *after* the response headers are written,
    /// so the caller's `send()` resolves and it blocks reading the body instead.
    /// That is the shape a server which accepted the request and then went quiet
    /// actually produces.
    SendHttpBody,
    /// Nothing — every request is answered, for the no-cancellation control case.
    Nothing,
}

/// True if `target` is a lookup of an already-finished query. Distinct prefixes
/// from `/queries/v1/query-request` and `/queries/v1/abort-request`, so this
/// cannot swallow either.
fn is_finished_query_lookup(target: &str) -> bool {
    target.starts_with("/monitoring/queries/")
        || (target.starts_with("/queries/") && target.ends_with("/result"))
}

/// A mock that logs in successfully and then, unlike [`HangingLogin`], parks on a
/// configurable later request while recording any abort-request that arrives.
///
/// Separate from `HangingLogin` because these tests need the session to exist
/// before the operation under test starts: every one of them acts on a query,
/// which requires a completed login.
struct HangingServer {
    /// Signalled once the request the test is waiting on has been read off the
    /// wire, so the canceller knows the operation is genuinely in flight — and,
    /// for the query-request, that its `requestId` has been published into the
    /// statement's in-flight slot.
    request_received: Mutex<SyncSender<()>>,
    /// The `requestId` query parameter the execute used, so the abort can be
    /// matched against it.
    query_request_id: Mutex<Option<String>>,
    /// Bodies of every `POST /queries/v1/abort-request` received. Asserting on
    /// the count is what pins "exactly one abort per cancelled query".
    abort_bodies: Mutex<Vec<String>>,
    /// Answer the query-request with [`RESULT_WITHOUT_QUERY_ID`] instead of
    /// [`EMPTY_RESULT`]. Orthogonal to [`HangOn`]: this is about *what* is answered,
    /// not where the mock parks.
    omit_query_id: AtomicBool,
    /// Request target the mock actually parked on. `HangOn::FinishedQueryLookup`
    /// matches two different endpoints, so without recording this a test could
    /// pass having hung the wrong one.
    hung_target: Mutex<Option<String>>,
    connections: Mutex<Vec<tokio::task::JoinHandle<()>>>,
}

/// Owns the [`HangingServer`] mock's tasks and aborts all of them on drop — see
/// [`MockServer`] for why teardown is `Drop`-based rather than an explicit call.
struct HangingServerHandle {
    listener: tokio::task::JoinHandle<()>,
    state: Arc<HangingServer>,
}

impl Drop for HangingServerHandle {
    fn drop(&mut self) {
        self.listener.abort();
        for conn in self.state.connections.lock().unwrap().drain(..) {
            conn.abort();
        }
    }
}

/// Spawn a mock whose query-request never answers, so only cancellation can end
/// the call.
async fn spawn_hanging_query(state: Arc<HangingServer>) -> (SocketAddr, HangingServerHandle) {
    spawn_hanging_server(state, HangOn::QueryRequest).await
}

/// Spawn a mock that logs in, then never answers a finished-query lookup, so
/// only cancellation can end `connection_get_result_set` /
/// `connection_get_query_status`.
async fn spawn_hanging_lookup(state: Arc<HangingServer>) -> (SocketAddr, HangingServerHandle) {
    spawn_hanging_server(state, HangOn::FinishedQueryLookup).await
}

/// Spawn a mock that logs in, then never answers a request to
/// [`SEND_HTTP_PROBE_PATH`], so only cancellation can end `connection_send_http`.
async fn spawn_hanging_send_http(state: Arc<HangingServer>) -> (SocketAddr, HangingServerHandle) {
    spawn_hanging_server(state, HangOn::SendHttpRequest).await
}

/// Spawn a mock that answers [`SEND_HTTP_PROBE_PATH`]'s headers and then never
/// sends the body, so `connection_send_http` parks reading the response.
async fn spawn_hanging_send_http_body(
    state: Arc<HangingServer>,
) -> (SocketAddr, HangingServerHandle) {
    spawn_hanging_server(state, HangOn::SendHttpBody).await
}

/// Spawn a mock that logs in, then never answers `POST /session/token-request`,
/// so only cancellation can end `connection_request_token`.
async fn spawn_hanging_token_request(
    state: Arc<HangingServer>,
) -> (SocketAddr, HangingServerHandle) {
    spawn_hanging_server(state, HangOn::TokenRequest).await
}

/// Spawn a mock that answers the query-request immediately, for the
/// no-cancellation control case.
async fn spawn_answering_query(state: Arc<HangingServer>) -> (SocketAddr, HangingServerHandle) {
    spawn_hanging_server(state, HangOn::Nothing).await
}

async fn spawn_hanging_server(
    state: Arc<HangingServer>,
    hang_on: HangOn,
) -> (SocketAddr, HangingServerHandle) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let accept_state = state.clone();
    let handle = tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            let state = accept_state.clone();
            let conn_state = state.clone();
            // One task per connection so the abort-request is accepted and
            // handled while the query-request task is still parked.
            let conn = tokio::spawn(async move {
                let state = conn_state;
                let (head, body) = read_http_request(&mut stream).await;
                let target = request_target(&head);

                let response = if target.contains("/session/v1/login-request") {
                    LOGIN_OK.to_string()
                } else if target.contains("/queries/v1/query-request") {
                    *state.query_request_id.lock().unwrap() = query_param(&target, "requestId");
                    // A full/closed channel just means the canceller already has
                    // its signal.
                    let _ = state.request_received.lock().unwrap().try_send(());
                    if hang_on == HangOn::QueryRequest {
                        *state.hung_target.lock().unwrap() = Some(target.clone());
                        // Hold the query open, writing nothing: the only way out
                        // is cancellation.
                        std::future::pending::<()>().await;
                    }
                    if state.omit_query_id.load(Ordering::SeqCst) {
                        RESULT_WITHOUT_QUERY_ID.to_string()
                    } else {
                        EMPTY_RESULT.to_string()
                    }
                } else if target.contains("/queries/v1/abort-request") {
                    state.abort_bodies.lock().unwrap().push(body);
                    ABORT_OK.to_string()
                } else if is_finished_query_lookup(&target) {
                    let _ = state.request_received.lock().unwrap().try_send(());
                    if hang_on == HangOn::FinishedQueryLookup {
                        *state.hung_target.lock().unwrap() = Some(target.clone());
                        std::future::pending::<()>().await;
                    }
                    // Same body the catch-all served before this branch existed,
                    // so no pre-existing test changes behaviour.
                    CATCH_ALL_OK.to_string()
                } else if target.starts_with(SEND_HTTP_PROBE_PATH) {
                    let _ = state.request_received.lock().unwrap().try_send(());
                    if hang_on == HangOn::SendHttpRequest {
                        *state.hung_target.lock().unwrap() = Some(target.clone());
                        std::future::pending::<()>().await;
                    }
                    if hang_on == HangOn::SendHttpBody {
                        *state.hung_target.lock().unwrap() = Some(target.clone());
                        write_headers_only(&mut stream, CATCH_ALL_OK.len()).await;
                        std::future::pending::<()>().await;
                    }
                    CATCH_ALL_OK.to_string()
                } else if target.starts_with("/session/token-request") {
                    let _ = state.request_received.lock().unwrap().try_send(());
                    if hang_on == HangOn::TokenRequest {
                        *state.hung_target.lock().unwrap() = Some(target.clone());
                        std::future::pending::<()>().await;
                    }
                    CATCH_ALL_OK.to_string()
                } else {
                    CATCH_ALL_OK.to_string()
                };

                write_json_response(&mut stream, &response).await;
            });
            state.connections.lock().unwrap().push(conn);
        }
    });
    (
        addr,
        HangingServerHandle {
            listener: handle,
            state,
        },
    )
}

/// Write the status line and headers, leaving the body unsent, so the caller's
/// `send()` resolves and it then parks reading the body.
async fn write_headers_only(stream: &mut TcpStream, content_length: usize) {
    let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {content_length}\r\nConnection: close\r\n\r\n"
    );
    let _ = stream.write_all(head.as_bytes()).await;
    let _ = stream.flush().await;
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

/// Read a full HTTP/1.1 request: headers up to the blank line, then the body as
/// sized by `Content-Length`.
async fn read_http_request(stream: &mut TcpStream) -> (String, String) {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];

    let header_end = loop {
        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break pos + 4;
        }
        match stream.read(&mut tmp).await {
            Ok(0) | Err(_) => {
                return (String::from_utf8_lossy(&buf).into_owned(), String::new());
            }
            Ok(n) => buf.extend_from_slice(&tmp[..n]),
        }
    };

    let head = String::from_utf8_lossy(&buf[..header_end]).into_owned();
    let content_length = head
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.trim().eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.trim().parse().ok())
        .unwrap_or(0);

    let mut body = buf[header_end..].to_vec();
    while body.len() < content_length {
        match stream.read(&mut tmp).await {
            Ok(0) | Err(_) => break,
            Ok(n) => body.extend_from_slice(&tmp[..n]),
        }
    }

    (head, String::from_utf8_lossy(&body).into_owned())
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

/// Read just enough of the request to know it arrived.
async fn read_request_head(stream: &mut TcpStream) {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];
    loop {
        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            return;
        }
        match stream.read(&mut tmp).await {
            Ok(0) | Err(_) => return,
            Ok(n) => buf.extend_from_slice(&tmp[..n]),
        }
    }
}

fn setting(value: &str) -> ConfigSetting {
    ConfigSetting {
        value: Some(config_setting::Value::StringValue(value.to_string())),
    }
}

/// Mirrors the option set the other integration tests use (see
/// `SnowflakeTestClient::with_int_tests_params`) so `connection_init` gets as far
/// as the network instead of failing validation.
fn mock_options(addr: SocketAddr) -> HashMap<String, ConfigSetting> {
    [
        ("account", "test_account"),
        ("user", "test_user"),
        ("password", "test_password"),
        ("database", "test_database"),
        ("schema", "test_schema"),
        ("warehouse", "test_warehouse"),
        ("role", "test_role"),
        ("host", "localhost"),
        ("protocol", "http"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), setting(v)))
    .chain(std::iter::once((
        "server_url".to_string(),
        setting(&format!("http://{addr}")),
    )))
    .collect()
}

/// Issue a non-cancellable RPC through the transport, the way a blocking bridge
/// does. Only the setup calls use this; the call under test goes through
/// `handle_message_cancellable`.
async fn call<Req: prost::Message, Resp: prost::Message + Default>(
    transport: &RustTransport,
    method: &str,
    request: Req,
) -> Resp {
    let bytes = transport
        .handle_message("DatabaseDriver", method, request.encode_to_vec())
        .await
        .unwrap_or_else(|_| panic!("{method} should succeed"));
    Resp::decode(&bytes[..]).expect("response decodes")
}

/// Drive `database_new` → `database_init` → `connection_new` →
/// `connection_set_options` against the mock, returning the init request.
async fn connected(transport: &RustTransport, addr: SocketAddr) -> ConnectionInitRequest {
    let db = call::<_, DatabaseNewResponse>(transport, "database_new", DatabaseNewRequest {})
        .await
        .db_handle
        .expect("db handle");
    let _: DatabaseInitResponse = call(
        transport,
        "database_init",
        DatabaseInitRequest {
            db_handle: Some(db),
        },
    )
    .await;

    let conn =
        call::<_, ConnectionNewResponse>(transport, "connection_new", ConnectionNewRequest {})
            .await
            .conn_handle
            .expect("conn handle");
    let _: ConnectionSetOptionsResponse = call(
        transport,
        "connection_set_options",
        ConnectionSetOptionsRequest {
            conn_handle: Some(conn),
            options: mock_options(addr),
            no_connection_details: false,
        },
    )
    .await;

    ConnectionInitRequest {
        conn_handle: Some(conn),
        db_handle: Some(db),
        wrapper_identity: None,
    }
}

/// Drive the whole setup a submitted-query test needs — connect, open a
/// statement, and set [`SLOW_QUERY`] on it — returning the statement handle.
///
/// Shared by every test that cancels a query so they cannot drift in how the
/// statement was set up, which is what would let one of them silently stop
/// exercising a submitted query.
/// Drive [`connected`] and then `connection_init`, returning the open
/// connection handle. Shared by the statement setup and by the tests that act on
/// a query id directly and need no statement at all.
async fn open_connection(transport: &RustTransport, addr: SocketAddr) -> ConnectionHandle {
    let init = connected(transport, addr).await;
    let conn = init.conn_handle.expect("conn handle");
    let _: ConnectionInitResponse = call(transport, "connection_init", init).await;
    conn
}

async fn statement_with_slow_query(transport: &RustTransport, addr: SocketAddr) -> StatementHandle {
    let conn = open_connection(transport, addr).await;

    let stmt = call::<_, StatementNewResponse>(
        transport,
        "statement_new",
        StatementNewRequest {
            conn_handle: Some(conn),
        },
    )
    .await
    .stmt_handle
    .expect("stmt handle");
    let _: StatementSetSqlQueryResponse = call(
        transport,
        "statement_set_sql_query",
        StatementSetSqlQueryRequest {
            stmt_handle: Some(stmt),
            query: SLOW_QUERY.to_string(),
        },
    )
    .await;
    stmt
}

/// A fresh [`HangingServer`] mock state, plus the receiver the canceller thread
/// waits on to learn that the request under test has reached the server.
fn hanging_server_state() -> (Arc<HangingServer>, Receiver<()>) {
    let (tx, request_received) = sync_channel(1);
    (
        Arc::new(HangingServer {
            request_received: Mutex::new(tx),
            query_request_id: Mutex::new(None),
            abort_bodies: Mutex::new(Vec::new()),
            omit_query_id: AtomicBool::new(false),
            hung_target: Mutex::new(None),
            connections: Mutex::new(Vec::new()),
        }),
        request_received,
    )
}

/// Dispatch `method` under a fresh operation handle, cancelling it from a plain
/// OS thread as soon as the mock reports that the request under test is on the
/// wire.
///
/// The `method` stays at the call site so each test still names the RPC it
/// covers, and `awaited` names the request the mock parks on so the panic below
/// says which one never arrived. Panics if the mock never saw it: without that
/// check a test could pass having cancelled an operation that never reached the
/// network, which is the precise false pass this file exists to rule out.
async fn cancel_once_the_request_is_on_the_wire(
    transport: &Arc<RustTransport>,
    request_received: Receiver<()>,
    method: &str,
    awaited: &str,
    request: Vec<u8>,
) -> Result<Vec<u8>, ProtoError<Vec<u8>>> {
    let (operation, _token) = transport.register();

    let canceller = std::thread::spawn({
        let transport = transport.clone();
        move || {
            let saw_request = request_received.recv_timeout(BOUND).is_ok();
            transport.cancel(operation);
            saw_request
        }
    });

    let result = tokio::time::timeout(
        BOUND,
        transport.handle_message_cancellable("DatabaseDriver", method, request, operation),
    )
    .await
    .unwrap_or_else(|_| panic!("cancelling must unblock {method} well inside the bound"));

    let saw_request = canceller.join().expect("canceller thread panicked");
    assert!(
        saw_request,
        "the mock never received a {awaited}, so this did not test cancelling an in-flight {method}"
    );
    result
}

/// Decode `result` as the cancelled application error, asserting on `kind`
/// rather than on a message substring.
fn decode_cancelled(result: Result<Vec<u8>, ProtoError<Vec<u8>>>) -> DriverException {
    match result {
        Err(ProtoError::Application(bytes)) => {
            let ex = DriverException::decode(&bytes[..]).expect("decodes as DriverException");
            assert_eq!(
                ex.kind,
                ErrorKind::Cancelled as i32,
                "a cancelled operation must report ERROR_KIND_CANCELLED, got {ex:?}"
            );
            ex
        }
        other => panic!("expected a cancelled application error, got {other:?}"),
    }
}

/// Assert that `result` is the cancelled application error.
fn assert_cancelled(result: Result<Vec<u8>, ProtoError<Vec<u8>>>) {
    decode_cancelled(result);
}

/// Assert that `result` is the cancelled application error *and* that it carries
/// `expected` as the acknowledgement of the abort fired on cancellation.
/// `expected: None` asserts that no abort was issued.
fn assert_cancelled_with_abort(
    result: Result<Vec<u8>, ProtoError<Vec<u8>>>,
    expected: Option<CancellationAbortOutcome>,
) {
    let ex = decode_cancelled(result);
    assert_eq!(
        ex.cancellation_abort_outcome,
        expected.map(|e| e as i32),
        "expected cancellation_abort_outcome {expected:?}, got {ex:?}"
    );
}

/// Assert that cancellation emitted exactly one abort-request, and that it
/// targeted the `requestId` the mock actually saw on the query-request.
///
/// Matching against the observed `requestId` (rather than any well-formed UUID)
/// is what makes this an abort *of the cancelled query* instead of an abort of
/// something. The count pins at-most-one-abort-per-cancelled-query.
fn assert_aborted_the_submitted_query(state: &HangingServer) {
    let query_request_id = state
        .query_request_id
        .lock()
        .unwrap()
        .clone()
        .expect("query-request should have been received");
    let abort_bodies = state.abort_bodies.lock().unwrap();
    assert_eq!(
        abort_bodies.len(),
        1,
        "cancelling must emit exactly one abort-request; got {abort_bodies:?}"
    );
    assert!(
        abort_bodies[0].contains(&format!(r#""requestId":"{query_request_id}""#)),
        "abort body must target the cancelled query's requestId {query_request_id}; body: {}",
        abort_bodies[0]
    );
    assert!(
        abort_bodies[0].contains(&format!(r#""sqlText":"{SLOW_QUERY}""#)),
        "abort body must echo the cancelled query's sqlText; body: {}",
        abort_bodies[0]
    );
}

/// Assert that cancellation emitted **no** abort-request.
///
/// The inverse of [`assert_aborted_the_submitted_query`], and the whole point of
/// the tests that use it: the finished-query lookups read a query that has
/// already completed, and `connection_send_http` is not a query at all, so in
/// neither case is there anything of ours still running to abort. Without this
/// assertion those tests would pass just as well if a stray abort were fired,
/// which would mean cancelling a status poll could cancel someone else's query.
fn assert_no_abort_requests(state: &HangingServer) {
    let abort_bodies = state.abort_bodies.lock().unwrap();
    assert!(
        abort_bodies.is_empty(),
        "cancelling this operation must not abort anything; got {abort_bodies:?}"
    );
}

/// Assert which request the mock actually parked on.
///
/// [`HangOn::FinishedQueryLookup`] deliberately serves three RPCs across two
/// different endpoints, so a test that only checked "something hung" would still
/// pass if the RPC under test started talking to the other one.
///
/// `expected` is compared against the request path with the query string removed
/// and repeated slashes collapsed. The collapsing is not cosmetic tidying: the
/// monitoring URL really is sent as `/monitoring/queries//{id}`, because
/// `MONITORING_QUERIES_PATH` ends in `/` and `snowflake_get_query_status` then
/// appends the id with `path_segments_mut().push(..)`, which lands after the
/// empty trailing segment (`rest/snowflake/mod.rs:2063`, `:2085`). The server
/// accepts it, so this normalizes rather than asserting the stray segment.
fn assert_hung_on(state: &HangingServer, expected: &str) {
    let hung = state
        .hung_target
        .lock()
        .unwrap()
        .clone()
        .expect("the mock should have parked on a request");
    let path = hung.split('?').next().unwrap_or(&hung);
    let mut normalized = String::with_capacity(path.len());
    for ch in path.chars() {
        if ch == '/' && normalized.ends_with('/') {
            continue;
        }
        normalized.push(ch);
    }
    assert_eq!(
        normalized, expected,
        "expected the mock to park on {expected}; it parked on {hung}"
    );
}

/// Dispatch `connection_init` under `operation`, as the C API and JDBC bridges do.
async fn init_cancellable(
    transport: &RustTransport,
    init: ConnectionInitRequest,
    operation: u64,
) -> Result<Vec<u8>, ProtoError<Vec<u8>>> {
    transport
        .handle_message_cancellable(
            "DatabaseDriver",
            "connection_init",
            init.encode_to_vec(),
            operation,
        )
        .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancelling_a_handle_from_another_thread_unblocks_an_in_flight_login() {
    let (tx, request_received) = sync_channel(1);
    let state = Arc::new(HangingLogin {
        request_received: Mutex::new(tx),
        connections: Mutex::new(Vec::new()),
    });
    // `_server` (a named binding, not a bare `_`) lives to the end of the test,
    // and its `Drop` aborts the mock — on the panicking path as well as this one.
    let (addr, _server) = spawn_hanging_login(state.clone()).await;

    let transport = Arc::new(RustTransport::new());
    let init = connected(&transport, addr).await;

    // Mint the handle before the call, so the canceller holds it up front.
    let (operation, _token) = transport.register();

    // Cancel from a plain OS thread: no tokio runtime, exactly like SQLCancel.
    // The wait is bounded so a `connection_init` that never reaches the network
    // fails the assertion below instead of deadlocking this join.
    let canceller = std::thread::spawn({
        let transport = transport.clone();
        move || {
            // Bounded so a `connection_init` that never reaches the network
            // fails an assertion instead of deadlocking this join.
            let saw_request = request_received.recv_timeout(BOUND).is_ok();
            transport.cancel(operation);
            saw_request
        }
    });

    let result = tokio::time::timeout(BOUND, init_cancellable(&transport, init, operation))
        .await
        .expect("cancelling must unblock the call well inside the bound");

    let saw_request = canceller.join().expect("canceller thread panicked");
    assert!(
        saw_request,
        "the mock never received a request, so this did not test cancelling in-flight network work"
    );

    assert_cancelled(result);
}

/// The crux of query cancellation: cancelling the operation handle for an
/// in-flight `statement_execute_query` must not merely drop the local future —
/// it must **abort the query on the server**.
///
/// This is what distinguishes the current design from the transport-level race it
/// replaced. The abort cannot happen on the unwind itself (there is no async
/// `Drop`), so it runs on a task registered via `OperationCtx::arm_cleanup`; this
/// test is the proof that the task actually fires, carries the right `requestId`,
/// and is awaited before the caller is told the operation was cancelled.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancelling_an_in_flight_query_aborts_it_server_side() {
    let (state, query_received) = hanging_server_state();
    let (addr, _server) = spawn_hanging_query(state.clone()).await;

    let transport = Arc::new(RustTransport::new());
    let stmt = statement_with_slow_query(&transport, addr).await;

    let result = cancel_once_the_request_is_on_the_wire(
        &transport,
        query_received,
        "statement_execute_query",
        "query-request",
        StatementExecuteQueryRequest {
            stmt_handle: Some(stmt),
            bindings: None,
            timeout_seconds: None,
        }
        .encode_to_vec(),
    )
    .await;

    // `ABORTED` rather than just "cancelled": the acknowledgement is only
    // trustworthy if it reflects what the server actually said, and no sleep is
    // needed before reading it because `OperationCtx::run` awaits the registered
    // cleanup before reporting.
    assert_cancelled_with_abort(result, Some(CancellationAbortOutcome::Aborted));
    assert_aborted_the_submitted_query(&state);
}

/// The same guarantee for `statement_prepare`, which submits a `describe_only`
/// query-request and so can park on the network exactly as an execute does.
///
/// Worth its own test rather than trusting the execute coverage: `statement_prepare`
/// reaches the wire through its own entry point, and it was the last query-submitting
/// RPC left unmarked — it passed `None` for the ctx, so a cancel could only drop the
/// local future and the described query kept running server-side. This pins that the
/// marker is wired all the way through: ctx observed at the prepare boundary *and*
/// handed down so the abort is registered.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancelling_an_in_flight_prepare_aborts_it_server_side() {
    let (state, query_received) = hanging_server_state();
    let (addr, _server) = spawn_hanging_query(state.clone()).await;

    let transport = Arc::new(RustTransport::new());
    let stmt = statement_with_slow_query(&transport, addr).await;

    let result = cancel_once_the_request_is_on_the_wire(
        &transport,
        query_received,
        "statement_prepare",
        "query-request",
        StatementPrepareRequest {
            stmt_handle: Some(stmt),
        }
        .encode_to_vec(),
    )
    .await;

    assert_cancelled_with_abort(result, Some(CancellationAbortOutcome::Aborted));
    assert_aborted_the_submitted_query(&state);
}

/// A handle cancelled *before* dispatch leaves the acknowledgement unset, rather
/// than reporting one of the real abort outcomes.
///
/// This is the discrimination the acknowledgement exists to make: "cancelled, and
/// the server confirmed it stopped the query" must not look the same as
/// "cancelled before anything was submitted, so nothing was aborted". A caller
/// that cannot tell those apart learns nothing from the field.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancelling_before_dispatch_reports_no_abort_was_issued() {
    let (state, _query_received) = hanging_server_state();
    let (addr, _server) = spawn_hanging_query(state.clone()).await;

    let transport = Arc::new(RustTransport::new());
    let stmt = statement_with_slow_query(&transport, addr).await;

    let (operation, _token) = transport.register();
    // Cancelled before dispatch, so the operation body is never polled and no
    // abort cleanup is ever armed.
    transport.cancel(operation);

    let result = tokio::time::timeout(
        BOUND,
        transport.handle_message_cancellable(
            "DatabaseDriver",
            "statement_execute_query",
            StatementExecuteQueryRequest {
                stmt_handle: Some(stmt),
                bindings: None,
                timeout_seconds: None,
            }
            .encode_to_vec(),
            operation,
        ),
    )
    .await
    .expect("a pre-cancelled handle must resolve immediately");

    assert_cancelled_with_abort(result, None);
    assert!(
        state.abort_bodies.lock().unwrap().is_empty(),
        "a query that was never submitted must not be aborted"
    );
}

/// A query that completes normally must not emit an abort-request. Guards the
/// suppress path of the cleanup guard: an armed-then-completed query that still
/// aborted would cancel healthy work.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_query_that_completes_emits_no_abort_request() {
    let (state, _query_received) = hanging_server_state();
    // `hang: false` → the query-request is answered immediately.
    let (addr, _server) = spawn_answering_query(state.clone()).await;

    let transport = Arc::new(RustTransport::new());
    let stmt = statement_with_slow_query(&transport, addr).await;

    let (operation, _token) = transport.register();
    let _ = tokio::time::timeout(
        BOUND,
        transport.handle_message_cancellable(
            "DatabaseDriver",
            "statement_execute_query",
            StatementExecuteQueryRequest {
                stmt_handle: Some(stmt),
                bindings: None,
                timeout_seconds: None,
            }
            .encode_to_vec(),
            operation,
        ),
    )
    .await
    .expect("an answered query must return inside the bound");

    assert!(
        state.abort_bodies.lock().unwrap().is_empty(),
        "a query that was never cancelled must not be aborted"
    );
}

/// The same guarantee for `statement_execute_async`: an uncancelled submission
/// must not emit an abort-request.
///
/// The execute path has this above; the async path did not, and it is the one
/// where an accidental abort would be hardest to notice — the caller receives a
/// `query_id` and a success, so a spurious abort of that very query would only
/// show up later as a query that mysteriously stopped.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_completed_async_submission_emits_no_abort_request() {
    let (state, _request_received) = hanging_server_state();
    let (addr, _server) = spawn_answering_query(state.clone()).await;

    let transport = Arc::new(RustTransport::new());
    let stmt = statement_with_slow_query(&transport, addr).await;

    let (operation, _token) = transport.register();
    let _ = tokio::time::timeout(
        BOUND,
        transport.handle_message_cancellable(
            "DatabaseDriver",
            "statement_execute_async",
            StatementExecuteAsyncRequest {
                stmt_handle: Some(stmt),
                bindings: None,
            }
            .encode_to_vec(),
            operation,
        ),
    )
    .await
    .expect("an answered submission must return inside the bound");

    assert!(
        state.abort_bodies.lock().unwrap().is_empty(),
        "an async submission that was never cancelled must not be aborted"
    );
}

/// The same call with a handle that is never cancelled must not be short
/// circuited by the new observation point — it stays parked on the server.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_uncancelled_handle_does_not_short_circuit_the_operation() {
    let (tx, _request_received) = sync_channel(1);
    let state = Arc::new(HangingLogin {
        request_received: Mutex::new(tx),
        connections: Mutex::new(Vec::new()),
    });
    let (addr, _server) = spawn_hanging_login(state.clone()).await;

    let transport = RustTransport::new();
    let init = connected(&transport, addr).await;

    let (operation, _token) = transport.register();

    // The mock never answers and nothing cancels, so the call must still be
    // pending when the bound elapses.
    let outcome = tokio::time::timeout(
        Duration::from_secs(2),
        init_cancellable(&transport, init, operation),
    )
    .await;

    assert!(
        outcome.is_err(),
        "an uncancelled operation must not resolve on its own, got {:?}",
        outcome.map(|r| r.is_ok())
    );
}

/// Cancelling an in-flight `connection_get_result_set` must unblock the caller
/// with `ERROR_KIND_CANCELLED` rather than waiting for
/// `GET /queries/{id}/result` to answer.
///
/// Unlike the execute/prepare tests there is deliberately **no** abort here: the
/// query being fetched has already finished, so cancelling abandons only the
/// retrieval. [`assert_no_abort_requests`] pins that as intended behaviour — a
/// stray abort on this path would target a query the caller does not own.
///
/// What makes this worth an integration test even though cancellation already
/// worked before the RPC was marked: marking flips `observes_cancellation`, which
/// turns **off** the transport-level race. If the impl were marked without also
/// observing the ctx, nothing would watch the token and this call would park on
/// the mock forever — so the bounded wait below is what catches that trap.
/// Verified by removing the impl's `run_opt`: this test then fails on the bound
/// instead of passing.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancelling_an_in_flight_result_set_fetch_reports_cancelled_without_aborting() {
    let (state, request_received) = hanging_server_state();
    let (addr, _server) = spawn_hanging_lookup(state.clone()).await;

    let transport = Arc::new(RustTransport::new());
    let conn = open_connection(&transport, addr).await;

    let result = cancel_once_the_request_is_on_the_wire(
        &transport,
        request_received,
        "connection_get_result_set",
        "result lookup",
        ConnectionGetResultSetRequest {
            conn_handle: Some(conn),
            query_id: FINISHED_QUERY_ID.to_string(),
        }
        .encode_to_vec(),
    )
    .await;

    assert_cancelled(result);
    assert_hung_on(&state, &format!("/queries/{FINISHED_QUERY_ID}/result"));
    assert_no_abort_requests(&state);
}

/// The same guarantee for `connection_get_query_status`, which parks on
/// `GET /monitoring/queries/{id}`.
///
/// Worth its own test rather than trusting the result-set coverage: it is a
/// different endpoint reached through a different entry point, and it is the RPC
/// a wrapper polls in a loop while waiting on an async query — the case where a
/// caller most wants out without waiting for the server. Catches the same
/// marked-but-not-observing trap described on the result-set test above.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancelling_an_in_flight_query_status_poll_reports_cancelled_without_aborting() {
    let (state, request_received) = hanging_server_state();
    let (addr, _server) = spawn_hanging_lookup(state.clone()).await;

    let transport = Arc::new(RustTransport::new());
    let conn = open_connection(&transport, addr).await;

    let result = cancel_once_the_request_is_on_the_wire(
        &transport,
        request_received,
        "connection_get_query_status",
        "monitoring request",
        ConnectionGetQueryStatusRequest {
            conn_handle: Some(conn),
            query_id: FINISHED_QUERY_ID.to_string(),
        }
        .encode_to_vec(),
    )
    .await;

    assert_cancelled(result);
    assert_hung_on(&state, &format!("/monitoring/queries/{FINISHED_QUERY_ID}"));
    assert_no_abort_requests(&state);
}

/// The same guarantee for `connection_get_query_result`, which parks on the same
/// `GET /queries/{id}/result` as `connection_get_result_set`.
///
/// Worth its own test despite sharing that endpoint: it is a different entry
/// point that returns a full result rather than a result-set handle, and it is
/// the one of the three that reaches `extract_rowset_data` — so it is the only
/// finished-query lookup where the ctx it observes is also forwarded onward, to
/// abort an in-flight PUT/GET transfer. Catches the same
/// marked-but-not-observing trap described on the result-set test above.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancelling_an_in_flight_query_result_fetch_reports_cancelled_without_aborting() {
    let (state, request_received) = hanging_server_state();
    let (addr, _server) = spawn_hanging_lookup(state.clone()).await;

    let transport = Arc::new(RustTransport::new());
    let conn = open_connection(&transport, addr).await;

    let result = cancel_once_the_request_is_on_the_wire(
        &transport,
        request_received,
        "connection_get_query_result",
        "result lookup",
        ConnectionGetQueryResultRequest {
            conn_handle: Some(conn),
            query_id: FINISHED_QUERY_ID.to_string(),
        }
        .encode_to_vec(),
    )
    .await;

    assert_cancelled(result);
    assert_hung_on(&state, &format!("/queries/{FINISHED_QUERY_ID}/result"));
    assert_no_abort_requests(&state);
}

/// Cancelling an async *submission* aborts the query server-side, like the
/// execute and prepare paths — not drop-only like the finished-query lookups.
///
/// The reason is specific to this RPC: a cancelled submission never returns its
/// `query_id`, so a query left running is one the caller cannot poll, abort, or
/// even name. `assert_aborted_the_submitted_query` matches the abort against the
/// `requestId` the mock actually saw, which is what makes this an abort *of this
/// submission* rather than of something.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancelling_an_in_flight_async_submission_aborts_it_server_side() {
    let (state, request_received) = hanging_server_state();
    let (addr, _server) = spawn_hanging_query(state.clone()).await;

    let transport = Arc::new(RustTransport::new());
    let stmt = statement_with_slow_query(&transport, addr).await;

    let result = cancel_once_the_request_is_on_the_wire(
        &transport,
        request_received,
        "statement_execute_async",
        "query-request",
        StatementExecuteAsyncRequest {
            stmt_handle: Some(stmt),
            bindings: None,
        }
        .encode_to_vec(),
    )
    .await;

    assert_cancelled_with_abort(result, Some(CancellationAbortOutcome::Aborted));
    assert_aborted_the_submitted_query(&state);
}

/// `connection_request_token` exchanges the master token for a session token on
/// the same untimed client, so an unanswered `POST /session/token-request`
/// blocked the caller indefinitely before it was marked.
///
/// No abort is asserted because this is not a query: cancelling drops the
/// exchange, leaving the master token untouched and any session token the server
/// minted simply unused.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancelling_an_in_flight_token_request_reports_cancelled() {
    let (state, request_received) = hanging_server_state();
    let (addr, _server) = spawn_hanging_token_request(state.clone()).await;

    let transport = Arc::new(RustTransport::new());
    let conn = open_connection(&transport, addr).await;

    let result = cancel_once_the_request_is_on_the_wire(
        &transport,
        request_received,
        "connection_request_token",
        "token-request",
        ConnectionTokenRequest {
            conn_handle: Some(conn),
            request_type: TokenRequestType::Issue as i32,
        }
        .encode_to_vec(),
    )
    .await;

    assert_cancelled(result);
    assert_hung_on(&state, "/session/token-request");
    assert_no_abort_requests(&state);
}

/// `connection_send_http` forwards a caller-chosen request on the connection's
/// client, which carries no request timeout — so before it was marked, a server
/// that accepted the request and never answered blocked the calling wrapper
/// thread indefinitely with no way out. Cancelling must end it.
///
/// No abort is asserted for a different reason than the query lookups: this RPC
/// is not a query at all, so an abort-request here would target a query id the
/// caller never submitted.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancelling_an_in_flight_send_http_reports_cancelled() {
    let (state, request_received) = hanging_server_state();
    let (addr, _server) = spawn_hanging_send_http(state.clone()).await;

    let transport = Arc::new(RustTransport::new());
    let conn = open_connection(&transport, addr).await;

    let result = cancel_once_the_request_is_on_the_wire(
        &transport,
        request_received,
        "connection_send_http",
        SEND_HTTP_PROBE_PATH,
        ConnectionSendHttpRequest {
            conn_handle: Some(conn),
            method: "GET".to_string(),
            url: SEND_HTTP_PROBE_PATH.to_string(),
            headers: HashMap::new(),
            body: None,
        }
        .encode_to_vec(),
    )
    .await;

    assert_cancelled(result);
    assert_hung_on(&state, SEND_HTTP_PROBE_PATH);
    assert_no_abort_requests(&state);
}

/// The same guarantee once the response *headers* have already arrived, so the
/// cancel lands in `response.bytes()` rather than in `send()`.
///
/// Worth separating from the test above: a server that never answers at all is
/// the easy case — reqwest is still waiting on the first byte. The case the
/// changelog is actually about is a server that accepted the request, returned
/// `200`, and then went quiet holding the body open. Because the connection's
/// client carries no request timeout, that stalls the caller just as
/// indefinitely, and only cancellation ends it.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancelling_send_http_while_reading_the_body_reports_cancelled() {
    let (state, request_received) = hanging_server_state();
    let (addr, _server) = spawn_hanging_send_http_body(state.clone()).await;

    let transport = Arc::new(RustTransport::new());
    let conn = open_connection(&transport, addr).await;

    let result = cancel_once_the_request_is_on_the_wire(
        &transport,
        request_received,
        "connection_send_http",
        SEND_HTTP_PROBE_PATH,
        ConnectionSendHttpRequest {
            conn_handle: Some(conn),
            method: "GET".to_string(),
            url: SEND_HTTP_PROBE_PATH.to_string(),
            headers: HashMap::new(),
            body: None,
        }
        .encode_to_vec(),
    )
    .await;

    assert_cancelled(result);
    assert_hung_on(&state, SEND_HTTP_PROBE_PATH);
    assert_no_abort_requests(&state);
}

/// A submission GS accepted but which came back without a usable `query_id` must
/// abort the query it cannot name, and report an internal error rather than
/// blaming the caller.
///
/// This is the same orphan a cancelled submission would leave, reached without any
/// cancellation: `with_cleanup_opt`'s cleanup is armed against the operation token
/// and is already disarmed by the time the submit future returns `Ok`, so nothing
/// covers a 200 that yields no id. Without the explicit abort the query keeps
/// running and the caller has no id to stop it with.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_async_submission_without_a_query_id_aborts_the_query_it_cannot_name() {
    let (state, _request_received) = hanging_server_state();
    state.omit_query_id.store(true, Ordering::SeqCst);
    let (addr, _server) = spawn_answering_query(state.clone()).await;

    let transport = Arc::new(RustTransport::new());
    let stmt = statement_with_slow_query(&transport, addr).await;

    let (operation, _token) = transport.register();
    let result = tokio::time::timeout(
        BOUND,
        transport.handle_message_cancellable(
            "DatabaseDriver",
            "statement_execute_async",
            StatementExecuteAsyncRequest {
                stmt_handle: Some(stmt),
                bindings: None,
            }
            .encode_to_vec(),
            operation,
        ),
    )
    .await
    .expect("an answered submission must return inside the bound");

    let ex = match result {
        Err(ProtoError::Application(bytes)) => {
            DriverException::decode(&bytes[..]).expect("decodes as DriverException")
        }
        other => panic!("a submission with no query id must fail, got {other:?}"),
    };
    assert_eq!(
        ex.kind,
        ErrorKind::InternalError as i32,
        "a missing query id is a driver/server fault, not the caller's: {ex:?}"
    );

    // The abort is the whole point: it names the query by the `requestId` the
    // submission was sent with, which is the only handle left once GS has
    // withheld the id.
    assert_aborted_the_submitted_query(&state);
}
