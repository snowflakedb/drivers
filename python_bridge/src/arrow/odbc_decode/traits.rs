//! Decode-only adaptation of ODBC Arrow read traits.
//!
//! Copied from `odbc/src/conversion/traits.rs` (`SnowflakeType`,
//! `ReadArrowType`). Bind-side traits (`WriteODBCType`, `ReadODBC`,
//! `WriteWire`), SQL type metadata, and PyO3 are intentionally omitted.

use super::error::DecodeError;

/// Snowflake logical type that materializes a native Rust representation.
pub(crate) trait SnowflakeType {
    type Representation<'a>: Sized;
}

/// Read one cell of `ArrowArrayType` into [`SnowflakeType::Representation`].
pub(crate) trait ReadArrowType<ArrowArrayType>: SnowflakeType {
    #[allow(clippy::wrong_self_convention)]
    fn read_arrow_type<'a>(
        &self,
        array: &'a ArrowArrayType,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, DecodeError>;
}
