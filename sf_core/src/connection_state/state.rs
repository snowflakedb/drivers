//! Connection state definitions and transition validation.
//!
//! This module defines the explicit states a connection can be in,
//! along with the rules for valid state transitions.

use std::fmt;

/// The possible states of a Snowflake connection.
///
/// This enum represents an explicit, type-safe state machine for connection lifecycle.
/// Each state has specific behaviors and valid transitions to other states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    /// Initial state - no connection attempted yet.
    ///
    /// Valid transitions:
    /// - `Connecting` (via connect())
    Pristine,

    /// Login/authentication in progress (async operation).
    ///
    /// While in this state, requests are queued and will be processed
    /// once the connection transitions to `Connected`.
    ///
    /// Valid transitions:
    /// - `Connected` (login success)
    /// - `Disconnected` (login failure)
    Connecting,

    /// Successfully authenticated and ready for requests.
    ///
    /// This is the normal operating state where queries can be executed.
    ///
    /// Valid transitions:
    /// - `Renewing` (session token expired)
    /// - `Disconnected` (user close or fatal error)
    Connected,

    /// Session token refresh in progress.
    ///
    /// The session token has expired but the master token is still valid.
    /// A refresh request is in flight. Requests are queued during this state.
    ///
    /// Valid transitions:
    /// - `Connected` (refresh success)
    /// - `Disconnected` (master token expired or refresh failed)
    Renewing,

    /// Connection is closed or in an error state.
    ///
    /// Depending on the `reason`, the connection may or may not be
    /// able to reconnect.
    ///
    /// Valid transitions:
    /// - `Connecting` (only if `reason.can_reconnect()` is true)
    Disconnected {
        /// The reason for disconnection
        reason: DisconnectReason,
    },
}

impl ConnectionState {
    /// Returns true if the connection is ready to execute requests.
    pub fn is_ready(&self) -> bool {
        matches!(self, ConnectionState::Connected)
    }

    /// Returns true if requests can potentially be executed (may be queued).
    ///
    /// In `Renewing` state, requests are queued and will be executed
    /// once the token refresh completes.
    pub fn can_accept_requests(&self) -> bool {
        matches!(self, ConnectionState::Connected | ConnectionState::Renewing)
    }

    /// Returns true if this is a terminal state that cannot transition further.
    ///
    /// Note: `Disconnected` is only terminal if `!reason.can_reconnect()`.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            ConnectionState::Disconnected {
                reason: DisconnectReason::InternalError { .. }
            }
        )
    }

    /// Returns true if requests should be queued in this state.
    pub fn should_queue_requests(&self) -> bool {
        matches!(
            self,
            ConnectionState::Connecting | ConnectionState::Renewing
        )
    }

    /// Returns true if this state can transition to `Connecting`.
    pub fn can_connect(&self) -> bool {
        match self {
            ConnectionState::Pristine => true,
            ConnectionState::Disconnected { reason } => reason.can_reconnect(),
            _ => false,
        }
    }

    /// Returns the display name for this state.
    pub fn name(&self) -> &'static str {
        match self {
            ConnectionState::Pristine => "Pristine",
            ConnectionState::Connecting => "Connecting",
            ConnectionState::Connected => "Connected",
            ConnectionState::Renewing => "Renewing",
            ConnectionState::Disconnected { .. } => "Disconnected",
        }
    }
}

impl fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectionState::Pristine => write!(f, "Pristine"),
            ConnectionState::Connecting => write!(f, "Connecting"),
            ConnectionState::Connected => write!(f, "Connected"),
            ConnectionState::Renewing => write!(f, "Renewing"),
            ConnectionState::Disconnected { reason } => write!(f, "Disconnected({reason})"),
        }
    }
}

/// The reason why a connection entered the `Disconnected` state.
///
/// Some reasons allow reconnection, others are terminal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisconnectReason {
    /// User explicitly called close() - CAN reconnect.
    UserInitiated,

    /// Master token expired, full re-authentication needed - CAN reconnect.
    MasterTokenExpired,

    /// Initial login failed - CAN retry with different credentials.
    LoginFailed {
        /// Snowflake error code
        code: i32,
        /// Error message
        message: String,
    },

    /// Session refresh failed - CAN reconnect with fresh login.
    SessionRefreshFailed {
        /// Error message
        message: String,
    },

    /// Unrecoverable internal error - CANNOT reconnect.
    ///
    /// This indicates a bug or fundamental issue that won't be fixed
    /// by simply reconnecting.
    InternalError {
        /// Error description
        message: String,
    },
}

impl DisconnectReason {
    /// Returns true if the connection can attempt to reconnect from this state.
    ///
    /// Most disconnect reasons allow reconnection. Only `InternalError`
    /// is considered unrecoverable.
    pub fn can_reconnect(&self) -> bool {
        !matches!(self, DisconnectReason::InternalError { .. })
    }

    /// Returns a user-friendly description of this disconnect reason.
    pub fn description(&self) -> String {
        match self {
            DisconnectReason::UserInitiated => "Connection closed by user".to_string(),
            DisconnectReason::MasterTokenExpired => {
                "Master token expired, re-authentication required".to_string()
            }
            DisconnectReason::LoginFailed { code, message } => {
                format!("Login failed (code {code}): {message}")
            }
            DisconnectReason::SessionRefreshFailed { message } => {
                format!("Session refresh failed: {message}")
            }
            DisconnectReason::InternalError { message } => {
                format!("Internal error: {message}")
            }
        }
    }
}

impl fmt::Display for DisconnectReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DisconnectReason::UserInitiated => write!(f, "UserInitiated"),
            DisconnectReason::MasterTokenExpired => write!(f, "MasterTokenExpired"),
            DisconnectReason::LoginFailed { code, .. } => write!(f, "LoginFailed(code={code})"),
            DisconnectReason::SessionRefreshFailed { .. } => write!(f, "SessionRefreshFailed"),
            DisconnectReason::InternalError { .. } => write!(f, "InternalError"),
        }
    }
}

/// Checks if a transition from one state to another is valid.
///
/// # Arguments
/// * `from` - The current state
/// * `to` - The desired target state
///
/// # Returns
/// `true` if the transition is valid, `false` otherwise.
pub fn is_valid_transition(from: &ConnectionState, to: &ConnectionState) -> bool {
    use ConnectionState::*;

    match (from, to) {
        // From Pristine: can only start connecting
        (Pristine, Connecting) => true,

        // From Connecting: success or failure
        (Connecting, Connected) => true,
        (Connecting, Disconnected { .. }) => true,

        // From Connected: can renew token or disconnect
        (Connected, Renewing) => true,
        (Connected, Disconnected { .. }) => true,

        // From Renewing: success or failure
        (Renewing, Connected) => true,
        (Renewing, Disconnected { .. }) => true,

        // From Disconnected: can reconnect if reason allows
        (Disconnected { reason }, Connecting) => reason.can_reconnect(),

        // Same state is always valid (idempotent)
        _ if from == to => true,

        // Everything else is invalid
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pristine_can_connect() {
        assert!(ConnectionState::Pristine.can_connect());
    }

    #[test]
    fn test_connected_is_ready() {
        assert!(ConnectionState::Connected.is_ready());
        assert!(!ConnectionState::Connecting.is_ready());
        assert!(!ConnectionState::Renewing.is_ready());
    }

    #[test]
    fn test_renewing_can_accept_requests() {
        assert!(ConnectionState::Renewing.can_accept_requests());
        assert!(ConnectionState::Connected.can_accept_requests());
        assert!(!ConnectionState::Connecting.can_accept_requests());
    }

    #[test]
    fn test_disconnected_user_initiated_can_reconnect() {
        let state = ConnectionState::Disconnected {
            reason: DisconnectReason::UserInitiated,
        };
        assert!(state.can_connect());
    }

    #[test]
    fn test_disconnected_internal_error_cannot_reconnect() {
        let state = ConnectionState::Disconnected {
            reason: DisconnectReason::InternalError {
                message: "fatal".to_string(),
            },
        };
        assert!(!state.can_connect());
        assert!(state.is_terminal());
    }

    #[test]
    fn test_valid_transitions() {
        use ConnectionState::*;

        // Pristine -> Connecting
        assert!(is_valid_transition(&Pristine, &Connecting));

        // Connecting -> Connected
        assert!(is_valid_transition(&Connecting, &Connected));

        // Connected -> Renewing
        assert!(is_valid_transition(&Connected, &Renewing));

        // Renewing -> Connected
        assert!(is_valid_transition(&Renewing, &Connected));

        // Disconnected(UserInitiated) -> Connecting (reconnect)
        let disconnected = Disconnected {
            reason: DisconnectReason::UserInitiated,
        };
        assert!(is_valid_transition(&disconnected, &Connecting));
    }

    #[test]
    fn test_invalid_transitions() {
        use ConnectionState::*;

        // Cannot go from Pristine directly to Connected
        assert!(!is_valid_transition(&Pristine, &Connected));

        // Cannot go from Connected directly to Connecting
        assert!(!is_valid_transition(&Connected, &Connecting));

        // Cannot reconnect from InternalError
        let internal_error = Disconnected {
            reason: DisconnectReason::InternalError {
                message: "fatal".to_string(),
            },
        };
        assert!(!is_valid_transition(&internal_error, &Connecting));
    }

    #[test]
    fn test_same_state_transition_is_valid() {
        assert!(is_valid_transition(
            &ConnectionState::Connected,
            &ConnectionState::Connected
        ));
    }
}
