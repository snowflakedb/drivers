//! Fallback Snowflake error-code → ANSI SQLSTATE lookup.
//!
//! The Snowflake server normally returns an explicit `sqlState` field on the
//! query-failure response (see `query_response::Data::sql_state`), and that
//! value is forwarded verbatim through `RestError::QueryFailed::sql_state`
//! and onto `DriverException::sql_state`.
//!
//! However, a few code paths receive only the numeric error code and never an
//! `sqlState` string — for example:
//!
//! - The async polling path constructs `RestError::QueryFailed { sql_state:
//!   None, .. }` from `SfError::SnowflakeBody { code, .. }` (see
//!   `apis::database_driver_v1::error::map_async_query_error`).
//! - The query-monitoring response (`snowflake_query_status`) emits
//!   `error_code` without a SQLSTATE.
//!
//! Without a SQLSTATE on the wire, downstream consumers (ODBC, JDBC, ADBC)
//! lose the ability to classify the error and fall back to a generic
//! `HY000` / `08000`. This table fills that gap by mapping the small set of
//! well-known Snowflake error codes whose ANSI SQLSTATE is stable to the
//! corresponding string, so the rest of the stack can keep treating
//! `sql_state` as the single source of truth.
//!
//! The table is intentionally narrow: only codes whose mapping is unambiguous
//! and stable across server versions belong here. Add a new entry when a new
//! code is observed in production telemetry and confirmed against the
//! Snowflake error registry — and add a unit test alongside it.

/// Look up the ANSI SQLSTATE string for a Snowflake server error code.
///
/// Returns `None` for codes that have no canonical SQLSTATE mapping; callers
/// should treat that as "leave `sql_state` unset" and let the consumer apply
/// its own default (typically `HY000`).
pub fn sql_state_from_code(code: i32) -> Option<&'static str> {
    match code {
        // SQL compilation error — syntax, unresolved identifier, type mismatch.
        // Matches the server's own `sqlState` for this code, included here as
        // a safety net for paths (async poll, monitoring) where the server
        // omits the field.
        1003 => Some("42000"),
        // Numeric value out of representable range for the target column type.
        // e.g. "Number out of representable range: type FIXED[SB2](3,0), value 99999".
        100038 => Some("22003"),
        // String length exceeds the column's declared maximum and would be
        // truncated. e.g. "String 'hello world' is too long and would be truncated".
        100078 => Some("22001"),
        _ => {
            // Surface unmapped codes so we can grow this table as new ones
            // appear in production telemetry. Logged at `debug` because this
            // function is on a hot error path and we don't want to spam
            // operator logs for every unknown code; bumping verbosity is
            // sufficient when investigating an HY000 fallback.
            tracing::debug!(
                snowflake_error_code = code,
                "no SQLSTATE mapping for Snowflake error code; consider adding it to sql_state_from_code"
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_codes_map_to_expected_sqlstate() {
        assert_eq!(sql_state_from_code(1003), Some("42000"));
        assert_eq!(sql_state_from_code(100038), Some("22003"));
        assert_eq!(sql_state_from_code(100078), Some("22001"));
    }

    #[test]
    fn unknown_code_returns_none() {
        assert_eq!(sql_state_from_code(0), None);
        assert_eq!(sql_state_from_code(-1), None);
        assert_eq!(sql_state_from_code(999_999), None);
    }
}
