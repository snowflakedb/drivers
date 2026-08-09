//! Per-operation context.
//!
//! One [`OperationCtx`] represents a single in-flight operation. Today it
//! carries the operation's identity and its cancellation signal; it is the
//! natural place to hang other per-operation concerns (tracing spans,
//! deadlines, telemetry) as they arrive.
//!
//! It is threaded as an explicit parameter from the FFI boundary down to the
//! operation layer, so an operation can observe cancellation where it still has
//! the state to unwind cleanly, rather than being dropped mid-`await` from
//! above.
//!
//! Callers with no cancellation trigger — a blocking FFI entry, an internal
//! caller, a test — pass `None`, so "nothing can cancel this" is a named state
//! rather than an inert ctx that looks live but silently loses cancellability.
//!
//! **An operation must be raced against its token in exactly one place.** Two
//! racing wrappers is a silent bug: the outer one wins, having no work to do,
//! and drops the inner one before any cooperative unwind can finish — leaving
//! code that reads as though it cleans up but never does.

use crate::apis::database_driver_v1::ApiError;
use tokio_util::sync::CancellationToken;

/// Identity and cancellation signal for one in-flight operation.
///
/// Deliberately **not** `Clone`: [`Drop`] cancels the token, so a stray clone
/// going out of scope would cancel a live operation. Pass `&OperationCtx` down
/// the stack instead.
#[derive(Debug)]
pub struct OperationCtx {
    /// Cancelled by `cancel(handle)` from any thread, or on drop.
    token: CancellationToken,
    /// Operation handle for log/telemetry correlation. `0` means the ctx has no
    /// entry in the transport's registry (an in-process owner minted it).
    id: u64,
}

impl OperationCtx {
    /// Wrap the token registered for `id` so the operation can observe it.
    pub fn from_registered(id: u64, token: CancellationToken) -> Self {
        Self { token, id }
    }

    /// A ctx whose token this struct owns, for an in-process caller that holds
    /// its own trigger (Node, ODBC) rather than going through the transport's
    /// handle registry.
    ///
    /// Deliberately not `new`/`Default`: a `Default` impl would let an inert ctx
    /// be conjured implicitly (`..Default::default()`, any `T: Default` bound),
    /// which is the sentinel hazard `Option<&OperationCtx>` exists to avoid.
    pub fn with_own_token() -> Self {
        Self {
            token: CancellationToken::new(),
            id: 0,
        }
    }

    /// Request cancellation, from any thread. Safe to call more than once and
    /// after the operation has finished.
    pub fn cancel(&self) {
        self.token.cancel();
    }

    /// The operation handle, or `0` for a ctx with no registry entry.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// The underlying token — for deriving child tokens for spawned subtasks,
    /// and for the polling/`select!` shapes [`Self::run`] does not cover
    /// (`ctx.token().is_cancelled()`, `ctx.token().cancelled().await`).
    pub fn token(&self) -> &CancellationToken {
        &self.token
    }

    /// Run `fut` as this operation, returning `ApiError::Cancelled` (converted
    /// into the caller's error type) if cancellation is observed first.
    ///
    /// This is *the* cancellation observation point for an operation — see the
    /// module docs on why there must be exactly one.
    ///
    /// Delegates to [`CancellationToken::run_until_cancelled`] rather than
    /// hand-rolling a `select!`, so this shares one implementation of the
    /// semantics with the transport's fallback for unmarked RPCs:
    ///
    /// * **Already cancelled on entry → `fut` is never polled.** A handle
    ///   cancelled before dispatch must not start any work.
    /// * **Otherwise biased towards completion.** `fut` is polled before the
    ///   token on every wake, so an operation that finishes returns its real
    ///   result even if a cancel raced in.
    pub async fn run<T, E, F>(&self, method: &str, fut: F) -> Result<T, E>
    where
        F: Future<Output = Result<T, E>>,
        E: From<ApiError>,
    {
        self.token
            .run_until_cancelled(fut)
            .await
            .unwrap_or_else(|| {
                tracing::debug!(operation_id = self.id, method, "operation cancelled");
                Err(cancelled().into())
            })
    }
}

/// Build an [`ApiError::Cancelled`]. The captured `Location` is this function
/// rather than the cancelled operation; the operation is identified by the
/// `method` field on the debug log and by the RPC the caller invoked.
fn cancelled() -> ApiError {
    snafu::IntoError::into_error(
        crate::apis::database_driver_v1::error::CancelledSnafu,
        snafu::NoneError,
    )
}

/// Run `fut` under `ctx` if there is one, otherwise run it unguarded.
///
/// `None` means "nothing can cancel this call" — the operation was reached
/// through a path that carries no operation handle. See the module docs on why
/// that is an `Option` rather than an inert ctx.
pub async fn run_opt<T, E, F>(ctx: Option<&OperationCtx>, method: &str, fut: F) -> Result<T, E>
where
    F: std::future::Future<Output = Result<T, E>>,
    E: From<ApiError>,
{
    match ctx {
        Some(ctx) => ctx.run(method, fut).await,
        None => fut.await,
    }
}

impl Drop for OperationCtx {
    /// Cancelling on drop guarantees that *every* unwind path signals anything
    /// derived from this token, including paths that never run a cancel arm —
    /// a panic, or an enclosing future being dropped.
    ///
    /// A no-op today (nothing outlives the operation yet); here to hold the
    /// invariant before the work that depends on it lands. Once an operation
    /// spawns a subtask that performs cleanup, that subtask survives its parent
    /// being dropped, and only this guarantees somebody still tells it to stop.
    fn drop(&mut self) {
        self.token.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apis::database_driver_v1::ApiError;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    #[test]
    fn fresh_ctx_is_not_cancelled_and_has_no_registry_id() {
        let ctx = OperationCtx::with_own_token();
        assert!(!ctx.token().is_cancelled());
        assert_eq!(ctx.id(), 0);
    }

    #[test]
    fn registered_ctx_observes_the_registry_token() {
        let token = CancellationToken::new();
        let ctx = OperationCtx::from_registered(7, token.clone());

        assert_eq!(ctx.id(), 7);
        assert!(!ctx.token().is_cancelled());
        token.cancel();
        assert!(ctx.token().is_cancelled());
    }

    #[test]
    fn drop_cancels_the_token_so_derived_work_is_always_signalled() {
        let token = CancellationToken::new();
        let child = token.child_token();

        drop(OperationCtx::from_registered(1, token));

        assert!(
            child.is_cancelled(),
            "dropping the ctx must signal tokens derived from it"
        );
    }

    #[tokio::test]
    async fn run_reports_cancelled_without_polling_a_body_cancelled_before_entry() {
        let token = CancellationToken::new();
        let ctx = OperationCtx::from_registered(3, token.clone());
        token.cancel();
        let started = Arc::new(AtomicBool::new(false));

        let flag = started.clone();
        let result: Result<u8, ApiError> = ctx
            .run("op", async move {
                flag.store(true, Ordering::SeqCst);
                Ok(1)
            })
            .await;

        assert!(matches!(result, Err(ApiError::Cancelled { .. })));
        assert!(
            !started.load(Ordering::SeqCst),
            "a handle cancelled before dispatch must not start any work"
        );
    }

    #[tokio::test]
    async fn run_reports_cancelled_when_flipped_while_the_body_is_pending() {
        let token = CancellationToken::new();
        let ctx = OperationCtx::from_registered(4, token.clone());

        tokio::spawn({
            let token = token.clone();
            async move {
                tokio::time::sleep(Duration::from_millis(20)).await;
                token.cancel();
            }
        });

        let result: Result<u8, ApiError> = ctx
            .run("op", async {
                tokio::time::sleep(Duration::from_secs(30)).await;
                Ok(1)
            })
            .await;

        assert!(matches!(result, Err(ApiError::Cancelled { .. })));
    }

    #[tokio::test]
    async fn run_is_biased_towards_completion_so_a_finished_operation_still_returns_its_result() {
        let token = CancellationToken::new();
        let ctx = OperationCtx::from_registered(5, token.clone());

        // Body is immediately ready and the token is flipped concurrently: the
        // pre-move transport documented "completion wins ties", so the real
        // result must survive.
        let result: Result<u8, ApiError> = ctx
            .run("op", async {
                token.cancel();
                Ok(42)
            })
            .await;

        assert!(matches!(result, Ok(42)));
    }

    #[tokio::test]
    async fn run_passes_through_when_never_cancelled() {
        let ctx = OperationCtx::with_own_token();

        let result: Result<u8, ApiError> = ctx.run("op", async { Ok(42) }).await;

        assert!(matches!(result, Ok(42)));
    }

    #[tokio::test]
    async fn run_opt_without_a_ctx_runs_unguarded() {
        let result: Result<u8, ApiError> = run_opt(None, "op", async { Ok(7) }).await;
        assert!(matches!(result, Ok(7)));
    }

    #[tokio::test]
    async fn run_opt_with_a_cancelled_ctx_reports_cancelled() {
        let token = CancellationToken::new();
        let ctx = OperationCtx::from_registered(6, token.clone());
        token.cancel();

        let result: Result<u8, ApiError> = run_opt(Some(&ctx), "op", async { Ok(1) }).await;

        assert!(matches!(result, Err(ApiError::Cancelled { .. })));
    }
}
