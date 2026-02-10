//! Configuration for session logout behavior
//!
//! This module provides the Strategy pattern implementation for error handling
//! during logout operations.

use crate::rest::snowflake::logout::LogoutError;
use std::fmt::Debug;
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

    /// Timeout for logout HTTP request
    pub timeout: Duration,
}

impl Default for LogoutConfig {
    fn default() -> Self {
        Self {
            server_session_keep_alive: None,
            enable_auto_detection: None,
            error_strategy: ErrorStrategy::Strict,
            timeout: Duration::from_secs(5),
        }
    }
}

/// Session gone error code from Snowflake
pub const SESSION_GONE_ERROR_CODE: i32 = 390111;

// ============================================================================
// Strategy Pattern: Error Handling Strategy Trait
// ============================================================================

/// Trait defining the strategy for handling logout errors.
///
/// This follows the Strategy design pattern, allowing different error handling
/// behaviors to be swapped at runtime without changing the connection close logic.
pub trait ErrorHandlingStrategy: Debug + Send + Sync {
    /// Determine if an error should be ignored (not propagated to caller).
    ///
    /// # Arguments
    /// * `error` - The logout error that occurred
    ///
    /// # Returns
    /// * `true` if the error should be ignored and close() should return Ok
    /// * `false` if the error should be handled according to `should_raise_error`
    fn should_ignore_error(&self, error: &LogoutError) -> bool;

    /// Determine if an error should be raised to the caller.
    ///
    /// This is called only if `should_ignore_error` returns false.
    ///
    /// # Arguments
    /// * `error` - The logout error that occurred
    ///
    /// # Returns
    /// * `true` if the error should be returned from close()
    /// * `false` if the error should be logged and suppressed
    fn should_raise_error(&self, error: &LogoutError) -> bool;

    /// Log the error appropriately based on the strategy.
    ///
    /// # Arguments
    /// * `error` - The logout error to log
    /// * `will_raise` - Whether the error will be raised to the caller
    fn log_error(&self, error: &LogoutError, will_raise: bool);

    /// Get a human-readable name for this strategy (for logging)
    fn name(&self) -> &'static str;
}

// ============================================================================
// Strategy Implementations
// ============================================================================

/// Strict error handling strategy.
///
/// - Only ignores SESSION_GONE (390111) - session already terminated
/// - All other errors are raised to the caller
/// - close() may fail and throw an error
#[derive(Debug, Clone, Copy, Default)]
pub struct StrictStrategy;

impl ErrorHandlingStrategy for StrictStrategy {
    fn should_ignore_error(&self, error: &LogoutError) -> bool {
        // Only ignore SESSION_GONE (390111) - session already terminated
        match error {
            LogoutError::LogoutFailed { code, .. } => *code == SESSION_GONE_ERROR_CODE,
            _ => false,
        }
    }

    fn should_raise_error(&self, _error: &LogoutError) -> bool {
        // Strict strategy always raises errors that aren't ignored
        true
    }

    fn log_error(&self, error: &LogoutError, will_raise: bool) {
        if will_raise {
            tracing::error!(?error, strategy = self.name(), "Logout failed");
        } else {
            tracing::info!(
                ?error,
                strategy = self.name(),
                "Logout error ignored (SESSION_GONE)"
            );
        }
    }

    fn name(&self) -> &'static str {
        "Strict"
    }
}

/// Best-effort error handling strategy.
///
/// - Never raises errors to the caller
/// - All errors are logged as WARN and suppressed
/// - close() always succeeds
#[derive(Debug, Clone, Copy, Default)]
pub struct BestEffortStrategy;

impl ErrorHandlingStrategy for BestEffortStrategy {
    fn should_ignore_error(&self, error: &LogoutError) -> bool {
        // Check for SESSION_GONE first (ignore silently)
        match error {
            LogoutError::LogoutFailed { code, .. } => *code == SESSION_GONE_ERROR_CODE,
            _ => false,
        }
    }

    fn should_raise_error(&self, _error: &LogoutError) -> bool {
        // Best-effort never raises errors
        false
    }

    fn log_error(&self, error: &LogoutError, _will_raise: bool) {
        // Best-effort always logs as WARN (not ERROR)
        tracing::warn!(
            ?error,
            strategy = self.name(),
            "Logout failed but suppressed"
        );
    }

    fn name(&self) -> &'static str {
        "BestEffort"
    }
}

// ============================================================================
// ErrorStrategy Enum (for configuration/serialization)
// ============================================================================

/// Strategy selector for error handling during logout.
///
/// This enum is used in configuration to select which strategy to use.
/// Use `to_handler()` to get the actual strategy implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ErrorStrategy {
    /// Strict strategy: Only ignore SESSION_GONE (390111), raise all other errors
    #[default]
    Strict,

    /// Best-effort strategy: Never throw from close()
    BestEffort,
}

impl ErrorStrategy {
    /// Get the strategy handler for this configuration.
    ///
    /// Returns a boxed trait object implementing the strategy pattern.
    pub fn to_handler(&self) -> Box<dyn ErrorHandlingStrategy> {
        match self {
            ErrorStrategy::Strict => Box::new(StrictStrategy),
            ErrorStrategy::BestEffort => Box::new(BestEffortStrategy),
        }
    }

    /// Check if an error should be ignored based on the strategy.
    ///
    /// This is a convenience method that delegates to the strategy handler.
    /// For more control, use `to_handler()` directly.
    #[deprecated(note = "Use to_handler().should_ignore_error() for Strategy pattern")]
    pub fn should_ignore_error(&self, error_code: Option<i32>) -> bool {
        match self {
            ErrorStrategy::Strict => {
                // Only ignore SESSION_GONE (390111)
                error_code == Some(SESSION_GONE_ERROR_CODE)
            }
            ErrorStrategy::BestEffort => {
                // Log all errors but never throw
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LogoutConfig::default();
        assert_eq!(config.server_session_keep_alive, None);
        assert_eq!(config.enable_auto_detection, None);
        assert_eq!(config.error_strategy, ErrorStrategy::Strict);
        assert_eq!(config.timeout, Duration::from_secs(5));
    }

    #[test]
    fn test_default_error_strategy_is_strict() {
        let strategy = ErrorStrategy::default();
        assert_eq!(strategy, ErrorStrategy::Strict);
    }

    // ========================================================================
    // Strategy Pattern Tests
    // ========================================================================

    use snafu::Location;

    fn make_session_gone_error() -> LogoutError {
        LogoutError::LogoutFailed {
            code: SESSION_GONE_ERROR_CODE,
            message: "Session gone".to_string(),
            location: Location::default(),
        }
    }

    fn make_generic_error() -> LogoutError {
        LogoutError::LogoutFailed {
            code: 400,
            message: "Bad request".to_string(),
            location: Location::default(),
        }
    }

    fn make_http_error() -> LogoutError {
        // Use Http variant which has a source - we need to mock it
        // For testing, we use LogoutFailed with a different code
        LogoutError::LogoutFailed {
            code: 503,
            message: "Service unavailable".to_string(),
            location: Location::default(),
        }
    }

    #[test]
    fn test_strict_strategy_ignores_session_gone() {
        let strategy = StrictStrategy;
        let error = make_session_gone_error();
        assert!(
            strategy.should_ignore_error(&error),
            "Strict should ignore SESSION_GONE"
        );
    }

    #[test]
    fn test_strict_strategy_raises_other_errors() {
        let strategy = StrictStrategy;

        let generic_error = make_generic_error();
        assert!(
            !strategy.should_ignore_error(&generic_error),
            "Strict should not ignore generic errors"
        );
        assert!(
            strategy.should_raise_error(&generic_error),
            "Strict should raise generic errors"
        );

        let http_error = make_http_error();
        assert!(
            !strategy.should_ignore_error(&http_error),
            "Strict should not ignore HTTP errors"
        );
        assert!(
            strategy.should_raise_error(&http_error),
            "Strict should raise HTTP errors"
        );
    }

    #[test]
    fn test_strict_strategy_name() {
        let strategy = StrictStrategy;
        assert_eq!(strategy.name(), "Strict");
    }

    #[test]
    fn test_best_effort_strategy_ignores_session_gone() {
        let strategy = BestEffortStrategy;
        let error = make_session_gone_error();
        assert!(
            strategy.should_ignore_error(&error),
            "BestEffort should ignore SESSION_GONE"
        );
    }

    #[test]
    fn test_best_effort_strategy_never_raises_errors() {
        let strategy = BestEffortStrategy;

        let generic_error = make_generic_error();
        assert!(
            !strategy.should_raise_error(&generic_error),
            "BestEffort should never raise generic errors"
        );

        let http_error = make_http_error();
        assert!(
            !strategy.should_raise_error(&http_error),
            "BestEffort should never raise HTTP errors"
        );
    }

    #[test]
    fn test_best_effort_strategy_name() {
        let strategy = BestEffortStrategy;
        assert_eq!(strategy.name(), "BestEffort");
    }

    #[test]
    fn test_error_strategy_to_handler_returns_correct_type() {
        let strict_handler = ErrorStrategy::Strict.to_handler();
        assert_eq!(strict_handler.name(), "Strict");

        let best_effort_handler = ErrorStrategy::BestEffort.to_handler();
        assert_eq!(best_effort_handler.name(), "BestEffort");
    }

    #[test]
    fn test_handler_can_be_used_polymorphically() {
        // Test that handlers can be used via trait object
        let handlers: Vec<Box<dyn ErrorHandlingStrategy>> = vec![
            ErrorStrategy::Strict.to_handler(),
            ErrorStrategy::BestEffort.to_handler(),
        ];

        let session_gone = make_session_gone_error();
        let generic_error = make_generic_error();

        // Both should ignore SESSION_GONE
        for handler in &handlers {
            assert!(
                handler.should_ignore_error(&session_gone),
                "{} should ignore SESSION_GONE",
                handler.name()
            );
        }

        // Strict should raise generic, BestEffort should not
        assert!(handlers[0].should_raise_error(&generic_error));
        assert!(!handlers[1].should_raise_error(&generic_error));
    }

    // ========================================================================
    // Backward compatibility tests (deprecated method)
    // ========================================================================

    #[test]
    #[allow(deprecated)]
    fn test_deprecated_should_ignore_error_strict() {
        let strategy = ErrorStrategy::Strict;
        assert!(
            strategy.should_ignore_error(Some(390111)),
            "Should ignore SESSION_GONE"
        );
        assert!(
            !strategy.should_ignore_error(Some(390112)),
            "Should not ignore SESSION_TOKEN_EXPIRED"
        );
        assert!(
            !strategy.should_ignore_error(Some(400)),
            "Should not ignore 400"
        );
        assert!(
            !strategy.should_ignore_error(None),
            "Should not ignore unknown errors"
        );
    }

    #[test]
    #[allow(deprecated)]
    fn test_deprecated_should_ignore_error_best_effort() {
        let strategy = ErrorStrategy::BestEffort;
        assert!(
            strategy.should_ignore_error(Some(390111)),
            "Should ignore SESSION_GONE"
        );
        assert!(
            strategy.should_ignore_error(Some(390112)),
            "Should ignore SESSION_TOKEN_EXPIRED"
        );
        assert!(strategy.should_ignore_error(Some(500)), "Should ignore 500");
        assert!(
            strategy.should_ignore_error(None),
            "Should ignore unknown errors"
        );
    }
}
