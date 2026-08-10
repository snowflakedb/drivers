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
//! * the caller gets `STATUS_CODE_CANCELLED` **without** waiting for the server,
//!   which is the whole point of cancelling.
//!
//! Every wait here is bounded: the mock deliberately never answers, so an
//! unbounded wait would hang the suite rather than fail it.

use prost::Message as _;
use proto_utils::{ProtoError, Transport};
use sf_core::protobuf::apis::RustTransport;
use sf_core::protobuf::generated::database_driver_v1::{
    ConfigSetting, ConnectionInitRequest, ConnectionNewRequest, ConnectionNewResponse,
    ConnectionSetOptionsRequest, ConnectionSetOptionsResponse, DatabaseInitRequest,
    DatabaseInitResponse, DatabaseNewRequest, DatabaseNewResponse, DriverException, StatusCode,
    config_setting,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::mpsc::{SyncSender, sync_channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::AsyncReadExt;
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

    match result {
        Err(ProtoError::Application(bytes)) => {
            let ex = DriverException::decode(&bytes[..]).expect("decodes as DriverException");
            assert_eq!(
                ex.status_code,
                StatusCode::Cancelled as i32,
                "a cancelled operation must report STATUS_CODE_CANCELLED, got {ex:?}"
            );
        }
        other => panic!("expected a cancelled application error, got {other:?}"),
    }
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
