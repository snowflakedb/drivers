//! Connection state machine for managing Snowflake connection lifecycle.
//!
//! This module provides an explicit, type-safe state machine for managing
//! the connection lifecycle. It addresses issues found in other drivers'
//! state management implementations by providing:
//!
//! - **Explicit state enum**: State is a real type, not hidden in closures
//! - **Validated transitions**: Invalid state changes return errors
//! - **Observable state changes**: Subscribe to state transitions via broadcast channel
//! - **Bounded operation queue**: Pending operations have timeouts and limits
//! - **Reconnection support**: Can reconnect from most disconnected states
//!
//! # States
//!
//! The connection can be in one of five states:
//!
//! - [`ConnectionState::Pristine`] - Initial state, no connection attempted
//! - [`ConnectionState::Connecting`] - Login in progress
//! - [`ConnectionState::Connected`] - Authenticated and ready for requests
//! - [`ConnectionState::Renewing`] - Session token refresh in progress
//! - [`ConnectionState::Disconnected`] - Connection closed or in error state
//!
//! # Usage Example
//!
//! ```ignore
//! use sf_core::connection_state::{ConnectionStateMachine, ConnectionState, DisconnectReason};
//! use std::time::Duration;
//!
//! // Create a new state machine
//! let sm = ConnectionStateMachine::new();
//!
//! // Subscribe to state changes
//! let mut rx = sm.subscribe();
//! tokio::spawn(async move {
//!     while let Ok(state) = rx.recv().await {
//!         println!("State: {:?}", state);
//!     }
//! });
//!
//! // Start connecting
//! sm.start_connecting().await?;
//!
//! // ... perform login ...
//!
//! // Mark as connected
//! sm.connection_established().await?;
//!
//! // Check state
//! assert!(sm.is_ready().await);
//!
//! // Wait for connection to be ready (useful from other tasks)
//! sm.wait_ready(Duration::from_secs(30)).await?;
//!
//! // Close connection (can reconnect later)
//! sm.close().await?;
//!
//! // Reconnect
//! sm.start_connecting().await?;
//! ```
//!
//! # Thread Safety
//!
//! The [`ConnectionStateMachine`] is fully thread-safe and can be cloned.
//! All clones share the same underlying state. Operations during transient
//! states (Connecting, Renewing) are automatically queued and processed
//! when the connection becomes ready.
//!
//! # Design Rationale
//!
//! This design addresses issues observed in the Node.js Snowflake connector:
//!
//! | Node.js Problem | This Solution |
//! |-----------------|---------------|
//! | Hidden closure state | Explicit [`ConnectionState`] enum |
//! | Callback mutation | Async/await with [`Result`] |
//! | No observability | [`broadcast`] channel subscription |
//! | Unbounded queue | Bounded queue with timeouts |
//! | No type safety | Strong typing with [`snafu`] errors |

pub mod error;
pub mod machine;
pub mod pending_ops;
pub mod state;

// Re-export main types
pub use error::{Result, StateMachineError};
pub use machine::{ConnectionStateMachine, DEFAULT_WAIT_TIMEOUT};
pub use pending_ops::{
    DEFAULT_MAX_PENDING_OPS, DEFAULT_PENDING_OP_TIMEOUT, PendingOperation, PendingOpsQueue,
};
pub use state::{ConnectionState, DisconnectReason, is_valid_transition};
