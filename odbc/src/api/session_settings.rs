use std::collections::HashMap;

use sf_core::protobuf::generated::database_driver_v1::{
    ConfigSetting, ConnectionGetAllParametersRequest, ConnectionHandle,
};
use snafu::ResultExt;

use crate::api::OdbcResult;
use crate::api::error::OdbcRuntimeSnafu;
use crate::api::runtime::global;
use crate::api::utils::{config_setting_bool, config_setting_string, config_setting_u64};
use crate::conversion::timestamp::parse_tz_offset_format;
use crate::conversion::{NumericSettings, SF_DEFAULT_VARCHAR_MAX_LEN, TzOffsetFormatCache};

pub(crate) struct ServerSessionParam {
    pub name: &'static str,
    pub apply: fn(&mut NumericSettings, Option<&ConfigSetting>),
}

pub(crate) static SERVER_SESSION_PARAMS: &[ServerSessionParam] = &[
    ServerSessionParam {
        name: "ODBC_TREAT_DECIMAL_AS_INT",
        apply: apply_treat_decimal_as_int,
    },
    ServerSessionParam {
        name: "ODBC_TREAT_BIG_NUMBER_AS_STRING",
        apply: apply_treat_big_number_as_string,
    },
    ServerSessionParam {
        name: "ODBC_USE_CUSTOM_SQL_DATA_TYPES",
        apply: apply_use_custom_sql_data_types,
    },
    ServerSessionParam {
        name: "VARCHAR_AND_BINARY_MAX_SIZE_IN_RESULT",
        apply: apply_max_varchar_size,
    },
    ServerSessionParam {
        name: "TIMESTAMP_TZ_OUTPUT_FORMAT",
        apply: apply_timestamp_tz_output_format,
    },
];

fn apply_treat_decimal_as_int(settings: &mut NumericSettings, value: Option<&ConfigSetting>) {
    settings.treat_decimal_as_int = value.map(config_setting_bool).unwrap_or(false);
}

fn apply_treat_big_number_as_string(settings: &mut NumericSettings, value: Option<&ConfigSetting>) {
    settings.treat_big_number_as_string = value.map(config_setting_bool).unwrap_or(false);
}

fn apply_use_custom_sql_data_types(settings: &mut NumericSettings, value: Option<&ConfigSetting>) {
    settings.use_custom_sql_data_types = value.map(config_setting_bool).unwrap_or(false);
}

fn apply_max_varchar_size(settings: &mut NumericSettings, value: Option<&ConfigSetting>) {
    settings.max_varchar_size = value
        .and_then(config_setting_u64)
        .unwrap_or(SF_DEFAULT_VARCHAR_MAX_LEN);
}

fn apply_timestamp_tz_output_format(settings: &mut NumericSettings, value: Option<&ConfigSetting>) {
    let parsed = match value.and_then(config_setting_string).as_deref() {
        Some(v) if !v.is_empty() => parse_tz_offset_format(v),
        _ => None,
    };
    settings.tz_offset_format_cache = TzOffsetFormatCache::Loaded(parsed);
}

pub(crate) fn apply_server_params(
    settings: &mut NumericSettings,
    params: &HashMap<String, ConfigSetting>,
) {
    for p in SERVER_SESSION_PARAMS {
        (p.apply)(settings, params.get(p.name));
    }
}

fn load_server_session_settings(
    conn_handle: &ConnectionHandle,
    base: &NumericSettings,
) -> OdbcResult<NumericSettings> {
    let typed_parameters = global().context(OdbcRuntimeSnafu)?.block_on(async |c| {
        c.connection_get_all_parameters(ConnectionGetAllParametersRequest {
            conn_handle: Some(*conn_handle),
        })
        .await
        .map(|resp| resp.typed_parameters)
        .map_err(crate::api::OdbcError::from)
    })?;
    let params: HashMap<String, ConfigSetting> = typed_parameters
        .into_iter()
        .map(|(k, v)| (k.to_uppercase(), v))
        .collect();
    let mut settings = *base;
    apply_server_params(&mut settings, &params);
    Ok(settings)
}

pub(crate) fn refresh_server_session_settings(
    conn_handle: &ConnectionHandle,
    settings: &mut NumericSettings,
) {
    match load_server_session_settings(conn_handle, settings) {
        Ok(updated) if updated == *settings => {}
        Ok(updated) => {
            tracing::info!(
                treat_decimal_as_int = updated.treat_decimal_as_int,
                treat_big_number_as_string = updated.treat_big_number_as_string,
                use_custom_sql_data_types = updated.use_custom_sql_data_types,
                max_varchar_size = updated.max_varchar_size,
                tz_offset_format_cache = ?updated.tz_offset_format_cache,
                "server session settings changed"
            );
            *settings = updated;
        }
        Err(e) => {
            tracing::warn!(
                error = ?e,
                "failed to load server session settings; keeping cached state"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversion::timestamp::TzOffsetFormat;
    use sf_core::protobuf::generated::database_driver_v1::config_setting;

    fn bool_setting(v: bool) -> ConfigSetting {
        ConfigSetting {
            value: Some(config_setting::Value::BoolValue(v)),
        }
    }

    fn int_setting(v: i64) -> ConfigSetting {
        ConfigSetting {
            value: Some(config_setting::Value::IntValue(v)),
        }
    }

    fn string_setting(v: &str) -> ConfigSetting {
        ConfigSetting {
            value: Some(config_setting::Value::StringValue(v.to_string())),
        }
    }

    #[test]
    fn registry_names_are_unique_and_uppercase() {
        let mut seen = HashMap::new();
        for p in SERVER_SESSION_PARAMS {
            assert_eq!(p.name, p.name.to_uppercase());
            assert!(
                seen.insert(p.name, ()).is_none(),
                "duplicate registry name {}",
                p.name
            );
        }
        assert_eq!(seen.len(), SERVER_SESSION_PARAMS.len());
    }

    #[test]
    fn missing_keys_reset_to_defaults() {
        let mut settings = NumericSettings {
            treat_decimal_as_int: true,
            treat_big_number_as_string: true,
            use_custom_sql_data_types: true,
            max_varchar_size: 134_217_728,
            tz_offset_format_cache: TzOffsetFormatCache::Loaded(Some(TzOffsetFormat::Colon)),
            map_to_long_varchar: Some(4000),
            default_varchar_size: 4000,
            default_binary_size: 2000,
        };
        apply_server_params(&mut settings, &HashMap::new());
        assert!(!settings.treat_decimal_as_int);
        assert!(!settings.treat_big_number_as_string);
        assert!(!settings.use_custom_sql_data_types);
        assert_eq!(settings.max_varchar_size, SF_DEFAULT_VARCHAR_MAX_LEN);
        assert_eq!(
            settings.tz_offset_format_cache,
            TzOffsetFormatCache::Loaded(None)
        );
        assert_eq!(settings.map_to_long_varchar, Some(4000));
        assert_eq!(settings.default_varchar_size, 4000);
        assert_eq!(settings.default_binary_size, 2000);
    }

    #[test]
    fn bool_and_int_coercion_matches_config_setting_helpers() {
        let mut settings = NumericSettings::default();
        let params = HashMap::from([
            ("ODBC_TREAT_DECIMAL_AS_INT".to_string(), bool_setting(true)),
            (
                "ODBC_TREAT_BIG_NUMBER_AS_STRING".to_string(),
                ConfigSetting {
                    value: Some(config_setting::Value::StringValue("TRUE".to_string())),
                },
            ),
            (
                "ODBC_USE_CUSTOM_SQL_DATA_TYPES".to_string(),
                ConfigSetting {
                    value: Some(config_setting::Value::StringValue("1".to_string())),
                },
            ),
            (
                "VARCHAR_AND_BINARY_MAX_SIZE_IN_RESULT".to_string(),
                int_setting(134_217_728),
            ),
        ]);
        apply_server_params(&mut settings, &params);
        assert!(settings.treat_decimal_as_int);
        assert!(settings.treat_big_number_as_string);
        assert!(!settings.use_custom_sql_data_types);
        assert_eq!(settings.max_varchar_size, 134_217_728);
    }

    #[test]
    fn empty_or_missing_tz_format_loads_none() {
        let mut settings = NumericSettings {
            tz_offset_format_cache: TzOffsetFormatCache::Unloaded,
            ..NumericSettings::default()
        };
        apply_server_params(&mut settings, &HashMap::new());
        assert_eq!(
            settings.tz_offset_format_cache,
            TzOffsetFormatCache::Loaded(None)
        );

        let mut settings = NumericSettings::default();
        apply_server_params(
            &mut settings,
            &HashMap::from([("TIMESTAMP_TZ_OUTPUT_FORMAT".to_string(), string_setting(""))]),
        );
        assert_eq!(
            settings.tz_offset_format_cache,
            TzOffsetFormatCache::Loaded(None)
        );
    }

    #[test]
    fn recognised_tz_format_loads_token() {
        let mut settings = NumericSettings::default();
        apply_server_params(
            &mut settings,
            &HashMap::from([(
                "TIMESTAMP_TZ_OUTPUT_FORMAT".to_string(),
                string_setting("YYYY-MM-DD HH24:MI:SS.FF TZH:TZM"),
            )]),
        );
        assert_eq!(
            settings.tz_offset_format_cache,
            TzOffsetFormatCache::Loaded(Some(TzOffsetFormat::Colon))
        );
    }

    #[test]
    fn unrecognised_tz_format_loads_none() {
        let mut settings = NumericSettings {
            tz_offset_format_cache: TzOffsetFormatCache::Loaded(Some(TzOffsetFormat::Colon)),
            ..NumericSettings::default()
        };
        apply_server_params(
            &mut settings,
            &HashMap::from([(
                "TIMESTAMP_TZ_OUTPUT_FORMAT".to_string(),
                string_setting("YYYY-MM-DD HH24:MI:SS"),
            )]),
        );
        assert_eq!(
            settings.tz_offset_format_cache,
            TzOffsetFormatCache::Loaded(None)
        );
    }
}
