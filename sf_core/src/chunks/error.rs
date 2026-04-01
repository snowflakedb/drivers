use arrow::error::ArrowError;
use snafu::{Location, Snafu};

#[derive(Snafu, Debug, error_trace::ErrorTrace)]
#[snafu(visibility(pub(crate)))]
pub enum ChunkError {
    #[snafu(display("Invalid header name for {key}"))]
    HeaderName {
        key: String,
        source: reqwest::header::InvalidHeaderName,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Invalid header value for {key}"))]
    HeaderValue {
        key: String,
        source: reqwest::header::InvalidHeaderValue,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to communicate with Snowflake to get chunk data"))]
    Communication {
        source: reqwest::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Snowflake responded with non-successful HTTP status"))]
    UnsuccessfulHttpStatusCode {
        status: reqwest::StatusCode,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to read chunk data"))]
    ChunkReading {
        source: ArrowError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to decode base64 data"))]
    Base64Decoding {
        source: base64::DecodeError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to decompress gzip chunk data"))]
    ChunkDecompression {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("No initial inline data and no remote chunks available to derive schema"))]
    InitialChunkMissing {
        #[snafu(implicit)]
        location: Location,
    },
}
