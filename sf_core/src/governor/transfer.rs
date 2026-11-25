//! Transfer-operation queue and RAII permit types.
//!
//! Fairness is work-conserving round-robin across *operations* (commands).
//! V1 does not centrally schedule files.

use crate::utils::sync::MutexRecoverExt;
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

pub(super) type OpId = u64;
type WaiterId = u64;

/// Per-command maximum concurrent logical cloud requests (`data.parallel`).
///
/// Production code should pass values already resolved by
/// `file_manager::multipart::resolve_command_parallel`; `from_resolved(0)`
/// maps to 1 as a defensive fallback for raw callers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandParallelism(NonZeroUsize);

impl CommandParallelism {
    pub fn from_resolved(value: usize) -> Self {
        Self(NonZeroUsize::new(value).unwrap_or(NonZeroUsize::MIN))
    }

    pub fn get(self) -> usize {
        self.0.get()
    }
}

/// Kind of logical cloud request currently exercised by wired leaves or tests.
/// Crate-scoped; add variants with the integrations that use them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RequestKind {
    CompleteMultipart,
    RangedGet,
    UploadPart,
    WholeObjectGet,
    WholeObjectPut,
    PutBlock,
    PutBlockList,
}

/// Why [`TransferOp::acquire`] failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcquireError {
    /// The [`TransferOp`] was dropped, or the waiter was otherwise
    /// removed before a permit was granted.
    OperationDropped,
}

impl fmt::Display for AcquireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OperationDropped => f.write_str("transfer operation dropped"),
        }
    }
}

impl std::error::Error for AcquireError {}

/// RAII handle for one registered command. Cloning shares the registration;
/// the last clone's drop cancels remaining queued waiters.
#[derive(Clone)]
#[must_use = "dropping TransferOp unregisters the command and cancels queued waiters"]
pub struct TransferOp {
    inner: Arc<OpRegistration>,
}

struct OpRegistration {
    state: Arc<Inner>,
    op_id: OpId,
    command_parallel: usize,
    #[cfg(test)]
    local_work: std::sync::atomic::AtomicUsize,
}

impl Drop for OpRegistration {
    fn drop(&mut self) {
        Inner::unregister(&self.state, self.op_id);
    }
}

impl fmt::Debug for TransferOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TransferOp")
            .field("op_id", &self.inner.op_id)
            .finish()
    }
}

impl TransferOp {
    pub(super) fn register(state: Arc<Inner>, command_parallel: CommandParallelism) -> Self {
        let limit = command_parallel.get();
        let op_id = Inner::register(&state, limit);
        Self {
            inner: Arc::new(OpRegistration {
                state,
                op_id,
                command_parallel: limit,
                #[cfg(test)]
                local_work: std::sync::atomic::AtomicUsize::new(0),
            }),
        }
    }

    /// Isolated registration for tests.
    ///
    /// Rust integration tests compile as an external crate and cannot reach
    /// crate-private constructors. Available only under `cfg(test)` or the
    /// `test-utils` feature (which those tests already enable). Builds a
    /// private [`crate::governor::Governor`] (not
    /// [`crate::governor::process_governor`]) and registers against it. The
    /// governor has unlimited process capacity; 32 is a wide command
    /// `data.parallel`, not a process cap. The operation is still admitted.
    #[cfg(any(test, feature = "test-utils"))]
    pub fn isolated_for_tests() -> Self {
        Self::isolated_for_tests_with_parallel(32)
    }

    /// Isolated registration with an explicit command width.
    ///
    /// Use this when a test's file fan-out must match a chosen `data.parallel`
    /// (for example cancellation tests that also resolve that width into
    /// multipart params). [`Self::isolated_for_tests`] keeps the historical
    /// wide default of 32.
    #[cfg(any(test, feature = "test-utils"))]
    pub fn isolated_for_tests_with_parallel(parallel: usize) -> Self {
        crate::governor::Governor::new().register_command(parallel)
    }

    pub fn file_fanout(&self) -> usize {
        self.inner.command_parallel
    }

    #[cfg(test)]
    pub(super) fn op_id(&self) -> OpId {
        self.inner.op_id
    }

    #[cfg(test)]
    pub(crate) fn command_in_flight(&self) -> usize {
        self.inner.state.command_in_flight(self.inner.op_id)
    }

    /// Test probe: how many times a caller marked local transfer work as
    /// started (preprocess, dest open, or a part/range read) after admission.
    #[cfg(test)]
    pub(crate) fn local_work_started(&self) -> usize {
        self.inner
            .local_work
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Record that governed local I/O is about to start under a held permit.
    /// Compiles to a no-op outside tests so production call sites stay a
    /// single method invocation rather than a `#[cfg(test)]` block.
    #[inline]
    pub(crate) fn on_local_work(&self) {
        let _ = self;
        #[cfg(test)]
        self.inner
            .local_work
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    /// Queue this command and wait for a slot.
    ///
    /// The command ceiling is the registered `data.parallel` (`op.limit`):
    /// extra acquires wait in this command's governor waiter queue. Call
    /// structure bounds that queue (file `.buffered(file_fanout())`, one
    /// pending multipart-producer acquire per file, ranges ≤ per-file
    /// concurrency).
    ///
    /// The returned future does **not** keep this handle alive: dropping the
    /// last [`TransferOp`] clone unregisters the command and fails
    /// queued acquires with [`AcquireError::OperationDropped`]. Dropping the
    /// future removes only that waiter (`QueueGuard`). There is no separate
    /// cancellation token.
    ///
    /// Granted [`RequestPermit`]s are `Send + 'static` so they can move into
    /// a streaming-GET producer task and cover response-body drain.
    pub(crate) fn acquire(
        &self,
        kind: RequestKind,
    ) -> impl std::future::Future<Output = Result<RequestPermit, AcquireError>> + Send + 'static
    {
        let state = Arc::clone(&self.inner.state);
        let op_id = self.inner.op_id;
        async move { queue_and_wait(state, op_id, kind).await }
    }
}

async fn queue_and_wait(
    state: Arc<Inner>,
    op_id: OpId,
    kind: RequestKind,
) -> Result<RequestPermit, AcquireError> {
    let (tx, rx) = oneshot::channel();
    let waiter_id = Inner::enqueue(&state, op_id, kind, tx)?;
    let _guard = QueueGuard {
        state: Arc::clone(&state),
        op_id,
        waiter_id,
    };
    match rx.await {
        Ok(permit) => Ok(permit),
        Err(_) => Err(AcquireError::OperationDropped),
    }
}

/// RAII permit for one active-or-retrying logical cloud request. Release
/// happens on drop, including when the owning retry/producer task ends or is
/// aborted.
#[must_use = "dropping RequestPermit releases process-wide transfer capacity"]
pub(crate) struct RequestPermit {
    state: Arc<Inner>,
    op_id: OpId,
}

impl fmt::Debug for RequestPermit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RequestPermit").finish_non_exhaustive()
    }
}

impl Drop for RequestPermit {
    fn drop(&mut self) {
        Inner::release(&self.state, self.op_id);
    }
}

struct QueueGuard {
    state: Arc<Inner>,
    op_id: OpId,
    waiter_id: WaiterId,
}

impl Drop for QueueGuard {
    fn drop(&mut self) {
        Inner::remove_waiter(&self.state, self.op_id, self.waiter_id);
    }
}

struct Waiter {
    id: WaiterId,
    kind: RequestKind,
    tx: oneshot::Sender<RequestPermit>,
}

struct OpState {
    queued: VecDeque<Waiter>,
    in_flight: usize,
    limit: usize,
}

struct State {
    /// Ordinary process cap. `None` is unlimited (production default).
    max_in_flight: Option<NonZeroUsize>,
    in_flight: usize,
    ops: HashMap<OpId, OpState>,
    /// Live operations in registration order; round-robin walks this list.
    op_order: Vec<OpId>,
    rr_index: usize,
    next_op_id: OpId,
    next_waiter_id: WaiterId,
}

impl State {
    fn process_has_room(&self) -> bool {
        match self.max_in_flight {
            None => true,
            Some(cap) => self.in_flight < cap.get(),
        }
    }
}

struct PendingGrant {
    tx: oneshot::Sender<RequestPermit>,
    permit: RequestPermit,
}

pub(super) struct Inner {
    state: Mutex<State>,
}

impl fmt::Debug for Inner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = self.state.lock_recover();
        let queued: usize = state.ops.values().map(|o| o.queued.len()).sum();
        f.debug_struct("Inner")
            .field("max_in_flight", &state.max_in_flight)
            .field("in_flight", &state.in_flight)
            .field("queued", &queued)
            .finish()
    }
}

impl Inner {
    pub(super) fn new(max_in_flight: Option<NonZeroUsize>) -> Self {
        Self::with_next_ids(max_in_flight, 1, 1)
    }

    pub(super) fn with_next_ids(
        max_in_flight: Option<NonZeroUsize>,
        next_op_id: OpId,
        next_waiter_id: WaiterId,
    ) -> Self {
        Self {
            state: Mutex::new(State {
                max_in_flight,
                in_flight: 0,
                ops: HashMap::new(),
                op_order: Vec::new(),
                rr_index: 0,
                next_op_id,
                next_waiter_id,
            }),
        }
    }

    pub(super) fn capacity(&self) -> Option<usize> {
        self.state
            .lock_recover()
            .max_in_flight
            .map(NonZeroUsize::get)
    }

    pub(super) fn in_flight(&self) -> usize {
        self.state.lock_recover().in_flight
    }

    pub(super) fn queued(&self) -> usize {
        self.state
            .lock_recover()
            .ops
            .values()
            .map(|op| op.queued.len())
            .sum()
    }

    fn next_id(current: u64) -> u64 {
        current.wrapping_add(1)
    }

    fn register(this: &Arc<Self>, limit: usize) -> OpId {
        let (op_id, pending) = {
            let mut state = this.state.lock_recover();
            let op_id = state.next_op_id;
            state.next_op_id = Self::next_id(state.next_op_id);
            state.ops.insert(
                op_id,
                OpState {
                    queued: VecDeque::new(),
                    in_flight: 0,
                    limit,
                },
            );
            state.op_order.push(op_id);
            (op_id, try_grant(this, &mut state))
        };
        deliver(pending);
        op_id
    }

    #[cfg(test)]
    fn command_in_flight(&self, op_id: OpId) -> usize {
        self.state
            .lock_recover()
            .ops
            .get(&op_id)
            .map(|op| op.in_flight)
            .unwrap_or(0)
    }

    fn unregister(this: &Arc<Self>, op_id: OpId) {
        let pending = {
            let mut state = this.state.lock_recover();
            if state.ops.remove(&op_id).is_none() {
                return;
            }
            if let Some(removed) = state.op_order.iter().position(|&id| id == op_id) {
                state.op_order.remove(removed);
                if state.op_order.is_empty() {
                    state.rr_index = 0;
                } else if removed < state.rr_index {
                    state.rr_index -= 1;
                } else if state.rr_index >= state.op_order.len() {
                    state.rr_index = 0;
                }
            }
            try_grant(this, &mut state)
        };
        deliver(pending);
    }

    pub(super) fn enqueue(
        this: &Arc<Self>,
        op_id: OpId,
        kind: RequestKind,
        tx: oneshot::Sender<RequestPermit>,
    ) -> Result<WaiterId, AcquireError> {
        let (waiter_id, pending) = {
            let mut state = this.state.lock_recover();
            let waiter_id = state.next_waiter_id;
            state.next_waiter_id = Self::next_id(state.next_waiter_id);
            {
                let op = state
                    .ops
                    .get_mut(&op_id)
                    .ok_or(AcquireError::OperationDropped)?;
                op.queued.push_back(Waiter {
                    id: waiter_id,
                    kind,
                    tx,
                });
            }
            (waiter_id, try_grant(this, &mut state))
        };
        deliver(pending);
        Ok(waiter_id)
    }

    fn remove_waiter(this: &Arc<Self>, op_id: OpId, waiter_id: WaiterId) {
        let mut state = this.state.lock_recover();
        if let Some(op) = state.ops.get_mut(&op_id) {
            op.queued.retain(|w| w.id != waiter_id);
        }
    }

    fn release(this: &Arc<Self>, op_id: OpId) {
        let pending = {
            let mut state = this.state.lock_recover();
            debug_assert!(
                state.in_flight > 0,
                "RequestPermit drop underflowed in_flight"
            );
            state.in_flight = state.in_flight.saturating_sub(1);
            if let Some(op) = state.ops.get_mut(&op_id) {
                debug_assert!(
                    op.in_flight > 0,
                    "RequestPermit drop underflowed operation in_flight"
                );
                op.in_flight = op.in_flight.saturating_sub(1);
            }
            try_grant(this, &mut state)
        };
        deliver(pending);
    }
}

fn try_grant(inner: &Arc<Inner>, state: &mut State) -> Vec<PendingGrant> {
    let mut pending = Vec::new();
    while state.process_has_room() {
        let Some((op_idx, op_id, waiter)) = pop_next_waiter(state) else {
            break;
        };
        tracing::debug!(waiter_id = waiter.id, kind = ?waiter.kind, "granted transfer permit");
        if !state.op_order.is_empty() {
            state.rr_index = (op_idx + 1) % state.op_order.len();
        }
        state.in_flight = state.in_flight.checked_add(1).expect("in_flight overflow");
        if let Some(op) = state.ops.get_mut(&op_id) {
            op.in_flight = op
                .in_flight
                .checked_add(1)
                .expect("operation in_flight overflow");
        }
        pending.push(PendingGrant {
            tx: waiter.tx,
            permit: RequestPermit {
                state: Arc::clone(inner),
                op_id,
            },
        });
    }
    pending
}

fn deliver(pending: Vec<PendingGrant>) {
    for grant in pending {
        let _ = grant.tx.send(grant.permit);
    }
}

fn pop_next_waiter(state: &mut State) -> Option<(usize, OpId, Waiter)> {
    let n = state.op_order.len();
    if n == 0 {
        return None;
    }
    let start = state.rr_index % n;
    for step in 0..n {
        let idx = (start + step) % n;
        let op_id = state.op_order[idx];
        if let Some(op) = state.ops.get_mut(&op_id) {
            if op.in_flight >= op.limit {
                continue;
            }
            if let Some(waiter) = op.queued.pop_front() {
                return Some((idx, op_id, waiter));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governor::Governor;
    use std::time::Duration;

    fn test_governor(max_in_flight: usize) -> Governor {
        Governor::try_with_capacity(max_in_flight).expect("test capacity must be nonzero")
    }

    fn test_parallel(max_in_flight: usize) -> CommandParallelism {
        CommandParallelism::from_resolved(max_in_flight)
    }

    fn register(governor: &Governor, command_parallel: CommandParallelism) -> TransferOp {
        governor.register_transfer(command_parallel)
    }

    async fn wait_until(mut cond: impl FnMut() -> bool) {
        let start = tokio::time::Instant::now();
        while !cond() {
            assert!(
                start.elapsed() < Duration::from_secs(2),
                "timed out waiting for governor condition"
            );
            tokio::task::yield_now().await;
        }
    }

    #[test]
    fn permit_is_send_static() {
        fn assert_send_static<T: Send + 'static>() {}
        assert_send_static::<RequestPermit>();
        assert_send_static::<TransferOp>();
    }

    #[tokio::test]
    async fn new_governor_admits_beyond_thirty_two_when_command_allows() {
        let governor = Governor::new();
        assert_eq!(governor.capacity(), None);
        let op = register(&governor, test_parallel(40));
        let mut permits = Vec::new();
        for _ in 0..40 {
            permits.push(op.acquire(RequestKind::WholeObjectPut).await.unwrap());
        }
        assert_eq!(governor.in_flight(), 40);
        assert_eq!(op.command_in_flight(), 40);
    }

    #[tokio::test]
    async fn new_governor_lets_two_commands_reach_their_limits_simultaneously() {
        let governor = Governor::new();
        let op_a = register(&governor, test_parallel(3));
        let op_b = register(&governor, test_parallel(5));
        let mut permits = Vec::new();

        for _ in 0..3 {
            permits.push(op_a.acquire(RequestKind::WholeObjectPut).await.unwrap());
        }
        for _ in 0..5 {
            permits.push(op_b.acquire(RequestKind::WholeObjectPut).await.unwrap());
        }

        assert_eq!(op_a.command_in_flight(), 3);
        assert_eq!(op_b.command_in_flight(), 5);
        assert_eq!(governor.in_flight(), 8);
        assert_eq!(governor.queued(), 0);
    }

    #[tokio::test]
    async fn process_cap_binds_below_command_limit() {
        let governor = test_governor(2);
        let op = register(&governor, test_parallel(8));
        let p1 = op.acquire(RequestKind::WholeObjectPut).await.unwrap();
        let p2 = op.acquire(RequestKind::WholeObjectPut).await.unwrap();
        assert_eq!(governor.in_flight(), 2);

        let third = {
            let op = op.clone();
            tokio::spawn(async move { op.acquire(RequestKind::WholeObjectPut).await })
        };
        wait_until(|| governor.queued() == 1).await;
        assert!(!third.is_finished());
        assert_eq!(governor.in_flight(), 2);

        drop(p1);
        let p3 = third
            .await
            .expect("join")
            .expect("third granted after process-cap release");
        assert_eq!(governor.in_flight(), 2);
        drop(p2);
        drop(p3);
        assert_eq!(governor.in_flight(), 0);
        assert_eq!(governor.queued(), 0);
    }

    #[tokio::test]
    async fn operation_round_robin() {
        let governor = test_governor(1);
        let op_a = register(&governor, test_parallel(32));
        let op_b = register(&governor, test_parallel(32));

        let permit_a = op_a.acquire(RequestKind::WholeObjectPut).await.unwrap();
        assert_eq!(governor.in_flight(), 1);

        let b_join = {
            let op_b = op_b.clone();
            tokio::spawn(async move { op_b.acquire(RequestKind::WholeObjectPut).await })
        };
        wait_until(|| governor.queued() == 1).await;

        let a_join = {
            let op_a = op_a.clone();
            tokio::spawn(async move { op_a.acquire(RequestKind::WholeObjectPut).await })
        };
        wait_until(|| governor.queued() == 2).await;

        drop(permit_a);
        let b_permit = b_join.await.expect("join").expect("b granted first");
        assert_eq!(governor.in_flight(), 1);
        assert_eq!(governor.queued(), 1);

        drop(b_permit);
        let _a2 = a_join.await.expect("join").expect("a granted second");
        assert_eq!(governor.in_flight(), 1);
        assert_eq!(governor.queued(), 0);
    }

    #[tokio::test]
    async fn unregister_before_cursor_preserves_next_operation() {
        // A granted, cursor at B. Unregister A (index 0 < cursor). B then C
        // are queued; dropping A's permit must grant B, not skip to C.
        let governor = test_governor(1);
        let op_a = register(&governor, test_parallel(32));
        let op_b = register(&governor, test_parallel(32));
        let op_c = register(&governor, test_parallel(32));

        let permit_a = op_a.acquire(RequestKind::WholeObjectPut).await.unwrap();
        let b_join = {
            let op_b = op_b.clone();
            tokio::spawn(async move { op_b.acquire(RequestKind::WholeObjectPut).await })
        };
        wait_until(|| governor.queued() == 1).await;
        let c_join = {
            let op_c = op_c.clone();
            tokio::spawn(async move { op_c.acquire(RequestKind::WholeObjectPut).await })
        };
        wait_until(|| governor.queued() == 2).await;

        drop(op_a);
        drop(permit_a);
        let b_permit = b_join.await.expect("join").expect("b next after A churn");
        assert_eq!(governor.in_flight(), 1);
        assert_eq!(governor.queued(), 1);
        drop(b_permit);
        let _c = c_join.await.expect("join").expect("c after b");
    }

    #[tokio::test]
    async fn work_conservation_fills_from_one_operation() {
        let governor = test_governor(2);
        let op = register(&governor, test_parallel(2));

        let p1 = op.acquire(RequestKind::WholeObjectPut).await.unwrap();
        let p2 = op.acquire(RequestKind::WholeObjectPut).await.unwrap();
        assert_eq!(
            governor.in_flight(),
            2,
            "a single operation must be able to consume the whole cap"
        );
        assert_eq!(governor.queued(), 0);

        let third = {
            let op = op.clone();
            tokio::spawn(async move { op.acquire(RequestKind::WholeObjectPut).await })
        };
        wait_until(|| governor.queued() == 1).await;
        assert!(!third.is_finished());
        drop(p1);
        drop(p2);
        let _ = third.await.expect("join").expect("third granted");
    }

    #[tokio::test]
    async fn queued_acquire_drop_releases_waiter() {
        let governor = test_governor(1);
        let op = register(&governor, test_parallel(2));
        let hold = op.acquire(RequestKind::WholeObjectPut).await.unwrap();

        let queued = {
            let op = op.clone();
            tokio::spawn(async move { op.acquire(RequestKind::WholeObjectPut).await })
        };
        wait_until(|| governor.queued() == 1).await;
        queued.abort();
        let _ = queued.await;
        wait_until(|| governor.queued() == 0).await;

        drop(hold);
        assert_eq!(governor.in_flight(), 0);
        let _next = op.acquire(RequestKind::WholeObjectPut).await.unwrap();
        assert_eq!(governor.in_flight(), 1);
    }

    #[tokio::test]
    async fn grant_drop_race_does_not_leak_or_hang() {
        let governor = test_governor(1);
        let op = register(&governor, test_parallel(2));
        for _ in 0..64 {
            let hold = op.acquire(RequestKind::WholeObjectPut).await.unwrap();
            let queued = {
                let op = op.clone();
                tokio::spawn(async move { op.acquire(RequestKind::WholeObjectPut).await })
            };
            wait_until(|| governor.queued() == 1).await;
            drop(hold);
            queued.abort();
            let _ = queued.await;
            wait_until(|| governor.in_flight() == 0 && governor.queued() == 0).await;
        }
    }

    #[tokio::test]
    async fn closed_receiver_during_grant_does_not_deadlock_or_leak() {
        let governor = test_governor(1);
        let op = register(&governor, test_parallel(2));
        governor.enqueue_closed_receiver(&op);
        assert_eq!(governor.in_flight(), 0);
        assert_eq!(governor.queued(), 0);
        let _ok = tokio::time::timeout(
            Duration::from_millis(200),
            op.acquire(RequestKind::WholeObjectPut),
        )
        .await
        .expect("acquire after closed grant must not hang")
        .expect("granted");
        assert_eq!(governor.in_flight(), 1);
    }

    #[tokio::test]
    async fn raii_release_on_permit_drop() {
        let governor = test_governor(1);
        let op = register(&governor, test_parallel(2));
        {
            let _permit = op.acquire(RequestKind::WholeObjectPut).await.unwrap();
            assert_eq!(governor.in_flight(), 1);
        }
        assert_eq!(governor.in_flight(), 0);
        let _again = op.acquire(RequestKind::WholeObjectPut).await.unwrap();
        assert_eq!(governor.in_flight(), 1);
    }

    #[tokio::test]
    async fn operation_drop_cancels_queued_waiters() {
        let governor = test_governor(1);
        let op_a = register(&governor, test_parallel(32));
        let op_b = register(&governor, test_parallel(32));
        let _hold = op_a.acquire(RequestKind::WholeObjectPut).await.unwrap();

        let queued = tokio::spawn(op_b.acquire(RequestKind::WholeObjectPut));
        wait_until(|| governor.queued() == 1).await;
        drop(op_b);
        assert_eq!(
            queued.await.expect("join").map(|_| ()),
            Err(AcquireError::OperationDropped)
        );
        assert_eq!(governor.queued(), 0);
        assert_eq!(governor.in_flight(), 1);
    }

    #[tokio::test]
    async fn permit_can_move_into_producer_task() {
        let governor = test_governor(1);
        let op = register(&governor, test_parallel(2));
        let permit = op.acquire(RequestKind::WholeObjectPut).await.unwrap();
        assert_eq!(governor.in_flight(), 1);

        let drained = tokio::sync::oneshot::channel();
        let join = tokio::spawn(async move {
            let _permit = permit;
            tokio::task::yield_now().await;
            drained.0.send(()).expect("drain signal");
        });
        drained.1.await.expect("producer drained");
        join.await.expect("producer joined");
        assert_eq!(governor.in_flight(), 0);
    }

    #[tokio::test]
    async fn producer_task_abort_releases_permit() {
        let governor = test_governor(1);
        let op = register(&governor, test_parallel(2));
        let permit = op.acquire(RequestKind::WholeObjectPut).await.unwrap();
        assert_eq!(governor.in_flight(), 1);

        let join = tokio::spawn(async move {
            let _permit = permit;
            std::future::pending::<()>().await;
        });
        join.abort();
        let _ = join.await;
        wait_until(|| governor.in_flight() == 0).await;
    }

    #[tokio::test]
    async fn per_command_cap_holds_when_process_cap_is_higher() {
        let governor = test_governor(8);
        let op = register(&governor, test_parallel(2));
        let _p1 = op.acquire(RequestKind::WholeObjectPut).await.unwrap();
        let _p2 = op.acquire(RequestKind::WholeObjectPut).await.unwrap();
        assert_eq!(op.command_in_flight(), 2);

        let third = {
            let op = op.clone();
            tokio::spawn(async move { op.acquire(RequestKind::WholeObjectPut).await })
        };
        wait_until(|| governor.queued() == 1).await;
        assert!(!third.is_finished(), "third acquire should remain queued");
        assert_eq!(governor.queued(), 1);
        assert_eq!(governor.in_flight(), 2);
        third.abort();
        let _ = third.await;
    }

    #[tokio::test]
    async fn one_command_can_fill_process_cap_up_to_its_limit() {
        let governor = test_governor(8);
        let op = register(&governor, test_parallel(4));
        let mut permits = Vec::new();
        for _ in 0..4 {
            permits.push(op.acquire(RequestKind::WholeObjectPut).await.unwrap());
        }
        assert_eq!(governor.in_flight(), 4);
        assert_eq!(op.command_in_flight(), 4);

        let blocked = {
            let op = op.clone();
            tokio::spawn(async move { op.acquire(RequestKind::WholeObjectPut).await })
        };
        wait_until(|| governor.queued() == 1).await;
        assert!(!blocked.is_finished(), "extra acquire should remain queued");
        assert_eq!(governor.queued(), 1);
        blocked.abort();
        let _ = blocked.await;
    }

    #[tokio::test]
    async fn two_commands_with_different_limits_round_robin() {
        let governor = test_governor(2);
        let op_a = register(&governor, test_parallel(1));
        let op_b = register(&governor, test_parallel(2));

        let b1 = op_b.acquire(RequestKind::WholeObjectPut).await.unwrap();
        let b2 = op_b.acquire(RequestKind::WholeObjectPut).await.unwrap();
        assert_eq!(op_b.command_in_flight(), 2);
        assert_eq!(governor.in_flight(), 2);

        let a_blocked = {
            let op_a = op_a.clone();
            tokio::spawn(async move { op_a.acquire(RequestKind::WholeObjectPut).await })
        };
        wait_until(|| governor.queued() == 1).await;

        let b3_blocked = {
            let op_b = op_b.clone();
            tokio::spawn(async move { op_b.acquire(RequestKind::WholeObjectPut).await })
        };
        wait_until(|| governor.queued() == 2).await;
        assert!(
            !b3_blocked.is_finished(),
            "op_b's extra acquire should remain queued"
        );
        assert_eq!(
            governor.queued(),
            2,
            "op_b is at its command cap so extra b waiters sit in the governor queue with op_a"
        );

        drop(b1);
        let a1 = a_blocked
            .await
            .expect("join")
            .expect("round-robin grants op_a after op_b releases");
        assert_eq!(op_a.command_in_flight(), 1);
        assert_eq!(op_b.command_in_flight(), 1);
        assert_eq!(governor.in_flight(), 2);
        wait_until(|| governor.queued() == 1).await;
        assert!(
            !b3_blocked.is_finished(),
            "op_b's extra waiter must not overtake round-robin into op_a"
        );

        drop(b2);
        let b3 = b3_blocked
            .await
            .expect("join")
            .expect("op_b re-acquires after second release");
        assert_eq!(op_b.command_in_flight(), 1);
        assert_eq!(governor.in_flight(), 2);
        assert_eq!(governor.queued(), 0);

        let a2_blocked = {
            let op_a = op_a.clone();
            tokio::spawn(async move { op_a.acquire(RequestKind::WholeObjectPut).await })
        };
        wait_until(|| governor.queued() == 1).await;
        assert!(
            !a2_blocked.is_finished(),
            "op_a's extra acquire should remain queued"
        );
        assert_eq!(
            governor.queued(),
            1,
            "op_a is at its command cap so extra a waiters sit in the governor queue"
        );
        assert_eq!(op_a.command_in_flight(), 1, "op_a stays at its limit of 1");

        drop(a1);
        drop(b3);
        let a2 = a2_blocked
            .await
            .expect("join")
            .expect("op_a second acquire after releases");
        assert_eq!(op_a.command_in_flight(), 1);
        assert_eq!(governor.in_flight(), 1);

        drop(a2);
        assert_eq!(governor.in_flight(), 0);
        assert_eq!(op_a.command_in_flight(), 0);
        assert_eq!(op_b.command_in_flight(), 0);
    }

    #[test]
    fn command_parallel_equals_server_value_not_clamped() {
        let parallel = CommandParallelism::from_resolved(50);
        assert_eq!(parallel.get(), 50);
        let op = test_governor(64).register_transfer(parallel);
        assert_eq!(op.file_fanout(), 50);
    }

    #[test]
    fn file_fanout_falls_back_to_one_for_missing_parallel() {
        let op = test_governor(32).register_transfer(CommandParallelism::from_resolved(0));
        assert_eq!(op.file_fanout(), 1);
    }

    #[tokio::test]
    async fn extra_acquire_waits_in_the_governor_queue() {
        let governor = test_governor(8);
        let op = register(&governor, test_parallel(2));
        let _p1 = op.acquire(RequestKind::WholeObjectPut).await.unwrap();
        let _p2 = op.acquire(RequestKind::WholeObjectPut).await.unwrap();
        assert_eq!(op.command_in_flight(), 2);

        let extra = {
            let op = op.clone();
            tokio::spawn(async move { op.acquire(RequestKind::WholeObjectPut).await })
        };
        wait_until(|| governor.queued() == 1).await;
        assert!(!extra.is_finished(), "extra acquire should remain queued");
        assert_eq!(
            governor.queued(),
            1,
            "command-limit extras wait in the governor queue"
        );
        extra.abort();
        let _ = extra.await;
    }

    #[tokio::test]
    async fn dropping_transfer_op_unblocks_queued_acquire() {
        let governor = test_governor(8);
        let op = register(&governor, test_parallel(1));
        let _hold = op.acquire(RequestKind::WholeObjectPut).await.unwrap();
        // `acquire`'s future does not keep `op` alive; unregistering the
        // command must fail this waiter instead of hanging.
        let pending = op.acquire(RequestKind::WholeObjectPut);
        drop(op);
        assert_eq!(
            pending.await.map(|_| ()),
            Err(AcquireError::OperationDropped)
        );
    }

    #[tokio::test]
    async fn single_command_admits_full_command_parallel() {
        let governor = Governor::new();
        let op = register(&governor, test_parallel(16));
        let mut permits = Vec::new();
        for _ in 0..16 {
            permits.push(op.acquire(RequestKind::UploadPart).await.unwrap());
        }
        assert_eq!(op.command_in_flight(), 16);
        assert_eq!(governor.in_flight(), 16);
        assert_eq!(governor.queued(), 0);
    }

    #[tokio::test]
    async fn waiter_queue_depth_matches_callers_beyond_the_command_limit() {
        let governor = test_governor(8);
        let op = register(&governor, test_parallel(2));
        let _p1 = op.acquire(RequestKind::WholeObjectPut).await.unwrap();
        let _p2 = op.acquire(RequestKind::WholeObjectPut).await.unwrap();
        let extras: Vec<_> = (0..6)
            .map(|_| {
                let op = op.clone();
                tokio::spawn(async move { op.acquire(RequestKind::WholeObjectPut).await })
            })
            .collect();
        wait_until(|| governor.queued() == 6).await;
        assert_eq!(op.command_in_flight(), 2);
        for extra in extras {
            extra.abort();
            let _ = extra.await;
        }
    }
}
