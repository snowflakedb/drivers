use crate::protobuf::apis::database_driver_v1::{DatabaseDriverImpl, DriverProviders};
use crate::protobuf::generated::database_driver_v1::{
    DatabaseDriverServer, DriverException, StatusCode,
};
use crate::utils::sync::MutexRecoverExt;
use proto_utils::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use tokio_util::sync::CancellationToken;

pub mod database_driver_v1;

pub struct RustTransport {
    driver: DatabaseDriverImpl,
    /// The single cancel sink for this transport instance (see [`CancellationRegistry`]).
    cancellations: CancellationRegistry,
}

impl Default for RustTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl RustTransport {
    pub fn new() -> Self {
        Self::new_with(DriverProviders::default())
    }

    pub fn new_with(providers: DriverProviders) -> Self {
        Self {
            driver: DatabaseDriverImpl::new_with(providers),
            cancellations: CancellationRegistry::new(),
        }
    }

    pub fn is_troubleshooting(&self) -> bool {
        self.driver.is_troubleshooting()
    }

    /// Mint an operation handle + [`CancellationToken`]; see [`CancellationRegistry::register`].
    pub fn register(&self) -> (u64, CancellationToken) {
        self.cancellations.register()
    }

    /// Cancel an in-flight operation by handle, from any thread. Unknown or
    /// already-completed handles are ignored.
    pub fn cancel(&self, handle: u64) {
        self.cancellations.cancel(handle);
    }

    /// Runs an operation registered under `handle` (via [`Self::register`]),
    /// racing the work against `token` and deregistering `handle` on completion.
    ///
    /// Completion wins ties: if the operation finishes it returns the real
    /// result, even if a cancel raced in. Only if `token` is cancelled *before*
    /// the work completes is the in-flight `handle_message` future dropped —
    /// aborting the current HTTP request (best-effort, future-drop) — and the
    /// call resolves to an application error carrying a [`DriverException`] with
    /// `STATUS_CODE_CANCELLED`. Used by bridges that expose async-first,
    /// cancellable RPCs (the JDBC bridge, the async C API).
    pub async fn handle_message_cancellable(
        &self,
        handle: u64,
        service: &str,
        method: &str,
        message: Vec<u8>,
        token: CancellationToken,
    ) -> Result<Vec<u8>, ProtoError<Vec<u8>>> {
        // Deregister `handle` on completion — success, error, cancel, or panic —
        // so callers never have to pair register/deregister themselves.
        let _guard = self.cancellations.deregister_guard(handle);
        // TODO(SNOW-3675196): handle_message should accept the CancellationToken and
        //       pass it down the stack, so every operation can race against this token
        //       and cancel at a proper checkpoint instead of only via future-drop.
        token
            .run_until_cancelled(self.handle_message(service, method, message))
            .await
            .unwrap_or_else(|| Err(ProtoError::Application(encode_cancelled())))
    }
}

/// Encode a `DriverException { status_code: CANCELLED }` — the response bytes a
/// cancelled operation surfaces through the application-error path.
fn encode_cancelled() -> Vec<u8> {
    use prost::Message as _;
    DriverException {
        status_code: StatusCode::Cancelled as i32,
        ..Default::default()
    }
    .encode_to_vec()
}

/// Registry of in-flight cancellable operations, handle → token: the single
/// cancel sink for a [`RustTransport`]. Bridges (C API, JDBC) register a handle
/// before starting an async-first RPC and cancel by handle from any thread.
/// `std::sync::Mutex` — `cancel` runs on arbitrary native threads and must not
/// need to `.await` to take the lock; the guarded sections are tiny.
struct CancellationRegistry {
    tokens: Mutex<HashMap<u64, CancellationToken>>,
    /// Monotonic handle source (starts at 1 so 0 stays reserved for "no handle").
    next_handle: AtomicU64,
}

impl CancellationRegistry {
    fn new() -> Self {
        Self {
            tokens: Mutex::new(HashMap::new()),
            next_handle: AtomicU64::new(1),
        }
    }

    fn tokens(&self) -> MutexGuard<'_, HashMap<u64, CancellationToken>> {
        self.tokens.lock_recover()
    }

    /// Mint a handle, register a fresh token, return both. No tombstone is
    /// needed: the handle is minted here and only exposed afterwards, so it can
    /// never be cancelled before it exists.
    fn register(&self) -> (u64, CancellationToken) {
        let handle = self.next_handle.fetch_add(1, Ordering::Relaxed);
        let token = CancellationToken::new();
        self.tokens().insert(handle, token.clone());
        (handle, token)
    }

    /// The entry is left in place so a canceller racing an in-flight await still
    /// finds the token; [`Self::deregister`] removes it on completion.
    fn cancel(&self, handle: u64) {
        if let Some(token) = self.tokens().get(&handle) {
            token.cancel();
        }
    }

    fn deregister(&self, handle: u64) {
        self.tokens().remove(&handle);
    }

    /// A guard that deregisters `handle` when dropped, so an operation's entry is
    /// removed on completion, error, cancel, or panic (RAII).
    fn deregister_guard(&self, handle: u64) -> DeregisterGuard<'_> {
        DeregisterGuard {
            registry: self,
            handle,
        }
    }
}

/// RAII guard that removes `handle` from its [`CancellationRegistry`] on drop.
struct DeregisterGuard<'a> {
    registry: &'a CancellationRegistry,
    handle: u64,
}

impl Drop for DeregisterGuard<'_> {
    fn drop(&mut self) {
        self.registry.deregister(self.handle);
    }
}

impl Transport for RustTransport {
    async fn handle_message(
        &self,
        service: &str,
        method: &str,
        message: Vec<u8>,
    ) -> Result<Vec<u8>, ProtoError<Vec<u8>>> {
        match service {
            "DatabaseDriver" => self.driver.handle_message(method, message).await,
            _ => Err(ProtoError::Transport(format!("Unknown API: {}", service))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost::Message as _;

    #[tokio::test]
    async fn cancelled_token_yields_cancelled_driver_exception() {
        let transport = RustTransport::new();
        let (handle, token) = transport.register();
        token.cancel(); // already cancelled before the call starts

        let result = transport
            .handle_message_cancellable(handle, "DatabaseDriver", "connection_new", vec![], token)
            .await;

        match result {
            Err(ProtoError::Application(bytes)) => {
                let ex = DriverException::decode(&bytes[..]).expect("decodes as DriverException");
                assert_eq!(ex.status_code, StatusCode::Cancelled as i32);
            }
            _ => panic!("expected a cancelled application error"),
        }
    }

    #[tokio::test]
    async fn uncancelled_token_delegates_to_handle_message() {
        let transport = RustTransport::new();
        let (handle, token) = transport.register(); // never cancelled

        let result = transport
            .handle_message_cancellable(handle, "UnknownService", "whatever", vec![], token)
            .await;

        match result {
            Err(ProtoError::Transport(msg)) => assert!(msg.contains("Unknown API")),
            _ => panic!("expected the passed-through transport error"),
        }
    }

    #[test]
    fn register_mints_distinct_handles() {
        let transport = RustTransport::new();
        let (h1, _t1) = transport.register();
        let (h2, _t2) = transport.register();
        assert_ne!(h1, h2);
    }

    #[test]
    fn cancel_flips_the_registered_token() {
        let transport = RustTransport::new();
        let (handle, token) = transport.register();
        assert!(!token.is_cancelled());
        transport.cancel(handle);
        assert!(token.is_cancelled());
    }

    #[test]
    fn cancel_unknown_handle_is_noop() {
        let transport = RustTransport::new();
        transport.cancel(u64::MAX); // must not panic
    }

    #[test]
    fn deregister_guard_removes_entry_on_drop() {
        let registry = CancellationRegistry::new();
        let (handle, token) = registry.register();
        drop(registry.deregister_guard(handle));
        registry.cancel(handle);
        assert!(!token.is_cancelled());
    }
}
