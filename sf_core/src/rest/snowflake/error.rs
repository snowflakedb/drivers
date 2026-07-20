use crate::http::retry::HttpError;
use crate::logging::url_for_log;
use reqwest::StatusCode;
use snafu::{Location, Snafu};
use std::panic::Location as StdLocation;
use std::time::Duration;
use url::ParseError;

#[derive(Snafu, Debug, error_trace::ErrorTrace)]
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
    /// Error 612 from async polling - triggers automatic retry with sync mode
    /// only on first poll. If we've made progress, don't retry.
    #[snafu(display("Async poll returned error 612 (result not found)"))]
    AsyncPollResultNotFound {
        /// True if this was the first poll attempt (safe to retry with sync)
        is_first_poll: bool,
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
    #[snafu(display("Failed to parse getResultUrl ({url_safe})", url_safe = url_for_log(url)))]
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

impl SfError {
    pub fn is_cancelled(&self) -> bool {
        matches!(self, SfError::Cancelled { .. })
    }
}

// Intentionally no From<reqwest::Error> to force explicit location on construction

/// Capture the caller's source location for use in snafu error construction.
///
/// Must be called inside a `#[track_caller]` function so the captured location
/// is the call site in user code, not an internal library frame.
#[track_caller]
pub(crate) fn current_location() -> Location {
    let caller = StdLocation::caller();
    Location::new(caller.file(), caller.line(), caller.column())
}

/// Map an HTTP retry error to the corresponding [`SfError`] variant.
///
/// Use as `.map_err(map_http_error)`. The `#[track_caller]` attribute ensures
/// the location embedded in the error points to the `.map_err()` call site.
#[track_caller]
pub(crate) fn map_http_error(err: HttpError) -> SfError {
    let location = current_location();
    match err {
        HttpError::Cancelled { .. } => SfError::Cancelled { location },
        HttpError::Transport { source, .. } => SfError::Transport { source, location },
        HttpError::DeadlineExceeded {
            configured,
            elapsed,
            ..
        } => SfError::DeadlineExceeded {
            configured,
            elapsed,
            location,
        },
        HttpError::MaxAttempts {
            attempts,
            last_status,
            ..
        } => SfError::RetryAttemptsExhausted {
            attempts,
            last_status,
            location,
        },
        HttpError::RetryAfterExceeded {
            retry_after,
            remaining,
            ..
        } => SfError::RetryBudgetExceeded {
            retry_after,
            remaining,
            location,
        },
        // Not produced by the Snowflake REST path (only the size-capped CRL
        // fetch emits it), but the match must stay exhaustive.
        HttpError::ResponseTooLarge { .. } => SfError::HttpStatus {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            location,
        },
    }
}
