use reqwest::StatusCode;
use snafu::{Location, Snafu};

#[derive(Snafu, Debug)]
#[snafu(visibility(pub))]
pub enum SfError {
    #[snafu(display("Transport error communicating with Snowflake"))]
    Transport {
        source: reqwest::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("HTTP status error: {status}"))]
    HttpStatus {
        status: StatusCode,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Snowflake error {code}: {message}"))]
    SnowflakeBody {
        code: i32,
        message: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Session expired"))]
    SessionExpired {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Warehouse resuming or queued"))]
    WarehouseResuming {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Deadline exceeded"))]
    DeadlineExceeded {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Cancelled"))]
    Cancelled {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to parse response body"))]
    BodyParse {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
}

// Intentionally no From<reqwest::Error> to force explicit location on construction
