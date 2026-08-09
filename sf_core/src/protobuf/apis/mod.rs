use crate::apis::operation_ctx::OperationCtx;
use crate::protobuf::apis::database_driver_v1::{DatabaseDriverImpl, DriverProviders};
use crate::protobuf::generated::database_driver_v1::{
    DatabaseDriverServer, DriverException, StatusCode, observes_cancellation,
};
use crate::utils::sync::MutexRecoverExt;
use der::Encode;
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

    /// Route a decoded RPC to its service. Shared by both [`Transport`] entry
    /// points so they cannot diverge in how they dispatch.
    async fn dispatch(
        &self,
        ctx: Option<&OperationCtx>,
        service: &str,
        method: &str,
        message: Vec<u8>,
    ) -> Result<Vec<u8>, ProtoError<Vec<u8>>> {
        match service {
            "DatabaseDriver" => self.driver.handle_message(ctx, method, message).await,
            _ => Err(ProtoError::Transport(format!("Unknown API: {}", service))),
        }
    }

    /// Mint an operation handle and register a token for it; see
    /// [`CancellationRegistry::register`].
    ///
    /// The token comes back as well as the handle so an in-process caller can
    /// tie cancellation to a lifetime — `python_bridge` arms a
    /// [`tokio_util::sync::CancellationToken::drop_guard`] with it, so dropping
    /// the Python awaitable cancels the operation. Callers that only need to
    /// cancel explicitly can ignore it and use [`Self::cancel`].
    pub fn register(&self) -> (u64, CancellationToken) {
        self.cancellations.register()
    }

    /// Cancel an in-flight operation by handle, from any thread. Unknown or
    /// already-completed handles are ignored.
    pub fn cancel(&self, handle: u64) {
        self.cancellations.cancel(handle);
    }
}

/// Encode a `DriverException { status_code: CANCELLED }` for the case where an
/// operation could not even be started under its handle.
///
/// The normal cancelled response does **not** come from here: an operation that
/// observes its token returns `ApiError::Cancelled`, which the regular
/// `ApiError → DriverException` converter maps to `STATUS_CODE_CANCELLED`. This
/// covers only the pre-dispatch race where the handle was cancelled (or had
/// already completed) before the ctx could be built, plus the unmarked-RPC
/// fallback above.
///
/// TODO(SNOW-3675196): goes away with that fallback, leaving the pre-dispatch
/// race as the only caller — at which point returning `ApiError::Cancelled`
/// through the converter would cover it too.
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

    /// The token registered for `handle`, if the operation is still in flight.
    fn token(&self, handle: u64) -> Option<CancellationToken> {
        self.tokens().get(&handle).cloned()
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
    /// Dispatch without a cancellation trigger: the operation gets no ctx, so
    /// nothing can cancel it.
    async fn handle_message(
        &self,
        service: &str,
        method: &str,
        message: Vec<u8>,
    ) -> Result<Vec<u8>, ProtoError<Vec<u8>>> {
        self.dispatch(None, service, method, message).await
    }
}

/// Cancellation entry point. Inherent rather than part of [`Transport`]: every
/// bridge holds the concrete `RustTransport`, so nothing needs to reach it
/// through the trait, and `proto_utils` — which has no dependencies at all —
/// would otherwise have to know about cancellation to declare it.
impl RustTransport {
    /// Dispatch under `operation`'s registered token.
    ///
    /// Two shapes, picked by the proto's `async_first` marker via the generated
    /// [`observes_cancellation`]:
    ///
    /// * **Marked** — the ctx is handed down and the operation
    ///   observes the token itself, where it still has the state to unwind
    ///   cleanly. Deliberately *not* raced here as well: this layer would resolve
    ///   in nanoseconds and drop the operation before any cleanup could finish.
    /// * **Unmarked** — no operation to observe the token, so this layer keeps the
    ///   pre-existing race: the work is dropped and the call reports cancelled.
    ///   This is what preserves today's behaviour for every RPC not yet marked.
    ///
    /// The handle is deregistered on every exit — success, error, cancel, or
    /// panic — so callers never pair register/deregister themselves. An unknown
    /// handle means the operation was cancelled or completed before dispatch,
    /// and resolves straight to a cancelled response.
    pub async fn handle_message_cancellable(
        &self,
        service: &str,
        method: &str,
        message: Vec<u8>,
        operation: u64,
    ) -> Result<Vec<u8>, ProtoError<Vec<u8>>> {
        let _guard = self.cancellations.deregister_guard(operation);
        let Some(token) = self.cancellations.token(operation) else {
            return Err(ProtoError::Application(encode_cancelled()));
        };

        let marked = observes_cancellation(method);
        let ctx = marked.then(|| OperationCtx::from_registered(operation, token.clone()));

        // One instantiation of the dispatch future, awaited by whichever branch
        // below applies.
        let dispatched = self.dispatch(ctx.as_ref(), service, method, message);

        if marked {
            return dispatched.await;
        }

        // TODO(SNOW-3675196): remove this fallback once every RPC that can block
        // is marked `async_first` and therefore observes cancellation itself.
        //
        // It exists only to keep unmarked RPCs behaving exactly as they did
        // before cancellation moved into the operation layer — Python's aio
        // client routes all of its RPCs through here, so dropping it early would
        // silently turn cancellation into a no-op for everything not yet marked.
        // What is left unmarked at the end (setters, handle allocators) cannot
        // block, so racing those achieves nothing and this branch — along with
        // `encode_cancelled` — can then go away entirely.
        token
            .run_until_cancelled(dispatched)
            .await
            .unwrap_or_else(|| Err(ProtoError::Application(encode_cancelled())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost::Message as _;

    fn assert_cancelled(result: Result<Vec<u8>, ProtoError<Vec<u8>>>) {
        match result {
            Err(ProtoError::Application(bytes)) => {
                let ex = DriverException::decode(&bytes[..]).expect("decodes as DriverException");
                assert_eq!(ex.status_code, StatusCode::Cancelled as i32);
            }
            _ => panic!("expected a cancelled application error"),
        }
    }

    #[tokio::test]
    async fn cancelling_before_dispatch_yields_a_cancelled_driver_exception() {
        let transport = RustTransport::new();
        let (handle, _token) = transport.register();
        transport.cancel(handle); // already cancelled before the call starts

        assert_cancelled(
            transport
                .handle_message_cancellable("DatabaseDriver", "connection_new", vec![], handle)
                .await,
        );
    }

    /// An unknown handle means the operation was cancelled (or already
    /// completed) before dispatch could resolve it, so it must not silently run
    /// uncancellably.
    #[tokio::test]
    async fn unknown_handle_yields_a_cancelled_driver_exception() {
        let transport = RustTransport::new();

        assert_cancelled(
            transport
                .handle_message_cancellable("DatabaseDriver", "connection_new", vec![], u64::MAX)
                .await,
        );
    }

    #[tokio::test]
    async fn live_handle_dispatches_normally() {
        let transport = RustTransport::new();
        let (handle, _token) = transport.register(); // never cancelled

        let result = transport
            .handle_message_cancellable("UnknownService", "whatever", vec![], handle)
            .await;

        match result {
            Err(ProtoError::Transport(msg)) => assert!(msg.contains("Unknown API")),
            _ => panic!("expected the passed-through transport error"),
        }
    }

    #[tokio::test]
    async fn plain_handle_message_is_never_cancellable() {
        let transport = RustTransport::new();

        // No handle, so nothing can cancel it: the detached ctx's token has no
        // canceller and the call must dispatch normally.
        let result = transport
            .handle_message("UnknownService", "whatever", vec![])
            .await;

        match result {
            Err(ProtoError::Transport(msg)) => assert!(msg.contains("Unknown API")),
            _ => panic!("expected the passed-through transport error"),
        }
    }

    /// The handle is deregistered on every exit, so a cancel arriving after the
    /// operation finished is a no-op rather than affecting a later operation.
    #[tokio::test]
    async fn handle_is_deregistered_once_the_operation_completes() {
        let transport = RustTransport::new();
        let (handle, _token) = transport.register();

        let _ = transport
            .handle_message_cancellable("UnknownService", "whatever", vec![], handle)
            .await;

        assert!(transport.cancellations.token(handle).is_none());
    }

    #[test]
    fn register_mints_distinct_handles() {
        let transport = RustTransport::new();
        assert_ne!(transport.register().0, transport.register().0);
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

    /// The proto's `async_first` marker is the single source of truth for which
    /// operations observe cancellation themselves. If this drifts, an operation
    /// either loses its ctx or gets raced at two layers at once.
    #[test]
    fn marked_set_follows_the_proto_marker() {
        assert!(
            observes_cancellation("connection_init"),
            "connection_init is marked async_first in the proto"
        );
        assert!(
            !observes_cancellation("connection_new"),
            "unmarked RPCs must not claim to observe cancellation themselves"
        );
        assert!(!observes_cancellation("no_such_method"));
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
