use crate::error::ReadArrowError;

/// A Snowflake logical type, plus the Rust value it decodes to.
///
/// `Representation` is deliberately a plain Rust/chrono type (`bool`,
/// `Cow<str>`, `i128`, `NaiveDate`, …) rather than anything belonging to a
/// particular driver front end, so the same decode step can feed ODBC
/// buffers, napi values, or JNI values. Each front end pairs this with its
/// own output trait — `WriteODBCType` in the `odbc` crate,
/// `WriteSqlValue` in `nodejs_bridge` — keyed on the same `Representation`.
///
/// Front-end-specific *policy* does not belong here. The SQL `0001..9999`
/// datetime range, for instance, is enforced by the `odbc` crate's own
/// `ValidateSqlValue` trait, because a JavaScript `Date` has no such limit.
pub trait SnowflakeType {
    type Representation<'a>: Sized;
}

/// Decodes one cell of an Arrow array of type `ArrowArrayType` into this
/// Snowflake type's [`SnowflakeType::Representation`].
///
/// A single Snowflake type can implement this for several Arrow array types,
/// because the physical encoding the server picks varies with scale and
/// precision — FIXED arrives as any of `Int8`/`Int16`/`Int32`/`Int64`/
/// `Decimal128`, TIMESTAMP as either a flat `Int64` or a `Struct`. Callers
/// downcast once per column and then stay on a statically dispatched path.
pub trait ReadArrowType<ArrowArrayType>: SnowflakeType {
    #[allow(clippy::wrong_self_convention)]
    fn read_arrow_type<'a>(
        &self,
        array: &'a ArrowArrayType,
        row_idx: usize,
    ) -> Result<Self::Representation<'a>, ReadArrowError>;
}
