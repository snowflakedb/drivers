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
//!
//! That single race is why work needing to *survive* cancellation cannot simply
//! `select!` on the token deeper in the stack: there is no async `Drop`, so a
//! future dropped by the race above can never await its own cleanup. Inner
//! layers therefore **register** cleanup with [`OperationCtx::with_cleanup`]
//! instead of racing. Registration spawns a task that watches the token
//! independently of the future tree, so it still runs when the operation future
//! is dropped — and because it is not a race, it does not violate the invariant
//! above. [`OperationCtx::run`] then waits (briefly, bounded) for those tasks
//! before reporting cancellation.
//!
//! A layer too deep to hold the ctx takes a [`CleanupScope`] instead — the token
//! plus the registry, minus the ability to cancel or complete the operation. The
//! file-transfer stack uses this: a PUT registers its multipart abort from inside
//! the per-cloud upload, which is where the upload id to abort first exists.

use crate::apis::database_driver_v1::ApiError;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

/// Upper bound on how long [`OperationCtx::run`] waits for registered cleanup
/// before reporting the operation's outcome.
///
/// Deliberately short and deliberately *not* the same knob as the deadline a
/// cleanup applies to its own work (e.g. `ABORT_REQUEST_TIMEOUT` bounds the
/// abort-request POST itself). This one only decides how long the cancelled
/// *caller* blocks: long enough that "the operation reported cancelled" implies
/// "cleanup was issued" in the healthy case, short enough that a wedged cleanup
/// is not felt as a hung cancel. Exceeding it does not abandon the cleanup —
/// the task keeps running detached.
const CLEANUP_WAIT: std::time::Duration = std::time::Duration::from_secs(5);

/// Identity and cancellation signal for one in-flight operation.
///
/// Deliberately **not** `Clone`: [`Drop`] cancels the token, so a stray clone
/// going out of scope would cancel a live operation. Pass `&OperationCtx` down
/// the stack instead — or, for a layer that only needs to register cleanup and
/// must not be able to cancel or complete the operation, the [`CleanupScope`]
/// from [`Self::cleanup_scope`].
#[derive(Debug)]
pub struct OperationCtx {
    /// Held as one field so [`CleanupScope`] owns the arm/disarm logic and this
    /// type carries no second copy of it.
    scope: CleanupScope,
    /// Operation handle for log/telemetry correlation. `0` means the ctx has no
    /// entry in the transport's registry (an in-process owner minted it).
    id: u64,
}

impl OperationCtx {
    /// Wrap the token registered for `id` so the operation can observe it.
    pub fn from_registered(id: u64, token: CancellationToken) -> Self {
        Self {
            scope: CleanupScope::new(token),
            id,
        }
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
            scope: CleanupScope::new(CancellationToken::new()),
            id: 0,
        }
    }

    /// Request cancellation, from any thread. Safe to call more than once and
    /// after the operation has finished.
    pub fn cancel(&self) {
        self.scope.token.cancel();
    }

    /// The operation handle, or `0` for a ctx with no registry entry.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// The underlying token — for deriving child tokens for spawned subtasks,
    /// and for the polling/`select!` shapes [`Self::run`] does not cover
    /// (`ctx.token().is_cancelled()`, `ctx.token().cancelled().await`).
    pub fn token(&self) -> &CancellationToken {
        &self.scope.token
    }

    /// This operation's cleanup registry, for a layer that must be able to
    /// register cleanup but not to cancel or complete the operation.
    pub fn cleanup_scope(&self) -> &CleanupScope {
        &self.scope
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
        let outcome = self.scope.token.run_until_cancelled(fut).await;
        // Before reporting: give anything registered via `with_cleanup` a chance
        // to finish. On the success path every guard has already suppressed its
        // task, so this returns immediately and only buys leak-freedom.
        self.await_cleanup(method).await;
        outcome.unwrap_or_else(|| {
            tracing::debug!(operation_id = self.id, method, "operation cancelled");
            Err(cancelled().into())
        })
    }

    /// Run `work`, having registered `cleanup` to run if this operation is
    /// cancelled before `work` finishes. Delegates to
    /// [`CleanupScope::with_cleanup`] — see it for the semantics.
    pub async fn with_cleanup<T, C, F>(&self, cleanup: C, work: F) -> T
    where
        C: Future<Output = ()> + Send + 'static,
        F: Future<Output = T>,
    {
        self.scope.with_cleanup(cleanup, work).await
    }

    /// Close the cleanup tracker and wait, bounded by [`CLEANUP_WAIT`], for any
    /// registered cleanup to finish. Exceeding the bound leaves the task running
    /// detached rather than abandoning the work.
    async fn await_cleanup(&self, method: &str) {
        let tracker = &self.scope.cleanup;
        tracker.close();
        if tracker.is_empty() {
            return;
        }
        if tokio::time::timeout(CLEANUP_WAIT, tracker.wait())
            .await
            .is_err()
        {
            tracing::warn!(
                operation_id = self.id,
                method,
                wait_secs = CLEANUP_WAIT.as_secs(),
                "cancellation cleanup did not finish in time; continuing detached"
            );
        }
    }
}

/// One operation's cancellation signal plus its cleanup registry.
///
/// Unlike [`OperationCtx`], this is `Clone` and dropping it cancels nothing — it
/// is a handle to the registry, not the owner of the operation. That is what makes
/// it safe to thread into a lower layer, and being `'static` keeps a lifetime out
/// of that layer's types.
///
/// A clone must not outlive the [`OperationCtx`] it came from. Registering against
/// a scope whose ctx has already reported would arm a cleanup on a closed tracker
/// and an already-cancelled token: it would fire immediately, alongside the work it
/// was meant to guard, with nothing waiting for it. Every caller today borrows the
/// scope for the duration of one operation (`TransferCtx<'a>`), which the borrow
/// checker enforces — do not stash a clone anywhere longer-lived.
#[derive(Clone, Debug)]
pub struct CleanupScope {
    /// Cancelled by `cancel(handle)` from any thread, or when the owning
    /// [`OperationCtx`] is dropped. This is what wakes a registered cleanup.
    token: CancellationToken,
    /// Cleanup tasks registered by [`Self::with_cleanup`]. Tracked so
    /// [`OperationCtx::run`] can wait for them; spawned so they outlive a
    /// dropped operation future.
    cleanup: TaskTracker,
}

impl CleanupScope {
    fn new(token: CancellationToken) -> Self {
        Self {
            token,
            cleanup: TaskTracker::new(),
        }
    }

    /// Register `cleanup` to run if the operation is cancelled, returning a
    /// guard that suppresses it once the guarded work is no longer in flight.
    ///
    /// `cleanup` runs on a spawned task, so — unlike a `select!` arm — it still
    /// runs when the operation future is dropped by [`OperationCtx::run`]'s
    /// race. That is the whole point: the abort of a server-side query, or of a
    /// cloud multipart upload, must survive the local future that started it.
    ///
    /// Private on purpose: the only way to register cleanup is
    /// [`Self::with_cleanup`], which owns both arming and disarming. Handing out
    /// a bare guard would make "disarm at the right moment" a per-call-site
    /// obligation, and getting it wrong is silent.
    fn arm_cleanup<F>(&self, cleanup: F) -> CleanupGuard
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let done = CancellationToken::new();
        let (done_task, op_task) = (done.clone(), self.token.clone());
        self.cleanup.spawn(async move {
            tokio::select! {
                // Biased so that a guard which suppressed before the ctx was
                // dropped wins the tie — both tokens are cancelled by then.
                biased;
                _ = done_task.cancelled() => {}
                _ = op_task.cancelled() => cleanup.await,
            }
        });
        CleanupGuard { done }
    }

    /// Run `work`, having registered `cleanup` to run if the operation is
    /// cancelled before `work` finishes.
    ///
    /// Completion is decided by *reaching the disarm below*, not by inspecting the
    /// token. That distinction is load-bearing: a token check mis-handles the
    /// success-with-cancel race, where `work` completes and a cancel lands before
    /// the guard drops — the token is cancelled, yet the work finished and must
    /// not be undone. Here, `work` returning at all (`Ok` or `Err`) disarms;
    /// only being dropped mid-`await` leaves the cleanup armed.
    ///
    /// `cleanup` must not be cancellable by the operation's token: it is woken
    /// *by* that token, so anything deriving a child token from it (or reusing
    /// this scope) would abort itself instantly.
    pub async fn with_cleanup<T, C, F>(&self, cleanup: C, work: F) -> T
    where
        C: Future<Output = ()> + Send + 'static,
        F: Future<Output = T>,
    {
        let guard = self.arm_cleanup(cleanup);
        let out = work.await;
        guard.disarm();
        out
    }
}

/// Keeps a cleanup registration armed until [`Self::disarm`] is reached.
///
/// Private, and only ever created and consumed inside
/// [`CleanupScope::with_cleanup`] — so "armed but never disarmed" means exactly
/// "the guarded work did not finish", with no call site able to get it wrong.
struct CleanupGuard {
    /// Cancelled to tell the cleanup task the work is no longer in flight.
    done: CancellationToken,
}

impl CleanupGuard {
    /// The guarded work finished, so suppress the cleanup. Not called when the
    /// enclosing future is dropped mid-`await`, which is what leaves the cleanup
    /// armed to fire.
    fn disarm(self) {
        self.done.cancel();
    }
}

/// Build an [`ApiError::Cancelled`] with `abort: None` — an operation that fires
/// an abort attaches its outcome on the way out, which is what keeps this module
/// free of Snowflake concepts.
///
/// The captured `Location` is this function rather than the cancelled operation,
/// which is identified by the `method` field on the debug log instead.
fn cancelled() -> ApiError {
    ApiError::Cancelled {
        abort: None,
        location: snafu::location!(),
    }
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

/// Run `work` with cancellation `cleanup` registered under `ctx` if there is one.
///
/// With no ctx the call is unguarded — no cancellation can arrive for the cleanup
/// to respond to — so `work` simply runs and `cleanup` is dropped unused.
pub async fn with_cleanup_opt<T, C, F>(ctx: Option<&OperationCtx>, cleanup: C, work: F) -> T
where
    C: Future<Output = ()> + Send + 'static,
    F: Future<Output = T>,
{
    with_cleanup_scope_opt(ctx.map(OperationCtx::cleanup_scope), cleanup, work).await
}

/// [`CleanupScope`] sibling of [`with_cleanup_opt`], for a layer that receives
/// the scope rather than the whole ctx (the file-transfer stack). `None` means
/// nothing can cancel this call, so `work` simply runs and `cleanup` is dropped
/// unused.
pub async fn with_cleanup_scope_opt<T, C, F>(scope: Option<&CleanupScope>, cleanup: C, work: F) -> T
where
    C: Future<Output = ()> + Send + 'static,
    F: Future<Output = T>,
{
    match scope {
        Some(scope) => scope.with_cleanup(cleanup, work).await,
        None => work.await,
    }
}

/// Run `work`, undoing it with `abort` if it does not complete successfully —
/// whether it returns `Err` or is dropped by cancellation.
///
/// Those are two different mechanisms: cancellation is only observable through a
/// registered cleanup (the future is dropped, so it can never run its own unwind),
/// while an `Err` return has to be handled inline because the operation's token
/// never fired. Taking one `abort` closure for both is what stops them drifting —
/// a third failure path added inside `work` cannot forget to undo it.
///
/// `abort` is called at most once: [`CleanupScope::with_cleanup`] disarms the
/// registration as soon as `work` returns, so the registered copy runs only on a
/// cancellation-drop and the inline copy only on `Err`.
pub async fn with_abort_on_unwind<T, E, C, Fut, F>(
    scope: Option<&CleanupScope>,
    abort: C,
    work: F,
) -> Result<T, E>
where
    C: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
    F: Future<Output = Result<T, E>>,
{
    let abort = std::sync::Arc::new(abort);
    let on_cancel = {
        let abort = abort.clone();
        async move { (*abort)().await }
    };
    let outcome = with_cleanup_scope_opt(scope, on_cancel, work).await;
    if outcome.is_err() {
        (*abort)().await;
    }
    outcome
}

/// Runs a cancellable operation and cancels it as soon as `trigger` resolves,
/// for tests that need to observe what cancellation cleans up.
///
/// `build` receives the operation's [`CleanupScope`] so the work under test
/// registers cleanup exactly as it does in production. Returns `Some(output)` if
/// the work finished first, `None` if it was cancelled.
///
/// A `None` return implies **every registered cleanup has already finished**,
/// because [`OperationCtx::run`] awaits the tracker before reporting — which is
/// what lets a caller assert on cleanup effects with no sleep and no polling.
#[cfg(any(test, feature = "test-utils"))]
pub async fn cancelled_by<T, F>(
    trigger: impl Future<Output = ()> + Send + 'static,
    build: impl FnOnce(CleanupScope) -> F,
) -> Option<T>
where
    F: Future<Output = T>,
{
    let ctx = OperationCtx::with_own_token();
    let work = build(ctx.cleanup_scope().clone());

    let token = ctx.token().clone();
    tokio::spawn(async move {
        trigger.await;
        token.cancel();
    });

    let outcome: Result<Option<T>, ApiError> = ctx
        .run("cancelled_by", async { Ok(Some(work.await)) })
        .await;
    outcome.unwrap_or(None)
}

impl Drop for OperationCtx {
    /// Cancelling on drop guarantees that *every* unwind path signals anything
    /// derived from this token, including paths that never run a cancel arm —
    /// a panic, or an enclosing future being dropped.
    ///
    /// This is what makes [`Self::with_cleanup`] sound: a registered cleanup task
    /// outlives the operation future, so if the operation goes away for a reason
    /// other than an explicit cancel, only this tells the task to stop waiting.
    fn drop(&mut self) {
        self.scope.token.cancel();
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

    /// Records whether a registered cleanup ran, so the guard's suppress-vs-fire
    /// decision is asserted on observed behaviour rather than on token state.
    fn cleanup_probe() -> (Arc<AtomicBool>, impl Future<Output = ()> + Send + 'static) {
        let ran = Arc::new(AtomicBool::new(false));
        let flag = ran.clone();
        (ran, async move { flag.store(true, Ordering::SeqCst) })
    }

    #[tokio::test]
    async fn cleanup_is_suppressed_when_the_guarded_work_succeeds() {
        let token = CancellationToken::new();
        let ctx = OperationCtx::from_registered(8, token);
        let (ran, cleanup) = cleanup_probe();

        let result: Result<u8, ApiError> = ctx
            .run("op", ctx.with_cleanup(cleanup, async { Ok(1) }))
            .await;

        assert!(matches!(result, Ok(1)), "expected Ok(1), got {result:?}");
        assert!(
            !ran.load(Ordering::SeqCst),
            "work that completed needs no cleanup"
        );
    }

    #[tokio::test]
    async fn cleanup_is_suppressed_when_the_guarded_work_fails() {
        let token = CancellationToken::new();
        let ctx = OperationCtx::from_registered(9, token);
        let (ran, cleanup) = cleanup_probe();

        // A failure *unrelated* to cancellation, so the assertion below cannot
        // pass for the wrong reason: the guard must suppress because the scope
        // exited under its own steam, not because the error happened to be
        // `Cancelled`.
        let result: Result<u8, ApiError> = ctx
            .run(
                "op",
                ctx.with_cleanup(cleanup, async {
                    crate::apis::database_driver_v1::error::InvalidArgumentSnafu {
                        argument: "bad handle".to_string(),
                    }
                    .fail()
                }),
            )
            .await;

        assert!(
            matches!(result, Err(ApiError::InvalidArgument { .. })),
            "expected InvalidArgument, got {result:?}"
        );
        assert!(
            !ran.load(Ordering::SeqCst),
            "an errored operation is no longer in flight, so must not be aborted"
        );
    }

    #[tokio::test]
    async fn cleanup_runs_when_the_operation_is_cancelled_mid_flight() {
        let token = CancellationToken::new();
        let ctx = OperationCtx::from_registered(10, token.clone());
        let (ran, cleanup) = cleanup_probe();

        tokio::spawn({
            let token = token.clone();
            async move {
                tokio::time::sleep(Duration::from_millis(20)).await;
                token.cancel();
            }
        });

        let result: Result<u8, ApiError> = ctx
            .run(
                "op",
                ctx.with_cleanup(cleanup, async {
                    tokio::time::sleep(Duration::from_secs(30)).await;
                    Ok(1)
                }),
            )
            .await;

        assert!(
            matches!(result, Err(ApiError::Cancelled { .. })),
            "expected Cancelled, got {result:?}"
        );
        assert!(
            ran.load(Ordering::SeqCst),
            "cleanup must survive the operation future being dropped by the race"
        );
    }

    #[tokio::test]
    async fn run_waits_for_slow_cleanup_before_reporting_cancellation() {
        let token = CancellationToken::new();
        let ctx = OperationCtx::from_registered(11, token.clone());
        let finished = Arc::new(AtomicBool::new(false));

        tokio::spawn({
            let token = token.clone();
            async move {
                tokio::time::sleep(Duration::from_millis(10)).await;
                token.cancel();
            }
        });

        let flag = finished.clone();
        let result: Result<u8, ApiError> = ctx
            .run(
                "op",
                ctx.with_cleanup(
                    // Cleanup deliberately outlasts the cancel by enough that a
                    // `run` which only spawned it would return with the flag unset.
                    async move {
                        tokio::time::sleep(Duration::from_millis(50)).await;
                        flag.store(true, Ordering::SeqCst);
                    },
                    async {
                        tokio::time::sleep(Duration::from_secs(30)).await;
                        Ok(1)
                    },
                ),
            )
            .await;

        assert!(
            matches!(result, Err(ApiError::Cancelled { .. })),
            "expected Cancelled, got {result:?}"
        );
        // A cancelled operation that has returned implies its cleanup already
        // finished — this is what makes cancel tests deterministic.
        assert!(
            finished.load(Ordering::SeqCst),
            "run must await registered cleanup, not just spawn it"
        );
    }

    #[tokio::test]
    async fn an_operation_cancelled_before_entry_registers_no_cleanup() {
        let token = CancellationToken::new();
        let ctx = OperationCtx::from_registered(13, token.clone());
        let (ran, cleanup) = cleanup_probe();
        token.cancel();

        let result: Result<u8, ApiError> = ctx
            .run("op", ctx.with_cleanup(cleanup, async { Ok(1) }))
            .await;

        assert!(
            matches!(result, Err(ApiError::Cancelled { .. })),
            "expected Cancelled, got {result:?}"
        );
        // The body was never polled, so no cleanup was armed. This is the
        // property that keeps us from aborting a query that was never sent.
        assert!(
            !ran.load(Ordering::SeqCst),
            "work that never started must not be cleaned up"
        );
    }

    /// The success-with-cancel race: the guarded work finishes and a cancel lands
    /// before the operation reports. Completion must win — the query returned, so
    /// aborting it would target work that is already done.
    ///
    /// This is the case a token-based suppression check gets wrong: at guard-drop
    /// the token *is* cancelled, so it would leave the cleanup armed and POST a
    /// pointless abort. Reaching the disarm is what decides it instead.
    #[tokio::test]
    async fn cleanup_is_suppressed_when_work_completes_and_a_cancel_races_in() {
        let token = CancellationToken::new();
        let ctx = OperationCtx::from_registered(12, token.clone());
        let (ran, cleanup) = cleanup_probe();

        let result: Result<u8, ApiError> = ctx
            .run(
                "op",
                ctx.with_cleanup(cleanup, async {
                    // The work is already complete when the cancel arrives.
                    token.cancel();
                    Ok(7)
                }),
            )
            .await;

        assert!(
            matches!(result, Ok(7)),
            "completion must win the race, got {result:?}"
        );
        assert!(
            !ran.load(Ordering::SeqCst),
            "a query that completed must not be aborted by a racing cancel"
        );
    }

    #[tokio::test]
    async fn with_cleanup_opt_without_a_ctx_runs_work_and_registers_nothing() {
        let (ran, cleanup) = cleanup_probe();

        let out = with_cleanup_opt(None, cleanup, async { 7u8 }).await;

        assert_eq!(out, 7, "work must still run without a ctx");
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(!ran.load(Ordering::SeqCst));
    }
}
