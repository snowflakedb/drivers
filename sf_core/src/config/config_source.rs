use std::collections::HashMap;

use snafu::{Location, Snafu};

use crate::config::{
    ConfigError, InvalidParameterValueSnafu, MissingParameterSnafu,
    param_registry::{ParamDef, ValueType},
    settings::Setting,
};

enum ConfigSettingMeta {
    EnvironmentVariable {
        name: String,
    },
    OdbcConnectionString {
        connection_string: String,
        col: Option<usize>,
        name: String,
        value: String,
    },
    TomlFile {
        path: String,
        section: String,
        /* We could include the line number and column number of the value in the TOML file */
        name: String,
        value: toml::Value,
    },
    PythonKwargs {
        name: String,
        value: ValueType,
    },
}

enum ConfigSourceMeta {
    EnvironmentVariables,
    OdbcConnectionStrings { connection_string: String },
    TomlFiles { path: String },
}

pub struct ConfigSetting {
    parameter: &'static ParamDef,
    value: Setting,
    meta: ConfigSettingMeta,
}

impl ConfigSetting {
    pub fn original_name(&self) -> &str {
        match &self.meta {
            ConfigSettingMeta::EnvironmentVariable { name } => &name,
            ConfigSettingMeta::OdbcConnectionString { name, .. } => &name,
            ConfigSettingMeta::TomlFile { name, .. } => &name,
            ConfigSettingMeta::PythonKwargs { name, .. } => &name,
        }
    }
    pub fn as_string(self) -> Result<String, Vec<ConfigError>> {
        match self.value {
            Setting::String(value) => Ok(value),
            _ => Err(vec![
                InvalidParameterValueSnafu {
                    parameter: self.original_name(),
                    value: "".to_string(),
                    explanation: "Expected a string value".to_string(),
                }
                .build(),
            ]),
        }
    }
}

/*
Config source, represents a single source of configuration.
It can be one of the following:
- environment variables
- ODBC connection strings
- TOML files

@get:
- returns a ConfigSetting if the parameter is found
  - it contains the parameter, value, and meta information
  - the meta information will be used for error reporting

@meta:
- returns the meta information for the source, allows us to log the sources of the configuration
*/

pub trait ConfigSource {
    fn get(&self, parameter: &'static ParamDef) -> Option<ConfigSetting>;
    fn get_required(
        &self,
        parameter: &'static ParamDef,
    ) -> Result<ConfigSetting, Vec<ConfigError>> {
        self.get(parameter).ok_or_else(|| {
            vec![
                MissingParameterSnafu {
                    parameter: parameter.canonical_name.to_string(),
                }
                .build(),
            ]
        })
    }
    fn meta(&self) -> Vec<ConfigSourceMeta>;
}

// Example implementations:

/*
OdbcConnectionString implements ConfigSource for ODBC connection strings.
- Note thanks to meta information we can easily log the source of the misconfiguration if we need to.
*/
struct OdbcConnectionString {
    connection_string: String,
    parsed_connection_string: HashMap<String, String>,
}

impl OdbcConnectionString {
    fn new(connection_string: String) -> Self {
        // Naive parsing of the connection string
        let mut parsed_connection_string = HashMap::new();
        let pairs = connection_string.split(';');
        for pair in pairs {
            let parts: Vec<&str> = pair.splitn(2, '=').collect();
            if parts.len() == 2 {
                parsed_connection_string.insert(parts[0].to_string(), parts[1].to_string());
            }
        }
        Self {
            connection_string,
            parsed_connection_string,
        }
    }
}

impl ConfigSource for OdbcConnectionString {
    fn get(&self, parameter: &'static ParamDef) -> Option<ConfigSetting> {
        if let Some(alias) = parameter.odbc_alias {
            self.parsed_connection_string
                .get(alias)
                .map(|value| ConfigSetting {
                    parameter: parameter,
                    value: Setting::String(value.to_string()),
                    meta: ConfigSettingMeta::OdbcConnectionString {
                        connection_string: self.connection_string.clone(),
                        col: None, /* Should be a location in the connection string */
                        name: parameter.canonical_name.to_string(),
                        value: value.clone(),
                    },
                })
        } else {
            let uppercase_name = parameter.canonical_name.to_uppercase();
            self.parsed_connection_string
                .get(&uppercase_name)
                .map(|value| ConfigSetting {
                    parameter: parameter,
                    value: Setting::String(value.to_string()),
                    meta: ConfigSettingMeta::OdbcConnectionString {
                        connection_string: self.connection_string.clone(),
                        col: None, /* Should be a location in the connection string */
                        name: uppercase_name,
                        value: value.clone(),
                    },
                })
        }
    }
    fn meta(&self) -> Vec<ConfigSourceMeta> {
        vec![ConfigSourceMeta::OdbcConnectionStrings {
            connection_string: self.connection_string.clone(),
        }]
    }
}

/*
MergedConfigSource, represents a merged configuration source.
It contains a list of sources and merges them together.
*/
struct MergedConfigSource {
    sources: Vec<Box<dyn ConfigSource>>,
}

impl MergedConfigSource {
    fn new(sources: Vec<Box<dyn ConfigSource>>) -> Self {
        Self { sources }
    }

    fn get(&self, parameter: &'static ParamDef) -> Option<ConfigSetting> {
        self.sources.iter().find_map(|source| source.get(parameter))
    }

    fn meta(&self) -> Vec<ConfigSourceMeta> {
        self.sources
            .iter()
            .flat_map(|source| source.meta())
            .collect()
    }
}

/*
FromConfigSource, allows us to read a ConfigSource and return a specific type.
*/
pub trait FromConfigSource: Sized {
    fn from_config_source(source: &Box<dyn ConfigSource>) -> Result<Self, Vec<ConfigError>>;
}
