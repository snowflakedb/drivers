use std::sync::LazyLock;

use regex::Regex;
use snafu::prelude::*;
use snafu::Location;

use crate::model::{
    Direction, HandleGraph, HandleType, OdbcCall, ParamValue, Parameter, ReturnCode, TraceEntry,
    TraceFormat, TraceHeader, TraceLog, TracedCall,
};

#[derive(Snafu, Debug)]
pub enum WinOdbcParserError {
    #[snafu(display("Invalid trace format: not a Windows ODBC DM trace file"))]
    InvalidFormat {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display(
        "Missing return code on EXIT block for {function} at line {line}: \
         the Windows DM trace did not include a parseable return code, and we \
         refuse to silently default to SQL_SUCCESS because a failing call \
         would then be asserted as a successful one"
    ))]
    MissingReturnCode {
        function: String,
        line: usize,
        #[snafu(implicit)]
        location: Location,
    },
}

type Result<T> = std::result::Result<T, WinOdbcParserError>;

pub fn parse_str(content: &str) -> Result<TraceLog> {
    if detect_winodbc_header(content).is_none() {
        return Err(WinOdbcParserError::InvalidFormat {
            location: Location::default(),
        });
    }

    let entries = parse_entries(content);
    let (calls, handle_graph) = pair_entries(entries)?;

    let header = TraceHeader {
        format: TraceFormat::WinOdbc,
        ..Default::default()
    };

    Ok(TraceLog {
        header,
        calls,
        handle_graph,
    })
}

/// Returns true if `content` looks like a Windows ODBC DM trace.
pub fn looks_like_winodbc(content: &str) -> bool {
    detect_winodbc_header(content).is_some()
}

fn detect_winodbc_header(content: &str) -> Option<()> {
    for line in content.lines().filter(|l| !l.trim().is_empty()).take(10) {
        if HEADER_RE.is_match(line) {
            return Some(());
        }
    }
    None
}

struct RawBlock {
    thread_tag: String,
    direction: Direction,
    function_name: String,
    return_code: Option<ReturnCode>,
    body_lines: Vec<String>,
    line_number: usize,
}

static HEADER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(.+?)\t(ENTER|EXIT)\s+(\w+)(?:\s+with return code\s+(-?\d+)\s+\((\w+)\))?\s*$")
        .unwrap()
});

static BODY_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s+(.+?)\s{2,}(.+)$").unwrap());

static OUTPUT_ADDR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(0x[0-9A-Fa-f]+)\s+\(\s*(0x[0-9A-Fa-f]+)\s*\)").unwrap());

static OUTPUT_INT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(0x[0-9A-Fa-f]+)\s+\(\s*(-?\d+)\s*\)(?:\s*<([A-Z_][A-Z_0-9]+)>)?").unwrap()
});

static INT_NAMED_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(-?\d+)\s*<([A-Z_][A-Z_0-9]+)>$").unwrap());

// Captures the **outermost** `"..."` literal on a WinODBC body line. We
// match from the first `"` to the *last* `"` followed by optional trailing
// whitespace and end-of-string, because the Windows DM trace format does
// **not** escape embedded `"` characters inside string values — e.g.
// `SQLGetInfo(SQL_IDENTIFIER_QUOTE_CHAR)` returns `"` and is rendered as
// `[       2] """` (open quote, the actual `"`, close quote). A non-greedy
// inner match would stop at the embedded `"` and capture an empty string.
//
// The `\s*$` anchor is what makes the greedy `(.*)` backtrack to the right
// closing quote: it forces the close quote to be the *last* one on the line.
// Genuine C-style escapes (`\"`, `\\`, etc.) and the Windows DM's `\ X` hex
// escapes are still resolved downstream by `decode_winodbc_string`.
static STRING_VALUE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?s).*?"(.*)"\s*$"#).unwrap());

static HEX_ADDR_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^0x[0-9A-Fa-f]+$").unwrap());

fn split_into_blocks(content: &str) -> Vec<RawBlock> {
    let mut blocks = Vec::new();
    let mut current_lines: Vec<(usize, String)> = Vec::new();

    for (idx, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            if let Some(block) = parse_block_from_lines(&current_lines) {
                blocks.push(block);
            }
            current_lines.clear();
        } else {
            current_lines.push((idx + 1, line.to_string()));
        }
    }

    if let Some(block) = parse_block_from_lines(&current_lines) {
        blocks.push(block);
    }

    blocks
}

fn parse_block_from_lines(lines: &[(usize, String)]) -> Option<RawBlock> {
    if lines.is_empty() {
        return None;
    }

    let (line_number, header_line) = &lines[0];
    let caps = HEADER_RE.captures(header_line)?;

    let thread_tag = caps[1].to_string();
    let direction = match &caps[2] {
        "ENTER" => Direction::Enter,
        _ => Direction::Exit,
    };
    let function_name = caps[3].to_string();

    let return_code = caps.get(4).and_then(|code_m| {
        let code: i32 = code_m.as_str().parse().ok()?;
        let name = caps.get(5).map(|m| m.as_str()).unwrap_or("");
        ReturnCode::from_code_and_name(code, name).or_else(|| ReturnCode::from_name(name))
    });

    let body_lines: Vec<String> = lines.iter().skip(1).map(|(_, l)| l.clone()).collect();

    Some(RawBlock {
        thread_tag,
        direction,
        function_name,
        return_code,
        body_lines,
        line_number: *line_number,
    })
}

fn parse_param_value(raw: &str) -> ParamValue {
    let trimmed = raw.trim();

    if trimmed == "0x0000000000000000"
        || trimmed == "0x0"
        || trimmed.starts_with("<Invalid")
        || trimmed.starts_with('[')
    {
        if trimmed == "0x0000000000000000" || trimmed == "0x0" {
            return ParamValue::NullPointer;
        }
        if let Ok(v) = trimmed
            .trim_matches(|c| c == '[' || c == ']')
            .parse::<i64>()
        {
            return ParamValue::Integer(v);
        }
        return ParamValue::NullPointer;
    }

    if let Some(caps) = OUTPUT_ADDR_RE.captures(trimmed) {
        return ParamValue::OutputAddress {
            address: caps[1].to_string(),
            output_address: caps[2].to_string(),
        };
    }

    if let Some(caps) = OUTPUT_INT_RE.captures(trimmed) {
        // Captured by `(-?\d+)` so the parse cannot fail. Use a panic
        // (rather than silent `unwrap_or(0)`) to surface any future regex
        // change that broadens the group without updating this site.
        let value = parse_winodbc_signed_int(&caps[2])
            .expect("OUTPUT_INT_RE `(-?\\d+)` group must parse as i64");
        if let Some(name) = caps.get(3) {
            return ParamValue::OutputNamedConstant {
                address: caps[1].to_string(),
                name: name.as_str().to_string(),
                value: Some(value),
            };
        }
        return ParamValue::OutputInteger {
            address: caps[1].to_string(),
            value,
        };
    }

    if let Some(caps) = STRING_VALUE_RE.captures(trimmed) {
        let text = decode_winodbc_string(&caps[1]);
        return ParamValue::StringValue {
            value: text,
            truncated: false,
        };
    }

    if let Some(caps) = INT_NAMED_RE.captures(trimmed) {
        return ParamValue::NamedConstant {
            // Captured by `(-?\d+)`; an infallible parse modelled as a panic
            // rather than `unwrap_or(0)` so a future regex broadening
            // surfaces immediately instead of silently substituting `0`.
            value: Some(
                caps[1]
                    .parse()
                    .expect("INT_NAMED_RE `(-?\\d+)` group must parse as i64"),
            ),
            name: caps[2].to_string(),
        };
    }

    if let Some(angle) = trimmed.split_once('<') {
        let name = angle.1.trim_end_matches('>').trim();
        if is_constant_name(name) {
            // Preserve `None` rather than substituting `0` when the prefix
            // is missing or unparseable — `0` is itself a valid value for
            // many ODBC fields (e.g. `SQL_INFO_FIRST` for InfoType).
            let value = angle.0.trim().parse::<i64>().ok();
            return ParamValue::NamedConstant {
                value,
                name: name.to_string(),
            };
        }
    }

    // Windows DM occasionally emits `<unknown>` (lowercase) when its symbol
    // table doesn't recognize an InfoType / FieldIdentifier ID — e.g.
    // `UWORD 169 <unknown>` for `SQL_AGGREGATE_FUNCTIONS`. Without this
    // branch the line falls through to `Address("169 <unknown>")` and the
    // integer is permanently lost, materialising as `SQLGetInfo(dbc0, 0,
    // ...)` (a different real call) downstream. Treat any bracketed tag
    // that isn't a SQL_-style constant as a plain integer when the prefix
    // parses. Other variants (`<Invalid *>`, `<unknown type>`, `<zero
    // length>`) have already been handled above or do not carry an
    // integer prefix and continue to fall through unchanged.
    if let Some((head, _tail)) = trimmed.split_once('<') {
        if let Ok(v) = head.trim().parse::<i64>() {
            return ParamValue::Integer(v);
        }
    }

    if HEX_ADDR_RE.is_match(trimmed) {
        return ParamValue::Address(trimmed.to_string());
    }

    if let Ok(v) = trimmed.parse::<i64>() {
        return ParamValue::Integer(v);
    }

    if is_constant_name(trimmed) {
        // Bare symbolic name with no rendered numeric value (e.g. a body
        // line emitted as just `SQL_HANDLE_STMT`). Represent the missing
        // numeric form as `None` so downstream consumers don't confuse it
        // with `Some(0)`.
        return ParamValue::NamedConstant {
            name: trimmed.to_string(),
            value: None,
        };
    }

    ParamValue::Address(trimmed.to_string())
}

/// Parse an integer captured from a WinODBC `SQLLEN *` / `SQLINTEGER *` output
/// dump, recovering its signed interpretation.
///
/// The Windows DM trace formatter prints 32-bit-truncated integer outputs via
/// `%u`, so a negative `SQLLEN` like `-2` (e.g. `SQL_BINARY` for
/// `SQL_DESC_CONCISE_TYPE`) appears in the trace as `4294967294`. On a
/// 64-bit Unix replay platform `SQLLEN` is `i64`, and the driver returns the
/// signed value as-is; without this normalisation, every captured
/// `SQLColAttribute` numeric output for a negative-valued descriptor field
/// would replay as a mismatch.
///
/// Heuristic: any value in `[2^31, 2^32)` is the unsigned representation of a
/// signed `i32` negative — sign-extend it. Values outside that range pass
/// through unchanged so genuinely-large `SQLULEN` outputs (e.g. blob lengths)
/// are preserved.
///
/// Returns `None` for unparseable input (preserved as `None` through the IR
/// rather than collapsed to `0`, because `0` is itself a meaningful value
/// for many ODBC numeric outputs).
fn parse_winodbc_signed_int(s: &str) -> Option<i64> {
    let raw: i64 = s.parse().ok()?;
    if (0x8000_0000..=0xFFFF_FFFF).contains(&(raw as u64)) {
        Some((raw as i32) as i64)
    } else {
        Some(raw)
    }
}

/// Decode the contents of a quoted string parameter as emitted by the
/// Windows ODBC Driver Manager trace.
///
/// The DM emits the underlying wide-string buffer as a C-style quoted literal
/// where:
///   * non-printable bytes are rendered as `\ <hex>` (backslash, space, single
///     lowercase hex digit) — e.g. `\ a` is `0x0a` (LF), `\ 0` is `0x00` (NUL),
///     `\ 9` would be `0x09` (TAB)
///   * standard C escapes (`\\`, `\"`, `\n`, `\r`, `\t`, `\0`) are honored as
///     a fallback so synthetic / test traces using C-string conventions still
///     parse correctly
///   * the trailing NUL terminator written by the DM is stripped
fn decode_winodbc_string(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some(' ') => match chars.next() {
                Some(h) if h.is_ascii_hexdigit() => {
                    let v = h.to_digit(16).unwrap() as u8;
                    out.push(v as char);
                }
                Some(other) => {
                    out.push('\\');
                    out.push(' ');
                    out.push(other);
                }
                None => {
                    out.push('\\');
                    out.push(' ');
                }
            },
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('0') => out.push('\0'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    if out.ends_with('\0') {
        out.pop();
    }
    out
}

fn is_constant_name(s: &str) -> bool {
    s.len() > 2
        && s.contains('_')
        && s.chars()
            .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit())
}

fn synthetic_names(func: &str) -> &'static [&'static str] {
    let normalized = func.strip_suffix('W').unwrap_or(func);
    match normalized {
        "SQLAllocHandle" => &["Handle Type", "Input Handle", "Output Handle"],
        "SQLFreeHandle" => &["Handle Type", "Input Handle"],
        "SQLSetEnvAttr" => &["Environment", "Attribute", "Value", "StrLen"],
        "SQLSetConnectAttr" => &["Connection", "Attribute", "Value", "StrLen"],
        "SQLSetStmtAttr" => &["Statement", "Attribute", "Value", "StrLen"],
        "SQLDriverConnect" => &[
            "Connection",
            "WindowHandle",
            "InConnectionString",
            "InConnectionStringLength",
            "OutConnectionString",
            "OutConnectionStringLength",
            "OutConnectionStringLengthPtr",
            "DriverCompletion",
        ],
        "SQLDisconnect" => &["Connection"],
        "SQLGetInfo" => &[
            "Connection",
            "InfoType",
            "InfoValue",
            "BufferLength",
            "StringLength",
        ],
        "SQLPrepare" => &["Statement", "SQL", "SQLLength"],
        "SQLExecute" => &["Statement"],
        "SQLExecDirect" => &["Statement", "SQL", "SQLLength"],
        "SQLNumResultCols" => &["Statement", "Count"],
        "SQLColAttribute" => &[
            "Statement",
            "Column Number",
            "Field Identifier",
            "CharacterAttributePtr",
            "Buffer Length",
            "String Length",
            "Numeric Attribute",
        ],
        "SQLDescribeCol" => &[
            "Statement",
            "Column Number",
            "Column Name",
            "Buffer Length",
            "Data Type",
            "Column Size",
            "Decimal Digits",
            "Nullable",
        ],
        "SQLFetch" => &["Statement"],
        "SQLFetchScroll" => &["Statement", "Fetch Orientation", "Fetch Offset"],
        "SQLGetData" => &[
            "Statement",
            "Column Number",
            "Target Type",
            "TargetValue",
            "Buffer Length",
            "Strlen Or Ind",
        ],
        "SQLRowCount" => &["Statement", "Row Count"],
        "SQLMoreResults" => &["Statement"],
        "SQLCloseCursor" => &["Statement"],
        "SQLGetDiagRec" => &[
            "Handle Type",
            "Handle",
            "RecNumber",
            "SqlState",
            "NativeError",
            "MessageText",
            "BufferLength",
            "TextLength",
        ],
        "SQLGetFunctions" => &["Connection", "FunctionId", "Supported"],
        "SQLTables" => &[
            "Statement",
            "CatalogName",
            "SchemaName",
            "TableName",
            "TableType",
        ],
        "SQLColumns" => &[
            "Statement",
            "CatalogName",
            "SchemaName",
            "TableName",
            "ColumnName",
        ],
        "SQLBindCol" => &[
            "Statement",
            "Column Number",
            "Target Type",
            "TargetValue",
            "Buffer Length",
            "Strlen Or Ind",
        ],
        _ => &[],
    }
}

fn parse_body(body_lines: &[String], function_name: &str) -> Vec<Parameter> {
    let names = synthetic_names(function_name);
    let mut params = Vec::new();

    for (idx, line) in body_lines.iter().enumerate() {
        let Some(caps) = BODY_RE.captures(line) else {
            continue;
        };
        let type_name = caps[1].trim().to_string();
        let raw_value = caps[2].trim().to_string();
        // Prefer the synthetic-names table entry when it exists; fall back
        // to the raw C type token (e.g. `SQLHENV`) so downstream lookups
        // by name still have something concrete to match. Explicit branch
        // — never silently substitute the empty string, which would
        // defeat every `find_param_int(_, "InfoType")` etc.
        let param_name = match names.get(idx).copied() {
            Some(name) => name.to_string(),
            None => type_name.clone(),
        };
        params.push(Parameter {
            type_name: param_name,
            value: parse_param_value(&raw_value),
        });
    }

    params
}

fn parse_entries(content: &str) -> Vec<TraceEntry> {
    let blocks = split_into_blocks(content);
    let mut entries = Vec::with_capacity(blocks.len());

    for block in blocks {
        let parameters = parse_body(&block.body_lines, &block.function_name);
        entries.push(TraceEntry {
            timestamp: String::new(),
            thread_id: Some(block.thread_tag),
            direction: block.direction,
            function_name: block.function_name,
            return_code: block.return_code,
            return_code_raw: None,
            parameters,
            line_number: Some(block.line_number),
        });
    }

    entries
}

fn pair_entries(entries: Vec<TraceEntry>) -> Result<(Vec<TracedCall>, HandleGraph)> {
    let mut calls = Vec::new();
    let mut handle_graph = HandleGraph::new();
    let mut pending_enters: Vec<TraceEntry> = Vec::new();

    for entry in entries {
        match entry.direction {
            Direction::Enter => {
                pending_enters.push(entry);
            }
            Direction::Exit => {
                let enter_idx = pending_enters.iter().rposition(|e| {
                    e.function_name == entry.function_name && e.thread_id == entry.thread_id
                });

                let (input_params, entry_line) = if let Some(idx) = enter_idx {
                    let enter = pending_enters.remove(idx);
                    (enter.parameters, enter.line_number)
                } else {
                    (Vec::new(), None)
                };

                // Refuse to silently default to `SQL_SUCCESS` — a failed-to-parse
                // return code on a real failure would otherwise produce a test
                // that asserts success on a failing call.
                let return_code =
                    entry
                        .return_code
                        .ok_or_else(|| WinOdbcParserError::MissingReturnCode {
                            function: entry.function_name.clone(),
                            line: entry.line_number.unwrap_or(0),
                            location: Location::default(),
                        })?;
                let exit_line = entry.line_number;
                let output_params = entry.parameters;

                let normalized = entry
                    .function_name
                    .strip_suffix('W')
                    .unwrap_or(&entry.function_name);
                if normalized == "SQLAllocHandle" && return_code.is_success() {
                    register_alloc(&input_params, &output_params, &mut handle_graph);
                }

                calls.push(TracedCall {
                    call: OdbcCall::from_raw(
                        &entry.function_name,
                        input_params,
                        output_params,
                        return_code,
                    ),
                    entry_line,
                    exit_line,
                });
            }
        }
    }

    Ok((calls, handle_graph))
}

fn register_alloc(
    input_params: &[Parameter],
    output_params: &[Parameter],
    graph: &mut HandleGraph,
) {
    let Some(handle_type_int) = find_param_int(input_params, "Handle Type") else {
        return;
    };
    let Some(handle_type) = HandleType::from_value(handle_type_int) else {
        return;
    };
    let Some(parent_addr) = find_param_addr(input_params, "Input Handle") else {
        return;
    };
    let Some(child_addr) = find_param_addr(output_params, "Output Handle") else {
        return;
    };

    graph.register_alloc(handle_type, &parent_addr, &child_addr);
}

fn find_param_int(params: &[Parameter], key: &str) -> Option<i64> {
    params
        .iter()
        .find(|p| p.type_name == key)
        .and_then(|p| match &p.value {
            ParamValue::Integer(v) => Some(*v),
            ParamValue::NamedConstant { value, .. } => *value,
            _ => None,
        })
}

fn find_param_addr(params: &[Parameter], key: &str) -> Option<String> {
    params
        .iter()
        .find(|p| p.type_name == key)
        .and_then(|p| match &p.value {
            ParamValue::Address(a) => Some(a.clone()),
            ParamValue::OutputAddress { output_address, .. } => Some(output_address.clone()),
            _ => None,
        })
}

#[cfg(test)]
const WIN_SAMPLE_TRACE: &str = "\
proc-1 1234-5678\tENTER SQLAllocHandle
\t\tSQLSMALLINT                  1 <SQL_HANDLE_ENV>
\t\tSQLHANDLE           0x0000000000000000
\t\tSQLHANDLE *         0x0000018E00866BE0

proc-1 1234-5678\tEXIT  SQLAllocHandle  with return code 0 (SQL_SUCCESS)
\t\tSQLSMALLINT                  1 <SQL_HANDLE_ENV>
\t\tSQLHANDLE           0x0000000000000000
\t\tSQLHANDLE *         0x0000018E00866BE0 ( 0x0000018E656DAC50)

proc-1 1234-5678\tENTER SQLExecDirectW
\t\tHSTMT               0x0000018E656DA1E0
\t\tWCHAR *             0x0000018E006DF854 [      -3] \"SELECT 1;\\0\"
\t\tSDWORD                    -3

proc-1 1234-5678\tEXIT  SQLExecDirectW  with return code 0 (SQL_SUCCESS)
\t\tHSTMT               0x0000018E656DA1E0
\t\tWCHAR *             0x0000018E006DF854 [      -3] \"SELECT 1;\\0\"
\t\tSDWORD                    -3

proc-1 1234-5678\tENTER SQLColAttributeW
\t\tSQLHSTMT            0x0000018E656DA1E0
\t\tSQLSMALLINT                  1
\t\tSQLSMALLINT                  2 <SQL_DESC_CONCISE_TYPE>
\t\tSQLPOINTER         0x0000000000000000
\t\tSQLSMALLINT                  0
\t\tSQLSMALLINT *       0x00000001669FE580

proc-1 1234-5678\tEXIT  SQLColAttributeW  with return code 0 (SQL_SUCCESS)
\t\tSQLHSTMT            0x0000018E656DA1E0
\t\tSQLSMALLINT                  1
\t\tSQLSMALLINT                  2 <SQL_DESC_CONCISE_TYPE>
\t\tSQLPOINTER         0x0000000000000000
\t\tSQLSMALLINT                  0
\t\tSQLSMALLINT *       0x00000001669FE580 (8)

proc-1 1234-5678\tENTER SQLColAttributeW
\t\tSQLHSTMT            0x0000018E656DA1E0
\t\tSQLSMALLINT                  1
\t\tSQLSMALLINT                 32 <unknown>
\t\tSQLPOINTER         0x0000000000000000
\t\tSQLSMALLINT                  0
\t\tSQLSMALLINT *       0x00000001669FE510

proc-1 1234-5678\tEXIT  SQLColAttributeW  with return code 0 (SQL_SUCCESS)
\t\tSQLHSTMT            0x0000018E656DA1E0
\t\tSQLSMALLINT                  1
\t\tSQLSMALLINT                 32 <unknown>
\t\tSQLPOINTER         0x0000000000000000
\t\tSQLSMALLINT                  0
\t\tSQLSMALLINT *       0x00000001669FE510 (8)

proc-1 1234-5678\tENTER SQLSetStmtAttrW
\t\tSQLHSTMT            0x0000018E656DA1E0
\t\tSQLINTEGER                  26 <SQL_ATTR_ROWS_FETCHED_PTR>
\t\tSQLPOINTER          0x0000000000000000
\t\tSQLINTEGER                   0

proc-1 1234-5678\tEXIT  SQLSetStmtAttrW  with return code 0 (SQL_SUCCESS)
\t\tSQLHSTMT            0x0000018E656DA1E0
\t\tSQLINTEGER                  26 <SQL_ATTR_ROWS_FETCHED_PTR>
\t\tSQLPOINTER          0x0000000000000000
\t\tSQLINTEGER                   0
";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ExecDirect;

    #[test]
    fn test_parse_sample_trace() {
        let trace = parse_str(WIN_SAMPLE_TRACE).expect("parse");
        assert_eq!(trace.header.format, TraceFormat::WinOdbc);

        let exec = trace
            .calls
            .iter()
            .find(|c| matches!(c.call, OdbcCall::ExecDirect(_)))
            .expect("exec direct");
        if let OdbcCall::ExecDirect(ExecDirect { sql, .. }) = &exec.call {
            assert_eq!(sql.as_deref(), Some("SELECT 1;"));
        } else {
            panic!("expected ExecDirect");
        }

        assert!(
            trace
                .calls
                .iter()
                .any(|c| matches!(c.call, OdbcCall::ColAttribute(_))),
            "expected SQLColAttribute calls (W-suffix normalized)"
        );
        assert!(
            trace
                .calls
                .iter()
                .any(|c| matches!(c.call, OdbcCall::SetStmtAttr(_))),
            "expected SQLSetStmtAttr calls (W-suffix normalized)"
        );

        assert!(
            !trace
                .calls
                .iter()
                .any(|c| matches!(c.call, OdbcCall::Unsupported(_))),
            "no unsupported calls"
        );
    }

    #[test]
    fn decode_string_handles_dm_hex_escapes() {
        assert_eq!(decode_winodbc_string("SELECT 1;\\ 0"), "SELECT 1;");
        assert_eq!(decode_winodbc_string("a\\ ab"), "a\nb");
        assert_eq!(decode_winodbc_string("\\ 9"), "\t");
        assert_eq!(decode_winodbc_string("\\ d"), "\r");
        assert_eq!(
            decode_winodbc_string(
                "SELECT * REPLACE(\\ a  DATEADD('day', -1, TSLTZ) AS TSLTZ\\ a) \
                 FROM ALLDATATYPES;\\ 0"
            ),
            "SELECT * REPLACE(\n  DATEADD('day', -1, TSLTZ) AS TSLTZ\n) FROM ALLDATATYPES;",
        );
    }

    #[test]
    fn decode_string_handles_c_style_escapes_for_synthetic_traces() {
        assert_eq!(decode_winodbc_string("SELECT 1;\\0"), "SELECT 1;");
        assert_eq!(decode_winodbc_string("a\\nb"), "a\nb");
        assert_eq!(decode_winodbc_string("a\\\\b"), "a\\b");
        assert_eq!(decode_winodbc_string("a\\\"b"), "a\"b");
    }

    #[test]
    fn parse_extracts_multiline_sql_from_dm_escapes() {
        let trace = "proc-1 1234-5678\tENTER SQLAllocHandle\n\
             \t\tSQLSMALLINT                  1 <SQL_HANDLE_ENV>\n\
             \t\tSQLHANDLE           0x0000000000000000\n\
             \t\tSQLHANDLE *         0x0000018E00866BE0\n\
             \n\
             proc-1 1234-5678\tEXIT  SQLAllocHandle  with return code 0 (SQL_SUCCESS)\n\
             \t\tSQLSMALLINT                  1 <SQL_HANDLE_ENV>\n\
             \t\tSQLHANDLE           0x0000000000000000\n\
             \t\tSQLHANDLE *         0x0000018E00866BE0 ( 0x0000018E656DAC50)\n\
             \n\
             proc-1 1234-5678\tENTER SQLExecDirectW\n\
             \t\tHSTMT               0x0000018E656DA1E0\n\
             \t\tWCHAR *             0x0000018E006DF854 [      -3] \"SELECT *\\ a  FROM T;\\ 0\"\n\
             \t\tSDWORD                    -3\n\
             \n\
             proc-1 1234-5678\tEXIT  SQLExecDirectW  with return code 0 (SQL_SUCCESS)\n\
             \t\tHSTMT               0x0000018E656DA1E0\n\
             \t\tWCHAR *             0x0000018E006DF854 [      -3] \"SELECT *\\ a  FROM T;\\ 0\"\n\
             \t\tSDWORD                    -3\n";

        let parsed = parse_str(trace).expect("parse");
        let exec = parsed
            .calls
            .iter()
            .find_map(|c| match &c.call {
                OdbcCall::ExecDirect(e) => Some(e),
                _ => None,
            })
            .expect("exec direct");
        assert_eq!(exec.sql.as_deref(), Some("SELECT *\n  FROM T;"));
    }

    #[test]
    fn parses_string_values_containing_embedded_double_quote() {
        // The WinODBC DM doesn't escape `"` inside string values, so
        // `SQL_IDENTIFIER_QUOTE_CHAR == "\""` shows up on the wire as the
        // three-quote sequence `"""` and `SQL_COLUMN_ESCAPE_CHAR == "\""` as
        // `"""`. Make sure the regex picks up the inner character rather
        // than collapsing to an empty string.
        assert_eq!(
            parse_param_value(r#"0x0000015380842E08 [       2] """"#),
            ParamValue::StringValue {
                value: "\"".to_string(),
                truncated: false,
            },
        );
        // Sanity-check normal single-character strings still parse correctly.
        assert_eq!(
            parse_param_value(r#"0x0000015380843588 [       1] "." "#),
            ParamValue::StringValue {
                value: ".".to_string(),
                truncated: false,
            },
        );
        // Empty strings should still parse to an empty value.
        assert_eq!(
            parse_param_value(r#"0x0000015380843588 [       0] """#),
            ParamValue::StringValue {
                value: String::new(),
                truncated: false,
            },
        );
        // Backslash-quote escapes used by synthetic traces keep working.
        assert_eq!(
            parse_param_value(r#"0x0000015380843588 [       3] "a\"b""#),
            ParamValue::StringValue {
                value: "a\"b".to_string(),
                truncated: false,
            },
        );
    }

    #[test]
    fn parses_sql_get_info_numeric_outputs_via_all_three_winodbc_renderings() {
        // The WinODBC DM renders numeric `SQLGetInfo` outputs in three
        // mutually exclusive ways depending on whether the value is decoded
        // to a symbolic constant and how it's formatted. All three must end
        // up populating `GetInfo.info_value_numeric` so the generator can
        // emit a `CHECK(numericValue == ...)` assertion.
        let trace = "\
proc-1 1234-5678\tENTER SQLAllocHandle\n\
\t\tSQLSMALLINT                  1 <SQL_HANDLE_ENV>\n\
\t\tSQLHANDLE           0x0000000000000000\n\
\t\tSQLHANDLE *         0x0000000000000010\n\
\n\
proc-1 1234-5678\tEXIT  SQLAllocHandle  with return code 0 (SQL_SUCCESS)\n\
\t\tSQLSMALLINT                  1 <SQL_HANDLE_ENV>\n\
\t\tSQLHANDLE           0x0000000000000000\n\
\t\tSQLHANDLE *         0x0000000000000010 ( 0x0000000000000020)\n\
\n\
proc-1 1234-5678\tENTER SQLAllocHandle\n\
\t\tSQLSMALLINT                  2 <SQL_HANDLE_DBC>\n\
\t\tSQLHANDLE           0x0000000000000020\n\
\t\tSQLHANDLE *         0x0000000000000030\n\
\n\
proc-1 1234-5678\tEXIT  SQLAllocHandle  with return code 0 (SQL_SUCCESS)\n\
\t\tSQLSMALLINT                  2 <SQL_HANDLE_DBC>\n\
\t\tSQLHANDLE           0x0000000000000020\n\
\t\tSQLHANDLE *         0x0000000000000030 ( 0x0000000000000040)\n\
\n\
proc-1 1234-5678\tENTER SQLGetInfoW \n\
\t\tHDBC                0x0000000000000040\n\
\t\tUWORD                       91 <SQL_OWNER_USAGE>\n\
\t\tPTR                 0x0000000000000100\n\
\t\tSWORD                        4 \n\
\t\tSWORD *             0x0000000000000200\n\
\n\
proc-1 1234-5678\tEXIT  SQLGetInfoW  with return code 0 (SQL_SUCCESS)\n\
\t\tHDBC                0x0000000000000040\n\
\t\tUWORD                       91 <SQL_OWNER_USAGE>\n\
\t\tPTR                 0x0000000000000100 ( 0x0000000000000015)\n\
\t\tSWORD                        4 \n\
\t\tSWORD *             0x0000000000000200 (4)\n\
\n\
proc-1 1234-5678\tENTER SQLGetInfoW \n\
\t\tHDBC                0x0000000000000040\n\
\t\tUWORD                       99 <SQL_MAX_COLUMNS_IN_ORDER_BY>\n\
\t\tPTR                 0x0000000000000110\n\
\t\tSWORD                        4 \n\
\t\tSWORD *             0x0000000000000200\n\
\n\
proc-1 1234-5678\tEXIT  SQLGetInfoW  with return code 0 (SQL_SUCCESS)\n\
\t\tHDBC                0x0000000000000040\n\
\t\tUWORD                       99 <SQL_MAX_COLUMNS_IN_ORDER_BY>\n\
\t\tPTR                 0x0000000000000110 (65535)\n\
\t\tSWORD                        4 \n\
\t\tSWORD *             0x0000000000000200 (2)\n\
\n\
proc-1 1234-5678\tENTER SQLGetInfoW \n\
\t\tHDBC                0x0000000000000040\n\
\t\tUWORD                      114 <SQL_CATALOG_LOCATION>\n\
\t\tPTR                 0x0000000000000120\n\
\t\tSWORD                        4 \n\
\t\tSWORD *             0x0000000000000200\n\
\n\
proc-1 1234-5678\tEXIT  SQLGetInfoW  with return code 0 (SQL_SUCCESS)\n\
\t\tHDBC                0x0000000000000040\n\
\t\tUWORD                      114 <SQL_CATALOG_LOCATION>\n\
\t\tPTR                 0x0000000000000120 (1) <SQL_CL_START>\n\
\t\tSWORD                        4 \n\
\t\tSWORD *             0x0000000000000200 (2)\n";

        let parsed = parse_str(trace).expect("parse");
        let get_infos: Vec<_> = parsed
            .calls
            .iter()
            .filter_map(|c| match &c.call {
                OdbcCall::GetInfo(g) => Some(g),
                _ => None,
            })
            .collect();
        assert_eq!(get_infos.len(), 3, "expected three SQLGetInfo calls");

        // OutputAddress path: `0xPTR ( 0xVALUE)` where the value is hex.
        assert_eq!(get_infos[0].info_type.as_deref(), Some("SQL_OWNER_USAGE"),);
        assert_eq!(get_infos[0].info_value_numeric, Some(0x15));

        // OutputInteger path: `0xPTR (DECIMAL)` with no symbolic name.
        assert_eq!(
            get_infos[1].info_type.as_deref(),
            Some("SQL_MAX_COLUMNS_IN_ORDER_BY"),
        );
        assert_eq!(get_infos[1].info_value_numeric, Some(65535));

        // OutputNamedConstant path: `0xPTR (DECIMAL) <SQL_NAME>`.
        assert_eq!(
            get_infos[2].info_type.as_deref(),
            Some("SQL_CATALOG_LOCATION"),
        );
        assert_eq!(get_infos[2].info_value_numeric, Some(1));
    }

    #[test]
    fn parses_unknown_bracketed_tag_as_integer() {
        // Windows DM emits `<unknown>` (lowercase) for InfoTypes /
        // FieldIdentifiers that its symbol table doesn't recognize. The
        // parser must preserve the integer prefix - dropping it would
        // materialize as a different (real) ODBC call downstream because
        // every `unwrap_or(...)` default in the generator is itself a
        // valid ODBC value (`0` = `SQL_INFO_FIRST` etc.).
        assert_eq!(parse_param_value("169 <unknown>"), ParamValue::Integer(169),);
        // The same path also handles other non-SQL_ bracketed tags as
        // long as a parseable integer leads.
        assert_eq!(
            parse_param_value("180 <some-future-tag>"),
            ParamValue::Integer(180),
        );
        // `<unknown type>` and `<zero length>` appear *after* a `PTR
        // 0xADDR` and don't have a parseable integer prefix - those
        // continue to fall through to `Address` unchanged.
        assert_eq!(
            parse_param_value("0x0000000000000100 <unknown type>"),
            ParamValue::Address("0x0000000000000100 <unknown type>".to_string()),
        );
    }

    #[test]
    fn preserves_sql_owner_usage_named_constant_after_unknown_branch_added() {
        // Regression guard: the well-formed `<SQL_OWNER_USAGE>` branch
        // runs *before* the new `<unknown>` fallback, so legitimate
        // named constants must still produce `NamedConstant`, not get
        // demoted to `Integer`.
        assert_eq!(
            parse_param_value("91 <SQL_OWNER_USAGE>"),
            ParamValue::NamedConstant {
                value: Some(91),
                name: "SQL_OWNER_USAGE".to_string(),
            },
        );
    }

    #[test]
    fn unknown_info_type_surfaces_as_info_type_value_in_ir() {
        // End-to-end: `UWORD 169 <unknown>` for the InfoType parameter
        // must show up in `GetInfo.info_type_value`, not be silently
        // lost. The previous behaviour produced `info_type: null,
        // info_type_value: null`, which the generator then materialized
        // as the (different!) call `SQLGetInfo(dbc0, 0, ...)`.
        let trace = "\
proc-1 1234-5678\tENTER SQLAllocHandle\n\
\t\tSQLSMALLINT                  1 <SQL_HANDLE_ENV>\n\
\t\tSQLHANDLE           0x0000000000000000\n\
\t\tSQLHANDLE *         0x0000000000000010\n\
\n\
proc-1 1234-5678\tEXIT  SQLAllocHandle  with return code 0 (SQL_SUCCESS)\n\
\t\tSQLSMALLINT                  1 <SQL_HANDLE_ENV>\n\
\t\tSQLHANDLE           0x0000000000000000\n\
\t\tSQLHANDLE *         0x0000000000000010 ( 0x0000000000000020)\n\
\n\
proc-1 1234-5678\tENTER SQLAllocHandle\n\
\t\tSQLSMALLINT                  2 <SQL_HANDLE_DBC>\n\
\t\tSQLHANDLE           0x0000000000000020\n\
\t\tSQLHANDLE *         0x0000000000000030\n\
\n\
proc-1 1234-5678\tEXIT  SQLAllocHandle  with return code 0 (SQL_SUCCESS)\n\
\t\tSQLSMALLINT                  2 <SQL_HANDLE_DBC>\n\
\t\tSQLHANDLE           0x0000000000000020\n\
\t\tSQLHANDLE *         0x0000000000000030 ( 0x0000000000000040)\n\
\n\
proc-1 1234-5678\tENTER SQLGetInfoW \n\
\t\tHDBC                0x0000000000000040\n\
\t\tUWORD                      169 <unknown>\n\
\t\tPTR                 0x0000000000000100\n\
\t\tSWORD                        4 \n\
\t\tSWORD *             0x0000000000000200\n\
\n\
proc-1 1234-5678\tEXIT  SQLGetInfoW  with return code 0 (SQL_SUCCESS)\n\
\t\tHDBC                0x0000000000000040\n\
\t\tUWORD                      169 <unknown>\n\
\t\tPTR                 0x0000000000000100 ( 0x000000000000007F)\n\
\t\tSWORD                        4 \n\
\t\tSWORD *             0x0000000000000200 (4)\n";

        let parsed = parse_str(trace).expect("parse");
        let get_info = parsed
            .calls
            .iter()
            .find_map(|c| match &c.call {
                OdbcCall::GetInfo(g) => Some(g),
                _ => None,
            })
            .expect("expected one SQLGetInfo");

        assert_eq!(get_info.info_type, None, "no symbolic name to recover");
        assert_eq!(
            get_info.info_type_value,
            Some(169),
            "integer prefix must survive parse_param_value's `<unknown>` branch",
        );
    }

    #[test]
    fn pair_entries_rejects_missing_return_code() {
        // Build a trace whose EXIT block has no parseable return code
        // (drop the `with return code N (NAME)` suffix). Today this would
        // silently default to `SUCCESS` - the worst-case substitution
        // because a failing call would then be asserted as successful.
        // The new error path makes the parser refuse instead.
        let trace = "\
proc-1 1234-5678\tENTER SQLAllocHandle\n\
\t\tSQLSMALLINT                  1 <SQL_HANDLE_ENV>\n\
\t\tSQLHANDLE           0x0000000000000000\n\
\t\tSQLHANDLE *         0x0000000000000010\n\
\n\
proc-1 1234-5678\tEXIT  SQLAllocHandle\n\
\t\tSQLSMALLINT                  1 <SQL_HANDLE_ENV>\n\
\t\tSQLHANDLE           0x0000000000000000\n\
\t\tSQLHANDLE *         0x0000000000000010 ( 0x0000000000000020)\n";

        let err = parse_str(trace).expect_err("must reject missing return code");
        match err {
            WinOdbcParserError::MissingReturnCode { function, .. } => {
                assert_eq!(function, "SQLAllocHandle");
            }
            other => panic!("expected MissingReturnCode, got {other:?}"),
        }
    }
}
