pub mod config_manager;
pub mod connection_config;
pub mod param_registry;
pub mod param_store;
pub use param_registry::ParamKey;
pub use param_registry::ParamScope;
pub use param_registry::param_names;
pub use param_store::ParamStore;
pub mod path_resolver;
pub mod resolver;
pub mod rest_parameters;
pub mod retry;
pub mod settings;
pub mod toml_loader;

use error_trace::ErrorTrace;
use snafu::{Location, Snafu};

#[derive(Debug, Snafu, ErrorTrace)]
pub enum ConfigError {
    #[snafu(display("Missing required parameter: {parameter}"))]
    MissingParameter {
        parameter: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Invalid value '{value}' for parameter '{parameter}' - {explanation}"))]
    InvalidParameterValue {
        parameter: String,
        value: String,
        explanation: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Conflicting parameters: {explanation}"))]
    ConflictingParameters {
        explanation: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to read config file at {path}: {source}"))]
    ConfigFileRead {
        path: String,
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to parse TOML from {path}: {source}"))]
    TomlParse {
        path: String,
        #[snafu(source(from(toml::de::Error, Box::new)))]
        source: Box<toml::de::Error>,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Insecure file permissions on {path}: {reason}"))]
    InsecurePermissions {
        path: String,
        reason: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Connection '{name}' not found in config files"))]
    ConnectionNotFound {
        name: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "Could not determine platform config directory. Set SNOWFLAKE_HOME environment variable to specify the configuration directory."
    ))]
    ConfigDirNotFound {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "Configuration validation failed ({} issue(s)): {}",
        issues.len(),
        issues
            .first()
            .map(|i| format!("{}: {}", i.parameter, i.message))
            .unwrap_or_default()
    ))]
    ValidationFailed {
        issues: Vec<crate::config::connection_config::ValidationIssue>,
        #[snafu(implicit)]
        location: Location,
    },
}
