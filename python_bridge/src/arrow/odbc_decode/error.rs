//! Endpoint-neutral decode errors.
//!
//! Copied from `odbc/src/conversion/error.rs` (`ReadArrowError`). No ODBC
//! SQLSTATE, C types, or Python exception types live here.

use error_trace::ErrorTrace;
use snafu::{Location, Snafu};

/// Failure while reading one Arrow cell into a native Rust value.
#[derive(Snafu, Debug, ErrorTrace)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum DecodeError {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_value_displays_endpoint_neutral_message() {
        let err = NullValueSnafu.build();
        assert_eq!(err.to_string(), "Value is null");
        assert!(
            matches!(err, DecodeError::NullValue { .. }),
            "expected NullValue, got {err:?}"
        );
    }

    #[test]
    fn invalid_arrow_value_preserves_reason() {
        let err = InvalidArrowValueSnafu {
            reason: "fraction out of range",
        }
        .build();
        assert_eq!(
            err.to_string(),
            "Invalid Arrow value: fraction out of range"
        );
        match &err {
            DecodeError::InvalidArrowValue { reason, .. } => {
                assert_eq!(reason, "fraction out of range");
            }
            other => panic!("expected InvalidArrowValue, got {other:?}"),
        }
    }
}
