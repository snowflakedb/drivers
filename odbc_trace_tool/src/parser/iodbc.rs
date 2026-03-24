use std::sync::LazyLock;

use regex::Regex;
use snafu::prelude::*;
use snafu::Location;

use crate::model::{
    Direction, HandleGraph, HandleType, OdbcCall, ParamValue, Parameter, ReturnCode, TraceEntry,
    TraceFormat, TraceHeader, TraceLog,
};

#[derive(Snafu, Debug)]
pub enum IodbcParserError {
    #[snafu(display("Failed to read trace file"))]
    FileRead {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Invalid trace header: not an iODBC trace file"))]
    InvalidHeader {
        #[snafu(implicit)]
        location: Location,
    },
}

type Result<T> = std::result::Result<T, IodbcParserError>;

pub fn parse_file(path: &std::path::Path) -> Result<TraceLog> {
    let content = std::fs::read_to_string(path).context(FileReadSnafu)?;
    parse_str(&content)
}

pub fn parse_str(content: &str) -> Result<TraceLog> {
    let header = parse_header(content)?;
    let entries = parse_entries(content);
    let (calls, handle_graph) = pair_entries(entries);

    Ok(TraceLog {
        header,
        calls,
        handle_graph,
    })
}

fn parse_header(content: &str) -> Result<TraceHeader> {
    let lines: Vec<&str> = content.lines().collect();

    if lines.is_empty() || !lines[0].starts_with("** iODBC Trace file") {
        return Err(IodbcParserError::InvalidHeader {
            location: Location::default(),
        });
    }

    let mut header = TraceHeader {
        format: TraceFormat::IOdbc,
        ..Default::default()
    };

    for line in &lines[1..] {
        if line.starts_with("** Trace started on ") {
            header.started = Some(line.trim_start_matches("** Trace started on ").to_string());
        } else if line.starts_with("** Driver Manager: ") {
            header.driver_manager_version =
                Some(line.trim_start_matches("** Driver Manager: ").to_string());
        } else if !line.starts_with("**") {
            break;
        }
    }

    Ok(header)
}

struct RawBlock {
    timestamp: String,
    header_line: String,
    param_lines: Vec<String>,
}

static TIMESTAMP_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\[(\d{6}\.\d{6})\]$").unwrap());

fn split_into_blocks(content: &str) -> Vec<RawBlock> {
    let timestamp_re = &*TIMESTAMP_RE;
    let mut blocks = Vec::new();
    let mut lines = content.lines().peekable();

    while let Some(line) = lines.next() {
        if let Some(caps) = timestamp_re.captures(line) {
            let timestamp = caps[1].to_string();

            let header_line = match lines.next() {
                Some(h) => h.to_string(),
                None => continue,
            };

            let mut param_lines = Vec::new();
            while let Some(peeked) = lines.peek() {
                if peeked.is_empty() || timestamp_re.is_match(peeked) || peeked.starts_with("**") {
                    break;
                }
                param_lines.push(lines.next().unwrap().to_string());
            }

            blocks.push(RawBlock {
                timestamp,
                header_line,
                param_lines,
            });
        }
    }

    blocks
}

struct HeaderLine {
    direction: Direction,
    function_name: String,
    return_info: Option<(i32, String)>,
}

static EXIT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\S+\s+\S+\s+EXIT\s+(\w+)\s+with return code (-?\d+)\s+\((\w+)\)").unwrap()
});

static ENTER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\S+\s+\S+\s+ENTER\s+(\w+)").unwrap());

fn parse_header_line(line: &str) -> Option<HeaderLine> {
    if let Some(caps) = EXIT_RE.captures(line) {
        let func = caps[1].to_string();
        let code: i32 = caps[2].parse().unwrap_or(0);
        let name = caps[3].to_string();
        return Some(HeaderLine {
            direction: Direction::Exit,
            function_name: func,
            return_info: Some((code, name)),
        });
    }

    if let Some(caps) = ENTER_RE.captures(line) {
        let func = caps[1].to_string();
        return Some(HeaderLine {
            direction: Direction::Enter,
            function_name: func,
            return_info: None,
        });
    }

    None
}

static ADDR_OUT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(0x[0-9a-fA-F]+)\s+\((0x[0-9a-fA-F]+)\)$").unwrap());

static ADDR_NAMED_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(0x[0-9a-fA-F]+)\s+\(([A-Z_][A-Z_0-9]+)\)$").unwrap());

static ADDR_INT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(0x[0-9a-fA-F]+)\s+\((-?\d+)\)$").unwrap());

static PLAIN_ADDR_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^0x[0-9a-fA-F]+$").unwrap());

static INT_NAMED_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(-?\d+)\s+\(([A-Z_][A-Z_0-9]+)\)$").unwrap());

fn parse_param_value(raw_value: &str) -> ParamValue {
    let trimmed = raw_value.trim();

    if trimmed == "0x0" {
        return ParamValue::NullPointer;
    }

    if let Some(caps) = ADDR_OUT_RE.captures(trimmed) {
        return ParamValue::OutputAddress {
            address: caps[1].to_string(),
            output_address: caps[2].to_string(),
        };
    }

    if let Some(caps) = ADDR_NAMED_RE.captures(trimmed) {
        return ParamValue::OutputNamedConstant {
            address: caps[1].to_string(),
            name: caps[2].to_string(),
        };
    }

    if let Some(caps) = ADDR_INT_RE.captures(trimmed) {
        return ParamValue::OutputInteger {
            address: caps[1].to_string(),
            value: caps[2].parse().unwrap_or(0),
        };
    }

    if PLAIN_ADDR_RE.is_match(trimmed) {
        return ParamValue::Address(trimmed.to_string());
    }

    if let Some(caps) = INT_NAMED_RE.captures(trimmed) {
        return ParamValue::NamedConstant {
            value: caps[1].parse().unwrap_or(0),
            name: caps[2].to_string(),
        };
    }

    if let Ok(v) = trimmed.parse::<i64>() {
        return ParamValue::Integer(v);
    }

    ParamValue::Address(trimmed.to_string())
}

static PTR_PARAM_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\t\t(\w+)\s+\*\s+(.+)$").unwrap());

static VAL_PARAM_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\t\t(\w+)\s{2,}(.+)$").unwrap());

static STRING_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\t+\s*\|\s*(.*?)\s*\|$").unwrap());

fn parse_parameters(param_lines: &[String]) -> Vec<Parameter> {
    let mut params = Vec::new();
    let ptr_param_re = &*PTR_PARAM_RE;
    let val_param_re = &*VAL_PARAM_RE;
    let string_re = &*STRING_RE;

    let mut i = 0;
    while i < param_lines.len() {
        let line = &param_lines[i];

        let (type_name, raw_value) = if let Some(caps) = ptr_param_re.captures(line) {
            (format!("{} *", caps[1].trim()), caps[2].trim().to_string())
        } else if let Some(caps) = val_param_re.captures(line) {
            (caps[1].trim().to_string(), caps[2].trim().to_string())
        } else {
            i += 1;
            continue;
        };

        if i + 1 < param_lines.len() {
            if let Some(str_caps) = string_re.captures(&param_lines[i + 1]) {
                let string_val = str_caps[1].trim().to_string();
                params.push(Parameter {
                    type_name,
                    value: ParamValue::StringValue(string_val),
                });
                i += 2;
                continue;
            }
        }

        params.push(Parameter {
            type_name,
            value: parse_param_value(&raw_value),
        });

        i += 1;
    }

    params
}

fn parse_entries(content: &str) -> Vec<TraceEntry> {
    let blocks = split_into_blocks(content);
    let mut entries = Vec::new();

    for block in blocks {
        let Some(header) = parse_header_line(&block.header_line) else {
            continue;
        };

        let (return_code, return_code_raw) = match header.return_info {
            Some((code, ref name)) => (ReturnCode::from_code_and_name(code, name), Some(code)),
            None => (None, None),
        };

        let parameters = parse_parameters(&block.param_lines);

        entries.push(TraceEntry {
            timestamp: block.timestamp,
            direction: header.direction,
            function_name: header.function_name,
            return_code,
            return_code_raw,
            parameters,
        });
    }

    entries
}

fn pair_entries(entries: Vec<TraceEntry>) -> (Vec<OdbcCall>, HandleGraph) {
    let mut calls = Vec::new();
    let mut handle_graph = HandleGraph::new();
    let mut pending_enters: Vec<TraceEntry> = Vec::new();

    for entry in entries {
        match entry.direction {
            Direction::Enter => {
                pending_enters.push(entry);
            }
            Direction::Exit => {
                let enter_idx = pending_enters
                    .iter()
                    .rposition(|e| e.function_name == entry.function_name);

                let input_params = if let Some(idx) = enter_idx {
                    let enter = pending_enters.remove(idx);
                    enter.parameters
                } else {
                    Vec::new()
                };

                let return_code = entry.return_code.unwrap_or(ReturnCode::Success);

                if entry.function_name == "SQLAllocHandle" && return_code.is_success() {
                    register_alloc_from_params(&entry.parameters, &mut handle_graph);
                }

                calls.push(OdbcCall {
                    function_name: entry.function_name,
                    input_params,
                    output_params: entry.parameters,
                    return_code,
                });
            }
        }
    }

    (calls, handle_graph)
}

fn register_alloc_from_params(params: &[Parameter], graph: &mut HandleGraph) {
    if params.len() < 3 {
        return;
    }

    let handle_type_value = match &params[0].value {
        ParamValue::NamedConstant { value, .. } => *value,
        ParamValue::Integer(v) => *v,
        _ => return,
    };

    let Some(handle_type) = HandleType::from_value(handle_type_value) else {
        return;
    };

    let parent_address = match &params[1].value {
        ParamValue::Address(a) => a.clone(),
        _ => return,
    };

    let child_address = match &params[2].value {
        ParamValue::OutputAddress { output_address, .. } => output_address.clone(),
        _ => return,
    };

    graph.register_alloc(handle_type, &parent_address, &child_address);
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_TRACE: &str =
        include_str!("../../../odbc_tests/tests/replay/iodbctest/select_1.log");

    #[test]
    fn test_parse_header_line_exit() {
        let line = "iodbctest       20108F080 EXIT  SQLDriverConnect with return code 1 (SQL_SUCCESS_WITH_INFO)";
        let h = parse_header_line(line).unwrap();
        assert_eq!(h.direction, Direction::Exit);
        assert_eq!(h.function_name, "SQLDriverConnect");
        let (code, name) = h.return_info.unwrap();
        assert_eq!(code, 1);
        assert_eq!(name, "SQL_SUCCESS_WITH_INFO");
    }

    #[test]
    fn test_parse_header_line_enter() {
        let line = "iodbctest       20108F080 ENTER SQLGetInfo";
        let h = parse_header_line(line).unwrap();
        assert_eq!(h.direction, Direction::Enter);
        assert_eq!(h.function_name, "SQLGetInfo");
        assert!(h.return_info.is_none());
    }

    #[test]
    fn test_parse_param_value_address() {
        assert_eq!(
            parse_param_value("0x104ee5c60"),
            ParamValue::Address("0x104ee5c60".to_string())
        );
    }

    #[test]
    fn test_parse_param_value_null() {
        assert_eq!(parse_param_value("0x0"), ParamValue::NullPointer);
    }

    #[test]
    fn test_parse_param_value_integer() {
        assert_eq!(parse_param_value("255"), ParamValue::Integer(255));
        assert_eq!(parse_param_value("-3"), ParamValue::Integer(-3));
    }

    #[test]
    fn test_parse_param_value_named_constant() {
        assert_eq!(
            parse_param_value("3 (SQL_HANDLE_STMT)"),
            ParamValue::NamedConstant {
                value: 3,
                name: "SQL_HANDLE_STMT".to_string()
            }
        );
    }

    #[test]
    fn test_parse_param_value_output_integer() {
        assert_eq!(
            parse_param_value("0x16b5980ec (1)"),
            ParamValue::OutputInteger {
                address: "0x16b5980ec".to_string(),
                value: 1
            }
        );
    }

    #[test]
    fn test_parse_param_value_output_named() {
        assert_eq!(
            parse_param_value("0x16b5980ea (SQL_DECIMAL)"),
            ParamValue::OutputNamedConstant {
                address: "0x16b5980ea".to_string(),
                name: "SQL_DECIMAL".to_string()
            }
        );
    }

    #[test]
    fn test_parse_param_value_output_address() {
        assert_eq!(
            parse_param_value("0x1048a9010 (0x104956bc0)"),
            ParamValue::OutputAddress {
                address: "0x1048a9010".to_string(),
                output_address: "0x104956bc0".to_string()
            }
        );
    }

    #[test]
    fn test_parse_full_trace_header() {
        let trace = parse_str(SAMPLE_TRACE).expect("Failed to parse sample trace");
        assert_eq!(trace.header.format, TraceFormat::IOdbc);
        assert_eq!(
            trace.header.started.as_deref(),
            Some("Sun Mar 22 19:13:50 2026")
        );
        assert_eq!(
            trace.header.driver_manager_version.as_deref(),
            Some("03.52.1623.0502")
        );
    }

    #[test]
    fn test_parse_full_trace_call_count() {
        let trace = parse_str(SAMPLE_TRACE).expect("Failed to parse sample trace");
        assert!(
            trace.calls.len() >= 19,
            "Expected at least 19 calls, got {}",
            trace.calls.len()
        );
    }

    #[test]
    fn test_parse_full_trace_function_names() {
        let trace = parse_str(SAMPLE_TRACE).expect("Failed to parse sample trace");
        let names: Vec<&str> = trace
            .calls
            .iter()
            .map(|c| c.function_name.as_str())
            .collect();

        assert_eq!(names[0], "SQLDriverConnect");
        assert!(names.contains(&"SQLGetDiagRec"));
        assert!(names.contains(&"SQLGetInfo"));
        assert!(names.contains(&"SQLAllocHandle"));
        assert!(names.contains(&"SQLPrepare"));
        assert!(names.contains(&"SQLExecute"));
        assert!(names.contains(&"SQLNumResultCols"));
        assert!(names.contains(&"SQLDescribeCol"));
        assert!(names.contains(&"SQLFetchScroll"));
        assert!(names.contains(&"SQLGetData"));
        assert!(names.contains(&"SQLMoreResults"));
        assert!(names.contains(&"SQLCloseCursor"));
        assert!(names.contains(&"SQLFreeHandle"));
        assert!(names.contains(&"SQLDisconnect"));
    }

    #[test]
    fn test_parse_full_trace_return_codes() {
        let trace = parse_str(SAMPLE_TRACE).expect("Failed to parse sample trace");

        assert_eq!(trace.calls[0].return_code, ReturnCode::SuccessWithInfo);

        let prepare = trace
            .calls
            .iter()
            .find(|c| c.function_name == "SQLPrepare")
            .unwrap();
        assert_eq!(prepare.return_code, ReturnCode::Success);

        let fetch_scrolls: Vec<_> = trace
            .calls
            .iter()
            .filter(|c| c.function_name == "SQLFetchScroll")
            .collect();
        assert_eq!(fetch_scrolls.len(), 2);
        assert_eq!(fetch_scrolls[0].return_code, ReturnCode::Success);
        assert_eq!(fetch_scrolls[1].return_code, ReturnCode::NoData);

        let close = trace
            .calls
            .iter()
            .find(|c| c.function_name == "SQLCloseCursor")
            .unwrap();
        assert_eq!(close.return_code, ReturnCode::Error);
    }

    #[test]
    fn test_parse_full_trace_string_values() {
        let trace = parse_str(SAMPLE_TRACE).expect("Failed to parse sample trace");

        let prepare = trace
            .calls
            .iter()
            .find(|c| c.function_name == "SQLPrepare")
            .unwrap();
        let sql = prepare.input_params.iter().find_map(|p| match &p.value {
            ParamValue::StringValue(s) => Some(s.as_str()),
            _ => None,
        });
        assert_eq!(sql, Some("SELECT 1"));

        let get_data = trace
            .calls
            .iter()
            .find(|c| c.function_name == "SQLGetData")
            .unwrap();
        let data = get_data.output_params.iter().find_map(|p| match &p.value {
            ParamValue::StringValue(s) => Some(s.as_str()),
            _ => None,
        });
        assert_eq!(data, Some("1"));
    }

    #[test]
    fn test_parse_full_trace_handle_graph() {
        let trace = parse_str(SAMPLE_TRACE).expect("Failed to parse sample trace");

        let stmt_handle = trace
            .handle_graph
            .handles
            .values()
            .find(|h| h.handle_type == HandleType::Stmt);
        assert!(stmt_handle.is_some(), "STMT handle should be registered");

        let stmt = stmt_handle.unwrap();
        assert_eq!(stmt.address, "0x104956bc0");
        assert_eq!(stmt.logical_name, "stmt0");
    }

    #[test]
    fn test_parse_full_trace_pointer_type_params() {
        let trace = parse_str(SAMPLE_TRACE).expect("Failed to parse sample trace");

        let num_cols = trace
            .calls
            .iter()
            .find(|c| c.function_name == "SQLNumResultCols")
            .unwrap();
        let count = num_cols.output_params.iter().find_map(|p| match &p.value {
            ParamValue::OutputInteger { value, .. } => Some(*value),
            _ => None,
        });
        assert_eq!(count, Some(1));

        let desc = trace
            .calls
            .iter()
            .find(|c| c.function_name == "SQLDescribeCol")
            .unwrap();
        let data_type = desc.output_params.iter().find_map(|p| match &p.value {
            ParamValue::OutputNamedConstant { name, .. } if name == "SQL_DECIMAL" => {
                Some(name.as_str())
            }
            _ => None,
        });
        assert_eq!(data_type, Some("SQL_DECIMAL"));
    }

    #[test]
    fn test_parse_full_trace_alloc_handle_params() {
        let trace = parse_str(SAMPLE_TRACE).expect("Failed to parse sample trace");

        let alloc = trace
            .calls
            .iter()
            .find(|c| c.function_name == "SQLAllocHandle")
            .unwrap();

        let ht = &alloc.output_params[0].value;
        assert!(
            matches!(ht, ParamValue::NamedConstant { value: 3, name } if name == "SQL_HANDLE_STMT")
        );

        let out = &alloc.output_params[2].value;
        assert!(matches!(
            out,
            ParamValue::OutputAddress {
                output_address, ..
            } if output_address == "0x104956bc0"
        ));
    }
}
