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
}
