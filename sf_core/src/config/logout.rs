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
/// Shared by both `from_settings()` (init-time) and `merge_with_request()` (close-time)
/// to ensure consistent validation of timeout fields.
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

/// INTERNAL configuration for logout behavior during connection close.
///
/// This struct is constructed from Connection fields, not passed by users.
/// Users configure logout behavior via ConnectionSetOption* before ConnectionInit.
#[derive(Debug, Clone, PartialEq)]
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
    /// - Some(3): 3 attempts, 2 retries (Core default per DD)
    /// - None: Skip Core override; RetryPolicy default (6 attempts) applies
    ///
    /// Note: This is TOTAL ATTEMPTS, not retry count. To disable retries, set to 1.
    /// Core default is Some(3); language wrappers override to their historical values.
    pub max_attempts: Option<u32>,

    /// Per-request socket timeout for individual HTTP requests
    /// - Some(duration): Each request times out after this duration (dynamically adjusted to min(this, remaining_budget))
    /// - None: No per-request timeout (only total budget applies, like login/query operations)
    pub logout_request_timeout: Option<Duration>,
}

impl Default for LogoutConfig {
    /// Core is the single source of truth for logout defaults.
    /// Language wrappers override only error_strategy (Python uses BestEffort);
    /// timeout and attempt defaults are owned by Core.
    fn default() -> Self {
        Self {
            server_session_keep_alive: None,
            enable_server_session_keep_alive_auto_detection: None,
            error_strategy: ErrorStrategy::Strict,
            logout_total_timeout: Duration::from_secs(15),
            max_attempts: Some(3),
            logout_request_timeout: None,
        }
    }
}

/// Optional close-time overrides for logout config, passed via `ConnectionCloseRequest`.
///
/// Hierarchy: override here > init-time base (from `connection_set_options`) > Core defaults.
/// `None` fields mean "keep init-time value unchanged".
#[derive(Debug, Clone, Default)]
pub struct CloseParamsOverrides {
    pub server_session_keep_alive: Option<bool>,
    pub enable_server_session_keep_alive_auto_detection: Option<bool>,
    pub error_strategy: Option<ErrorStrategy>,
    pub logout_total_timeout_seconds: Option<i32>,
    pub max_attempts: Option<i32>,
    pub logout_request_timeout_seconds: Option<i32>,
}

impl LogoutConfig {
    /// Create a new `LogoutConfig` by merging close-time overrides into `self`.
    ///
    /// Hierarchy: override > `self` (init-time value) > Core default.
    /// `None` fields in `overrides` leave `self`'s value unchanged.
    ///
    /// `max_attempts` uses total-attempt semantics (1-based): 1 = no retry, 3 = 2 retries.
    pub fn merge_with_request(
        &self,
        overrides: &CloseParamsOverrides,
    ) -> Result<Self, ConfigError> {
        let logout_total_timeout = match overrides.logout_total_timeout_seconds {
            Some(seconds) => {
                validate_positive_seconds("logout_total_timeout_seconds", seconds as i64)?
            }
            None => self.logout_total_timeout,
        };

        let max_attempts = match overrides.max_attempts {
            Some(attempts) => {
                if attempts <= 0 {
                    return InvalidParameterValueSnafu {
                        parameter: "max_attempts",
                        value: attempts.to_string(),
                        explanation: "Must be positive (minimum 1 attempt required)",
                    }
                    .fail();
                }
                Some(attempts as u32)
            }
            None => self.max_attempts,
        };

        let logout_request_timeout = match overrides.logout_request_timeout_seconds {
            Some(seconds) => Some(validate_positive_seconds(
                "logout_request_timeout_seconds",
                seconds as i64,
            )?),
            None => self.logout_request_timeout,
        };

        Ok(Self {
            server_session_keep_alive: overrides
                .server_session_keep_alive
                .or(self.server_session_keep_alive),
            enable_server_session_keep_alive_auto_detection: overrides
                .enable_server_session_keep_alive_auto_detection
                .or(self.enable_server_session_keep_alive_auto_detection),
            error_strategy: overrides.error_strategy.unwrap_or(self.error_strategy),
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
        let mut config = Self::default();

        if let Some(v) = settings.get_string("logout_error_strategy") {
            config.error_strategy = v.parse::<ErrorStrategy>()?;
        }

        if let Some(v) = settings.get_int("logout_total_timeout_seconds") {
            config.logout_total_timeout =
                validate_positive_seconds("logout_total_timeout_seconds", v)?;
        }

        if let Some(v) = settings.get_int("logout_max_attempts") {
            if v <= 0 {
                return InvalidParameterValueSnafu {
                    parameter: "logout_max_attempts",
                    value: v.to_string(),
                    explanation: "Must be positive (minimum 1 attempt required)",
                }
                .fail();
            }
            if v > u32::MAX as i64 {
                return InvalidParameterValueSnafu {
                    parameter: "logout_max_attempts",
                    value: v.to_string(),
                    explanation: "Must not exceed 4294967295 (u32::MAX)",
                }
                .fail();
            }
            config.max_attempts = Some(v as u32);
        }

        if let Some(v) = settings.get_int("logout_request_timeout_seconds") {
            config.logout_request_timeout = Some(validate_positive_seconds(
                "logout_request_timeout_seconds",
                v,
            )?);
        }

        config.server_session_keep_alive = settings.get_bool("server_session_keep_alive");
        config.enable_server_session_keep_alive_auto_detection =
            settings.get_bool("enable_server_session_keep_alive_auto_detection");

        Ok(config)
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
    use crate::config::settings::Setting;
    use snafu::Location;
    use std::collections::HashMap;

    fn create_test_settings(options: Vec<(&str, Setting)>) -> HashMap<String, Setting> {
        options
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect()
    }

    #[test]
    fn test_default_config() {
        let config = LogoutConfig::default();
        assert_eq!(config.server_session_keep_alive, None);
        assert_eq!(config.enable_server_session_keep_alive_auto_detection, None);
        assert_eq!(config.error_strategy, ErrorStrategy::Strict);
        assert_eq!(config.logout_total_timeout, Duration::from_secs(5));
        assert_eq!(config.max_attempts, Some(3));
        assert_eq!(config.logout_request_timeout, None);
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

    // --- from_settings() tests ---

    #[test]
    fn test_from_settings_all_defaults() {
        let settings = create_test_settings(vec![]);
        let config = LogoutConfig::from_settings(&settings).unwrap();
        assert_eq!(config, LogoutConfig::default());
    }

    #[test]
    fn test_from_settings_valid_error_strategy_best_effort() {
        let settings = create_test_settings(vec![(
            "logout_error_strategy",
            Setting::String("best_effort".to_string()),
        )]);
        let config = LogoutConfig::from_settings(&settings).unwrap();
        assert_eq!(config.error_strategy, ErrorStrategy::BestEffort);
    }

    #[test]
    fn test_from_settings_valid_error_strategy_strict() {
        let settings = create_test_settings(vec![(
            "logout_error_strategy",
            Setting::String("strict".to_string()),
        )]);
        let config = LogoutConfig::from_settings(&settings).unwrap();
        assert_eq!(config.error_strategy, ErrorStrategy::Strict);
    }

    #[test]
    fn test_from_settings_valid_total_timeout() {
        let settings =
            create_test_settings(vec![("logout_total_timeout_seconds", Setting::Int(30))]);
        let config = LogoutConfig::from_settings(&settings).unwrap();
        assert_eq!(config.logout_total_timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_from_settings_valid_max_attempts() {
        let settings = create_test_settings(vec![("logout_max_attempts", Setting::Int(5))]);
        let config = LogoutConfig::from_settings(&settings).unwrap();
        assert_eq!(config.max_attempts, Some(5));
    }

    #[test]
    fn test_from_settings_valid_request_timeout() {
        let settings =
            create_test_settings(vec![("logout_request_timeout_seconds", Setting::Int(2))]);
        let config = LogoutConfig::from_settings(&settings).unwrap();
        assert_eq!(config.logout_request_timeout, Some(Duration::from_secs(2)));
    }

    #[test]
    fn test_from_settings_rejects_zero_total_timeout() {
        let settings =
            create_test_settings(vec![("logout_total_timeout_seconds", Setting::Int(0))]);
        assert!(LogoutConfig::from_settings(&settings).is_err());
    }

    #[test]
    fn test_from_settings_rejects_negative_total_timeout() {
        let settings =
            create_test_settings(vec![("logout_total_timeout_seconds", Setting::Int(-1))]);
        assert!(LogoutConfig::from_settings(&settings).is_err());
    }

    #[test]
    fn test_from_settings_rejects_zero_max_attempts() {
        let settings = create_test_settings(vec![("logout_max_attempts", Setting::Int(0))]);
        assert!(LogoutConfig::from_settings(&settings).is_err());
    }

    #[test]
    fn test_from_settings_rejects_oversize_max_attempts() {
        let oversize = u32::MAX as i64 + 1;
        let settings = create_test_settings(vec![("logout_max_attempts", Setting::Int(oversize))]);
        assert!(LogoutConfig::from_settings(&settings).is_err());
    }

    #[test]
    fn test_from_settings_rejects_zero_request_timeout() {
        let settings =
            create_test_settings(vec![("logout_request_timeout_seconds", Setting::Int(0))]);
        assert!(LogoutConfig::from_settings(&settings).is_err());
    }

    #[test]
    fn test_from_settings_rejects_invalid_error_strategy() {
        let settings = create_test_settings(vec![(
            "logout_error_strategy",
            Setting::String("bad".to_string()),
        )]);
        assert!(LogoutConfig::from_settings(&settings).is_err());
    }

    // --- merge_with_request() tests ---

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
            .merge_with_request(&CloseParamsOverrides {
                server_session_keep_alive: Some(false),
                enable_server_session_keep_alive_auto_detection: Some(true),
                error_strategy: Some(ErrorStrategy::BestEffort),
                logout_total_timeout_seconds: Some(10),
                max_attempts: Some(1), // 1 total attempt = no retries
                logout_request_timeout_seconds: Some(4),
            })
            .unwrap();

        assert_eq!(merged.server_session_keep_alive, Some(false));
        assert_eq!(
            merged.enable_server_session_keep_alive_auto_detection,
            Some(true)
        );
        assert_eq!(merged.error_strategy, ErrorStrategy::BestEffort);
        assert_eq!(merged.logout_total_timeout, Duration::from_secs(10));
        assert_eq!(merged.max_attempts, Some(1));
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
            .merge_with_request(&CloseParamsOverrides::default())
            .unwrap();

        assert_eq!(merged, base);
    }

    #[test]
    fn test_merge_rejects_non_positive_max_attempts() {
        let base = LogoutConfig::default();
        assert!(
            base.merge_with_request(&CloseParamsOverrides {
                max_attempts: Some(0),
                ..Default::default()
            })
            .is_err()
        );
        assert!(
            base.merge_with_request(&CloseParamsOverrides {
                max_attempts: Some(-1),
                ..Default::default()
            })
            .is_err()
        );
    }

    #[test]
    fn test_merge_rejects_non_positive_total_timeout() {
        let base = LogoutConfig::default();
        assert!(
            base.merge_with_request(&CloseParamsOverrides {
                logout_total_timeout_seconds: Some(0),
                ..Default::default()
            })
            .is_err()
        );
        assert!(
            base.merge_with_request(&CloseParamsOverrides {
                logout_total_timeout_seconds: Some(-5),
                ..Default::default()
            })
            .is_err()
        );
    }

    #[test]
    fn test_merge_rejects_non_positive_request_timeout() {
        let base = LogoutConfig::default();
        assert!(
            base.merge_with_request(&CloseParamsOverrides {
                logout_request_timeout_seconds: Some(0),
                ..Default::default()
            })
            .is_err()
        );
        assert!(
            base.merge_with_request(&CloseParamsOverrides {
                logout_request_timeout_seconds: Some(-1),
                ..Default::default()
            })
            .is_err()
        );
    }
}
