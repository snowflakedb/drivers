//! A minimal HTTP `CONNECT` forward proxy for hermetic proxy-transfer tests.
//!
//! Unlike `wiremock` (a target-server stub with no `CONNECT` handling at all),
//! this is a real forward proxy: it reads the `CONNECT <authority> HTTP/1.1`
//! request line a client sends when tunnelling HTTPS through a proxy, **records
//! the exact authority**, answers `200 Connection Established`, and then
//! `copy_bidirectional`s the tunnel to a fixed backend (the loopback
//! [`crate::common::tls_proxy::TlsProxy`], which TLS-terminates in front of a
//! `wiremock::MockServer`).
//!
//! Because the authority is recorded *before* any backend byte flows, a test
//! that asserts the recorded authority is proving that the transfer's very
//! first action on the wire was a `CONNECT` to the intended origin host — i.e.
//! that the request genuinely transited the proxy, not merely that "some
//! transfer succeeded".
//!
//! Each proxy owns its own recording buffer (no process-global state) and binds
//! `127.0.0.1:0` (OS-assigned port), so tests are parallel-safe.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// A running CONNECT forward proxy that tunnels to `backend_addr` and records
/// the authority of every `CONNECT` it receives.
pub struct ConnectProxy {
    addr: SocketAddr,
    connects: Arc<Mutex<Vec<String>>>,
    accept_handle: tokio::task::JoinHandle<()>,
    child_handles: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
}

impl ConnectProxy {
    /// Start a CONNECT proxy on a random loopback port that bridges accepted
    /// tunnels to `backend_addr`. Must be called within a tokio runtime.
    pub async fn start(backend_addr: SocketAddr) -> Self {
        let connects = Arc::new(Mutex::new(Vec::new()));
        let child_handles = Arc::new(Mutex::new(Vec::new()));
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind CONNECT proxy listener");
        let addr = listener.local_addr().unwrap();

        let recorder = Arc::clone(&connects);
        let children = Arc::clone(&child_handles);
        let accept_handle = tokio::spawn(async move {
            loop {
                let Ok((client, _)) = listener.accept().await else {
                    continue;
                };
                let recorder = Arc::clone(&recorder);
                let handle = tokio::spawn(async move {
                    handle_tunnel(client, backend_addr, recorder).await;
                });
                children.lock().unwrap().push(handle);
            }
        });

        Self {
            addr,
            connects,
            accept_handle,
            child_handles,
        }
    }

    /// The loopback port this proxy listens on (feed it to `proxy_port`).
    pub fn port(&self) -> u16 {
        self.addr.port()
    }

    /// Authorities of every `CONNECT` observed so far, in arrival order.
    pub fn observed_connects(&self) -> Vec<String> {
        self.connects.lock().unwrap().clone()
    }
}

impl Drop for ConnectProxy {
    /// Aborts the accept-loop task and every per-connection tunnel task it
    /// spawned, so none of them keep running on the shared tokio runtime
    /// after this proxy goes out of scope at the end of a test (even one
    /// still mid-copy).
    fn drop(&mut self) {
        self.accept_handle.abort();
        for handle in self.child_handles.lock().unwrap().drain(..) {
            handle.abort();
        }
    }
}

/// Reads one `CONNECT` request, records its authority, replies `200`, and
/// bridges the tunnel to `backend_addr`. Any protocol deviation just drops the
/// connection — the test observes the failure as a transfer error.
async fn handle_tunnel(
    mut client: TcpStream,
    backend_addr: SocketAddr,
    connects: Arc<Mutex<Vec<String>>>,
) {
    // Read the request head up to the blank-line terminator, one byte at a time
    // so we never consume tunnel/TLS bytes that follow it.
    let mut head = Vec::with_capacity(128);
    let mut byte = [0u8; 1];
    loop {
        match client.read(&mut byte).await {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        head.push(byte[0]);
        if head.ends_with(b"\r\n\r\n") {
            break;
        }
        if head.len() > 8192 {
            return;
        }
    }

    let text = String::from_utf8_lossy(&head);
    let Some(request_line) = text.lines().next() else {
        return;
    };
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let authority = parts.next().unwrap_or("").to_string();
    if !method.eq_ignore_ascii_case("CONNECT") || authority.is_empty() {
        let _ = client
            .write_all(b"HTTP/1.1 405 Method Not Allowed\r\n\r\n")
            .await;
        return;
    }

    // Record BEFORE establishing the tunnel, so a recorded authority strictly
    // precedes any backend byte.
    connects.lock().unwrap().push(authority);

    let Ok(mut backend) = TcpStream::connect(backend_addr).await else {
        let _ = client.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\n").await;
        return;
    };
    if client
        .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
        .await
        .is_err()
    {
        return;
    }
    let _ = tokio::io::copy_bidirectional(&mut client, &mut backend).await;
}
