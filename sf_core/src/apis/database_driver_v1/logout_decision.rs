//! Logout decision logic
//!
//! Implements Phase 3 truth table for determining whether to send logout request.
//! See SNOW-2314152 for Phase 2/3 migration plan and semantics.

use super::async_query_registry::AsyncQueryRegistry;
use crate::config::logout::LogoutConfig;

/// Determine whether to send logout request based on configuration and async query state
///
/// Implements Phase 3 unified truth table (SNOW-2314152):
///
/// | server_session_keep_alive | enable_auto_detection | Auto-detect result | Logout? |
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
/// * `(send_logout, skip_reason)` - Whether to send logout and optional reason if skipped
pub fn should_send_logout(
    config: &LogoutConfig,
    registry: Option<&AsyncQueryRegistry>,
) -> (bool, Option<String>) {
    // Check explicit server_session_keep_alive first
    match config.server_session_keep_alive {
        Some(true) => {
            // Explicit keep-alive: never logout
            tracing::info!("Skipping logout: server_session_keep_alive=true (explicit keep-alive)");
            return (false, Some("server_session_keep_alive=true".to_string()));
        }
        Some(false) => {
            // Explicit kill: always logout (Phase 3 semantics - SNOW-2314152)
            tracing::info!("Sending logout: server_session_keep_alive=false (explicit kill)");
            return (true, None);
        }
        None => {
            // Delegate to auto-detection setting
        }
    }

    // server_session_keep_alive is None - check auto-detection setting
    match config.enable_auto_detection {
        Some(true) => {
            // Auto-detection enabled - check registry
            if let Some(reg) = registry {
                if reg.has_running_queries() {
                    tracing::info!("Skipping logout: auto-detection found running async queries");
                    (
                        false,
                        Some("auto_detection_found_running_queries".to_string()),
                    )
                } else {
                    tracing::info!("Sending logout: auto-detection found no running async queries");
                    (true, None)
                }
            } else {
                // Registry not available - default to logout
                tracing::warn!(
                    "Auto-detection enabled but registry not available, defaulting to logout"
                );
                (true, None)
            }
        }
        Some(false) | None => {
            // Auto-detection disabled or not set - default to logout (Phase 3 - SNOW-2314152)
            tracing::info!(
                "Sending logout: auto-detection disabled (enable_auto_detection={:?})",
                config.enable_auto_detection
            );
            (true, None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explicit_keep_alive_true() {
        // Given server_session_keep_alive = Some(true)
        let config = LogoutConfig {
            server_session_keep_alive: Some(true),
            enable_auto_detection: None,
            ..Default::default()
        };
        let registry = AsyncQueryRegistry::new();

        // When checking decision
        let (send, reason) = should_send_logout(&config, Some(&registry));

        // Then should NOT send logout
        assert!(!send, "Should not send logout when keep_alive=true");
        assert!(reason.is_some(), "Should have skip reason");
    }

    #[test]
    fn test_explicit_kill_false() {
        // Given server_session_keep_alive = Some(false)
        let config = LogoutConfig {
            server_session_keep_alive: Some(false),
            enable_auto_detection: Some(true), // Should be ignored
            ..Default::default()
        };
        let registry = AsyncQueryRegistry::new();
        registry.register("query1".to_string()); // Should be ignored

        // When checking decision
        let (send, reason) = should_send_logout(&config, Some(&registry));

        // Then should send logout (Phase 3: false means force logout - SNOW-2314152)
        assert!(send, "Should send logout when keep_alive=false");
        assert!(reason.is_none(), "Should not have skip reason");
    }

    #[test]
    fn test_auto_detection_enabled_with_running_queries() {
        // Given server_session_keep_alive = None, enable_auto_detection = Some(true)
        let config = LogoutConfig {
            server_session_keep_alive: None,
            enable_auto_detection: Some(true),
            ..Default::default()
        };
        let registry = AsyncQueryRegistry::new();
        registry.register("query1".to_string());

        // When checking decision
        let (send, reason) = should_send_logout(&config, Some(&registry));

        // Then should NOT send logout (running queries detected)
        assert!(!send, "Should not send logout when async queries running");
        assert!(reason.is_some(), "Should have skip reason");
    }

    #[test]
    fn test_auto_detection_enabled_with_no_queries() {
        // Given server_session_keep_alive = None, enable_auto_detection = Some(true)
        let config = LogoutConfig {
            server_session_keep_alive: None,
            enable_auto_detection: Some(true),
            ..Default::default()
        };
        let registry = AsyncQueryRegistry::new();
        // No queries registered

        // When checking decision
        let (send, reason) = should_send_logout(&config, Some(&registry));

        // Then should send logout (no running queries)
        assert!(send, "Should send logout when no async queries");
        assert!(reason.is_none(), "Should not have skip reason");
    }

    #[test]
    fn test_auto_detection_disabled() {
        // Given enable_auto_detection = Some(false)
        let config = LogoutConfig {
            server_session_keep_alive: None,
            enable_auto_detection: Some(false),
            ..Default::default()
        };
        let registry = AsyncQueryRegistry::new();
        registry.register("query1".to_string()); // Should be ignored

        // When checking decision
        let (send, reason) = should_send_logout(&config, Some(&registry));

        // Then should send logout (auto-detection disabled)
        assert!(send, "Should send logout when auto-detection disabled");
        assert!(reason.is_none(), "Should not have skip reason");
    }

    #[test]
    fn test_default_config_phase3() {
        // Given default config (Phase 3: both None - SNOW-2314152)
        let config = LogoutConfig::default();
        let registry = AsyncQueryRegistry::new();
        registry.register("query1".to_string()); // Should be ignored

        // When checking decision
        let (send, reason) = should_send_logout(&config, Some(&registry));

        // Then should send logout (Phase 3 default: always logout - SNOW-2314152)
        assert!(send, "Phase 3 default should send logout");
        assert!(reason.is_none(), "Should not have skip reason");
    }

    #[test]
    fn test_auto_detection_without_registry() {
        // Given auto-detection enabled but no registry provided
        let config = LogoutConfig {
            server_session_keep_alive: None,
            enable_auto_detection: Some(true),
            ..Default::default()
        };

        // When checking decision without registry
        let (send, reason) = should_send_logout(&config, None);

        // Then should send logout (fallback when registry unavailable)
        assert!(send, "Should send logout when registry unavailable");
        assert!(reason.is_none(), "Should not have skip reason");
    }
}
