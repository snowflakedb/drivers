//! Pending operations queue for connection state machine.
//!
//! When the connection is in a transient state (Connecting, Renewing),
//! incoming requests are queued and will be processed once the connection
//! transitions to Connected.

use std::time::{Duration, Instant};

use tokio::sync::oneshot;

use super::error::{QueueFullSnafu, Result, StateMachineError};

/// Default maximum number of pending operations.
pub const DEFAULT_MAX_PENDING_OPS: usize = 1000;

/// Default timeout for pending operations.
pub const DEFAULT_PENDING_OP_TIMEOUT: Duration = Duration::from_secs(60);

/// A pending operation waiting for the connection to become ready.
pub struct PendingOperation {
    /// Channel to signal when the operation can proceed.
    pub ready_tx: oneshot::Sender<Result<()>>,
    /// When this operation was queued.
    pub queued_at: Instant,
    /// Timeout for this operation.
    pub timeout: Duration,
    /// Description for debugging/logging.
    pub description: String,
}

impl PendingOperation {
    /// Creates a new pending operation with default timeout.
    pub fn new(description: impl Into<String>) -> (Self, oneshot::Receiver<Result<()>>) {
        Self::with_timeout(description, DEFAULT_PENDING_OP_TIMEOUT)
    }

    /// Creates a new pending operation with custom timeout.
    pub fn with_timeout(
        description: impl Into<String>,
        timeout: Duration,
    ) -> (Self, oneshot::Receiver<Result<()>>) {
        let (tx, rx) = oneshot::channel();
        let op = Self {
            ready_tx: tx,
            queued_at: Instant::now(),
            timeout,
            description: description.into(),
        };
        (op, rx)
    }

    /// Returns true if this operation has timed out.
    pub fn is_timed_out(&self) -> bool {
        self.queued_at.elapsed() > self.timeout
    }

    /// Returns the remaining time before timeout.
    pub fn remaining_time(&self) -> Duration {
        self.timeout.saturating_sub(self.queued_at.elapsed())
    }

    /// Signals that the operation can proceed (success).
    pub fn signal_ready(self) {
        let _ = self.ready_tx.send(Ok(()));
    }

    /// Signals that the operation failed with an error.
    pub fn signal_error(self, error: StateMachineError) {
        let _ = self.ready_tx.send(Err(error));
    }
}

/// Queue for pending operations.
///
/// Thread-safe queue that holds operations waiting for the connection
/// to transition to a ready state.
pub struct PendingOpsQueue {
    /// The pending operations.
    ops: Vec<PendingOperation>,
    /// Maximum queue size.
    max_size: usize,
}

impl Default for PendingOpsQueue {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_PENDING_OPS)
    }
}

impl PendingOpsQueue {
    /// Creates a new queue with the specified maximum size.
    pub fn new(max_size: usize) -> Self {
        Self {
            ops: Vec::with_capacity(max_size.min(100)), // Pre-allocate reasonably
            max_size,
        }
    }

    /// Returns the number of pending operations.
    pub fn len(&self) -> usize {
        self.ops.len()
    }

    /// Returns true if there are no pending operations.
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    /// Returns true if the queue is at capacity.
    pub fn is_full(&self) -> bool {
        self.ops.len() >= self.max_size
    }

    /// Adds a pending operation to the queue.
    ///
    /// # Errors
    /// Returns `QueueFull` if the queue is at capacity.
    pub fn enqueue(&mut self, op: PendingOperation) -> Result<()> {
        if self.is_full() {
            // Signal the operation that it was rejected
            op.signal_error(
                QueueFullSnafu {
                    max_size: self.max_size,
                }
                .build(),
            );
            return QueueFullSnafu {
                max_size: self.max_size,
            }
            .fail();
        }
        self.ops.push(op);
        Ok(())
    }

    /// Drains all pending operations, signaling each one as ready.
    ///
    /// This is called when the connection transitions to `Connected`.
    pub fn drain_ready(&mut self) {
        for op in self.ops.drain(..) {
            if op.is_timed_out() {
                tracing::warn!(
                    description = %op.description,
                    elapsed_ms = op.queued_at.elapsed().as_millis(),
                    "Pending operation timed out before becoming ready"
                );
                op.signal_error(super::error::OperationTimeoutSnafu.build());
            } else {
                tracing::debug!(
                    description = %op.description,
                    wait_ms = op.queued_at.elapsed().as_millis(),
                    "Signaling pending operation as ready"
                );
                op.signal_ready();
            }
        }
    }

    /// Drains all pending operations, signaling each one with an error.
    ///
    /// This is called when the connection transitions to `Disconnected`.
    pub fn drain_error(&mut self, error_factory: impl Fn() -> StateMachineError) {
        for op in self.ops.drain(..) {
            tracing::debug!(
                description = %op.description,
                wait_ms = op.queued_at.elapsed().as_millis(),
                "Signaling pending operation with error"
            );
            op.signal_error(error_factory());
        }
    }

    /// Removes and returns timed-out operations.
    ///
    /// Call this periodically to clean up stale operations.
    pub fn expire_timed_out(&mut self) -> usize {
        let before = self.ops.len();
        let ops = std::mem::take(&mut self.ops);

        for op in ops {
            if op.is_timed_out() {
                tracing::warn!(
                    description = %op.description,
                    elapsed_ms = op.queued_at.elapsed().as_millis(),
                    "Expiring timed out pending operation"
                );
                op.signal_error(super::error::OperationTimeoutSnafu.build());
            } else {
                self.ops.push(op);
            }
        }

        before - self.ops.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pending_op_ready() {
        let (op, rx) = PendingOperation::new("test");
        op.signal_ready();
        assert!(rx.await.unwrap().is_ok());
    }

    #[tokio::test]
    async fn test_pending_op_error() {
        let (op, rx) = PendingOperation::new("test");
        op.signal_error(super::super::error::OperationTimeoutSnafu.build());
        assert!(rx.await.unwrap().is_err());
    }

    #[tokio::test]
    async fn test_queue_drain_ready() {
        let mut queue = PendingOpsQueue::new(10);

        let (op1, rx1) = PendingOperation::new("op1");
        let (op2, rx2) = PendingOperation::new("op2");

        queue.enqueue(op1).unwrap();
        queue.enqueue(op2).unwrap();

        assert_eq!(queue.len(), 2);

        queue.drain_ready();

        assert!(queue.is_empty());
        assert!(rx1.await.unwrap().is_ok());
        assert!(rx2.await.unwrap().is_ok());
    }

    #[tokio::test]
    async fn test_queue_drain_error() {
        let mut queue = PendingOpsQueue::new(10);

        let (op, rx) = PendingOperation::new("test");
        queue.enqueue(op).unwrap();

        queue.drain_error(|| {
            super::super::error::ConnectionDisconnectedSnafu {
                reason: "test".to_string(),
            }
            .build()
        });

        assert!(queue.is_empty());
        assert!(rx.await.unwrap().is_err());
    }

    #[test]
    fn test_queue_full() {
        let mut queue = PendingOpsQueue::new(2);

        let (op1, _rx1) = PendingOperation::new("op1");
        let (op2, _rx2) = PendingOperation::new("op2");
        let (op3, _rx3) = PendingOperation::new("op3");

        assert!(queue.enqueue(op1).is_ok());
        assert!(queue.enqueue(op2).is_ok());
        assert!(queue.enqueue(op3).is_err()); // Queue full
    }

    #[tokio::test]
    async fn test_pending_op_timeout() {
        let (op, _rx) = PendingOperation::with_timeout("test", Duration::from_millis(1));

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(10)).await;

        assert!(op.is_timed_out());
    }
}
