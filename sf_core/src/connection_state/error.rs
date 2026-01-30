//! Error types for the connection state machine.

use snafu::{Location, Snafu};

/// Errors that can occur during state machine operations.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum StateMachineError {
    /// Attempted an invalid state transition.
    #[snafu(display("Invalid state transition from {from} to {to}"))]
    InvalidTransition {
        /// The current state
        from: String,
        /// The attempted target state
        to: String,
        #[snafu(implicit)]
        location: Location,
    },

    /// Operation timed out waiting for state to become ready.
    #[snafu(display("Operation timed out waiting for connection to become ready"))]
    OperationTimeout {
        #[snafu(implicit)]
        location: Location,
    },

    /// Operation was cancelled (sender dropped).
    #[snafu(display("Operation was cancelled"))]
    OperationCancelled {
        #[snafu(implicit)]
        location: Location,
    },

    /// Connection is not initialized (still in Pristine state).
    #[snafu(display("Connection is not initialized"))]
    ConnectionNotInitialized {
        #[snafu(implicit)]
        location: Location,
    },

    /// Connection is disconnected and cannot perform the operation.
    #[snafu(display("Connection is disconnected: {reason}"))]
    ConnectionDisconnected {
        /// The disconnect reason
        reason: String,
        #[snafu(implicit)]
        location: Location,
    },

    /// Cannot reconnect from the current disconnect reason.
    #[snafu(display("Cannot reconnect: {reason}"))]
    CannotReconnect {
        /// The disconnect reason that prevents reconnection
        reason: String,
        #[snafu(implicit)]
        location: Location,
    },

    /// The operation is not valid in the current state.
    #[snafu(display("Invalid state for operation: current state is {state}"))]
    InvalidStateForOperation {
        /// The current state
        state: String,
        #[snafu(implicit)]
        location: Location,
    },

    /// Pending operations queue is full.
    #[snafu(display("Pending operations queue is full (max: {max_size})"))]
    QueueFull {
        /// Maximum queue size
        max_size: usize,
        #[snafu(implicit)]
        location: Location,
    },

    /// Failed to acquire lock on state machine.
    #[snafu(display("Failed to acquire state machine lock"))]
    LockFailed {
        #[snafu(implicit)]
        location: Location,
    },
}

/// Result type alias for state machine operations.
pub type Result<T> = std::result::Result<T, StateMachineError>;
