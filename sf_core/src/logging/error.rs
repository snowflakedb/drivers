use snafu::{Location, Snafu};

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum LogError {
    #[snafu(display("Failed to initialize logging: {message}"))]
    Init {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("I/O error in logging subsystem"))]
    Io {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to parse log configuration: {message}"))]
    ConfigParse {
        message: String,
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
}
