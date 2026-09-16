//! A transparent plain-TCP splice proxy that counts accepted connections.
//!
//! Unlike [`crate::common::connect_proxy::ConnectProxy`], this speaks no
//! protocol of its own: every accepted connection is spliced verbatim to a
//! fixed backend (a `wiremock::MockServer`'s plain-`http://` listener), so it
//! is transparent to whatever HTTP the client sends. Counting connections
//! (not requests) is the point — HTTP/1.1 keep-alive means many requests can
//! ride one connection, so this is the only way to observe from outside the
//! client whether a batch of file transfers opened one TCP connection or one
//! per file.
//!
//! Each proxy binds `127.0.0.1:0` (OS-assigned port) and owns no process-global
//! state, so tests using it are parallel-safe.

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::net::{TcpListener, TcpStream};

/// A running splice proxy that forwards every accepted connection to
/// `backend_addr` and counts how many connections it has accepted.
pub struct CountingProxy {
    addr: SocketAddr,
    accepts: Arc<AtomicUsize>,
    accept_handle: tokio::task::JoinHandle<()>,
    child_handles: Arc<std::sync::Mutex<Vec<tokio::task::JoinHandle<()>>>>,
}

impl CountingProxy {
    /// Starts a splice proxy on a random loopback port that forwards every
    /// accepted connection to `backend_addr`. Must be called within a tokio
    /// runtime.
    pub async fn start(backend_addr: SocketAddr) -> Self {
        let accepts = Arc::new(AtomicUsize::new(0));
        let child_handles = Arc::new(std::sync::Mutex::new(Vec::new()));
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind counting proxy listener");
        let addr = listener.local_addr().unwrap();

        let counter = Arc::clone(&accepts);
        let children = Arc::clone(&child_handles);
        let accept_handle = tokio::spawn(async move {
            loop {
                let Ok((client, _)) = listener.accept().await else {
                    continue;
                };
                // Counted at accept time, before any byte is spliced, so the
                // count reflects TCP connections opened, not requests served.
                counter.fetch_add(1, Ordering::SeqCst);
                let handle = tokio::spawn(async move {
                    splice(client, backend_addr).await;
                });
                children.lock().unwrap().push(handle);
            }
        });

        Self {
            addr,
            accepts,
            accept_handle,
            child_handles,
        }
    }

    /// The loopback address this proxy listens on — feed `http://{addr}` to a
    /// stage as its endpoint.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// `http://` URI for this proxy's listening address, ready to use as a
    /// stage `endpoint` or presigned URL host.
    pub fn uri(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// Number of TCP connections accepted so far.
    pub fn accept_count(&self) -> usize {
        self.accepts.load(Ordering::SeqCst)
    }
}

impl Drop for CountingProxy {
    /// Aborts the accept loop and every splice task it spawned, so none keep
    /// running on the shared tokio runtime after this proxy goes out of scope.
    fn drop(&mut self) {
        self.accept_handle.abort();
        for handle in self.child_handles.lock().unwrap().drain(..) {
            handle.abort();
        }
    }
}

/// Bridges `client` to a fresh connection to `backend_addr`, copying bytes
/// verbatim in both directions. A backend connect failure just drops the
/// client connection — the test observes the failure as a transfer error.
async fn splice(mut client: TcpStream, backend_addr: SocketAddr) {
    let Ok(mut backend) = TcpStream::connect(backend_addr).await else {
        return;
    };
    let _ = tokio::io::copy_bidirectional(&mut client, &mut backend).await;
}
