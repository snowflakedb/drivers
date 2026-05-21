//! Configuration for PUT/GET file-transfer behavior.
//!
//! `put_get_max_attempts` is the canonical UD-core knob for what JDBC calls
//! `SFSessionProperty.PUT_GET_MAX_RETRIES` and libsnowflakeclient calls
//! `SF_CON_PUT_MAXRETRIES` / `SF_CON_GET_MAXRETRIES`. UD core uses
//! attempts-semantics (1 = no retry) to match the internal `RetryPolicy`
//! field and avoid the off-by-one ambiguity in the JDBC name; wrappers are
//! responsible for translating retries-named knobs into attempts
//! (`attempts = retries + 1`) before forwarding the value to core.

use crate::config::param_registry::param_names;
use crate::config::settings::Settings;

use super::{ConfigError, InvalidParameterValueSnafu};

/// Default attempts cap when the user has not set `put_get_max_attempts`.
/// Matches the historical hardcoded UD-core S3 retry budget.
pub const DEFAULT_PUT_GET_MAX_ATTEMPTS: u32 = 6;

/// Resolved PUT/GET file-transfer configuration.
///
/// `max_attempts` bounds the per-file HTTP/transport retry loop inside the
/// cloud-storage SDK (today wired through `s3_retry_policy`). It does NOT
/// govern STS-token refresh (which has its own coalescing window) nor
/// Snowflake session-token refresh (handled at the query layer).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PutGetConfig {
    /// Maximum total attempts for a single PUT/GET file transfer.
    /// `None` means: keep the default (`DEFAULT_PUT_GET_MAX_ATTEMPTS`).
    pub max_attempts: Option<u32>,
}

impl PutGetConfig {
    /// Parse PUT/GET configuration from connection settings.
    pub fn from_settings(settings: &dyn Settings) -> Result<Self, ConfigError> {
        let mut config = Self::default();

        if let Some(v) = settings.get_int(param_names::PUT_GET_MAX_ATTEMPTS.as_str()) {
            if v <= 0 {
                return InvalidParameterValueSnafu {
                    parameter: param_names::PUT_GET_MAX_ATTEMPTS.as_str(),
                    value: v.to_string(),
                    explanation: "Must be positive (minimum 1 attempt required; 1 = no retry)",
                }
                .fail();
            }
            if v > u32::MAX as i64 {
                return InvalidParameterValueSnafu {
                    parameter: param_names::PUT_GET_MAX_ATTEMPTS.as_str(),
                    value: v.to_string(),
                    explanation: "Must not exceed 4294967295 (u32::MAX)",
                }
                .fail();
            }
            config.max_attempts = Some(v as u32);
        }

        Ok(config)
    }

    /// Returns the effective attempts cap, applying the default when the
    /// user has not provided a value. Consumers (e.g. `s3_retry_policy`)
    /// should call this rather than reading `max_attempts` directly so the
    /// default lives in one place.
    pub fn resolved_max_attempts(&self) -> u32 {
        self.max_attempts.unwrap_or(DEFAULT_PUT_GET_MAX_ATTEMPTS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::settings::Setting;
    use std::collections::HashMap;

    fn create_test_settings(options: Vec<(&str, Setting)>) -> HashMap<String, Setting> {
        options
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect()
    }

    #[test]
    fn default_leaves_max_attempts_unset() {
        let config = PutGetConfig::default();
        assert_eq!(config.max_attempts, None);
    }

    #[test]
    fn resolved_max_attempts_falls_back_to_default_when_unset() {
        let config = PutGetConfig::default();
        assert_eq!(config.resolved_max_attempts(), DEFAULT_PUT_GET_MAX_ATTEMPTS);
    }

    #[test]
    fn resolved_max_attempts_returns_user_value_when_set() {
        let config = PutGetConfig {
            max_attempts: Some(25),
        };
        assert_eq!(config.resolved_max_attempts(), 25);
    }

    #[test]
    fn from_settings_empty_returns_default() {
        let settings = create_test_settings(vec![]);
        let config = PutGetConfig::from_settings(&settings).unwrap();
        assert_eq!(config, PutGetConfig::default());
    }

    #[test]
    fn from_settings_accepts_positive_value() {
        let settings = create_test_settings(vec![(
            param_names::PUT_GET_MAX_ATTEMPTS.as_str(),
            Setting::Int(25),
        )]);
        let config = PutGetConfig::from_settings(&settings).unwrap();
        assert_eq!(config.max_attempts, Some(25));
    }

    #[test]
    fn from_settings_accepts_one_attempt() {
        // 1 attempt = no retries; canonical "no retry" value under the
        // attempts-semantics convention. Wrappers translating libsf-style
        // `*_MAXRETRIES=0` should send 1 here, not 0.
        let settings = create_test_settings(vec![(
            param_names::PUT_GET_MAX_ATTEMPTS.as_str(),
            Setting::Int(1),
        )]);
        let config = PutGetConfig::from_settings(&settings).unwrap();
        assert_eq!(config.max_attempts, Some(1));
    }

    #[test]
    fn from_settings_rejects_zero() {
        let settings = create_test_settings(vec![(
            param_names::PUT_GET_MAX_ATTEMPTS.as_str(),
            Setting::Int(0),
        )]);
        assert!(PutGetConfig::from_settings(&settings).is_err());
    }

    #[test]
    fn from_settings_rejects_negative() {
        let settings = create_test_settings(vec![(
            param_names::PUT_GET_MAX_ATTEMPTS.as_str(),
            Setting::Int(-1),
        )]);
        assert!(PutGetConfig::from_settings(&settings).is_err());
    }

    #[test]
    fn from_settings_rejects_oversize() {
        let oversize = u32::MAX as i64 + 1;
        let settings = create_test_settings(vec![(
            param_names::PUT_GET_MAX_ATTEMPTS.as_str(),
            Setting::Int(oversize),
        )]);
        assert!(PutGetConfig::from_settings(&settings).is_err());
    }
}
