//! Front-end-agnostic Snowflake type "readers".
//!
//! A reader decodes one Arrow cell into a plain Rust/chrono value — the
//! *read* half of a conversion. The *write* half (turning that value into an
//! ODBC buffer, a napi value, a JNI value) and any front-end *policy* (e.g.
//! ODBC's SQL `0001..9999` calendar range) stay in the individual driver
//! crates, so `odbc`, `nodejs_bridge`, and `python_bridge` can share this one
//! decode step instead of each maintaining their own.
//!
//! DATE, BOOLEAN, TIME, TIMESTAMP_TZ, REAL, BINARY, and TEXT are here now.
//! NTZ/LTZ still use the Level-2 epoch helpers from this crate without their
//! own materializer types yet.
//!
//! A reader has two layers worth naming. The *materializer*
//! ([`ReadArrowType::read_arrow_type`]) produces a checked chrono value and is
//! what most front ends want. Below it sit pure integer *primitives* — the
//! calendar kernel in [`civil`] and the clock kernel in [`clock`] — that a
//! front end with a materialization-free hot path can call directly. Sharing
//! the primitive — not just the materializer — keeps the calendar/clock math in
//! one place across every driver.

mod binary;
mod boolean;
mod civil;
mod clock;
mod date;
mod error;
mod nullable;
mod real;
mod text;
mod time;
mod timestamp;
mod traits;

pub use binary::SnowflakeBinary;
pub use boolean::SnowflakeBoolean;
pub use civil::civil_from_unix_days;
pub use clock::split_time_raw;
pub use date::SnowflakeDate;
pub use error::{InvalidArrowValueSnafu, NullValueSnafu, ReadArrowError};
pub use nullable::Nullable;
pub use real::SnowflakeReal;
pub use text::SnowflakeText;
pub use time::SnowflakeTime;
pub use timestamp::{
    SnowflakeTimestampTz, TZ_OFFSET_BIAS_MINUTES, TZ_OFFSET_MAX_RAW, TzInstant,
    read_scaled_timestamp, read_struct_timestamp, split_scaled_epoch,
};
pub use traits::{ReadArrowType, SnowflakeType};
