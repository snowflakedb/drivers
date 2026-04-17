//! Platform detection for `CLIENT_ENVIRONMENT.PLATFORM`.
//!
//! This is the minimal stub: the full detector suite (AWS Lambda, EC2, Azure,
//! GCP, etc.) lands in a follow-up PR. For now this module only honors the
//! `SNOWFLAKE_DISABLE_PLATFORM_DETECTION` kill-switch and otherwise returns
//! an empty list. `DETECTION_TIMEOUT` and `DetectionConfig` are exported so
//! the follow-up can flesh them out without breaking this module's public API.

use std::time::Duration;

pub const DETECTION_TIMEOUT: Duration = Duration::from_millis(200);

const DISABLE_ENV: &str = "SNOWFLAKE_DISABLE_PLATFORM_DETECTION";

#[derive(Debug, Default, Clone)]
pub struct DetectionConfig;

/// Run platform detection and return the list of detected platforms.
///
/// In this stub implementation:
/// - returns `vec!["disabled"]` when `SNOWFLAKE_DISABLE_PLATFORM_DETECTION` is
///   set to `"true"` (case-insensitive),
/// - returns an empty `Vec` otherwise.
pub async fn detect_platforms(_config: &DetectionConfig) -> Vec<String> {
    if std::env::var(DISABLE_ENV)
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        return vec!["disabled".to_string()];
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn returns_disabled_when_env_flag_true() {
        temp_env::async_with_vars([(DISABLE_ENV, Some("true"))], async {
            let cfg = DetectionConfig;
            assert_eq!(detect_platforms(&cfg).await, vec!["disabled".to_string()]);
        })
        .await;
    }

    #[tokio::test]
    async fn disabled_flag_matches_case_insensitive() {
        temp_env::async_with_vars([(DISABLE_ENV, Some("TRUE"))], async {
            let cfg = DetectionConfig;
            assert_eq!(detect_platforms(&cfg).await, vec!["disabled".to_string()]);
        })
        .await;
    }

    #[tokio::test]
    async fn returns_empty_when_env_flag_unset() {
        temp_env::async_with_vars([(DISABLE_ENV, None::<&str>)], async {
            let cfg = DetectionConfig;
            let platforms = detect_platforms(&cfg).await;
            assert!(platforms.is_empty(), "expected empty, got {platforms:?}");
        })
        .await;
    }

    #[tokio::test]
    async fn returns_empty_when_env_flag_false() {
        temp_env::async_with_vars([(DISABLE_ENV, Some("false"))], async {
            let cfg = DetectionConfig;
            let platforms = detect_platforms(&cfg).await;
            assert!(platforms.is_empty(), "expected empty, got {platforms:?}");
        })
        .await;
    }
}
