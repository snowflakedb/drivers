pub mod rest_parameters;
pub mod settings;

use snafu::{Location, Snafu};

#[derive(Debug, Snafu)]
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
}
