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

    /// Maximum number of retry attempts for failed logout requests
    /// - Some(0): No retries, single attempt only
    /// - Some(n): Allow up to n retries
    /// - None: Use default from RetryPolicy (typically 6)
    pub max_retry_attempts: Option<u32>,

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
            error_strategy: ErrorStrategy::default(), // Uses #[default] annotation
            logout_total_timeout: Duration::from_secs(5),
            max_retry_attempts: None,
            logout_request_timeout: None,
        }
    }
}

impl LogoutConfig {
    /// Merge connection-wide config with per-request overrides.
    ///
    /// Hierarchy: request_override > self (connection-wide) > defaults
    ///
    /// Per-request overrides allow special-case behavior (e.g., Python's retry parameter)
    /// without modifying connection state, avoiding "wide implicit consequences".
    pub fn merge_with_request(
        &self,
        server_session_keep_alive: Option<bool>,
        enable_auto_detection: Option<bool>,
        error_strategy: Option<ErrorStrategy>,
        logout_total_timeout_seconds: Option<i32>,
        max_retry_attempts: Option<i32>,
        logout_request_timeout_seconds: Option<i32>,
    ) -> Self {
        use std::time::Duration;

        Self {
            server_session_keep_alive: server_session_keep_alive.or(self.server_session_keep_alive),

            enable_auto_detection: enable_auto_detection.or(self.enable_auto_detection),

            // Error strategy: request override > connection-wide
            // (connection-wide already has default from LogoutConfig::default())
            error_strategy: error_strategy.unwrap_or(self.error_strategy),

            // Timeout: request override > connection-wide
            logout_total_timeout: logout_total_timeout_seconds
                .map(|s| Duration::from_secs(s as u64))
                .unwrap_or(self.logout_total_timeout),

            // Max retry attempts: request override > connection-wide
            max_retry_attempts: max_retry_attempts
                .map(|v| v as u32)
                .or(self.max_retry_attempts),

            // Request timeout: request override > connection-wide
            logout_request_timeout: logout_request_timeout_seconds
                .map(|s| Duration::from_secs(s as u64))
                .or(self.logout_request_timeout),
        }
    }

    /// Parse logout configuration from connection settings.
    ///
    /// All validation happens here, once, at connection_init time.
    /// This follows the same pattern as LoginParameters::from_settings.
    pub fn from_settings(settings: &dyn Settings) -> Result<Self, ConfigError> {
        // Parse and validate error_strategy using ErrorStrategy's own parser (SRP)
        let error_strategy = match settings.get_string("logout_error_strategy") {
            Some(s) => ErrorStrategy::from_settings_str(&s)?,
            None => ErrorStrategy::default(), // Uses #[default] annotation
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

        // Parse and validate logout_max_retry_attempts
        let max_retry_attempts = match settings.get_int("logout_max_retry_attempts") {
            Some(v) => {
                if v < 0 {
                    return InvalidParameterValueSnafu {
                        parameter: "logout_max_retry_attempts",
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
            max_retry_attempts,
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
    /// Parse ErrorStrategy from settings string value.
    ///
    /// Valid values:
    /// - "strict": Propagate errors (close() may fail)
    /// - "best_effort": Suppress errors, log WARN (close() always succeeds)
    ///
    /// This encapsulates string-to-enum parsing logic in the type itself (SRP).
    pub fn from_settings_str(s: &str) -> Result<Self, ConfigError> {
        match s {
            "strict" => Ok(ErrorStrategy::Strict),
            "best_effort" => Ok(ErrorStrategy::BestEffort),
            _ => InvalidParameterValueSnafu {
                parameter: "logout_error_strategy",
                value: s.to_string(),
                explanation: "Must be 'strict' or 'best_effort'",
            }
            .fail(),
        }
    }

    /// Convert from protobuf ErrorStrategy enum value to internal ErrorStrategy.
    ///
    /// The protobuf enum values are:
    /// - 0: ERROR_STRATEGY_UNSPECIFIED → returns default strategy
    /// - 1: ERROR_STRATEGY_BEST_EFFORT
    /// - 2: ERROR_STRATEGY_STRICT
    ///
    /// Note: In practice, value 0 shouldn't be passed explicitly since protobuf
    /// uses Option<i32>. If the field is unset, the Option is None (not Some(0)).
    pub fn from_protobuf_enum(value: i32) -> Result<Self, String> {
        match value {
            0 => Ok(Self::default()), // UNSPECIFIED uses default
            1 => Ok(ErrorStrategy::BestEffort),
            2 => Ok(ErrorStrategy::Strict),
            _ => Err(format!("Invalid ErrorStrategy value: {}", value)),
        }
    }
}

impl ErrorStrategy {
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
    fn test_merge_request_overrides_connection_wide() {
        let conn_config = LogoutConfig {
            max_retry_attempts: Some(5),
            error_strategy: ErrorStrategy::BestEffort,
            ..Default::default()
        };

        let effective = conn_config.merge_with_request(
            None,                        // server_session_keep_alive
            None,                        // enable_auto_detection
            Some(ErrorStrategy::Strict), // Override error strategy
            None,                        // logout_total_timeout_seconds
            Some(1),                     // Override max_retry_attempts
            None,                        // logout_request_timeout_seconds
        );

        assert_eq!(effective.max_retry_attempts, Some(1));
        assert_eq!(effective.error_strategy, ErrorStrategy::Strict);
    }

    #[test]
    fn test_merge_uses_connection_wide_when_request_none() {
        let conn_config = LogoutConfig {
            max_retry_attempts: Some(5),
            error_strategy: ErrorStrategy::BestEffort,
            ..Default::default()
        };

        let effective = conn_config.merge_with_request(
            None, // No override
            None, None, None, None, // No override for max_retry_attempts
            None,
        );

        assert_eq!(effective.max_retry_attempts, Some(5));
        assert_eq!(effective.error_strategy, ErrorStrategy::BestEffort);
    }

    #[test]
    fn test_merge_fallback_to_defaults_when_both_none() {
        let conn_config = LogoutConfig {
            error_strategy: ErrorStrategy::Strict,
            max_retry_attempts: None, // No connection-wide setting
            ..Default::default()
        };

        let effective = conn_config.merge_with_request(
            None, None, None, // No override
            None, None, // No override
            None,
        );

        // Should use connection-wide error_strategy (Strict)
        assert_eq!(effective.error_strategy, ErrorStrategy::Strict);
        // Should maintain None for max_retry_attempts (will use RetryPolicy default)
        assert_eq!(effective.max_retry_attempts, None);
    }

    #[test]
    fn test_merge_converts_timeout_seconds_to_duration() {
        let conn_config = LogoutConfig::default();

        let effective = conn_config.merge_with_request(
            None,
            None,
            None,
            Some(10), // 10 seconds
            None,
            Some(5), // 5 seconds
        );

        assert_eq!(effective.logout_total_timeout, Duration::from_secs(10));
        assert_eq!(
            effective.logout_request_timeout,
            Some(Duration::from_secs(5))
        );
    }

    #[test]
    fn test_from_protobuf_enum_unspecified_uses_default() {
        let result = ErrorStrategy::from_protobuf_enum(0);
        assert_eq!(result.unwrap(), ErrorStrategy::default());
        // Verify default is Strict (defined by #[default] annotation)
        assert_eq!(ErrorStrategy::default(), ErrorStrategy::Strict);
    }

    #[test]
    fn test_from_protobuf_enum_best_effort() {
        let result = ErrorStrategy::from_protobuf_enum(1);
        assert_eq!(result.unwrap(), ErrorStrategy::BestEffort);
    }

    #[test]
    fn test_from_protobuf_enum_strict() {
        let result = ErrorStrategy::from_protobuf_enum(2);
        assert_eq!(result.unwrap(), ErrorStrategy::Strict);
    }

    #[test]
    fn test_from_protobuf_enum_invalid_value() {
        let result = ErrorStrategy::from_protobuf_enum(999);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_settings_str_strict() {
        let result = ErrorStrategy::from_settings_str("strict");
        assert_eq!(result.unwrap(), ErrorStrategy::Strict);
    }

    #[test]
    fn test_from_settings_str_best_effort() {
        let result = ErrorStrategy::from_settings_str("best_effort");
        assert_eq!(result.unwrap(), ErrorStrategy::BestEffort);
    }

    #[test]
    fn test_from_settings_str_invalid() {
        let result = ErrorStrategy::from_settings_str("invalid");
        assert!(result.is_err());
    }
}
