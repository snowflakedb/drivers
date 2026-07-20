//! Logout logic for session cleanup
//!
//! This module consolidates all logout-related functionality:
//! - Decision logic for whether to send logout
//! - Data extraction and validation
//! - HTTP logout request execution with token refresh
//!
//! The module exports a clean interface to connection.rs:
//! - `prepare_logout_from_conn()`: Synchronous; called while holding the connection lock
//! - `execute_logout_with_strategy()`: Async; called after the lock is released
//! - `send_logout_request()`: Sends the HTTP logout request

use super::async_query_registry::AsyncQueryRegistry;
use super::connection::{Connection, RefreshContext};
use super::error::*;
use crate::config::logout::LogoutConfig;
use crate::config::rest_parameters::ClientInfo;
use crate::config::retry::RetryPolicy;
use crate::rest::snowflake::logout::logout_session;

/// Data extracted from a locked connection needed to perform HTTP logout.
pub(super) struct LogoutData {
    client: reqwest::Client,
    url: String,
    info: ClientInfo,
    retry_policy: RetryPolicy,
    refresh_ctx: RefreshContext,
}

/// Decision returned by [`should_send_logout`].
///
/// Eliminates the `(bool, Option<String>)` tuple whose field order was ambiguous
/// and which permitted the meaningless state `(true, Some(reason))`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogoutDecision {
    /// Send the logout request.
    Send,
    /// Skip logout; `reason` describes why (for structured logging).
    Skip { reason: String },
}

/// Determine whether to send logout request based on configuration and async query state
///
/// Implements Phase 3 unified truth table (SNOW-2314152):
///
/// | server_session_keep_alive | enable_server_session_keep_alive_auto_detection | Auto-detect result | Logout? |
/// |---------------------------|----------------------|-------------------|---------|
/// | Some(true)                | any                  | not consulted     | No      |
/// | Some(false)               | any                  | not consulted     | Yes     |
/// | None                      | Some(false) / None   | not consulted     | Yes     |
/// | None                      | Some(true)           | has running       | No      |
/// | None                      | Some(true)           | no running        | Yes     |
///
/// # Arguments
///
/// * `config` - Logout configuration
/// * `registry` - Async query registry (may be None if not available)
///
/// # Returns
///
/// A [`LogoutDecision`] indicating whether to send logout or skip it with a reason.
pub fn should_send_logout(
    config: &LogoutConfig,
    registry: Option<&AsyncQueryRegistry>,
) -> LogoutDecision {
    // Check explicit server_session_keep_alive first
    match config.server_session_keep_alive {
        Some(true) => {
            // Explicit keep-alive: never logout
            tracing::info!("Skipping logout: server_session_keep_alive=true (explicit keep-alive)");
            return LogoutDecision::Skip {
                reason: "server_session_keep_alive=true".into(),
            };
        }
        Some(false) => {
            // Explicit kill: always logout (Phase 3 semantics - SNOW-2314152)
            tracing::info!("Sending logout: server_session_keep_alive=false (explicit kill)");
            return LogoutDecision::Send;
        }
        None => {
            // Delegate to auto-detection setting
        }
    }

    // server_session_keep_alive is None - check auto-detection setting
    match config.enable_server_session_keep_alive_auto_detection {
        Some(true) => {
            // Auto-detection enabled - check registry
            if let Some(reg) = registry {
                match reg.has_running_queries() {
                    Ok(true) => {
                        tracing::info!(
                            "Skipping logout: auto-detection found running async queries"
                        );
                        LogoutDecision::Skip {
                            reason: "auto_detection_found_running_queries".into(),
                        }
                    }
                    Ok(false) => {
                        tracing::info!(
                            "Sending logout: auto-detection found no running async queries"
                        );
                        LogoutDecision::Send
                    }
                    Err(e) => {
                        // Registry lock error - default to logout
                        tracing::error!(
                            error = %e,
                            "Failed to check running queries, defaulting to logout"
                        );
                        LogoutDecision::Send
                    }
                }
            } else {
                // Registry not available - default to logout
                tracing::warn!(
                    "Auto-detection enabled but registry not available, defaulting to logout"
                );
                LogoutDecision::Send
            }
        }
        Some(false) | None => {
            // Auto-detection disabled or not set - default to logout (Phase 3 - SNOW-2314152)
            tracing::info!(
                "Sending logout: auto-detection disabled (enable_server_session_keep_alive_auto_detection={:?})",
                config.enable_server_session_keep_alive_auto_detection
            );
            LogoutDecision::Send
        }
    }
}

/// Validate logout configuration values.
///
/// Checks for invalid timeout configurations that would cause immediate failure.
fn validate_config(config: &LogoutConfig) -> Result<(), ApiError> {
    // Zero timeout means immediate failure - reject at configuration time
    if let Some(timeout) = config.logout_request_timeout
        && timeout.is_zero()
    {
        return InvalidArgumentSnafu {
            argument:
                "logout_request_timeout: 0s. Zero timeout means immediate failure. Must be positive."
                    .to_string(),
        }.fail();
    }
    Ok(())
}

/// Prepare logout while the caller already holds the connection lock.
///
/// Called with `&conn` already extracted from the mutex so the caller can
/// consolidate all pre-network synchronous work into a single lock acquisition.
///
/// Returns:
/// - `Ok(Some(LogoutData))`: Logout should be sent with this data
/// - `Ok(None)`: Logout should be skipped (explicit config or missing fields)
/// - `Err(ApiError)`: Validation failed or preparation error
pub(super) fn prepare_logout_from_conn(
    conn: &Connection,
    config: &LogoutConfig,
) -> Result<Option<LogoutData>, ApiError> {
    // Validate config first (no lock needed — operates on config only)
    validate_config(config)?;

    tracing::info!("Closing connection");

    // Check if logout should be sent based on configuration and state
    match should_send_logout(config, Some(&conn.async_query_registry)) {
        LogoutDecision::Skip { reason } => {
            tracing::info!(
                reason = reason.as_str(),
                "Skipping logout based on configuration or state"
            );
            return Ok(None);
        }
        LogoutDecision::Send => {}
    }

    // Logout should be sent - extract required data (all pure reads from conn)
    match (
        conn.http_client.clone(),
        conn.server_url.clone(),
        conn.client_info.clone(),
    ) {
        (Some(client), Some(url), Some(info)) => {
            let refresh_ctx = RefreshContext::new(conn)?;

            let retry_policy = RetryPolicy::logout(&conn.connection_seed, config);

            tracing::debug!(
                total_timeout_secs = config.logout_total_timeout.as_secs(),
                max_attempts = retry_policy.max_attempts,
                per_request_timeout_secs = retry_policy.per_request_timeout.map(|t| t.as_secs()),
                "Configured logout retry policy"
            );

            Ok(Some(LogoutData {
                client,
                url,
                info,
                retry_policy,
                refresh_ctx,
            }))
        }
        _ => {
            // Connection was never fully initialized - missing required fields
            // This is not an error condition according to error_strategy:
            // - BestEffort: Always skip silently (already the behavior)
            // - Strict: Skip silently (connection never logged in, no session to kill)
            tracing::debug!(
                "Connection missing required fields (http_client, server_url, or client_info), skipping logout"
            );
            Ok(None)
        }
    }
}

/// Send the HTTP logout request with automatic token refresh on 390112.
///
/// Uses `RefreshContext::execute_with_refresh` — the shared refresh-retry loop.
/// Calls `execute_with_refresh` directly (not `with_valid_session`) because logout
/// uses `RefreshContext::new()` (no `is_closed` check — logout runs after close).
pub(super) async fn send_logout_request(
    data: LogoutData,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<(), ApiError> {
    let mut ctx = data.refresh_ctx;

    let result = ctx
        .execute_with_refresh(|token| {
            let client = &data.client;
            let url = &data.url;
            let info = &data.info;
            let retry_policy = &data.retry_policy;
            let cancel = cancel.clone();
            async move { logout_session(client, url, &token, info, retry_policy, cancel).await }
        })
        .await;

    // Remap ApiError::Query (from RefreshContext) to ApiError::Logout, but
    // preserve cancellation classification: a cancelled logout must surface as
    // ApiError::Cancelled rather than being flattened into ApiError::Logout
    // (which maps to InternalError).
    result.map_err(|e| match e {
        e if e.is_cancelled() => CancelledSnafu.build(),
        ApiError::Query { source, .. } => LogoutSnafu {
            message: format!("{source}"),
        }
        .build(),
        other => other,
    })
}

/// Execute logout with error strategy handling.
///
/// This helper encapsulates the pattern of:
/// 1. Sending logout if data is available
/// 2. Logging success
/// 3. Applying error strategy to handle failures
///
/// This keeps connection_close clean and makes the logout execution flow testable.
pub(super) async fn execute_logout_with_strategy(
    logout_data: Option<LogoutData>,
    error_strategy: crate::config::logout::ErrorStrategy,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<(), ApiError> {
    let logout_result = match logout_data {
        Some(data) => {
            let result = send_logout_request(data, cancel).await;
            if result.is_ok() {
                tracing::info!("Logout completed successfully");
            }
            result
        }
        None => {
            // Logout skipped (explicit config or connection not initialized)
            // Skip reason already logged by prepare_logout
            Ok(())
        }
    };

    // Cancellation is a caller-initiated abort, not an unrecoverable logout
    // failure, so it must surface as CANCELLED even under BestEffort (which
    // would otherwise suppress it into Ok(())).
    if logout_result.as_ref().is_err_and(|e| e.is_cancelled()) {
        return logout_result;
    }

    error_strategy.handle_failed_logout(logout_result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_validate_config_accepts_none() {
        let config = LogoutConfig {
            logout_request_timeout: None,
            ..Default::default()
        };
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_config_accepts_positive_timeout() {
        let config = LogoutConfig {
            logout_request_timeout: Some(Duration::from_secs(10)),
            ..Default::default()
        };
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_config_rejects_zero_timeout() {
        let config = LogoutConfig {
            logout_request_timeout: Some(Duration::ZERO),
            ..Default::default()
        };
        let result = validate_config(&config);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ApiError::InvalidArgument { .. }));
        assert!(err.to_string().contains("Zero timeout"));
    }

    #[test]
    fn test_explicit_keep_alive_true() {
        // Given server_session_keep_alive = Some(true)
        let config = LogoutConfig {
            server_session_keep_alive: Some(true),
            enable_server_session_keep_alive_auto_detection: None,
            ..Default::default()
        };
        let registry = AsyncQueryRegistry::new();

        // When checking decision
        let decision = should_send_logout(&config, Some(&registry));

        // Then should NOT send logout
        assert!(
            matches!(decision, LogoutDecision::Skip { .. }),
            "Should skip logout when keep_alive=true"
        );
    }

    #[test]
    fn test_explicit_kill_false() {
        // Given server_session_keep_alive = Some(false)
        let config = LogoutConfig {
            server_session_keep_alive: Some(false),
            enable_server_session_keep_alive_auto_detection: Some(true), // Should be ignored
            ..Default::default()
        };
        let registry = AsyncQueryRegistry::new();
        registry.register("query1".to_string()).unwrap(); // Should be ignored

        // When checking decision
        let decision = should_send_logout(&config, Some(&registry));

        // Then should send logout (Phase 3: false means force logout - SNOW-2314152)
        assert_eq!(
            decision,
            LogoutDecision::Send,
            "Should send logout when keep_alive=false"
        );
    }

    #[test]
    fn test_auto_detection_enabled_with_running_queries() {
        // Given server_session_keep_alive = None, enable_server_session_keep_alive_auto_detection = Some(true)
        let config = LogoutConfig {
            server_session_keep_alive: None,
            enable_server_session_keep_alive_auto_detection: Some(true),
            ..Default::default()
        };
        let registry = AsyncQueryRegistry::new();
        registry.register("query1".to_string()).unwrap();

        // When checking decision
        let decision = should_send_logout(&config, Some(&registry));

        // Then should NOT send logout (running queries detected)
        assert!(
            matches!(decision, LogoutDecision::Skip { .. }),
            "Should skip logout when async queries running"
        );
    }

    #[test]
    fn test_auto_detection_enabled_with_no_queries() {
        // Given server_session_keep_alive = None, enable_server_session_keep_alive_auto_detection = Some(true)
        let config = LogoutConfig {
            server_session_keep_alive: None,
            enable_server_session_keep_alive_auto_detection: Some(true),
            ..Default::default()
        };
        let registry = AsyncQueryRegistry::new();
        // No queries registered

        // When checking decision
        let decision = should_send_logout(&config, Some(&registry));

        // Then should send logout (no running queries)
        assert_eq!(
            decision,
            LogoutDecision::Send,
            "Should send logout when no async queries"
        );
    }

    #[test]
    fn test_auto_detection_disabled() {
        // Given enable_server_session_keep_alive_auto_detection = Some(false)
        let config = LogoutConfig {
            server_session_keep_alive: None,
            enable_server_session_keep_alive_auto_detection: Some(false),
            ..Default::default()
        };
        let registry = AsyncQueryRegistry::new();
        registry.register("query1".to_string()).unwrap(); // Should be ignored

        // When checking decision
        let decision = should_send_logout(&config, Some(&registry));

        // Then should send logout (auto-detection disabled)
        assert_eq!(
            decision,
            LogoutDecision::Send,
            "Should send logout when auto-detection disabled"
        );
    }

    #[test]
    fn test_default_config_unified() {
        // Given default config (Phase 3 (SNOW-2314152): both None)
        let config = LogoutConfig::default();
        let registry = AsyncQueryRegistry::new();
        registry.register("query1".to_string()).unwrap(); // Should be ignored

        // When checking decision
        let decision = should_send_logout(&config, Some(&registry));

        // Then should send logout (Phase 3 (SNOW-2314152) default: always logout)
        assert_eq!(
            decision,
            LogoutDecision::Send,
            "Phase 3 (SNOW-2314152) default should send logout"
        );
    }

    #[test]
    fn test_auto_detection_without_registry() {
        // Given auto-detection enabled but no registry provided
        let config = LogoutConfig {
            server_session_keep_alive: None,
            enable_server_session_keep_alive_auto_detection: Some(true),
            ..Default::default()
        };

        // When checking decision without registry
        let decision = should_send_logout(&config, None);

        // Then should send logout (fallback when registry unavailable)
        assert_eq!(
            decision,
            LogoutDecision::Send,
            "Should send logout when registry unavailable"
        );
    }
}
