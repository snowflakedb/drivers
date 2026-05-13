//! Configuration for the client-driven session heartbeat.
//!
//! Users configure it via ConnectionSetOption* before ConnectionInit, using
//! the same names Python and the old driver expose. Parsed once at
//! `connection_init` time, alongside `LogoutConfig`.

use std::time::Duration;

use crate::config::settings::Settings;

use super::{ConfigError, InvalidParameterValueSnafu, param_names};

/// Parsed heartbeat configuration.
///
/// The frequency is a hint — `compute_heartbeat_interval` applies the
/// `[master_validity / 16, master_validity / 4]` clamp at spawn time.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HeartbeatConfig {
    pub frequency: Option<Duration>,
}

impl HeartbeatConfig {
    pub fn from_settings(settings: &dyn Settings) -> Result<Self, ConfigError> {
        let mut config = Self::default();

        if let Some(v) =
            settings.get_int(param_names::CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY.as_str())
        {
            if v <= 0 {
                return InvalidParameterValueSnafu {
                    parameter: param_names::CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY.as_str(),
                    value: v.to_string(),
                    explanation: "Must be greater than 0",
                }
                .fail();
            }
            config.frequency = Some(Duration::from_secs(v as u64));
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::settings::Setting;
    use std::collections::HashMap;

    fn settings(entries: Vec<(&str, Setting)>) -> HashMap<String, Setting> {
        entries
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect()
    }

    #[test]
    fn default_is_no_frequency() {
        let config = HeartbeatConfig::from_settings(&settings(vec![])).unwrap();
        assert!(config.frequency.is_none());
    }

    #[test]
    fn frequency_is_parsed_as_seconds() {
        let config = HeartbeatConfig::from_settings(&settings(vec![(
            "client_session_keep_alive_heartbeat_frequency",
            Setting::Int(300),
        )]))
        .unwrap();
        assert_eq!(config.frequency, Some(Duration::from_secs(300)));
    }

    #[test]
    fn zero_frequency_is_rejected() {
        let err = HeartbeatConfig::from_settings(&settings(vec![(
            "client_session_keep_alive_heartbeat_frequency",
            Setting::Int(0),
        )]))
        .unwrap_err();
        assert!(matches!(err, ConfigError::InvalidParameterValue { .. }));
    }

    #[test]
    fn negative_frequency_is_rejected() {
        let err = HeartbeatConfig::from_settings(&settings(vec![(
            "client_session_keep_alive_heartbeat_frequency",
            Setting::Int(-1),
        )]))
        .unwrap_err();
        assert!(matches!(err, ConfigError::InvalidParameterValue { .. }));
    }
}
