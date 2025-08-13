pub mod rest_parameters;
pub mod settings;

#[derive(Debug)]
pub enum ConfigError {
    MissingParameter(String),
    InvalidArgument(String),
}
