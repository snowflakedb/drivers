//! Configuration for session logout behavior
//!
//! NOTE: This is an INTERNAL configuration struct used by connection_close().
//! Users do NOT pass this directly. Instead, users configure logout behavior
//! via ConnectionSetOption* calls before ConnectionInit, matching the pattern
//! used by all existing Snowflake drivers (Python, Go, JDBC, .NET, Node.js).

use crate::apis::database_driver_v1::error::ApiError;
use crate::config::settings::Settings;
use std::str::FromStr;
use std::time::Duration;

use super::{ConfigError, InvalidParameterValueSnafu};

/// Validate that a seconds value is strictly positive and return as `Duration`.
///
/// Used for `logout_total_timeout_seconds` at both init-time and close-time.
fn validate_positive_seconds(param: &str, value: i64) -> Result<Duration, ConfigError> {
    if value <= 0 {
        return InvalidParameterValueSnafu {
            parameter: param,
            value: value.to_string(),
            explanation: "Must be positive (greater than zero)",
        }
        .fail();
    }
    Ok(Duration::from_secs(value as u64))
}

/// Validate that a seconds value is non-negative and return as `Duration`.
///
/// Used for `logout_request_timeout_seconds` at both init-time and close-time.
fn validate_non_negative_seconds(param: &str, value: i64) -> Result<Duration, ConfigError> {
    if value < 0 {
        return InvalidParameterValueSnafu {
            parameter: param,
            value: value.to_string(),
            explanation: "Must be non-negative",
        }
        .fail();
    }
    Ok(Duration::from_secs(value as u64))
}

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
    pub enable_server_session_keep_alive_auto_detection: Option<bool>,

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
    /// Core defaults apply when no language wrapper overrides them.
    /// Language wrappers typically set their own defaults for backward compat:
    /// e.g. Python sets BestEffort, 15s timeout, 3 attempts
    /// (see python/.../logout_config_mapping.py::map_logout_config_phase2).
    fn default() -> Self {
        Self {
            server_session_keep_alive: None,
            enable_server_session_keep_alive_auto_detection: None,
            error_strategy: ErrorStrategy::Strict,
            logout_total_timeout: Duration::from_secs(5),
            max_attempts: None,
            logout_request_timeout: None,
        }
    }
}

impl LogoutConfig {
    /// Create a new `LogoutConfig` by merging close-time override values into `self`.
    ///
    /// Hierarchy: override parameter > `self` (connection-wide init-time value) > `self` default
    /// Any `None` override means "keep `self`'s value unchanged".
    ///
    /// `max_retry_attempts` uses retry count (0 = no retries = 1 total attempt), which is
    /// converted to total-attempts for the internal `max_attempts` field.
    pub fn merge_with_request(
        &self,
        server_session_keep_alive: Option<bool>,
        enable_server_session_keep_alive_auto_detection: Option<bool>,
        error_strategy: Option<ErrorStrategy>,
        logout_total_timeout_seconds: Option<i32>,
        max_retry_attempts: Option<i32>,
        logout_request_timeout_seconds: Option<i32>,
    ) -> Result<Self, ConfigError> {
        let logout_total_timeout = match logout_total_timeout_seconds {
            Some(s) => validate_positive_seconds("logout_total_timeout_seconds", s as i64)?,
            None => self.logout_total_timeout,
        };

        // max_retry_attempts is 0-based (retries), must be >= 0, converted to 1-based (total)
        let max_attempts = match max_retry_attempts {
            Some(r) => {
                if r < 0 {
                    return InvalidParameterValueSnafu {
                        parameter: "max_retry_attempts",
                        value: r.to_string(),
                        explanation: "Must be non-negative (0 = no retries)",
                    }
                    .fail();
                }
                Some(r as u32 + 1)
            }
            None => self.max_attempts,
        };

        let logout_request_timeout = match logout_request_timeout_seconds {
            Some(s) => Some(validate_non_negative_seconds(
                "logout_request_timeout_seconds",
                s as i64,
            )?),
            None => self.logout_request_timeout,
        };

        Ok(Self {
            server_session_keep_alive: server_session_keep_alive.or(self.server_session_keep_alive),
            enable_server_session_keep_alive_auto_detection:
                enable_server_session_keep_alive_auto_detection
                    .or(self.enable_server_session_keep_alive_auto_detection),
            error_strategy: error_strategy.unwrap_or(self.error_strategy),
            logout_total_timeout,
            max_attempts,
            logout_request_timeout,
        })
    }

    /// Parse logout configuration from connection settings.
    ///
    /// All validation happens here, once, at connection_init time.
    /// This follows the same pattern as LoginParameters::from_settings.
    pub fn from_settings(settings: &dyn Settings) -> Result<Self, ConfigError> {
        let error_strategy = match settings.get_string("logout_error_strategy") {
            Some(v) => v.parse::<ErrorStrategy>()?,
            None => ErrorStrategy::Strict,
        };

        let logout_total_timeout = match settings.get_int("logout_total_timeout_seconds") {
            Some(v) => validate_positive_seconds("logout_total_timeout_seconds", v)?,
            None => Duration::from_secs(5),
        };

        let max_attempts = match settings.get_int("logout_max_attempts") {
            Some(v) => {
                if v <= 0 {
                    return InvalidParameterValueSnafu {
                        parameter: "logout_max_attempts",
                        value: v.to_string(),
                        explanation: "Must be positive (minimum 1 attempt required)",
                    }
                    .fail();
                }
                Some(v as u32)
            }
            None => None,
        };

        let logout_request_timeout = match settings.get_int("logout_request_timeout_seconds") {
            Some(v) => Some(validate_non_negative_seconds(
                "logout_request_timeout_seconds",
                v,
            )?),
            None => None,
        };

        Ok(Self {
            server_session_keep_alive: settings.get_bool("server_session_keep_alive"),
            enable_server_session_keep_alive_auto_detection: settings
                .get_bool("enable_server_session_keep_alive_auto_detection"),
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
/// Configured via connection_set_option_string("logout_error_strategy", value)
/// using the string constants defined below ("best_effort" or "strict").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ErrorStrategy {
    /// Strict strategy: surface errors to the caller (close() may fail)
    #[default]
    Strict,

    /// Best-effort strategy: suppress errors, log WARN (close() always succeeds)
    BestEffort,
}

impl ErrorStrategy {
    /// Convert from a protobuf enum integer (as stored in prost optional enum fields).
    ///
    /// Returns `None` for `0` (ERROR_STRATEGY_UNSPECIFIED) meaning "use the
    /// connection-wide default". Returns `None` for unknown values (graceful fallback).
    pub fn from_proto_i32(value: i32) -> Option<Self> {
        match value {
            0 => None, // Unspecified: caller should use connection-wide default
            1 => Some(Self::BestEffort),
            2 => Some(Self::Strict),
            other => {
                tracing::debug!(
                    value = other,
                    "Unknown ErrorStrategy proto value, ignoring override"
                );
                None
            }
        }
    }

    /// String value for BestEffort error strategy.
    pub const BEST_EFFORT: &'static str = "best_effort";
    /// String value for Strict error strategy.
    pub const STRICT: &'static str = "strict";

    /// Convert to the canonical string representation.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BestEffort => Self::BEST_EFFORT,
            Self::Strict => Self::STRICT,
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

impl FromStr for ErrorStrategy {
    type Err = ConfigError;

    /// Parse from a string value set via connection_set_option_string.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            Self::BEST_EFFORT => Ok(Self::BestEffort),
            Self::STRICT => Ok(Self::Strict),
            _ => InvalidParameterValueSnafu {
                parameter: "logout_error_strategy",
                value: value.to_string(),
                explanation: format!(
                    "Must be {:?} (BestEffort) or {:?} (Strict)",
                    Self::BEST_EFFORT,
                    Self::STRICT
                ),
            }
            .fail(),
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
        assert_eq!(config.enable_server_session_keep_alive_auto_detection, None);
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
    fn test_error_strategy_parse() {
        assert_eq!(
            "best_effort".parse::<ErrorStrategy>().unwrap(),
            ErrorStrategy::BestEffort,
        );
        assert_eq!(
            "strict".parse::<ErrorStrategy>().unwrap(),
            ErrorStrategy::Strict,
        );
    }

    #[test]
    fn test_error_strategy_parse_invalid() {
        let result = "unknown".parse::<ErrorStrategy>();
        assert!(result.is_err(), "parse should reject unknown values");

        let result = "".parse::<ErrorStrategy>();
        assert!(result.is_err(), "parse should reject empty string");
    }

    #[test]
    fn test_error_strategy_as_str() {
        assert_eq!(ErrorStrategy::BestEffort.as_str(), "best_effort");
        assert_eq!(ErrorStrategy::Strict.as_str(), "strict");
    }

    #[test]
    fn test_from_proto_i32_known_values() {
        assert_eq!(ErrorStrategy::from_proto_i32(0), None); // Unspecified: use connection-wide
        assert_eq!(
            ErrorStrategy::from_proto_i32(1),
            Some(ErrorStrategy::BestEffort)
        );
        assert_eq!(
            ErrorStrategy::from_proto_i32(2),
            Some(ErrorStrategy::Strict)
        );
    }

    #[test]
    fn test_from_proto_i32_unknown_returns_none() {
        assert_eq!(ErrorStrategy::from_proto_i32(99), None);
        assert_eq!(ErrorStrategy::from_proto_i32(-1), None);
    }

    #[test]
    fn test_merge_with_request_override_wins() {
        let base = LogoutConfig {
            server_session_keep_alive: Some(true),
            enable_server_session_keep_alive_auto_detection: Some(false),
            error_strategy: ErrorStrategy::Strict,
            logout_total_timeout: Duration::from_secs(5),
            max_attempts: Some(3),
            logout_request_timeout: Some(Duration::from_secs(2)),
        };

        let merged = base
            .merge_with_request(
                Some(false),
                Some(true),
                Some(ErrorStrategy::BestEffort),
                Some(10),
                Some(0), // 0 retries = 1 total attempt
                Some(4),
            )
            .unwrap();

        assert_eq!(merged.server_session_keep_alive, Some(false));
        assert_eq!(
            merged.enable_server_session_keep_alive_auto_detection,
            Some(true)
        );
        assert_eq!(merged.error_strategy, ErrorStrategy::BestEffort);
        assert_eq!(merged.logout_total_timeout, Duration::from_secs(10));
        assert_eq!(merged.max_attempts, Some(1)); // 0 retries + 1 = 1 total attempt
        assert_eq!(merged.logout_request_timeout, Some(Duration::from_secs(4)));
    }

    #[test]
    fn test_merge_with_request_none_preserves_self() {
        let base = LogoutConfig {
            server_session_keep_alive: Some(true),
            enable_server_session_keep_alive_auto_detection: Some(true),
            error_strategy: ErrorStrategy::BestEffort,
            logout_total_timeout: Duration::from_secs(7),
            max_attempts: Some(5),
            logout_request_timeout: Some(Duration::from_secs(3)),
        };

        let merged = base
            .merge_with_request(None, None, None, None, None, None)
            .unwrap();

        assert_eq!(merged.server_session_keep_alive, Some(true));
        assert_eq!(
            merged.enable_server_session_keep_alive_auto_detection,
            Some(true)
        );
        assert_eq!(merged.error_strategy, ErrorStrategy::BestEffort);
        assert_eq!(merged.logout_total_timeout, Duration::from_secs(7));
        assert_eq!(merged.max_attempts, Some(5));
        assert_eq!(merged.logout_request_timeout, Some(Duration::from_secs(3)));
    }

    #[test]
    fn test_merge_max_retry_attempts_zero_means_one_total_attempt() {
        let base = LogoutConfig::default();
        let merged = base
            .merge_with_request(None, None, None, None, Some(0), None)
            .unwrap();
        assert_eq!(
            merged.max_attempts,
            Some(1),
            "0 retries should convert to 1 total attempt"
        );
    }

    #[test]
    fn test_merge_max_retry_attempts_none_preserves_self_max_attempts_none() {
        let base = LogoutConfig::default();
        let merged = base
            .merge_with_request(None, None, None, None, None, None)
            .unwrap();
        assert_eq!(
            merged.max_attempts, None,
            "None override should preserve None from self"
        );
    }

    #[test]
    fn test_merge_rejects_negative_max_retry_attempts() {
        let base = LogoutConfig::default();
        let result = base.merge_with_request(None, None, None, None, Some(-1), None);
        assert!(
            result.is_err(),
            "Negative max_retry_attempts must be rejected"
        );
    }

    #[test]
    fn test_merge_rejects_negative_total_timeout() {
        let base = LogoutConfig::default();
        let result = base.merge_with_request(None, None, None, Some(-5), None, None);
        assert!(
            result.is_err(),
            "Negative logout_total_timeout_seconds must be rejected"
        );
    }

    #[test]
    fn test_merge_rejects_zero_total_timeout() {
        let base = LogoutConfig::default();
        let result = base.merge_with_request(None, None, None, Some(0), None, None);
        assert!(
            result.is_err(),
            "Zero logout_total_timeout_seconds must be rejected"
        );
    }

    #[test]
    fn test_merge_rejects_negative_request_timeout() {
        let base = LogoutConfig::default();
        let result = base.merge_with_request(None, None, None, None, None, Some(-1));
        assert!(
            result.is_err(),
            "Negative logout_request_timeout_seconds must be rejected"
        );
    }

    #[test]
    fn test_merge_accepts_zero_request_timeout() {
        let base = LogoutConfig::default();
        let merged = base
            .merge_with_request(None, None, None, None, None, Some(0))
            .unwrap();
        assert_eq!(
            merged.logout_request_timeout,
            Some(Duration::from_secs(0)),
            "Zero request timeout is valid (means no per-request limit)"
        );
    }
}
