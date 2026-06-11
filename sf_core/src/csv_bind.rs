//! CSV serialization primitives for the stage-binding upload path.
//!
//! Every Snowflake driver that bulk-binds parameters above the
//! `CLIENT_STAGE_ARRAY_BINDING_THRESHOLD` uploads the binds as a CSV file to
//! `@SYSTEM$BIND/<request_id>/0.gz`.  The server then runs an *implicit* COPY
//! INTO against that file as part of the `INSERT … VALUES (?, ?)` statement.
//!
//! That implicit pipeline does NOT honor the stage's declared
//! `FIELD_OPTIONALLY_ENCLOSED_BY='"'` for UTF-8 multibyte content: bytes >=
//! 0x80 inside an enclosed (`"…"`) field round-trip as SQL NULL.  Empirically
//! verified end-to-end: see the `tests/csv_bind_multibyte_*` cases below,
//! the universal-driver large-bindings E2E suite, and the JDBC reference
//! implementation (`net.snowflake.client.jdbc.SnowflakeType.escapeForCSV`).
//!
//! The workaround is to write each cell **bare** when it contains no CSV
//! metacharacters — exactly what JDBC, .NET, Node, and Go's drivers already
//! do.  The function below centralises that escape rule so all universal-
//! driver frontends (ODBC, JDBC bridge, Python, Node) produce byte-for-byte
//! identical CSV output and inherit the multibyte-safe behaviour.

/// Append a CSV cell value to `out` using Snowflake's stage-binding CSV
/// conventions, mirroring JDBC's
/// `net.snowflake.client.jdbc.SnowflakeType.escapeForCSV`:
///
/// * `None` (SQL NULL) → nothing appended.  Combined with `EMPTY_FIELD_AS_NULL=TRUE`
///   on the server (default), an empty/absent cell is interpreted as SQL NULL.
/// * `Some("")` (empty string) → `""` (two double-quotes).  Distinguishes
///   the empty string from NULL on the server side.
/// * `Some(s)` containing `"`, `,`, `\n`, or `\\` → wrapped in `"…"` with
///   embedded `"` doubled to `""`.  Standard RFC-4180 quoting for the
///   characters that would otherwise break parsing.
/// * `Some(s)` otherwise → **bare** (no enclosing quotes).
///
/// The bare-when-safe rule is essential: when a UTF-8 multibyte value
/// (e.g. `日本語`) is enclosed by `"…"` the server's implicit bind-stage
/// COPY stores SQL NULL instead of the intended string.  Writing the same
/// value bare round-trips correctly.
pub fn append_csv_cell(out: &mut String, value: Option<&str>) {
    match value {
        None => {}
        Some("") => out.push_str("\"\""),
        Some(s) if needs_quoting(s) => {
            out.push('"');
            for ch in s.chars() {
                if ch == '"' {
                    out.push_str("\"\"");
                } else {
                    out.push(ch);
                }
            }
            out.push('"');
        }
        Some(s) => out.push_str(s),
    }
}

/// Returns `true` iff `s` contains any byte that requires the cell to be
/// wrapped in `"…"` quotes for the CSV to round-trip safely.
///
/// The set matches JDBC's `escapeForCSV`: `"`, `\n`, `,`, and `\\`.  Note we
/// scan **bytes** rather than chars — every offending character is single-byte
/// ASCII, so byte scanning is equivalent and avoids a UTF-8 decode pass for
/// the common ASCII-only payloads.
#[inline]
fn needs_quoting(s: &str) -> bool {
    s.bytes().any(|b| matches!(b, b'"' | b',' | b'\n' | b'\\'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(value: Option<&str>) -> String {
        let mut out = String::new();
        append_csv_cell(&mut out, value);
        out
    }

    // ----- NULL vs empty string ---------------------------------------------

    #[test]
    fn null_is_empty_unquoted_cell() {
        // SQL NULL produces nothing; the cell is the gap between commas.
        assert_eq!(cell(None), "");
    }

    #[test]
    fn empty_string_is_quoted_pair_to_distinguish_from_null() {
        assert_eq!(cell(Some("")), "\"\"");
    }

    // ----- bare (no metacharacters) -----------------------------------------

    #[test]
    fn plain_ascii_is_bare() {
        assert_eq!(cell(Some("hello")), "hello");
        assert_eq!(cell(Some("123")), "123");
        assert_eq!(cell(Some("2024-01-01")), "2024-01-01");
    }

    #[test]
    fn multibyte_utf8_without_metacharacters_is_bare() {
        // This is the key invariant: 日本語 must NOT be enclosed by quotes,
        // because Snowflake's implicit bind-stage COPY mis-parses multibyte
        // sequences inside enclosed fields (stores NULL).  Bare encoding
        // round-trips correctly.  Verified end-to-end against the running
        // service in the large-bindings E2E suite.
        assert_eq!(cell(Some("\u{65e5}\u{672c}\u{8a9e}")), "日本語");
        assert_eq!(cell(Some("\u{65e5}\u{672c}\u{8a9e}6")), "日本語6");
    }

    // ----- conditional quoting on each metacharacter ------------------------

    #[test]
    fn quotes_when_contains_comma() {
        assert_eq!(cell(Some("a,b")), "\"a,b\"");
        assert_eq!(cell(Some("val,0")), "\"val,0\"");
    }

    #[test]
    fn quotes_when_contains_double_quote_and_doubles_it() {
        assert_eq!(cell(Some("she said \"hi\"")), "\"she said \"\"hi\"\"\"");
        assert_eq!(cell(Some("say\"1\"")), "\"say\"\"1\"\"\"");
        assert_eq!(cell(Some("\"")), "\"\"\"\"");
    }

    #[test]
    fn quotes_when_contains_newline() {
        assert_eq!(cell(Some("line1\nline2")), "\"line1\nline2\"");
        assert_eq!(cell(Some("a\nb")), "\"a\nb\"");
    }

    #[test]
    fn quotes_when_contains_backslash() {
        // Backslash forces quoting because, in unenclosed fields, the server's
        // default `ESCAPE_UNENCLOSED_FIELD='\\'` would interpret it as an
        // escape.  Quoting the cell forces the enclosed path where `ESCAPE`
        // is `NONE` by default and `\` is a literal byte.
        assert_eq!(cell(Some("a\\b")), "\"a\\b\"");
        assert_eq!(cell(Some("C:\\dir\\3")), "\"C:\\dir\\3\"");
        assert_eq!(cell(Some("\\")), "\"\\\"");
    }

    // ----- carriage return is NOT a metacharacter (matches JDBC) -----------

    #[test]
    fn lone_carriage_return_does_not_force_quoting() {
        // JDBC's `escapeForCSV` only quotes for `"`, `,`, `\n`, `\\`.  CR
        // alone is not in the set — the stage CSV uses `RECORD_DELIMITER='\n'`
        // and the server tolerates a stray CR inside a bare field.  Keeping
        // the same rule preserves byte-for-byte parity with JDBC output.
        assert_eq!(cell(Some("a\rb")), "a\rb");
    }
}
