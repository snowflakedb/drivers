use reqwest::StatusCode;
use snafu::{Location, Snafu};
use std::time::Duration;
use url::ParseError;

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
    #[snafu(display("Deadline exceeded after {elapsed:?} (budget {configured:?})"))]
    DeadlineExceeded {
        configured: Duration,
        elapsed: Duration,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "Retry attempts exhausted after {attempts} attempts; last status {last_status}"
    ))]
    RetryAttemptsExhausted {
        attempts: u32,
        last_status: StatusCode,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Retry-After {retry_after:?} exceeds remaining budget {remaining:?}"))]
    RetryBudgetExceeded {
        retry_after: Duration,
        remaining: Duration,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Async query response missing getResultUrl; cannot poll for completion"))]
    MissingResultUrl {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Async query did not report a queryId"))]
    MissingQueryId {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to parse getResultUrl: {url}"))]
    ResultUrlParse {
        url: String,
        source: ParseError,
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
