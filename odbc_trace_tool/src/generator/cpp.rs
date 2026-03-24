use std::collections::HashMap;
use std::fmt::Write as FmtWrite;

use crate::model::{HandleType, OdbcCall, ParamValue, TraceLog};

pub struct GeneratorConfig {
    pub test_name: String,
    pub tag: String,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            test_name: "Replay trace".to_string(),
            tag: "replay".to_string(),
        }
    }
}

pub fn generate(trace: &TraceLog, config: &GeneratorConfig) -> String {
    let mut ctx = GenContext::new(trace, config);
    ctx.generate()
}

fn escape_cpp_string_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0' => out.push_str("\\0"),
            c if c.is_ascii_control() => {
                out.push_str(&format!("\\x{:02x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

// SQLGetDiagRec calls are diagnostic-only and not part of the functional replay.
// We can add them back later if we need to assert on diagnostics.
// SQLGetFunctions queries driver manager which vary across managers and are
// not meaningful to assert in replay tests.
const SKIPPED_FUNCTIONS: &[&str] = &["SQLGetDiagRec", "SQLGetFunctions"];

const DRIVER_SPECIFIC_INFO_TYPES: &[&str] = &[
    "SQL_DRIVER_VER",
    "SQL_DRIVER_NAME",
    "SQL_DRIVER_ODBC_VER",
    "SQL_DM_VER",
];

struct GenContext<'a> {
    trace: &'a TraceLog,
    config: &'a GeneratorConfig,
    output: String,
    indent: usize,
    handle_vars: HashMap<String, String>,
    dbc_address: Option<String>,
    stmt_counter: usize,
    declared_stmts: Vec<String>,
}

impl<'a> GenContext<'a> {
    fn new(trace: &'a TraceLog, config: &'a GeneratorConfig) -> Self {
        Self {
            trace,
            config,
            output: String::new(),
            indent: 1,
            handle_vars: HashMap::new(),
            dbc_address: None,
            stmt_counter: 0,
            declared_stmts: Vec::new(),
        }
    }

    fn generate(&mut self) -> String {
        self.identify_dbc_handle();
        self.emit_header();
        self.emit_test_open();

        for i in 0..self.trace.calls.len() {
            let call = &self.trace.calls[i];
            if SKIPPED_FUNCTIONS.iter().any(|&f| f == call.function_name) {
                continue;
            }
            self.emit_call(call);
        }

        self.emit_test_close();
        self.output.clone()
    }

    fn identify_dbc_handle(&mut self) {
        for call in &self.trace.calls {
            if call.function_name == "SQLDriverConnect" {
                if let Some(param) = call.output_params.first() {
                    if let ParamValue::Address(addr) = &param.value {
                        self.dbc_address = Some(addr.clone());
                        self.handle_vars
                            .insert(addr.clone(), "dbc_handle()".to_string());
                    }
                }
            }
        }
    }

    fn emit_header(&mut self) {
        let saved = self.indent;
        self.indent = 0;
        self.writeln("#include <catch2/catch_test_macros.hpp>");
        self.writeln("#include \"ODBCFixtures.hpp\"");
        self.writeln("#include \"odbc_cast.hpp\"");
        self.writeln("#include \"odbc_matchers.hpp\"");
        self.writeln("");
        self.indent = saved;
    }

    fn emit_test_open(&mut self) {
        let name = escape_cpp_string_literal(&self.config.test_name);
        let tag = escape_cpp_string_literal(&self.config.tag);
        let saved = self.indent;
        self.indent = 0;
        self.writeln(&format!(
            "TEST_CASE_METHOD(DbcDefaultDSNFixture, \"Replay: {name}\", \"[{tag}]\") {{"
        ));
        self.indent = saved;
    }

    fn emit_test_close(&mut self) {
        let saved = self.indent;
        self.indent = 0;
        self.writeln("}");
        self.indent = saved;
    }

    fn emit_call(&mut self, call: &OdbcCall) {
        match call.function_name.as_str() {
            "SQLDriverConnect" => self.emit_driver_connect(call),
            "SQLAllocHandle" => self.emit_alloc_handle(call),
            "SQLPrepare" => self.emit_prepare(call),
            "SQLExecute" => self.emit_execute(call),
            "SQLExecDirect" => self.emit_exec_direct(call),
            "SQLNumResultCols" => self.emit_num_result_cols(call),
            "SQLDescribeCol" => self.emit_describe_col(call),
            "SQLFetchScroll" => self.emit_fetch_scroll(call),
            "SQLFetch" => self.emit_fetch(call),
            "SQLGetData" => self.emit_get_data(call),
            "SQLMoreResults" => self.emit_more_results(call),
            "SQLCloseCursor" => self.emit_close_cursor(call),
            "SQLGetInfo" => self.emit_get_info(call),
            "SQLFreeHandle" => self.emit_free_handle(call),
            "SQLDisconnect" => self.emit_disconnect(call),
            _ => self.emit_generic_comment(call),
        }
    }

    fn emit_driver_connect(&mut self, call: &OdbcCall) {
        self.writeln("// SQLDriverConnect");
        self.writeln("{");
        self.indent += 1;
        self.writeln("SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr,");
        self.writeln("    sqlchar(connection_string().c_str()), SQL_NTS,");
        self.writeln("    nullptr, 0, nullptr, SQL_DRIVER_NOPROMPT);");
        self.emit_return_assertion(call, "SQL_HANDLE_DBC", "dbc_handle()", true, true);
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn emit_alloc_handle(&mut self, call: &OdbcCall) {
        let handle_type_value = extract_integer_or_named(&call.output_params, 0);
        let handle_type = handle_type_value.and_then(HandleType::from_value);
        let parent_addr = extract_address(&call.output_params, 1);
        let child_addr = extract_output_address(&call.output_params, 2);

        if let (Some(ht), Some(child)) = (handle_type, &child_addr) {
            if ht == HandleType::Stmt {
                let var_name = format!("stmt{}", self.stmt_counter);
                self.stmt_counter += 1;

                if !self.declared_stmts.contains(&var_name) {
                    self.writeln(&format!("SQLHSTMT {var_name} = SQL_NULL_HSTMT;"));
                    self.declared_stmts.push(var_name.clone());
                }

                self.handle_vars.insert(child.clone(), var_name.clone());

                let parent_var = parent_addr
                    .as_ref()
                    .and_then(|a| self.handle_vars.get(a))
                    .cloned()
                    .unwrap_or_else(|| "dbc_handle()".to_string());

                self.writeln(&format!("// SQLAllocHandle - {}", ht.c_type_name()));
                self.writeln("{");
                self.indent += 1;
                self.writeln(&format!(
                    "SQLRETURN ret = SQLAllocHandle({}, {parent_var}, &{var_name});",
                    ht.sql_handle_type_constant()
                ));
                self.emit_return_assertion(call, "SQL_HANDLE_DBC", &parent_var, true, false);
                self.indent -= 1;
                self.writeln("}");
                self.writeln("");
            }
        }
    }

    fn emit_prepare(&mut self, call: &OdbcCall) {
        let sql = escape_cpp_string_literal(
            &extract_string_value(&call.input_params).unwrap_or_default(),
        );
        let stmt_var = self.find_stmt_var(&call.input_params);

        self.writeln("// SQLPrepare");
        self.writeln("{");
        self.indent += 1;
        self.writeln(&format!(
            "SQLRETURN ret = SQLPrepare({stmt_var}, sqlchar(\"{sql}\"), SQL_NTS);"
        ));
        self.emit_return_assertion(call, "SQL_HANDLE_STMT", &stmt_var, true, false);
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn emit_execute(&mut self, call: &OdbcCall) {
        let stmt_var = self.find_stmt_var(&call.input_params);

        self.writeln("// SQLExecute");
        self.writeln("{");
        self.indent += 1;
        self.writeln(&format!("SQLRETURN ret = SQLExecute({stmt_var});"));
        self.emit_return_assertion(call, "SQL_HANDLE_STMT", &stmt_var, true, false);
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn emit_exec_direct(&mut self, call: &OdbcCall) {
        let sql = escape_cpp_string_literal(
            &extract_string_value(&call.input_params).unwrap_or_default(),
        );
        let stmt_var = self.find_stmt_var(&call.input_params);

        self.writeln("// SQLExecDirect");
        self.writeln("{");
        self.indent += 1;
        self.writeln(&format!(
            "SQLRETURN ret = SQLExecDirect({stmt_var}, sqlchar(\"{sql}\"), SQL_NTS);"
        ));
        self.emit_return_assertion(call, "SQL_HANDLE_STMT", &stmt_var, true, false);
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn emit_num_result_cols(&mut self, call: &OdbcCall) {
        let stmt_var = self.find_stmt_var(&call.output_params);
        let num_cols = extract_output_integer(&call.output_params, 1);

        self.writeln("// SQLNumResultCols");
        self.writeln("{");
        self.indent += 1;
        self.writeln("SQLSMALLINT numCols = 0;");
        self.writeln(&format!(
            "SQLRETURN ret = SQLNumResultCols({stmt_var}, &numCols);"
        ));
        self.emit_return_assertion(call, "SQL_HANDLE_STMT", &stmt_var, false, false);
        if let Some(n) = num_cols {
            self.writeln(&format!("CHECK(numCols == {n});"));
        }
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn emit_describe_col(&mut self, call: &OdbcCall) {
        let stmt_var = self.find_stmt_var(&call.output_params);
        let col_num = extract_integer_or_named(&call.output_params, 1).unwrap_or(1);
        let col_name = extract_string_value(&call.output_params);
        let buf_len = extract_integer_or_named(&call.output_params, 3).unwrap_or(50);
        let data_type = extract_output_named_constant(&call.output_params, 5);
        let col_size = extract_output_integer(&call.output_params, 6);
        let scale = extract_output_integer(&call.output_params, 7);
        let nullable = extract_output_named_constant(&call.output_params, 8);

        self.writeln(&format!("// SQLDescribeCol col {col_num}"));
        self.writeln("{");
        self.indent += 1;
        self.writeln(&format!("char colName[{}] = {{}};", buf_len + 1));
        self.writeln("SQLSMALLINT dataType = 0, scale = 0, nullable = 0;");
        self.writeln("SQLULEN colSize = 0;");
        self.writeln(&format!(
            "SQLRETURN ret = SQLDescribeCol({stmt_var}, {col_num},"
        ));
        self.writeln(&format!(
            "    reinterpret_cast<SQLCHAR*>(colName), {buf_len}, nullptr,"
        ));
        self.writeln("    &dataType, &colSize, &scale, &nullable);");
        self.emit_return_assertion(call, "SQL_HANDLE_STMT", &stmt_var, false, false);

        if let Some(name) = &col_name {
            let escaped = escape_cpp_string_literal(name);
            self.writeln(&format!("CHECK(std::string(colName) == \"{escaped}\");"));
        }
        if let Some(dt) = &data_type {
            self.writeln(&format!("CHECK(dataType == {dt});"));
        }
        if let Some(cs) = col_size {
            self.writeln(&format!("CHECK(colSize == {cs});"));
        }
        if let Some(s) = scale {
            self.writeln(&format!("CHECK(scale == {s});"));
        }
        if let Some(n) = &nullable {
            self.writeln(&format!("CHECK(nullable == {n});"));
        }

        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn emit_fetch_scroll(&mut self, call: &OdbcCall) {
        let stmt_var = self.find_stmt_var(&call.output_params);
        let fetch_orientation = extract_named_constant_name(&call.output_params, 1)
            .unwrap_or("SQL_FETCH_NEXT".to_string());
        let offset = extract_integer_or_named(&call.output_params, 2).unwrap_or(1);

        self.writeln("// SQLFetchScroll");
        self.writeln("{");
        self.indent += 1;
        self.writeln(&format!(
            "SQLRETURN ret = SQLFetchScroll({stmt_var}, {fetch_orientation}, {offset});"
        ));
        self.emit_return_assertion(call, "SQL_HANDLE_STMT", &stmt_var, false, false);
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn emit_fetch(&mut self, call: &OdbcCall) {
        let stmt_var = self.find_stmt_var(&call.output_params);

        self.writeln("// SQLFetch");
        self.writeln("{");
        self.indent += 1;
        self.writeln(&format!("SQLRETURN ret = SQLFetch({stmt_var});"));
        self.emit_return_assertion(call, "SQL_HANDLE_STMT", &stmt_var, false, false);
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn emit_get_data(&mut self, call: &OdbcCall) {
        let stmt_var = self.find_stmt_var(&call.output_params);
        let col_num = extract_integer_or_named(&call.output_params, 1).unwrap_or(1);
        let target_type =
            extract_named_constant_name(&call.output_params, 2).unwrap_or("SQL_C_CHAR".to_string());
        let buf_len = extract_integer_or_named(&call.output_params, 4).unwrap_or(1024);
        let string_val = extract_string_value(&call.output_params);
        let indicator = extract_output_integer(&call.output_params, 5);

        self.writeln(&format!("// SQLGetData col {col_num}"));
        self.writeln("{");
        self.indent += 1;

        if target_type == "SQL_C_CHAR" {
            self.writeln(&format!("char buf[{}] = {{}};", buf_len + 1));
            self.writeln("SQLLEN ind = 0;");
            self.writeln(&format!(
                "SQLRETURN ret = SQLGetData({stmt_var}, {col_num}, {target_type}, buf, {buf_len}, &ind);"
            ));
            self.emit_return_assertion(call, "SQL_HANDLE_STMT", &stmt_var, false, false);

            if let Some(val) = &string_val {
                let escaped = escape_cpp_string_literal(val);
                self.writeln(&format!("CHECK(std::string(buf) == \"{escaped}\");"));
            }
            if let Some(ind_val) = indicator {
                self.writeln(&format!("CHECK(ind == {ind_val});"));
            }
        } else {
            self.writeln("SQLLEN ind = 0;");
            self.writeln(&format!("char buf[{buf_len}] = {{}};"));
            self.writeln(&format!(
                "SQLRETURN ret = SQLGetData({stmt_var}, {col_num}, {target_type}, buf, {buf_len}, &ind);"
            ));
            self.emit_return_assertion(call, "SQL_HANDLE_STMT", &stmt_var, false, false);
        }

        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn emit_more_results(&mut self, call: &OdbcCall) {
        let stmt_var = self.find_stmt_var(&call.output_params);

        self.writeln("// SQLMoreResults");
        self.writeln("{");
        self.indent += 1;
        self.writeln(&format!("SQLRETURN ret = SQLMoreResults({stmt_var});"));
        self.emit_return_assertion(call, "SQL_HANDLE_STMT", &stmt_var, false, false);
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn emit_close_cursor(&mut self, call: &OdbcCall) {
        let stmt_var = self.find_stmt_var(&call.output_params);

        self.writeln("// SQLCloseCursor");
        self.writeln("{");
        self.indent += 1;
        self.writeln(&format!("SQLRETURN ret = SQLCloseCursor({stmt_var});"));
        self.emit_return_assertion(call, "SQL_HANDLE_STMT", &stmt_var, false, false);
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn emit_get_info(&mut self, call: &OdbcCall) {
        let info_type_name =
            extract_named_constant_name(&call.output_params, 1).unwrap_or("0".to_string());

        let is_driver_specific = DRIVER_SPECIFIC_INFO_TYPES
            .iter()
            .any(|&t| t == info_type_name);

        let string_val = extract_string_value(&call.output_params);

        self.writeln(&format!("// SQLGetInfo - {info_type_name}"));
        self.writeln("{");
        self.indent += 1;
        self.writeln("char buf[256] = {};");
        self.writeln("SQLSMALLINT len = 0;");
        self.writeln(&format!(
            "SQLRETURN ret = SQLGetInfo(dbc_handle(), {info_type_name}, buf, 255, &len);"
        ));

        if is_driver_specific {
            self.emit_return_assertion_relaxed("SQL_HANDLE_DBC", "dbc_handle()");
        } else {
            self.emit_return_assertion(call, "SQL_HANDLE_DBC", "dbc_handle()", false, false);
            if let Some(val) = &string_val {
                let escaped = escape_cpp_string_literal(val);
                self.writeln(&format!("CHECK(std::string(buf) == \"{escaped}\");"));
            }
        }

        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn emit_free_handle(&mut self, call: &OdbcCall) {
        let handle_type_value = extract_integer_or_named(&call.output_params, 0);
        let handle_type = handle_type_value.and_then(HandleType::from_value);

        let Some(ht) = handle_type else { return };

        if matches!(ht, HandleType::Env | HandleType::Dbc) {
            return;
        }

        let handle_addr = extract_address(&call.output_params, 1);
        let var_name = handle_addr
            .as_ref()
            .and_then(|a| self.handle_vars.get(a))
            .cloned()
            .unwrap_or_else(|| "stmt0".to_string());

        self.writeln(&format!(
            "SQLFreeHandle({}, {var_name});",
            ht.sql_handle_type_constant()
        ));
    }

    fn emit_disconnect(&mut self, _call: &OdbcCall) {
        self.writeln("SQLDisconnect(dbc_handle());");
    }

    fn emit_generic_comment(&mut self, call: &OdbcCall) {
        self.writeln(&format!(
            "// TODO: {} (return code: {})",
            call.function_name, call.return_code
        ));
        self.writeln("");
    }

    fn emit_return_assertion(
        &mut self,
        call: &OdbcCall,
        handle_type: &str,
        handle_var: &str,
        is_setup: bool,
        relaxed_connect: bool,
    ) {
        let macro_name = if is_setup {
            "REQUIRE_THAT"
        } else {
            "CHECK_THAT"
        };
        let matcher = if relaxed_connect {
            "OdbcMatchers::Succeeded()".to_string()
        } else {
            format!("OdbcMatchers::{}()", call.return_code.matcher_name())
        };

        self.writeln(&format!(
            "{macro_name}(OdbcResult(ret, {handle_type}, {handle_var}),"
        ));
        self.writeln(&format!("           {matcher});"));
    }

    fn emit_return_assertion_relaxed(&mut self, handle_type: &str, handle_var: &str) {
        self.writeln(&format!(
            "CHECK_THAT(OdbcResult(ret, {handle_type}, {handle_var}),"
        ));
        self.writeln("           OdbcMatchers::Succeeded());");
    }

    fn find_stmt_var(&self, params: &[crate::model::Parameter]) -> String {
        for param in params {
            if let ParamValue::Address(addr) = &param.value {
                if let Some(var) = self.handle_vars.get(addr) {
                    if var.starts_with("stmt") {
                        return var.clone();
                    }
                }
            }
        }
        "stmt0".to_string()
    }

    fn writeln(&mut self, line: &str) {
        if line.is_empty() {
            let _ = writeln!(self.output);
        } else {
            let indent_str = "  ".repeat(self.indent);
            let _ = writeln!(self.output, "{indent_str}{line}");
        }
    }
}

fn extract_integer_or_named(params: &[crate::model::Parameter], idx: usize) -> Option<i64> {
    params.get(idx).and_then(|p| match &p.value {
        ParamValue::Integer(v) => Some(*v),
        ParamValue::NamedConstant { value, .. } => Some(*value),
        _ => None,
    })
}

fn extract_named_constant_name(params: &[crate::model::Parameter], idx: usize) -> Option<String> {
    params.get(idx).and_then(|p| match &p.value {
        ParamValue::NamedConstant { name, .. } => Some(name.clone()),
        _ => None,
    })
}

fn extract_address(params: &[crate::model::Parameter], idx: usize) -> Option<String> {
    params.get(idx).and_then(|p| match &p.value {
        ParamValue::Address(addr) => Some(addr.clone()),
        _ => None,
    })
}

fn extract_output_address(params: &[crate::model::Parameter], idx: usize) -> Option<String> {
    params.get(idx).and_then(|p| match &p.value {
        ParamValue::OutputAddress { output_address, .. } => Some(output_address.clone()),
        _ => None,
    })
}

fn extract_output_integer(params: &[crate::model::Parameter], idx: usize) -> Option<i64> {
    params.get(idx).and_then(|p| match &p.value {
        ParamValue::OutputInteger { value, .. } => Some(*value),
        _ => None,
    })
}

fn extract_output_named_constant(params: &[crate::model::Parameter], idx: usize) -> Option<String> {
    params.get(idx).and_then(|p| match &p.value {
        ParamValue::OutputNamedConstant { name, .. } => Some(name.clone()),
        _ => None,
    })
}

fn extract_string_value(params: &[crate::model::Parameter]) -> Option<String> {
    params.iter().find_map(|p| match &p.value {
        ParamValue::StringValue(s) => Some(s.clone()),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::iodbc;

    const SAMPLE_TRACE: &str =
        include_str!("../../../odbc_tests/tests/replay/iodbctest/select_1.log");

    #[test]
    fn test_generate_from_sample_trace() {
        let trace = iodbc::parse_str(SAMPLE_TRACE).expect("Failed to parse sample trace");
        let config = GeneratorConfig {
            test_name: "SELECT 1".to_string(),
            tag: "replay".to_string(),
        };

        let output = generate(&trace, &config);

        assert!(output.contains("TEST_CASE_METHOD(DbcDefaultDSNFixture"));
        assert!(output.contains("SQLDriverConnect"));
        assert!(output.contains("OdbcMatchers::"));
        assert!(output.contains("SQLPrepare"));
        assert!(output.contains("SQLExecute"));
        assert!(output.contains("SQLNumResultCols"));
        assert!(output.contains("SQLDescribeCol"));
        assert!(output.contains("SQLFetchScroll"));
        assert!(output.contains("SQLGetData"));
        assert!(output.contains("SQLMoreResults"));
        assert!(!output.contains("SQLGetDiagRec"));
        assert!(output.contains("ODBCFixtures.hpp"));
        assert!(output.contains("odbc_matchers.hpp"));
    }

    #[test]
    fn test_generate_snapshot() {
        let trace = iodbc::parse_str(SAMPLE_TRACE).expect("Failed to parse sample trace");
        let config = GeneratorConfig {
            test_name: "SELECT 1".to_string(),
            tag: "replay".to_string(),
        };

        let output = generate(&trace, &config);

        assert!(output.contains("CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0),"));
        assert!(output.contains("OdbcMatchers::IsNoData()"));
        assert!(output.contains("OdbcMatchers::IsSuccess()"));
        assert!(output.contains("connection_string()"));
        assert!(output.contains("SQLHSTMT stmt0 = SQL_NULL_HSTMT"));
        assert!(output.contains("SQLDisconnect(dbc_handle())"));
        assert!(output.contains("SQLFreeHandle(SQL_HANDLE_STMT, stmt0)"));
    }
}
