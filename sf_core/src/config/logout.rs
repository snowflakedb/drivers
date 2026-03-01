//! Configuration for session logout behavior
//!
//! NOTE: This is an INTERNAL configuration struct used by connection_close().
//! Users do NOT pass this directly. Instead, users configure logout behavior
//! via ConnectionSetOption* calls before ConnectionInit, matching the pattern
//! used by all existing Snowflake drivers (Python, Go, JDBC, .NET, Node.js).

use crate::apis::database_driver_v1::error::ApiError;
use crate::config::settings::Settings;
use std::time::Duration;

use super::{ConfigError, InvalidParameterValueSnafu};

/// INTERNAL configuration for logout behavior during connection close.
///
/// This struct is constructed from Connection fields, not passed by users.
/// Users configure logout behavior via ConnectionSetOption* before ConnectionInit.
#[derive(Debug, Clone)]
pub struct LogoutConfig {
    /// Explicit control over server session lifecycle
    /// - Some(true): Never send logout (keep session alive - Fire & Forget)
    /// - Some(false): Always send logout (kill session and all jobs)
    /// - None: Delegate to auto-detection setting
    pub server_session_keep_alive: Option<bool>,

    /// Enable registry-based auto-detection of async queries
    /// - Some(true): Check async query registry before logout
    /// - Some(false): Don't check registry
    /// - None: Treated as false (no auto-detection)
    pub enable_auto_detection: Option<bool>,

    /// Error handling strategy for logout failures
    pub error_strategy: ErrorStrategy,

    /// Total timeout budget for logout operation (including all retry attempts)
    /// This is the maximum wall-clock time that close() will spend attempting logout.
    /// Individual HTTP request timeouts are derived from this total.
    pub logout_total_timeout: Duration,

    /// Maximum total attempts for logout requests (NOT number of retries)
    /// - Some(1): 1 attempt, 0 retries
    /// - Some(3): 3 attempts, 2 retries
    /// - None: Use default from RetryPolicy (typically 6)
    ///
    /// Note: This is TOTAL ATTEMPTS, not retry count. To disable retries, set to 1.
    pub max_attempts: Option<u32>,

    /// Per-request socket timeout for individual HTTP requests
    /// - Some(duration): Each request times out after this duration (dynamically adjusted to min(this, remaining_budget))
    /// - None: No per-request timeout (only total budget applies, like login/query operations)
    pub logout_request_timeout: Option<Duration>,
}

impl Default for LogoutConfig {
    fn default() -> Self {
        Self {
            server_session_keep_alive: None,
            enable_auto_detection: None,
            error_strategy: ErrorStrategy::Strict,
            logout_total_timeout: Duration::from_secs(5),
            max_attempts: None,
            logout_request_timeout: None,
        }
    }
}

impl LogoutConfig {
    /// Parse logout configuration from connection settings.
    ///
    /// All validation happens here, once, at connection_init time.
    /// This follows the same pattern as LoginParameters::from_settings.
    pub fn from_settings(settings: &dyn Settings) -> Result<Self, ConfigError> {
        // Parse and validate error_strategy (sent as protobuf enum int value)
        let error_strategy = match settings.get_int("logout_error_strategy") {
            Some(v) => ErrorStrategy::from_protobuf_value(v)?,
            None => ErrorStrategy::Strict, // default
        };

        // Parse and validate logout_total_timeout_seconds
        let logout_total_timeout = match settings.get_int("logout_total_timeout_seconds") {
            Some(v) => {
                if v < 0 {
                    return InvalidParameterValueSnafu {
                        parameter: "logout_total_timeout_seconds",
                        value: v.to_string(),
                        explanation: "Must be non-negative",
                    }
                    .fail();
                }
                Duration::from_secs(v as u64)
            }
            None => Duration::from_secs(5), // default
        };

        // Parse and validate logout_max_attempts
        let max_attempts = match settings.get_int("logout_max_attempts") {
            Some(v) => {
                if v < 0 {
                    return InvalidParameterValueSnafu {
                        parameter: "logout_max_attempts",
                        value: v.to_string(),
                        explanation: "Must be non-negative",
                    }
                    .fail();
                }
                Some(v as u32)
            }
            None => None, // Use RetryPolicy default
        };

        // Parse and validate logout_request_timeout_seconds
        let logout_request_timeout = match settings.get_int("logout_request_timeout_seconds") {
            Some(v) => {
                if v < 0 {
                    return InvalidParameterValueSnafu {
                        parameter: "logout_request_timeout_seconds",
                        value: v.to_string(),
                        explanation: "Must be non-negative",
                    }
                    .fail();
                }
                Some(Duration::from_secs(v as u64))
            }
            None => None, // No per-request timeout
        };

        Ok(Self {
            server_session_keep_alive: settings.get_bool("server_session_keep_alive"),
            enable_auto_detection: settings.get_bool("enable_logout_auto_detection"),
            error_strategy,
            logout_total_timeout,
            max_attempts,
            logout_request_timeout,
        })
    }
}

/// Strategy for error handling during logout.
///
/// Controls how errors are surfaced after all retry mechanisms have been exhausted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ErrorStrategy {
    /// Strict strategy: surface errors to the caller (close() may fail)
    #[default]
    Strict,

    /// Best-effort strategy: suppress errors, log WARN (close() always succeeds)
    BestEffort,
}

impl ErrorStrategy {
    pub const UNSPECIFIED_PROTOBUF: i64 = 0;
    pub const BEST_EFFORT_PROTOBUF: i64 = 1;
    pub const STRICT_PROTOBUF: i64 = 2;

    pub fn to_protobuf_value(self) -> i64 {
        match self {
            ErrorStrategy::BestEffort => Self::BEST_EFFORT_PROTOBUF,
            ErrorStrategy::Strict => Self::STRICT_PROTOBUF,
        }
    }

    pub fn from_protobuf_value(value: i64) -> Result<Self, ConfigError> {
        match value {
            Self::UNSPECIFIED_PROTOBUF => Ok(Self::default()),
            Self::BEST_EFFORT_PROTOBUF => Ok(ErrorStrategy::BestEffort),
            Self::STRICT_PROTOBUF => Ok(ErrorStrategy::Strict),
            _ => InvalidParameterValueSnafu {
                parameter: "logout_error_strategy",
                value: value.to_string(),
                explanation: "Must be 0 (UNSPECIFIED), 1 (BestEffort), or 2 (Strict)",
            }
            .fail(),
        }
    }

    /// Handle a failed logout after all retry mechanisms have been exhausted.
    ///
    /// Called after both retry layers have given up:
    /// - HTTP retries (execute_with_retry) for 503, 429, transport errors
    /// - Token refresh (RefreshContext) for 390112 session token expired
    ///
    /// By this point, recoverable errors (390111 session gone, 390112 token expired)
    /// have already been resolved. What remains are unrecoverable failures
    /// (network unreachable, timeout exceeded, unknown server errors).
    ///
    /// Strict: surface the error to the caller (close() may fail)
    /// BestEffort: suppress the error, log WARN (close() always succeeds)
    #[allow(clippy::result_large_err)]
    pub fn handle_failed_logout(self, result: Result<(), ApiError>) -> Result<(), ApiError> {
        match result {
            Ok(()) => Ok(()),
            Err(e) => match self {
                ErrorStrategy::Strict => {
                    tracing::error!(error = %e, "Logout failed after retries exhausted");
                    Err(e)
                }
                ErrorStrategy::BestEffort => {
                    tracing::warn!(error = %e, "Logout failed after retries exhausted, suppressed");
                    Ok(())
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snafu::Location;

    #[test]
    fn test_default_config() {
        let config = LogoutConfig::default();
        assert_eq!(config.server_session_keep_alive, None);
        assert_eq!(config.enable_auto_detection, None);
        assert_eq!(config.error_strategy, ErrorStrategy::Strict);
        assert_eq!(config.logout_total_timeout, Duration::from_secs(5));
    }

    #[test]
    fn test_default_error_strategy_is_strict() {
        let strategy = ErrorStrategy::default();
        assert_eq!(strategy, ErrorStrategy::Strict);
    }

    fn make_logout_error(message: &str) -> ApiError {
        ApiError::LogoutFailed {
            message: message.to_string(),
            location: Location::default(),
        }
    }

    #[test]
    fn test_strict_raises_error() {
        let error = make_logout_error("test error");
        let result = ErrorStrategy::Strict.handle_failed_logout(Err(error));
        assert!(result.is_err(), "Strict should raise errors");
    }

    #[test]
    fn test_strict_passes_through_ok() {
        let result = ErrorStrategy::Strict.handle_failed_logout(Ok(()));
        assert!(result.is_ok(), "Strict should pass through Ok");
    }

    #[test]
    fn test_best_effort_suppresses_error() {
        let error = make_logout_error("test error");
        let result = ErrorStrategy::BestEffort.handle_failed_logout(Err(error));
        assert!(result.is_ok(), "BestEffort should suppress errors");
    }

    #[test]
    fn test_best_effort_passes_through_ok() {
        let result = ErrorStrategy::BestEffort.handle_failed_logout(Ok(()));
        assert!(result.is_ok(), "BestEffort should pass through Ok");
    }

    #[test]
    fn test_error_strategy_from_protobuf_value() {
        // Test protobuf enum parsing
        assert_eq!(
            ErrorStrategy::from_protobuf_value(0).unwrap(),
            ErrorStrategy::Strict,
            "UNSPECIFIED (0) should default to Strict"
        );
        assert_eq!(
            ErrorStrategy::from_protobuf_value(1).unwrap(),
            ErrorStrategy::BestEffort,
            "BEST_EFFORT (1) should map to BestEffort"
        );
        assert_eq!(
            ErrorStrategy::from_protobuf_value(2).unwrap(),
            ErrorStrategy::Strict,
            "STRICT (2) should map to Strict"
        );
    }

    #[test]
    fn test_error_strategy_from_protobuf_value_invalid() {
        let result = ErrorStrategy::from_protobuf_value(999);
        assert!(
            result.is_err(),
            "from_protobuf_value should reject invalid values"
        );

        let result = ErrorStrategy::from_protobuf_value(-1);
        assert!(
            result.is_err(),
            "from_protobuf_value should reject negative values"
        );
    }
}
