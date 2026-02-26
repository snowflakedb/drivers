//! Configuration for session logout behavior

use crate::apis::database_driver_v1::error::ApiError;
use std::time::Duration;

/// Configuration for logout behavior during connection close
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
            error_strategy: ErrorStrategy::Strict,
            logout_total_timeout: Duration::from_secs(5),
            max_retry_attempts: None,
            logout_request_timeout: None,
        }
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
}
