//! IR validation pass that runs immediately before C++ code emission.
//!
//! Historically the [`crate::generator::cpp`] emitters defended themselves
//! with `unwrap_or(...)` defaults — `SQLGetInfo(dbc0, 0, ...)` when the
//! InfoType was lost, `SQLGetData(stmt0, col, SQL_C_CHAR, ...)` when the
//! TargetType was missing, `SQLSetEnvAttr(..., 0, 0)` when the captured
//! value couldn't be parsed. Every such default is a *real, distinct* ODBC
//! call that the test would silently exercise instead of the one the trace
//! actually recorded. This module enforces the contract that an IR with a
//! missing required field is **never** rendered to C++; the generator
//! aborts with a clear error pointing at the trace line so the upstream
//! parser/trace can be fixed.
//!
//! The required-field schema is described in
//! `.cursor/plans/trace-tool_unknown-info-type_fix_9efdaa62.plan.md`.

use crate::model::OdbcCall;

/// A required field that the IR didn't populate. Surfaced as a `GenerateError`
/// at the CLI boundary so generation fails-fast with a clear, actionable
/// message rather than emitting silently-wrong code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingRequired {
    /// Symbolic ODBC function name, e.g. `"SQLGetInfo"`.
    pub call: &'static str,
    /// IR field (or `field_a|field_b` if any one of a set is required).
    pub field: &'static str,
    /// `entry_line` from the originating `TracedCall`, when available.
    pub line: Option<usize>,
}

impl std::fmt::Display for MissingRequired {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.line {
            Some(line) => write!(
                f,
                "{} at trace line {} is missing required field `{}`",
                self.call, line, self.field
            ),
            None => write!(
                f,
                "{} is missing required field `{}`",
                self.call, self.field
            ),
        }
    }
}

/// Validate one call. Returns `Err(MissingRequired)` when an IR field that
/// the C++ emitter would otherwise substitute a *meaningful ODBC default*
/// for is absent. Calls whose required fields are all populated, or which
/// have no required fields, return `Ok`.
pub fn validate_call(call: &OdbcCall, line: Option<usize>) -> Result<(), MissingRequired> {
    match call {
        // Either the symbolic name (`SQL_OWNER_USAGE`) OR the captured
        // integer (`91`) is enough — the emitter falls back to the
        // integer when the symbol is missing. Both `None` means the
        // parser couldn't recover any identifier, and `0` is itself a
        // valid InfoType (`SQL_INFO_FIRST`), so we refuse to emit.
        OdbcCall::GetInfo(g) if g.info_type.is_none() && g.info_type_value.is_none() => {
            return Err(MissingRequired {
                call: "SQLGetInfo",
                field: "info_type|info_type_value",
                line,
            });
        }
        OdbcCall::GetInfo(_) => {}
        OdbcCall::GetData(g) => {
            require_some(g.column_number, "SQLGetData", "column_number", line)?;
            require_some(
                g.target_type_name.as_ref(),
                "SQLGetData",
                "target_type_name",
                line,
            )?;
            require_some(g.buffer_length, "SQLGetData", "buffer_length", line)?;
        }
        OdbcCall::DescribeCol(d) => {
            require_some(d.column_number, "SQLDescribeCol", "column_number", line)?;
            require_some(d.buffer_length, "SQLDescribeCol", "buffer_length", line)?;
        }
        OdbcCall::FetchScroll(f) => {
            require_some(
                f.orientation_name.as_ref(),
                "SQLFetchScroll",
                "orientation_name",
                line,
            )?;
            require_some(f.offset, "SQLFetchScroll", "offset", line)?;
        }
        OdbcCall::SetEnvAttr(s) => {
            require_some(s.attribute.as_ref(), "SQLSetEnvAttr", "attribute", line)?;
            require_some(s.value, "SQLSetEnvAttr", "value", line)?;
        }
        OdbcCall::SetConnectAttr(s) => {
            require_some(s.attribute.as_ref(), "SQLSetConnectAttr", "attribute", line)?;
            require_some(s.value, "SQLSetConnectAttr", "value", line)?;
        }
        OdbcCall::SetStmtAttr(s) => {
            require_some(s.attribute.as_ref(), "SQLSetStmtAttr", "attribute", line)?;
            require_some(s.value, "SQLSetStmtAttr", "value", line)?;
        }
        OdbcCall::ColAttribute(c) => {
            require_some(c.column_number, "SQLColAttribute", "column_number", line)?;
            // The emitter's *existing* skip policy drops calls with a
            // `<unknown>` field identifier (HY091 from the reference
            // driver), but it expects the IR to carry at least the
            // integer form so that policy is reached via an intentional
            // branch rather than via silent parser data loss. After the
            // `<unknown>` parser fix, every captured row has at least one
            // of these populated.
            if c.field_identifier.is_none() && c.field_identifier_value.is_none() {
                return Err(MissingRequired {
                    call: "SQLColAttribute",
                    field: "field_identifier|field_identifier_value",
                    line,
                });
            }
        }
        // Other call variants have no fields whose absence would silently
        // produce a different valid ODBC call; their emitters either
        // handle `Option::None` explicitly or operate purely on the
        // handle graph.
        _ => {}
    }
    Ok(())
}

fn require_some<T>(
    value: Option<T>,
    call: &'static str,
    field: &'static str,
    line: Option<usize>,
) -> Result<(), MissingRequired> {
    if value.is_some() {
        Ok(())
    } else {
        Err(MissingRequired { call, field, line })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        ColAttribute, DescribeCol, FetchScroll, GetData, GetInfo, OdbcCall, ReturnCode, SetEnvAttr,
    };

    fn ok(call: OdbcCall) {
        assert!(
            validate_call(&call, Some(42)).is_ok(),
            "expected validate_call to succeed for {call:?}"
        );
    }

    fn missing(call: OdbcCall, expect_call: &str, expect_field: &str) {
        let err = validate_call(&call, Some(42))
            .expect_err("validator must reject IR with missing required field");
        assert_eq!(err.call, expect_call);
        assert_eq!(err.field, expect_field);
        assert_eq!(err.line, Some(42));
        // Display should include the trace line so the CLI message is
        // immediately actionable.
        assert!(
            err.to_string().contains("trace line 42"),
            "Display must include trace line: {err}",
        );
    }

    #[test]
    fn get_info_passes_when_symbolic_name_present() {
        ok(OdbcCall::GetInfo(GetInfo {
            return_code: ReturnCode::Success,
            handle: Some("0xdbc".to_string()),
            info_type: Some("SQL_OWNER_USAGE".to_string()),
            info_type_value: Some(91),
            info_value: None,
            info_value_numeric: Some(0x15),
        }));
    }

    #[test]
    fn get_info_passes_when_only_numeric_present() {
        // The `<unknown>` parser-fix case: integer recovered without a
        // symbolic name. The emitter falls back to `SQLGetInfo(dbc0, 169,
        // ...)`, so validation passes.
        ok(OdbcCall::GetInfo(GetInfo {
            return_code: ReturnCode::Success,
            handle: Some("0xdbc".to_string()),
            info_type: None,
            info_type_value: Some(169),
            info_value: None,
            info_value_numeric: None,
        }));
    }

    #[test]
    fn get_info_rejects_when_both_name_and_value_are_none() {
        // The original silent-corruption case: the parser dropped the
        // identifier entirely, and today the emitter would substitute
        // `0` (= SQL_INFO_FIRST, a real call). Validator must refuse.
        missing(
            OdbcCall::GetInfo(GetInfo {
                return_code: ReturnCode::Success,
                handle: Some("0xdbc".to_string()),
                info_type: None,
                info_type_value: None,
                info_value: None,
                info_value_numeric: None,
            }),
            "SQLGetInfo",
            "info_type|info_type_value",
        );
    }

    #[test]
    fn get_data_rejects_missing_target_type() {
        missing(
            OdbcCall::GetData(GetData {
                return_code: ReturnCode::Success,
                handle: Some("0xstmt".to_string()),
                column_number: Some(1),
                target_type: Some(1),
                target_type_name: None,
                buffer_length: Some(256),
                ..Default::default()
            }),
            "SQLGetData",
            "target_type_name",
        );
    }

    #[test]
    fn get_data_rejects_missing_buffer_length() {
        missing(
            OdbcCall::GetData(GetData {
                return_code: ReturnCode::Success,
                handle: Some("0xstmt".to_string()),
                column_number: Some(1),
                target_type: Some(1),
                target_type_name: Some("SQL_C_CHAR".to_string()),
                buffer_length: None,
                ..Default::default()
            }),
            "SQLGetData",
            "buffer_length",
        );
    }

    #[test]
    fn set_env_attr_rejects_missing_value() {
        // `value: None` would today produce `SQLSetEnvAttr(env, attr,
        // (SQLPOINTER)0, 0)` which is a real, distinct ODBC call (e.g.
        // SQL_AUTOCOMMIT_OFF).
        missing(
            OdbcCall::SetEnvAttr(SetEnvAttr {
                return_code: ReturnCode::Success,
                handle: Some("0xenv".to_string()),
                attribute: Some("SQL_ATTR_CONNECTION_POOLING".to_string()),
                value: None,
                str_len: Some(0),
            }),
            "SQLSetEnvAttr",
            "value",
        );
    }

    #[test]
    fn fetch_scroll_rejects_missing_orientation() {
        missing(
            OdbcCall::FetchScroll(FetchScroll {
                return_code: ReturnCode::Success,
                handle: Some("0xstmt".to_string()),
                orientation: Some(1),
                orientation_name: None,
                offset: Some(1),
            }),
            "SQLFetchScroll",
            "orientation_name",
        );
    }

    #[test]
    fn describe_col_rejects_missing_buffer_length() {
        missing(
            OdbcCall::DescribeCol(DescribeCol {
                return_code: ReturnCode::Success,
                handle: Some("0xstmt".to_string()),
                column_number: Some(1),
                column_name: None,
                buffer_length: None,
                data_type: None,
                column_size: None,
                decimal_digits: None,
                nullable: None,
            }),
            "SQLDescribeCol",
            "buffer_length",
        );
    }

    #[test]
    fn col_attribute_passes_when_only_integer_field_id_present() {
        // SQLColAttribute with `<unknown>` after parser fix: the symbol
        // is None but the integer form is populated. Validator passes
        // (the emitter's separate skip-policy may still drop the call).
        ok(OdbcCall::ColAttribute(ColAttribute {
            return_code: ReturnCode::Success,
            handle: Some("0xstmt".to_string()),
            column_number: Some(1),
            field_identifier: None,
            field_identifier_value: Some(32),
            buffer_length: None,
            string_length: None,
            numeric_attribute: None,
            numeric_attribute_name: None,
            character_value: None,
        }));
    }

    #[test]
    fn col_attribute_rejects_when_field_id_completely_lost() {
        missing(
            OdbcCall::ColAttribute(ColAttribute {
                return_code: ReturnCode::Success,
                handle: Some("0xstmt".to_string()),
                column_number: Some(1),
                field_identifier: None,
                field_identifier_value: None,
                buffer_length: None,
                string_length: None,
                numeric_attribute: None,
                numeric_attribute_name: None,
                character_value: None,
            }),
            "SQLColAttribute",
            "field_identifier|field_identifier_value",
        );
    }
}
