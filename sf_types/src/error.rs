use error_trace::ErrorTrace;
use snafu::{Location, Snafu};

/// Failure decoding a single cell out of an Arrow array.
///
/// [`ReadArrowError::NullValue`] is not really an error: it is how a reader
/// reports "this cell is NULL" to [`crate::Nullable`], which turns it into
/// `None`. A reader used without that wrapper (a column the server declared
/// NOT NULL) surfaces it to the caller instead.
#[derive(Snafu, Debug, ErrorTrace)]
#[snafu(visibility(pub))]
pub enum ReadArrowError {
    #[snafu(display("Value is null"))]
    NullValue {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Invalid Arrow value: {reason}"))]
    InvalidArrowValue {
        reason: String,
        #[snafu(implicit)]
        location: Location,
    },
}
