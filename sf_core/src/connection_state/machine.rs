//! Connection state machine implementation.
//!
//! This module provides the core state machine that manages connection lifecycle,
//! validates state transitions, and coordinates pending operations.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{RwLock, broadcast};

use super::error::{
    ConnectionDisconnectedSnafu, ConnectionNotInitializedSnafu, InvalidTransitionSnafu,
    OperationCancelledSnafu, OperationTimeoutSnafu, Result,
};
use super::pending_ops::{DEFAULT_MAX_PENDING_OPS, PendingOperation, PendingOpsQueue};
use super::state::{ConnectionState, DisconnectReason, is_valid_transition};

/// Default timeout for waiting on state transitions.
pub const DEFAULT_WAIT_TIMEOUT: Duration = Duration::from_secs(30);

/// Capacity of the state change broadcast channel.
const STATE_BROADCAST_CAPACITY: usize = 16;

/// The connection state machine.
///
/// This struct manages:
/// - Current connection state
/// - State transition validation
/// - Pending operations queue
/// - State change notifications
///
/// # Thread Safety
///
/// The state machine is fully thread-safe and can be cloned. All clones
/// share the same underlying state via `Arc`.
#[derive(Clone)]
pub struct ConnectionStateMachine {
    /// Current state, protected by async RwLock for concurrent reads.
    state: Arc<RwLock<ConnectionState>>,

    /// Broadcast channel for state change notifications.
    state_tx: broadcast::Sender<ConnectionState>,

    /// Queue of operations waiting for connection to become ready.
    pending_ops: Arc<RwLock<PendingOpsQueue>>,
}

impl Default for ConnectionStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionStateMachine {
    /// Creates a new state machine in the `Pristine` state.
    pub fn new() -> Self {
        Self::with_queue_size(DEFAULT_MAX_PENDING_OPS)
    }

    /// Creates a new state machine with a custom pending ops queue size.
    pub fn with_queue_size(max_pending_ops: usize) -> Self {
        let (state_tx, _) = broadcast::channel(STATE_BROADCAST_CAPACITY);
        Self {
            state: Arc::new(RwLock::new(ConnectionState::Pristine)),
            state_tx,
            pending_ops: Arc::new(RwLock::new(PendingOpsQueue::new(max_pending_ops))),
        }
    }

    /// Returns the current connection state.
    pub async fn state(&self) -> ConnectionState {
        self.state.read().await.clone()
    }

    /// Returns the current state synchronously (blocking).
    ///
    /// Prefer `state()` for async contexts.
    pub fn state_blocking(&self) -> ConnectionState {
        self.state.blocking_read().clone()
    }

    /// Subscribe to state change notifications.
    ///
    /// Returns a receiver that will receive each new state after transitions.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let sm = ConnectionStateMachine::new();
    /// let mut rx = sm.subscribe();
    ///
    /// tokio::spawn(async move {
    ///     while let Ok(state) = rx.recv().await {
    ///         println!("State changed to: {:?}", state);
    ///     }
    /// });
    /// ```
    pub fn subscribe(&self) -> broadcast::Receiver<ConnectionState> {
        self.state_tx.subscribe()
    }

    /// Returns the number of pending operations in the queue.
    pub async fn pending_count(&self) -> usize {
        self.pending_ops.read().await.len()
    }

    /// Attempts to transition to a new state.
    ///
    /// This method validates that the transition is allowed before applying it.
    /// If the transition is valid:
    /// - The state is updated
    /// - State change is broadcast to subscribers
    /// - Pending operations are processed as appropriate
    ///
    /// # Errors
    ///
    /// Returns `InvalidTransition` if the transition is not allowed.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let sm = ConnectionStateMachine::new();
    ///
    /// // Valid: Pristine -> Connecting
    /// sm.transition(ConnectionState::Connecting).await?;
    ///
    /// // Invalid: Connecting -> Pristine (will error)
    /// sm.transition(ConnectionState::Pristine).await?; // Error!
    /// ```
    pub async fn transition(&self, new_state: ConnectionState) -> Result<()> {
        let mut state_guard = self.state.write().await;
        let current_state = state_guard.clone();

        // Validate the transition
        if !is_valid_transition(&current_state, &new_state) {
            return InvalidTransitionSnafu {
                from: current_state.to_string(),
                to: new_state.to_string(),
            }
            .fail();
        }

        // Skip if already in the target state
        if current_state == new_state {
            return Ok(());
        }

        tracing::info!(
            from = %current_state,
            to = %new_state,
            "Connection state transition"
        );

        // Update state
        *state_guard = new_state.clone();

        // Broadcast to subscribers (ignore if no receivers)
        let _ = self.state_tx.send(new_state.clone());

        // Release state lock before processing pending ops
        drop(state_guard);

        // Process pending operations based on new state
        match &new_state {
            ConnectionState::Connected => {
                // Signal all pending operations that they can proceed
                self.pending_ops.write().await.drain_ready();
            }
            ConnectionState::Disconnected { reason } => {
                // Signal all pending operations with error
                let reason_str = reason.description();
                self.pending_ops.write().await.drain_error(move || {
                    ConnectionDisconnectedSnafu {
                        reason: reason_str.clone(),
                    }
                    .build()
                });
            }
            _ => {
                // For other states, just expire any timed-out operations
                let expired = self.pending_ops.write().await.expire_timed_out();
                if expired > 0 {
                    tracing::debug!(expired, "Expired timed-out pending operations");
                }
            }
        }

        Ok(())
    }

    /// Waits until the connection is ready to accept requests.
    ///
    /// If the connection is already in `Connected` state, returns immediately.
    /// If in `Connecting` or `Renewing` state, the caller is queued and will
    /// be notified when the state transitions to `Connected`.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Maximum time to wait for the connection to become ready.
    ///
    /// # Errors
    ///
    /// - `ConnectionNotInitialized` - Connection is in `Pristine` state
    /// - `ConnectionDisconnected` - Connection is in `Disconnected` state
    /// - `OperationTimeout` - Timeout expired before connection became ready
    /// - `OperationCancelled` - The wait was cancelled
    ///
    /// # Example
    ///
    /// ```ignore
    /// let sm = ConnectionStateMachine::new();
    ///
    /// // In another task, connection is being established...
    /// sm.wait_ready(Duration::from_secs(30)).await?;
    ///
    /// // Now safe to execute queries
    /// ```
    pub async fn wait_ready(&self, timeout: Duration) -> Result<()> {
        let current_state = self.state().await;

        match current_state {
            // Already ready
            ConnectionState::Connected => Ok(()),

            // Not initialized
            ConnectionState::Pristine => ConnectionNotInitializedSnafu.fail(),

            // Cannot proceed
            ConnectionState::Disconnected { reason } => ConnectionDisconnectedSnafu {
                reason: reason.description(),
            }
            .fail(),

            // Need to wait
            ConnectionState::Connecting | ConnectionState::Renewing => {
                let (op, rx) = PendingOperation::with_timeout("wait_ready", timeout);

                // Enqueue the operation
                self.pending_ops.write().await.enqueue(op)?;

                // Wait for notification with timeout
                match tokio::time::timeout(timeout, rx).await {
                    Ok(Ok(result)) => result,
                    Ok(Err(_)) => {
                        // Sender dropped (shouldn't happen normally)
                        OperationCancelledSnafu.fail()
                    }
                    Err(_) => {
                        // Timeout
                        OperationTimeoutSnafu.fail()
                    }
                }
            }
        }
    }

    /// Waits until the connection is ready, using the default timeout.
    pub async fn wait_ready_default(&self) -> Result<()> {
        self.wait_ready(DEFAULT_WAIT_TIMEOUT).await
    }

    /// Checks if the connection can accept requests right now.
    ///
    /// This does not wait - it returns the current ability to accept requests.
    pub async fn can_accept_requests(&self) -> bool {
        self.state().await.can_accept_requests()
    }

    /// Checks if the connection is fully ready.
    pub async fn is_ready(&self) -> bool {
        self.state().await.is_ready()
    }

    /// Checks if a connect/reconnect operation can be performed.
    pub async fn can_connect(&self) -> bool {
        self.state().await.can_connect()
    }

    /// Performs state transition to `Connecting`.
    ///
    /// This is a convenience method that validates the transition.
    ///
    /// # Errors
    ///
    /// Returns error if not in `Pristine` state or a reconnectable `Disconnected` state.
    pub async fn start_connecting(&self) -> Result<()> {
        self.transition(ConnectionState::Connecting).await
    }

    /// Performs state transition to `Connected`.
    ///
    /// Call this after successful login or token refresh.
    ///
    /// # Errors
    ///
    /// Returns error if not in `Connecting` or `Renewing` state.
    pub async fn connection_established(&self) -> Result<()> {
        self.transition(ConnectionState::Connected).await
    }

    /// Performs state transition to `Renewing`.
    ///
    /// Call this when the session token has expired and refresh is starting.
    ///
    /// # Errors
    ///
    /// Returns error if not in `Connected` state.
    pub async fn start_renewing(&self) -> Result<()> {
        self.transition(ConnectionState::Renewing).await
    }

    /// Performs state transition to `Disconnected` with the given reason.
    ///
    /// This will signal all pending operations with an error.
    pub async fn disconnect(&self, reason: DisconnectReason) -> Result<()> {
        self.transition(ConnectionState::Disconnected { reason })
            .await
    }

    /// Closes the connection (user-initiated disconnect).
    ///
    /// The connection can be reconnected after this.
    pub async fn close(&self) -> Result<()> {
        self.disconnect(DisconnectReason::UserInitiated).await
    }
}

impl std::fmt::Debug for ConnectionStateMachine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionStateMachine")
            .field("state", &self.state_blocking())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::super::error::StateMachineError;
    use super::*;

    #[tokio::test]
    async fn test_initial_state_is_pristine() {
        let sm = ConnectionStateMachine::new();
        assert_eq!(sm.state().await, ConnectionState::Pristine);
    }

    #[tokio::test]
    async fn test_valid_transition_pristine_to_connecting() {
        let sm = ConnectionStateMachine::new();
        assert!(sm.start_connecting().await.is_ok());
        assert_eq!(sm.state().await, ConnectionState::Connecting);
    }

    #[tokio::test]
    async fn test_valid_transition_connecting_to_connected() {
        let sm = ConnectionStateMachine::new();
        sm.start_connecting().await.unwrap();
        assert!(sm.connection_established().await.is_ok());
        assert_eq!(sm.state().await, ConnectionState::Connected);
    }

    #[tokio::test]
    async fn test_invalid_transition_pristine_to_connected() {
        let sm = ConnectionStateMachine::new();
        let result = sm.connection_established().await;
        assert!(result.is_err());
        // State should remain unchanged
        assert_eq!(sm.state().await, ConnectionState::Pristine);
    }

    #[tokio::test]
    async fn test_reconnect_after_user_close() {
        let sm = ConnectionStateMachine::new();

        // Connect
        sm.start_connecting().await.unwrap();
        sm.connection_established().await.unwrap();

        // Close
        sm.close().await.unwrap();
        assert!(matches!(
            sm.state().await,
            ConnectionState::Disconnected {
                reason: DisconnectReason::UserInitiated
            }
        ));

        // Reconnect
        assert!(sm.start_connecting().await.is_ok());
        assert_eq!(sm.state().await, ConnectionState::Connecting);
    }

    #[tokio::test]
    async fn test_cannot_reconnect_from_internal_error() {
        let sm = ConnectionStateMachine::new();

        // Transition to internal error
        sm.start_connecting().await.unwrap();
        sm.disconnect(DisconnectReason::InternalError {
            message: "fatal".to_string(),
        })
        .await
        .unwrap();

        // Cannot reconnect
        let result = sm.start_connecting().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_state_change_broadcast() {
        let sm = ConnectionStateMachine::new();
        let mut rx = sm.subscribe();

        sm.start_connecting().await.unwrap();

        let received = rx.recv().await.unwrap();
        assert_eq!(received, ConnectionState::Connecting);
    }

    #[tokio::test]
    async fn test_wait_ready_when_already_connected() {
        let sm = ConnectionStateMachine::new();
        sm.start_connecting().await.unwrap();
        sm.connection_established().await.unwrap();

        // Should return immediately
        let result = sm.wait_ready(Duration::from_millis(10)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_wait_ready_when_pristine() {
        let sm = ConnectionStateMachine::new();

        let result = sm.wait_ready(Duration::from_millis(10)).await;
        assert!(matches!(
            result,
            Err(StateMachineError::ConnectionNotInitialized { .. })
        ));
    }

    #[tokio::test]
    async fn test_wait_ready_when_disconnected() {
        let sm = ConnectionStateMachine::new();
        sm.start_connecting().await.unwrap();
        sm.disconnect(DisconnectReason::UserInitiated)
            .await
            .unwrap();

        let result = sm.wait_ready(Duration::from_millis(10)).await;
        assert!(matches!(
            result,
            Err(StateMachineError::ConnectionDisconnected { .. })
        ));
    }

    #[tokio::test]
    async fn test_wait_ready_during_connecting() {
        let sm = ConnectionStateMachine::new();
        sm.start_connecting().await.unwrap();

        // Clone for the connecting task
        let sm_clone = sm.clone();

        // Spawn a task that waits for ready
        let wait_handle =
            tokio::spawn(async move { sm_clone.wait_ready(Duration::from_secs(5)).await });

        // Give the wait task time to enqueue
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Now establish connection
        sm.connection_established().await.unwrap();

        // Wait task should complete successfully
        let result = wait_handle.await.unwrap();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_pending_ops_drained_on_disconnect() {
        let sm = ConnectionStateMachine::new();
        sm.start_connecting().await.unwrap();

        let sm_clone = sm.clone();

        // Spawn a task that waits for ready
        let wait_handle =
            tokio::spawn(async move { sm_clone.wait_ready(Duration::from_secs(5)).await });

        // Give the wait task time to enqueue
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert_eq!(sm.pending_count().await, 1);

        // Disconnect instead of connecting
        sm.disconnect(DisconnectReason::UserInitiated)
            .await
            .unwrap();

        // Wait task should receive error
        let result = wait_handle.await.unwrap();
        assert!(matches!(
            result,
            Err(StateMachineError::ConnectionDisconnected { .. })
        ));

        // Queue should be empty
        assert_eq!(sm.pending_count().await, 0);
    }

    #[tokio::test]
    async fn test_renewing_state() {
        let sm = ConnectionStateMachine::new();

        // Connect
        sm.start_connecting().await.unwrap();
        sm.connection_established().await.unwrap();

        // Start renewing
        sm.start_renewing().await.unwrap();
        assert_eq!(sm.state().await, ConnectionState::Renewing);

        // Complete renewal
        sm.connection_established().await.unwrap();
        assert_eq!(sm.state().await, ConnectionState::Connected);
    }
}
