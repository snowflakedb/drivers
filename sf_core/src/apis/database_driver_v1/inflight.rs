//! In-flight query identity shared between the execute and cross-thread cancel
//! paths of a [`Statement`](super::statement::Statement).
//!
//! The execute path publishes the running query's client-generated `requestId`
//! (and SQL text) into an [`InflightSlot`] before the query-request is sent;
//! `statement_cancel` reads that slot to abort the query by `requestId`. The
//! slot is an independently-lockable `std::sync::Mutex`, deliberately *not*
//! guarded by the statement `Mutex`, so the execute and cancel paths never
//! contend on the statement mutex across a network round-trip (avoids
//! self-deadlock).

use std::sync::Arc;

use crate::utils::sync::MutexRecoverExt;

/// Identity of the query currently in flight on a statement, captured at
/// submit time so a cross-thread `statement_cancel` can abort it by its
/// client-generated `requestId` (the server-assigned `queryId` isn't known
/// until the query-request returns).
#[derive(Debug, Clone)]
pub(crate) struct InflightQuery {
    /// The `requestId` the query-request was (or will be) sent with.
    pub request_id: String,
    /// The SQL text, echoed back in the abort-request body.
    pub sql_text: String,
    /// Set once a cancel has been initiated for this query so a second
    /// concurrent cancel is a no-op (at most one abort-request is emitted).
    pub cancelling: bool,
}

/// Independently-lockable in-flight slot shared between the execute and cancel
/// paths. It is deliberately NOT read under the main statement `Mutex`: the
/// execute path drops the statement lock before its network await, and
/// `statement_cancel` locks only this slot, so the two never contend on the
/// statement mutex across a network round-trip (avoids self-deadlock).
pub(crate) type InflightSlot = Arc<std::sync::Mutex<Option<InflightQuery>>>;

/// Lock the in-flight slot, tolerating a poisoned mutex (a panic in a prior
/// critical section must not wedge cancel/execute — the slot is best-effort
/// cancellation state, not correctness-critical). Recovery is logged at ERROR
/// with the call site via [`MutexRecoverExt::lock_recover`].
fn lock_inflight(slot: &InflightSlot) -> std::sync::MutexGuard<'_, Option<InflightQuery>> {
    slot.lock_recover()
}

/// Publish the in-flight query identity so a concurrent cancel can find it.
pub(crate) fn set_inflight(slot: &InflightSlot, query: InflightQuery) {
    *lock_inflight(slot) = Some(query);
}

/// Atomically claim the in-flight query for cancellation.
///
/// Returns `Some((request_id, sql_text))` and sets the `cancelling` flag when a
/// query is in flight and no cancel has been started yet; returns `None` (a
/// no-op) when the slot is empty or a cancel is already in flight. The flag is
/// set under the slot lock so two racing cancels emit at most one abort-request.
pub(crate) fn take_and_mark_cancelling(slot: &InflightSlot) -> Option<(String, String)> {
    let mut guard = lock_inflight(slot);
    match guard.as_mut() {
        None => None,
        Some(q) if q.cancelling => None,
        Some(q) => {
            q.cancelling = true;
            Some((q.request_id.clone(), q.sql_text.clone()))
        }
    }
}

/// RAII guard that clears the in-flight slot on drop, covering every exit from
/// `execute_query_internal`: normal completion, an early `?` error, and
/// future-drop when the local-token cancel drops the execute future mid-await.
pub(crate) struct InflightGuard(pub(crate) InflightSlot);

impl Drop for InflightGuard {
    fn drop(&mut self) {
        *lock_inflight(&self.0) = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inflight(request_id: &str, sql: &str, cancelling: bool) -> InflightQuery {
        InflightQuery {
            request_id: request_id.to_string(),
            sql_text: sql.to_string(),
            cancelling,
        }
    }

    #[test]
    fn take_and_mark_cancelling_no_ops_when_slot_empty() {
        let slot: InflightSlot = Arc::new(std::sync::Mutex::new(None));
        assert_eq!(take_and_mark_cancelling(&slot), None);
    }

    #[test]
    fn take_and_mark_cancelling_claims_running_query_and_sets_flag() {
        let slot: InflightSlot = Arc::new(std::sync::Mutex::new(Some(inflight(
            "req-1", "SELECT 1", false,
        ))));

        let claimed = take_and_mark_cancelling(&slot);
        assert_eq!(claimed, Some(("req-1".to_string(), "SELECT 1".to_string())));
        // The claim marks `cancelling` so a second cancel is a no-op.
        assert!(lock_inflight(&slot).as_ref().unwrap().cancelling);
    }

    #[test]
    fn take_and_mark_cancelling_is_idempotent_for_double_cancel() {
        let slot: InflightSlot = Arc::new(std::sync::Mutex::new(Some(inflight(
            "req-1", "SELECT 1", false,
        ))));

        assert!(take_and_mark_cancelling(&slot).is_some());
        // A racing second cancel must not re-claim (→ at most one abort POST).
        assert_eq!(take_and_mark_cancelling(&slot), None);
    }

    #[test]
    fn inflight_guard_clears_slot_on_drop() {
        let slot: InflightSlot = Arc::new(std::sync::Mutex::new(None));
        set_inflight(&slot, inflight("req-1", "SELECT 1", false));
        assert!(lock_inflight(&slot).is_some());

        {
            let _guard = InflightGuard(slot.clone());
            assert!(lock_inflight(&slot).is_some());
        }
        // Dropping the guard clears the slot (covers success / `?` / future-drop).
        assert!(lock_inflight(&slot).is_none());
    }
}
