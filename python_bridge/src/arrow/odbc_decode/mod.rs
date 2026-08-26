//! Decode-only Arrow cell readers copied from the ODBC conversion layer.
//!
//! Source: `odbc/src/conversion/{traits,error}.rs` (`SnowflakeType`,
//! `ReadArrowType`, `ReadArrowError`). This module does not depend on the
//! `odbc` crate. Shared extraction into a common crate is deferred.
//!
//! Datatype readers land with their first consumer. Python materialization
//! (null → `None`, exception mapping) stays in `converters/`.

mod error;
mod traits;

pub(crate) use error::DecodeError;
#[cfg_attr(not(test), expect(unused_imports))]
pub(crate) use error::NullValueSnafu;
pub(crate) use traits::{ReadArrowType, SnowflakeType};

#[cfg(test)]
pub(crate) use error::InvalidArrowValueSnafu;
