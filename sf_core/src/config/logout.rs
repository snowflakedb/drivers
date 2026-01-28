//! Configuration for session logout behavior

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

/// Strategy for handling errors during logout
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorStrategy {
    /// Strict strategy: Only ignore SESSION_GONE (390111), raise all other errors
    /// - Retry on transient errors (503, connection reset, etc.)
    /// - Attempt token renewal on session expiry
    /// - Surface reauth errors to caller
    /// - close() may fail and throw
    Strict,
    
    /// Best-effort strategy: Never throw from close()
    /// - Retry on transient errors (503, connection reset, etc.)
    /// - Attempt token renewal on session expiry
    /// - After all retries exhausted, log as WARN and suppress error
    /// - close() always succeeds
    BestEffort,
}

impl ErrorStrategy {
    /// Check if an error should be ignored based on the strategy
    pub fn should_ignore_error(&self, error_code: Option<i32>) -> bool {
        match self {
            ErrorStrategy::Strict => {
                // Only ignore SESSION_GONE (390111)
                error_code == Some(390111)
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
    fn test_strict_strategy_only_ignores_session_gone() {
        let strategy = ErrorStrategy::Strict;
        assert!(strategy.should_ignore_error(Some(390111)), "Should ignore SESSION_GONE");
        assert!(!strategy.should_ignore_error(Some(390112)), "Should not ignore SESSION_TOKEN_EXPIRED");
        assert!(!strategy.should_ignore_error(Some(400)), "Should not ignore 400");
        assert!(!strategy.should_ignore_error(None), "Should not ignore unknown errors");
    }

    #[test]
    fn test_best_effort_strategy_ignores_all() {
        let strategy = ErrorStrategy::BestEffort;
        assert!(strategy.should_ignore_error(Some(390111)), "Should ignore SESSION_GONE");
        assert!(strategy.should_ignore_error(Some(390112)), "Should ignore SESSION_TOKEN_EXPIRED");
        assert!(strategy.should_ignore_error(Some(500)), "Should ignore 500");
        assert!(strategy.should_ignore_error(None), "Should ignore unknown errors");
    }
}
