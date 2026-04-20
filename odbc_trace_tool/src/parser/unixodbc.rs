use std::sync::LazyLock;

use regex::Regex;
use snafu::prelude::*;
use snafu::Location;

use crate::model::{
    Direction, HandleGraph, HandleType, OdbcCall, ParamValue, Parameter, ReturnCode, TraceEntry,
    TraceFormat, TraceHeader, TraceLog, TracedCall,
};

#[derive(Snafu, Debug)]
pub enum UnixOdbcParserError {
    #[snafu(display("Failed to read trace file"))]
    FileRead {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Invalid trace format: not a unixODBC trace file"))]
    InvalidFormat {
        #[snafu(implicit)]
        location: Location,
    },
}

type Result<T> = std::result::Result<T, UnixOdbcParserError>;

pub fn parse_file(path: &std::path::Path) -> Result<TraceLog> {
    let content = std::fs::read_to_string(path).context(FileReadSnafu)?;
    parse_str(&content)
}

pub fn parse_str(content: &str) -> Result<TraceLog> {
    if !content.starts_with("[ODBC]") {
        return Err(UnixOdbcParserError::InvalidFormat {
            location: Location::default(),
        });
    }

    let entries = parse_entries(content);
    let (calls, handle_graph) = pair_entries(entries);

    let header = TraceHeader {
        format: TraceFormat::UnixOdbc,
        ..Default::default()
    };

    Ok(TraceLog {
        header,
        calls,
        handle_graph,
    })
}

struct RawBlock {
    thread_id: String,
    timestamp: String,
    source_file: String,
    body_lines: Vec<String>,
    line_number: usize,
}

static HEADER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\[ODBC\]\[(\d+)\]\[([^\]]+)\]\[([^\]]+)\]\[(\d+)\]").unwrap());

static EXIT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"Exit:\[(\w+)\]").unwrap());

static KV_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s+([\w][\w\s]*?)\s*=\s*(.+)$").unwrap());

static OUTPUT_PTR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(0x[0-9a-fA-F]+)\s*->\s*(-?\d+)(?:\s*\(\d+\s*bits?\))?$").unwrap()
});

static NAMED_CONST_PARENS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([A-Z_][A-Z_0-9]+)\s+\((-?\d+)\)$").unwrap());

static NAMED_CONST_PREFIX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(-?\d+)\s+([A-Z_][A-Z_0-9]+)$").unwrap());

static HEX_ADDR_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^0x[0-9a-fA-F]+$").unwrap());

static STRING_WITH_LEN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)^\[(.*)\]\[length\s*=\s*\d+.*\]$").unwrap());

static STRING_BARE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)^\[(.*)\]$").unwrap());

fn split_into_blocks(content: &str) -> Vec<RawBlock> {
    let header_re = &*HEADER_RE;
    let mut blocks = Vec::new();
    let mut current: Option<RawBlock> = None;

    for (idx, line) in content.lines().enumerate() {
        if let Some(caps) = header_re.captures(line) {
            if let Some(block) = current.take() {
                blocks.push(block);
            }
            current = Some(RawBlock {
                thread_id: caps[1].to_string(),
                timestamp: caps[2].to_string(),
                source_file: caps[3].to_string(),
                body_lines: Vec::new(),
                line_number: idx + 1,
            });
        } else if let Some(ref mut block) = current {
            block.body_lines.push(line.to_string());
        }
    }

    if let Some(block) = current {
        blocks.push(block);
    }

    blocks
}

fn function_name_from_source(source_file: &str) -> String {
    let name = source_file.strip_suffix(".c").unwrap_or(source_file);
    if name == "__handles" {
        "SQLAllocHandle".to_string()
    } else {
        name.to_string()
    }
}

fn parse_param_value(raw: &str) -> ParamValue {
    let trimmed = raw.trim();

    if trimmed == "(nil)" || trimmed == "0x0" {
        return ParamValue::NullPointer;
    }

    if let Some(caps) = OUTPUT_PTR_RE.captures(trimmed) {
        return ParamValue::OutputInteger {
            address: caps[1].to_string(),
            value: caps[2].parse().unwrap_or(0),
        };
    }

    if let Some(caps) = STRING_WITH_LEN_RE.captures(trimmed) {
        let text = caps[1].to_string();
        let truncated = text.ends_with("...");
        return ParamValue::StringValue {
            value: text,
            truncated,
        };
    }

    if let Some(caps) = STRING_BARE_RE.captures(trimmed) {
        let text = caps[1].to_string();
        let truncated = text.ends_with("...");
        return ParamValue::StringValue {
            value: text,
            truncated,
        };
    }

    if let Some(caps) = NAMED_CONST_PARENS_RE.captures(trimmed) {
        return ParamValue::NamedConstant {
            name: caps[1].to_string(),
            value: caps[2].parse().unwrap_or(0),
        };
    }

    if let Some(caps) = NAMED_CONST_PREFIX_RE.captures(trimmed) {
        return ParamValue::NamedConstant {
            value: caps[1].parse().unwrap_or(0),
            name: caps[2].to_string(),
        };
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

fn has_closing_bracket(s: &str) -> bool {
    let mut depth = 0i32;
    for ch in s.chars() {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn parse_body(body_lines: &[String]) -> (Direction, Option<ReturnCode>, Vec<Parameter>) {
    let exit_re = &*EXIT_RE;
    let kv_re = &*KV_RE;

    let mut direction = Direction::Enter;
    let mut return_code = None;
    let mut params = Vec::new();

    let mut i = 0;
    while i < body_lines.len() {
        let line = body_lines[i].trim();

        if line.is_empty() {
            i += 1;
            continue;
        }

        if line == "Entry:" {
            direction = Direction::Enter;
            i += 1;
            continue;
        }

        if let Some(caps) = exit_re.captures(line) {
            direction = Direction::Exit;
            return_code = ReturnCode::from_name(&caps[1]);
            i += 1;
            continue;
        }

        if let Some(caps) = kv_re.captures(&body_lines[i]) {
            let key = caps[1].trim().to_string();
            let raw_value = caps[2].trim().to_string();

            if raw_value.starts_with('[') && !has_closing_bracket(&raw_value) {
                let mut full_value = raw_value.clone();
                while i + 1 < body_lines.len() {
                    i += 1;
                    let next = &body_lines[i];
                    full_value.push('\n');
                    full_value.push_str(next);
                    if has_closing_bracket(&full_value) {
                        break;
                    }
                }
                params.push(Parameter {
                    type_name: key,
                    value: parse_param_value(&full_value),
                });
            } else {
                params.push(Parameter {
                    type_name: key,
                    value: parse_param_value(&raw_value),
                });
            }
        }

        i += 1;
    }

    (direction, return_code, params)
}

fn parse_entries(content: &str) -> Vec<TraceEntry> {
    let blocks = split_into_blocks(content);
    let mut entries = Vec::with_capacity(blocks.len());

    for block in blocks {
        let function_name = function_name_from_source(&block.source_file);
        let (direction, return_code, parameters) = parse_body(&block.body_lines);

        entries.push(TraceEntry {
            timestamp: block.timestamp,
            thread_id: Some(block.thread_id),
            direction,
            function_name,
            return_code,
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
                if entry.function_name == "SQLAllocHandle" && return_code.is_success() {
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
    if input_params.is_empty() {
        // Implicit env creation from __handles.c (Exit-only, no Entry).
        if let Some(addr) = find_param_addr(output_params, "Environment") {
            graph.register_alloc(HandleType::Env, "SQL_NULL_HANDLE", &addr);
        }
        return;
    }

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
            _ => None,
        })
}

fn find_param_addr(params: &[Parameter], key: &str) -> Option<String> {
    params
        .iter()
        .find(|p| p.type_name == key)
        .and_then(|p| match &p.value {
            ParamValue::Address(a) => Some(a.clone()),
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ExecDirect, SetEnvAttr};

    const SAMPLE_TRACE: &str = "\
[ODBC][118][1774615098.017111][__handles.c][499]
\t\tExit:[SQL_SUCCESS]
\t\t\tEnvironment = 0x2e91620
[ODBC][118][1774615098.017167][SQLSetEnvAttr.c][189]
\t\tEntry:
\t\t\tEnvironment = 0x2e91620
\t\t\tAttribute = SQL_ATTR_ODBC_VERSION
\t\t\tValue = 0x17c
\t\t\tStrLen = 0
[ODBC][118][1774615098.017193][SQLSetEnvAttr.c][381]
\t\tExit:[SQL_SUCCESS]
[ODBC][118][1774615098.017216][SQLAllocHandle.c][395]
\t\tEntry:
\t\t\tHandle Type = 2
\t\t\tInput Handle = 0x2e91620
\t\tUNICODE Using encoding ASCII 'UTF-8' and UNICODE 'UTF16LE'

[ODBC][118][1774615098.017363][SQLAllocHandle.c][531]
\t\tExit:[SQL_SUCCESS]
\t\t\tOutput Handle = 0x2e92330
[ODBC][118][1774615098.017499][SQLDriverConnect.c][751]
\t\tEntry:
\t\t\tConnection = 0x2e92330
\t\t\tWindow Hdl = (nil)
\t\t\tStr In = [Driver=TestDriver;SERVER=test.snowflakecomputing.com][length = 52]
\t\t\tStr Out = 0x7ffc81545ea0
\t\t\tStr Out Max = 1024
\t\t\tStr Out Ptr = 0x7ffc81545e8a
\t\t\tCompletion = 0
[ODBC][118][1774615098.123288][SQLDriverConnect.c][1809]
\t\tExit:[SQL_SUCCESS]
[ODBC][118][1774615098.163786][SQLAllocHandle.c][578]
\t\tEntry:
\t\t\tHandle Type = 3
\t\t\tInput Handle = 0x2e92330
[ODBC][118][1774615098.163973][SQLAllocHandle.c][1123]
\t\tExit:[SQL_SUCCESS]
\t\t\tOutput Handle = 0x3086810
[ODBC][118][1774615098.164003][SQLExecDirect.c][240]
\t\tEntry:
\t\t\tStatement = 0x3086810
\t\t\tSQL = [SELECT 1][length = 8 (SQL_NTS)]
[ODBC][118][1774615098.237948][SQLExecDirect.c][521]
\t\tExit:[SQL_SUCCESS]
[ODBC][118][1774615098.238178][SQLFetch.c][162]
\t\tEntry:
\t\t\tStatement = 0x3086810
[ODBC][118][1774615098.238253][SQLFetch.c][352]
\t\tExit:[SQL_NO_DATA]
[ODBC][118][1774615098.238366][SQLFreeHandle.c][387]
\t\tEntry:
\t\t\tHandle Type = 3
\t\t\tInput Handle = 0x3086810
[ODBC][118][1774615098.238473][SQLFreeHandle.c][490]
\t\tExit:[SQL_SUCCESS]
[ODBC][118][1774615098.509432][SQLDisconnect.c][208]
\t\tEntry:
\t\t\tConnection = 0x2e92330
[ODBC][118][1774615098.511068][SQLDisconnect.c][358]
\t\tExit:[SQL_SUCCESS]
[ODBC][118][1774615098.511124][SQLFreeHandle.c][290]
\t\tEntry:
\t\t\tHandle Type = 2
\t\t\tInput Handle = 0x2e92330
[ODBC][118][1774615098.511160][SQLFreeHandle.c][339]
\t\tExit:[SQL_SUCCESS]
[ODBC][118][1774615098.511189][SQLFreeHandle.c][220]
\t\tEntry:
\t\t\tHandle Type = 1
\t\t\tInput Handle = 0x2e91620
[ODBC][118][1774615098.511200][SQLFreeHandle.c][250]
\t\tExit:[SQL_SUCCESS]
";

    #[test]
    fn test_split_into_blocks() {
        let blocks = split_into_blocks(SAMPLE_TRACE);
        assert!(
            blocks.len() >= 20,
            "Expected >=20 blocks, got {}",
            blocks.len()
        );
        assert_eq!(blocks[0].thread_id, "118");
        assert_eq!(blocks[0].source_file, "__handles.c");
    }

    #[test]
    fn test_function_name_from_source() {
        assert_eq!(
            function_name_from_source("SQLAllocHandle.c"),
            "SQLAllocHandle"
        );
        assert_eq!(function_name_from_source("__handles.c"), "SQLAllocHandle");
        assert_eq!(
            function_name_from_source("SQLDriverConnect.c"),
            "SQLDriverConnect"
        );
    }

    #[test]
    fn test_parse_param_value_nil() {
        assert_eq!(parse_param_value("(nil)"), ParamValue::NullPointer);
        assert_eq!(parse_param_value("0x0"), ParamValue::NullPointer);
    }

    #[test]
    fn test_parse_param_value_output_ptr() {
        assert_eq!(
            parse_param_value("0x7ffc8154786e -> 2"),
            ParamValue::OutputInteger {
                address: "0x7ffc8154786e".to_string(),
                value: 2,
            }
        );
        assert_eq!(
            parse_param_value("0x7ffc815476e8 -> 256 (64 bits)"),
            ParamValue::OutputInteger {
                address: "0x7ffc815476e8".to_string(),
                value: 256,
            }
        );
    }

    #[test]
    fn test_parse_param_value_string_with_length() {
        assert_eq!(
            parse_param_value("[SELECT 1][length = 8 (SQL_NTS)]"),
            ParamValue::StringValue {
                value: "SELECT 1".to_string(),
                truncated: false
            }
        );
    }

    #[test]
    fn test_parse_param_value_string_bare() {
        assert_eq!(
            parse_param_value("[SCHEMANAME]"),
            ParamValue::StringValue {
                value: "SCHEMANAME".to_string(),
                truncated: false
            }
        );
    }

    #[test]
    fn test_parse_param_value_named_const_parens() {
        assert_eq!(
            parse_param_value("SQL_DBMS_NAME (17)"),
            ParamValue::NamedConstant {
                name: "SQL_DBMS_NAME".to_string(),
                value: 17,
            }
        );
    }

    #[test]
    fn test_parse_param_value_named_const_prefix() {
        assert_eq!(
            parse_param_value("1 SQL_CHAR"),
            ParamValue::NamedConstant {
                value: 1,
                name: "SQL_CHAR".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_param_value_address() {
        assert_eq!(
            parse_param_value("0x2e91620"),
            ParamValue::Address("0x2e91620".to_string())
        );
    }

    #[test]
    fn test_parse_param_value_integer() {
        assert_eq!(parse_param_value("256"), ParamValue::Integer(256));
        assert_eq!(parse_param_value("0"), ParamValue::Integer(0));
    }

    #[test]
    fn test_parse_param_value_named_const_bare() {
        assert_eq!(
            parse_param_value("SQL_ATTR_ODBC_VERSION"),
            ParamValue::NamedConstant {
                name: "SQL_ATTR_ODBC_VERSION".to_string(),
                value: 0,
            }
        );
    }

    #[test]
    fn test_parse_full_trace() {
        let trace = parse_str(SAMPLE_TRACE).expect("Failed to parse sample trace");
        assert_eq!(trace.header.format, TraceFormat::UnixOdbc);

        let names: Vec<&str> = trace.calls.iter().map(|c| c.call.function_name()).collect();
        assert!(names.contains(&"SQLAllocHandle"), "missing SQLAllocHandle");
        assert!(names.contains(&"SQLSetEnvAttr"), "missing SQLSetEnvAttr");
        assert!(
            names.contains(&"SQLDriverConnect"),
            "missing SQLDriverConnect"
        );
        assert!(names.contains(&"SQLExecDirect"), "missing SQLExecDirect");
        assert!(names.contains(&"SQLFetch"), "missing SQLFetch");
        assert!(names.contains(&"SQLFreeHandle"), "missing SQLFreeHandle");
        assert!(names.contains(&"SQLDisconnect"), "missing SQLDisconnect");
    }

    #[test]
    fn test_parse_full_trace_return_codes() {
        let trace = parse_str(SAMPLE_TRACE).expect("Failed to parse");

        let fetch = trace
            .calls
            .iter()
            .find(|c| c.call.function_name() == "SQLFetch")
            .unwrap();
        assert_eq!(fetch.call.return_code(), ReturnCode::NoData);

        let exec = trace
            .calls
            .iter()
            .find(|c| c.call.function_name() == "SQLExecDirect")
            .unwrap();
        assert_eq!(exec.call.return_code(), ReturnCode::Success);
    }

    #[test]
    fn test_parse_full_trace_handle_graph() {
        let trace = parse_str(SAMPLE_TRACE).expect("Failed to parse");

        let env = trace.handle_graph.get("0x2e91620");
        assert!(env.is_some(), "env handle should be registered");
        assert_eq!(env.unwrap().handle_type, HandleType::Env);
        assert_eq!(env.unwrap().logical_name, "env0");

        let dbc = trace.handle_graph.get("0x2e92330");
        assert!(dbc.is_some(), "dbc handle should be registered");
        assert_eq!(dbc.unwrap().handle_type, HandleType::Dbc);
        assert_eq!(dbc.unwrap().parent_address.as_deref(), Some("0x2e91620"));

        let stmt = trace.handle_graph.get("0x3086810");
        assert!(stmt.is_some(), "stmt handle should be registered");
        assert_eq!(stmt.unwrap().handle_type, HandleType::Stmt);
        assert_eq!(stmt.unwrap().parent_address.as_deref(), Some("0x2e92330"));
    }

    #[test]
    fn test_parse_full_trace_string_values() {
        let trace = parse_str(SAMPLE_TRACE).expect("Failed to parse");

        let exec = trace
            .calls
            .iter()
            .find(|c| c.call.function_name() == "SQLExecDirect")
            .unwrap();
        let sql = match &exec.call {
            OdbcCall::ExecDirect(ExecDirect { sql, .. }) => sql.as_deref(),
            _ => None,
        };
        assert_eq!(sql, Some("SELECT 1"));
    }

    #[test]
    fn test_parse_multiline_string() {
        let trace_with_multiline = "\
[ODBC][100][1774615098.100000][__handles.c][499]
\t\tExit:[SQL_SUCCESS]
\t\t\tEnvironment = 0xaaa
[ODBC][100][1774615098.200000][SQLAllocHandle.c][395]
\t\tEntry:
\t\t\tHandle Type = 2
\t\t\tInput Handle = 0xaaa
[ODBC][100][1774615098.200001][SQLAllocHandle.c][531]
\t\tExit:[SQL_SUCCESS]
\t\t\tOutput Handle = 0xbbb
[ODBC][100][1774615098.300000][SQLAllocHandle.c][578]
\t\tEntry:
\t\t\tHandle Type = 3
\t\t\tInput Handle = 0xbbb
[ODBC][100][1774615098.300001][SQLAllocHandle.c][1123]
\t\tExit:[SQL_SUCCESS]
\t\t\tOutput Handle = 0xccc
[ODBC][100][1774615098.400000][SQLExecDirect.c][240]
\t\tEntry:
\t\t\tStatement = 0xccc
\t\t\tSQL = [SELECT
  1
  FROM dual][length = 22 (SQL_NTS)]
[ODBC][100][1774615098.500000][SQLExecDirect.c][521]
\t\tExit:[SQL_SUCCESS]
";
        let trace = parse_str(trace_with_multiline).expect("Failed to parse");
        let exec = trace
            .calls
            .iter()
            .find(|c| c.call.function_name() == "SQLExecDirect")
            .unwrap();
        let sql = match &exec.call {
            OdbcCall::ExecDirect(ExecDirect { sql, .. }) => sql.as_deref(),
            _ => None,
        };
        assert_eq!(sql, Some("SELECT\n  1\n  FROM dual"));
    }

    #[test]
    fn test_parse_multithreaded_pairing() {
        let trace = "\
[ODBC][100][1774615098.100000][__handles.c][499]
\t\tExit:[SQL_SUCCESS]
\t\t\tEnvironment = 0xaaa
[ODBC][200][1774615098.100001][__handles.c][499]
\t\tExit:[SQL_SUCCESS]
\t\t\tEnvironment = 0xbbb
[ODBC][100][1774615098.200000][SQLSetEnvAttr.c][189]
\t\tEntry:
\t\t\tEnvironment = 0xaaa
\t\t\tAttribute = SQL_ATTR_ODBC_VERSION
\t\t\tValue = 0x17c
\t\t\tStrLen = 0
[ODBC][200][1774615098.200001][SQLSetEnvAttr.c][189]
\t\tEntry:
\t\t\tEnvironment = 0xbbb
\t\t\tAttribute = SQL_ATTR_ODBC_VERSION
\t\t\tValue = 0x17c
\t\t\tStrLen = 0
[ODBC][200][1774615098.300000][SQLSetEnvAttr.c][381]
\t\tExit:[SQL_SUCCESS]
[ODBC][100][1774615098.300001][SQLSetEnvAttr.c][381]
\t\tExit:[SQL_SUCCESS]
";
        let trace = parse_str(trace).expect("Failed to parse");

        let set_attrs: Vec<_> = trace
            .calls
            .iter()
            .filter(|c| c.call.function_name() == "SQLSetEnvAttr")
            .collect();
        assert_eq!(set_attrs.len(), 2);

        // Thread 200's Exit came first, so it should pair with thread 200's Enter (env 0xbbb)
        let first = &set_attrs[0];
        let env_addr = match &first.call {
            OdbcCall::SetEnvAttr(SetEnvAttr { handle, .. }) => handle.as_deref(),
            _ => None,
        };
        assert_eq!(env_addr, Some("0xbbb"));
    }
}
