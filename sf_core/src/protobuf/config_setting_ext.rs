use super::generated::database_driver_v1::{ConfigSetting, config_setting};

impl From<&str> for ConfigSetting {
    fn from(s: &str) -> Self {
        ConfigSetting {
            value: Some(config_setting::Value::StringValue(s.to_string())),
        }
    }
}

impl From<String> for ConfigSetting {
    fn from(s: String) -> Self {
        ConfigSetting {
            value: Some(config_setting::Value::StringValue(s)),
        }
    }
}

impl From<i64> for ConfigSetting {
    fn from(v: i64) -> Self {
        ConfigSetting {
            value: Some(config_setting::Value::IntValue(v)),
        }
    }
}

impl From<f64> for ConfigSetting {
    fn from(v: f64) -> Self {
        ConfigSetting {
            value: Some(config_setting::Value::DoubleValue(v)),
        }
    }
}

impl From<bool> for ConfigSetting {
    fn from(v: bool) -> Self {
        ConfigSetting {
            value: Some(config_setting::Value::BoolValue(v)),
        }
    }
}

impl From<Vec<u8>> for ConfigSetting {
    fn from(v: Vec<u8>) -> Self {
        ConfigSetting {
            value: Some(config_setting::Value::BytesValue(v)),
        }
    }
}

/// Build a `(key, ConfigSetting)` pair for use with `SetOptions` RPCs.
pub fn config_option(key: &str, value: impl Into<ConfigSetting>) -> (String, ConfigSetting) {
    (key.to_string(), value.into())
}
