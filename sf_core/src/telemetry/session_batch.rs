//! Shared per-session batching mechanism for both telemetry lanes.
//!
//! Two lanes feed `/telemetry/send`: the OTel span lane
//! ([`super::snowflake_processor`], entries are `SpanData`) and the raw-log lane
//! ([`super::log_batch`], entries are caller JSON that can't be represented as
//! scalar span attributes). The entry types and the code that turns a batch into
//! an HTTP send genuinely differ — but the bookkeeping is identical: a
//! per-session `Vec`, a flush threshold, fire-and-forget spawning on overflow,
//! and a time-bounded flush on connection release. That machinery lives here once.

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::utils::sync::MutexRecoverExt;

/// Bounded wait for the awaited flush path (connection release), shared by both
/// lanes so a slow `/telemetry/send` cannot stall `connection_close`. Sized to
/// accommodate p99 latency in degraded regions (observed 3-4s).
pub(crate) const FLUSH_TIMEOUT: Duration = Duration::from_secs(5);

/// Per-session buffer with a flush threshold. Registry- and egress-agnostic:
/// it owns only the bookkeeping, leaving each lane to decide how a drained batch
/// becomes a send. Clones share one `Arc<Mutex<..>>`, so a processor and its
/// flush handle observe the same buffers.
#[derive(Debug)]
pub(crate) struct SessionBuffer<T> {
    buffers: Arc<Mutex<HashMap<i64, Vec<T>>>>,
    threshold: usize,
}

// Manual `Clone`: only the `Arc` is cloned, so no `T: Clone` bound is needed
// (a `#[derive(Clone)]` would wrongly require it).
impl<T> Clone for SessionBuffer<T> {
    fn clone(&self) -> Self {
        Self {
            buffers: Arc::clone(&self.buffers),
            threshold: self.threshold,
        }
    }
}

impl<T> SessionBuffer<T> {
    pub(crate) fn new(threshold: usize) -> Self {
        Self {
            buffers: Arc::new(Mutex::new(HashMap::new())),
            threshold,
        }
    }

    /// Append `entry` to `session_id`'s buffer. When the buffer reaches the
    /// threshold, drain it and return the batch for the caller to send;
    /// otherwise return `None`. A single lock covers the push and the drain.
    pub(crate) fn push(&self, session_id: i64, entry: T) -> Option<Vec<T>> {
        let mut bufs = self.buffers.lock_recover();
        let buf = bufs.entry(session_id).or_default();
        buf.push(entry);
        // `>=` (not `==`) so an over-full buffer still drains rather than growing
        // unbounded. `mem::take` swaps the batch out and leaves an empty Vec.
        if buf.len() >= self.threshold {
            Some(std::mem::take(buf))
        } else {
            None
        }
    }

    /// Drain and return one session's buffer, removing the key.
    pub(crate) fn take(&self, session_id: i64) -> Vec<T> {
        let mut bufs = self.buffers.lock_recover();
        bufs.remove(&session_id).unwrap_or_default()
    }

    /// Drain every session's buffer. Used by the span lane's `force_flush`.
    pub(crate) fn drain_all(&self) -> Vec<(i64, Vec<T>)> {
        let mut bufs = self.buffers.lock_recover();
        bufs.drain().collect()
    }

    #[cfg(test)]
    pub(crate) fn len(&self, session_id: i64) -> usize {
        self.buffers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&session_id)
            .map(|v| v.len())
            .unwrap_or(0)
    }
}

/// Fire-and-forget a send future on the tokio runtime; no-op outside one (the
/// batch is dropped). Used on threshold overflow, where we must not block the
/// caller (span-end hot path / `add_log`). Telemetry is best-effort.
pub(crate) fn spawn_best_effort<F>(fut: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.spawn(fut);
    }
}

/// Await a send future bounded by [`FLUSH_TIMEOUT`]. On timeout the in-flight
/// send is cancelled (its already-drained batch is dropped) so a hung endpoint
/// cannot stall `connection_close`. The future is expected to swallow its own
/// send errors and yield `()`.
pub(crate) async fn flush_bounded<F>(session_id: i64, fut: F)
where
    F: Future<Output = ()>,
{
    if tokio::time::timeout(FLUSH_TIMEOUT, fut).await.is_err() {
        tracing::debug!(session_id, "telemetry flush timed out; continuing");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_below_threshold_returns_none_and_buffers() {
        let buf: SessionBuffer<i32> = SessionBuffer::new(3);
        assert!(buf.push(1, 10).is_none());
        assert!(buf.push(1, 11).is_none());
        assert_eq!(buf.len(1), 2);
    }

    #[test]
    fn push_at_threshold_drains_and_returns_batch() {
        let buf: SessionBuffer<i32> = SessionBuffer::new(3);
        assert!(buf.push(1, 10).is_none());
        assert!(buf.push(1, 11).is_none());
        let batch = buf.push(1, 12).expect("threshold reached returns batch");
        assert_eq!(batch, vec![10, 11, 12]);
        assert_eq!(buf.len(1), 0, "buffer drained after threshold");
    }

    #[test]
    fn take_removes_only_that_session() {
        let buf: SessionBuffer<i32> = SessionBuffer::new(100);
        buf.push(1, 10);
        buf.push(2, 20);
        assert_eq!(buf.take(1), vec![10]);
        assert_eq!(buf.len(1), 0);
        assert_eq!(buf.len(2), 1, "other session untouched");
    }

    #[test]
    fn take_absent_session_is_empty() {
        let buf: SessionBuffer<i32> = SessionBuffer::new(100);
        assert!(buf.take(999).is_empty());
    }

    #[test]
    fn drain_all_empties_every_session() {
        let buf: SessionBuffer<i32> = SessionBuffer::new(100);
        buf.push(1, 10);
        buf.push(2, 20);
        let mut drained = buf.drain_all();
        drained.sort_by_key(|(sid, _)| *sid);
        assert_eq!(drained, vec![(1, vec![10]), (2, vec![20])]);
        assert_eq!(buf.len(1), 0);
        assert_eq!(buf.len(2), 0);
    }

    #[tokio::test]
    async fn flush_bounded_returns_promptly_on_fast_future() {
        // Structural: a future that completes well within FLUSH_TIMEOUT returns
        // without tripping the timeout branch.
        flush_bounded(1, async {}).await;
    }
}
