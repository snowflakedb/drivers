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
}

type Result<T> = std::result::Result<T, WinOdbcParserError>;

pub fn parse_str(content: &str) -> Result<TraceLog> {
    if detect_winodbc_header(content).is_none() {
        return Err(WinOdbcParserError::InvalidFormat {
            location: Location::default(),
        });
    }

    let entries = parse_entries(content);
    let (calls, handle_graph) = pair_entries(entries);

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

static STRING_VALUE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?s).*?"((?:\\.|[^"\\])*)""#).unwrap());

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
        if let Some(name) = caps.get(3) {
            return ParamValue::OutputNamedConstant {
                address: caps[1].to_string(),
                name: name.as_str().to_string(),
            };
        }
        return ParamValue::OutputInteger {
            address: caps[1].to_string(),
            value: caps[2].parse().unwrap_or(0),
        };
    }

    if let Some(caps) = STRING_VALUE_RE.captures(trimmed) {
        let mut text = caps[1].to_string();
        // Windows DM traces NUL as `\ 0` at end of wide strings, e.g. `"SELECT 1;\ 0"`.
        if text.ends_with("\\ 0") {
            text.truncate(text.len() - 3);
        } else if text.ends_with(" 0") && text.contains(';') {
            text.truncate(text.len() - 2);
        }
        text = text.replace("\\0", "");
        if text.ends_with('\0') {
            text.pop();
        }
        return ParamValue::StringValue {
            value: text,
            truncated: false,
        };
    }

    if let Some(caps) = INT_NAMED_RE.captures(trimmed) {
        return ParamValue::NamedConstant {
            value: caps[1].parse().unwrap_or(0),
            name: caps[2].to_string(),
        };
    }

    if let Some(angle) = trimmed.split_once('<') {
        let name = angle.1.trim_end_matches('>').trim();
        if is_constant_name(name) {
            let value = angle.0.trim().parse::<i64>().unwrap_or(0);
            return ParamValue::NamedConstant {
                value,
                name: name.to_string(),
            };
        }
    }

    if HEX_ADDR_RE.is_match(trimmed) {
        return ParamValue::Address(trimmed.to_string());
    }

    if let Ok(v) = trimmed.parse::<i64>() {
        return ParamValue::Integer(v);
    }

    if is_constant_name(trimmed) {
        return ParamValue::NamedConstant {
            name: trimmed.to_string(),
            value: 0,
        };
    }

    ParamValue::Address(trimmed.to_string())
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
        let param_name = names.get(idx).copied().unwrap_or("").to_string();
        params.push(Parameter {
            type_name: if param_name.is_empty() {
                type_name
            } else {
                param_name
            },
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

fn pair_entries(entries: Vec<TraceEntry>) -> (Vec<TracedCall>, HandleGraph) {
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

                let return_code = entry.return_code.unwrap_or(ReturnCode::Success);
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

    (calls, handle_graph)
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
            ParamValue::NamedConstant { value, .. } => Some(*value),
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
}
