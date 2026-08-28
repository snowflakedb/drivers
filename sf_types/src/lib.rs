//! Front-end-agnostic Snowflake type "readers".
//!
//! A reader decodes one Arrow cell into a plain Rust/chrono value — the
//! *read* half of a conversion. The *write* half (turning that value into an
//! ODBC buffer, a napi value, a JNI value) and any front-end *policy* (e.g.
//! ODBC's SQL `0001..9999` calendar range) stay in the individual driver
//! crates, so `odbc`, `nodejs_bridge`, and `python_bridge` can share this one
//! decode step instead of each maintaining their own.
//!
//! DATE is the first type moved here; more will follow the same shape.

mod date;
mod error;
mod nullable;
mod traits;

pub use date::SnowflakeDate;
pub use error::{InvalidArrowValueSnafu, NullValueSnafu, ReadArrowError};
pub use nullable::Nullable;
pub use traits::{ReadArrowType, SnowflakeType};
