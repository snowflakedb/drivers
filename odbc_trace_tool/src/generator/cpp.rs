use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Write as FmtWrite;

use crate::model::{HandleType, OdbcCall, ReturnCode};
use crate::query_map::QueryMap;

pub struct GeneratorConfig {
    pub test_name: String,
    pub tag: String,
    pub query_map: Option<QueryMap>,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            test_name: "Replay trace".to_string(),
            tag: "replay".to_string(),
            query_map: None,
        }
    }
}

pub fn generate(calls: &[OdbcCall], config: &GeneratorConfig) -> String {
    let mut ctx = GenContext::new(calls, config);
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

const DRIVER_SPECIFIC_INFO_TYPES: &[&str] = &[
    "SQL_DRIVER_VER",
    "SQL_DRIVER_NAME",
    "SQL_DRIVER_ODBC_VER",
    "SQL_DM_VER",
];

struct GenContext<'a> {
    calls: &'a [OdbcCall],
    config: &'a GeneratorConfig,
    output: String,
    indent: usize,
    handle_vars: HashMap<String, String>,
    env_counter: usize,
    dbc_counter: usize,
    stmt_counter: usize,
    declared_handles: HashSet<String>,
    query_counter: usize,
}

impl<'a> GenContext<'a> {
    fn new(calls: &'a [OdbcCall], config: &'a GeneratorConfig) -> Self {
        Self {
            calls,
            config,
            output: String::new(),
            indent: 1,
            handle_vars: HashMap::new(),
            env_counter: 0,
            dbc_counter: 0,
            stmt_counter: 0,
            declared_handles: HashSet::new(),
            query_counter: 0,
        }
    }

    fn generate(&mut self) -> String {
        self.emit_header();
        self.emit_test_open();
        self.emit_config_install();
        self.emit_synthetic_handles();

        for call in self.calls {
            if matches!(call, OdbcCall::GetDiagRec(_) | OdbcCall::GetFunctions(_)) {
                continue;
            }
            self.emit_call(call);
        }

        self.emit_test_close();
        self.output.clone()
    }

    /// Find env/dbc handle addresses that are referenced by calls but never
    /// explicitly allocated via SQLAllocHandle in this call list. For each one,
    /// synthesize a variable declaration and allocation block at the top.
    fn emit_synthetic_handles(&mut self) {
        let explicitly_allocated: HashSet<String> = self
            .calls
            .iter()
            .filter_map(|c| {
                if let OdbcCall::AllocHandle(a) = c {
                    a.child_handle.clone()
                } else {
                    None
                }
            })
            .collect();

        let mut implicit_envs: Vec<String> = Vec::new();
        let mut implicit_dbcs: Vec<String> = Vec::new();

        let mut record_implicit = |addr: &str, ht: HandleType| {
            if explicitly_allocated.contains(addr) {
                return;
            }
            match ht {
                HandleType::Env => {
                    if !implicit_envs.contains(&addr.to_string()) {
                        implicit_envs.push(addr.to_string());
                    }
                }
                HandleType::Dbc => {
                    if !implicit_dbcs.contains(&addr.to_string()) {
                        implicit_dbcs.push(addr.to_string());
                    }
                }
                _ => {}
            }
        };

        for call in self.calls {
            match call {
                OdbcCall::SetEnvAttr(c) => {
                    if let Some(a) = &c.handle {
                        record_implicit(a, HandleType::Env);
                    }
                }
                OdbcCall::DriverConnect(c) => {
                    if let Some(a) = &c.handle {
                        record_implicit(a, HandleType::Dbc);
                    }
                }
                OdbcCall::Disconnect(c) => {
                    if let Some(a) = &c.handle {
                        record_implicit(a, HandleType::Dbc);
                    }
                }
                OdbcCall::SetConnectAttr(c) => {
                    if let Some(a) = &c.handle {
                        record_implicit(a, HandleType::Dbc);
                    }
                }
                OdbcCall::GetInfo(c) => {
                    if let Some(a) = &c.handle {
                        record_implicit(a, HandleType::Dbc);
                    }
                }
                OdbcCall::FreeHandle(c) => {
                    if let (Some(a), Some(ht)) = (&c.handle, c.handle_type) {
                        if matches!(ht, HandleType::Env | HandleType::Dbc) {
                            record_implicit(a, ht);
                        }
                    }
                }
                OdbcCall::AllocHandle(c) => {
                    if let Some(parent) = &c.parent_handle {
                        if !explicitly_allocated.contains(parent) {
                            let parent_ht = match c.handle_type {
                                Some(HandleType::Dbc) => Some(HandleType::Env),
                                Some(HandleType::Stmt) => Some(HandleType::Dbc),
                                _ => None,
                            };
                            if let Some(ht) = parent_ht {
                                record_implicit(parent, ht);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        for env_addr in &implicit_envs {
            let var = self.next_env_var();
            self.declare_handle(&var, HandleType::Env);
            self.writeln("// SQLAllocHandle - SQLHENV (implicit)");
            self.writeln("{");
            self.indent += 1;
            self.writeln(&format!(
                "SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &{var});"
            ));
            self.writeln("REQUIRE(ret == SQL_SUCCESS);");
            self.writeln(&format!("REQUIRE({var} != SQL_NULL_HENV);"));
            self.indent -= 1;
            self.writeln("}");
            self.writeln("");
            self.handle_vars.insert(env_addr.clone(), var.clone());

            self.writeln("// SQLSetEnvAttr - SQL_ATTR_ODBC_VERSION (implicit)");
            self.writeln("{");
            self.indent += 1;
            self.writeln(&format!(
                "SQLRETURN ret = SQLSetEnvAttr({var}, SQL_ATTR_ODBC_VERSION,"
            ));
            self.writeln("    (SQLPOINTER)SQL_OV_ODBC3, 0);");
            self.writeln(&format!(
                "REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, {var}),"
            ));
            self.writeln("           OdbcMatchers::IsSuccess());");
            self.indent -= 1;
            self.writeln("}");
            self.writeln("");
        }

        let env_var_for_implicit_dbc = if !implicit_envs.is_empty() {
            self.handle_vars
                .get(&implicit_envs[0])
                .cloned()
                .unwrap_or_else(|| "env0".to_string())
        } else {
            self.first_env_var()
        };

        for dbc_addr in &implicit_dbcs {
            let var = self.next_dbc_var();
            self.declare_handle(&var, HandleType::Dbc);
            self.writeln("// SQLAllocHandle - SQLHDBC (implicit)");
            self.writeln("{");
            self.indent += 1;
            self.writeln(&format!(
                "SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, {env_var_for_implicit_dbc}, &{var});"
            ));
            self.writeln(&format!(
                "REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, {env_var_for_implicit_dbc}),"
            ));
            self.writeln("           OdbcMatchers::IsSuccess());");
            self.writeln(&format!("REQUIRE({var} != SQL_NULL_HDBC);"));
            self.indent -= 1;
            self.writeln("}");
            self.writeln("");
            self.handle_vars.insert(dbc_addr.clone(), var);
        }
    }

    fn emit_header(&mut self) {
        let saved = self.indent;
        self.indent = 0;
        self.writeln("#include <catch2/catch_test_macros.hpp>");
        self.writeln("#include <vector>");
        self.writeln("#include \"ODBCConfig.hpp\"");
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
        self.writeln(&format!("TEST_CASE(\"Replay: {name}\", \"[{tag}]\") {{"));
        self.indent = saved;
    }

    fn emit_config_install(&mut self) {
        self.writeln("auto config = DataSourceConfig::Snowflake().install();");
        self.writeln("");
    }

    fn emit_test_close(&mut self) {
        let saved = self.indent;
        self.indent = 0;
        self.writeln("}");
        self.indent = saved;
    }

    fn emit_call(&mut self, call: &OdbcCall) {
        match call {
            OdbcCall::DriverConnect(c) => self.emit_driver_connect(c),
            OdbcCall::AllocHandle(c) => self.emit_alloc_handle(c),
            OdbcCall::SetEnvAttr(c) => self.emit_set_env_attr(c),
            OdbcCall::SetConnectAttr(c) => self.emit_set_connect_attr(c),
            OdbcCall::Prepare(c) => self.emit_prepare(c),
            OdbcCall::Execute(c) => self.emit_execute(c),
            OdbcCall::ExecDirect(c) => self.emit_exec_direct(c),
            OdbcCall::NumResultCols(c) => self.emit_num_result_cols(c),
            OdbcCall::DescribeCol(c) => self.emit_describe_col(c),
            OdbcCall::FetchScroll(c) => self.emit_fetch_scroll(c),
            OdbcCall::Fetch(c) => self.emit_fetch(c),
            OdbcCall::GetData(c) => self.emit_get_data(c),
            OdbcCall::RowCount(c) => self.emit_row_count(c),
            OdbcCall::MoreResults(c) => self.emit_more_results(c),
            OdbcCall::CloseCursor(c) => self.emit_close_cursor(c),
            OdbcCall::GetInfo(c) => self.emit_get_info(c),
            OdbcCall::FreeHandle(c) => self.emit_free_handle(c),
            OdbcCall::Disconnect(c) => self.emit_disconnect(c),
            _ => {}
        }
    }

    fn emit_driver_connect(&mut self, call: &crate::model::DriverConnect) {
        let dbc_var = self.dbc_var_for(&call.handle);

        self.writeln("// SQLDriverConnect");
        self.writeln("{");
        self.indent += 1;
        self.writeln(&format!(
            "SQLRETURN ret = SQLDriverConnect({dbc_var}, nullptr,"
        ));
        self.writeln("    sqlchar(config.connection_string().c_str()), SQL_NTS,");
        self.writeln("    nullptr, 0, nullptr, SQL_DRIVER_NOPROMPT);");
        self.emit_return_assertion(call.return_code, "SQL_HANDLE_DBC", &dbc_var, true, true);
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn emit_alloc_handle(&mut self, call: &crate::model::AllocHandle) {
        let Some(ht) = call.handle_type else { return };
        let Some(child) = &call.child_handle else {
            return;
        };

        if self.handle_vars.contains_key(child) {
            return;
        }

        let var_name = match ht {
            HandleType::Env => self.next_env_var(),
            HandleType::Dbc => self.next_dbc_var(),
            HandleType::Stmt => self.next_stmt_var(),
            HandleType::Desc => return,
        };

        self.declare_handle(&var_name, ht);
        self.handle_vars.insert(child.clone(), var_name.clone());

        let parent_var = match ht {
            HandleType::Env => "SQL_NULL_HANDLE".to_string(),
            _ => call
                .parent_handle
                .as_ref()
                .and_then(|a| self.handle_vars.get(a))
                .cloned()
                .unwrap_or_else(|| self.default_parent_var(ht)),
        };

        let diag_handle_type = match ht {
            HandleType::Env => None,
            HandleType::Dbc => Some("SQL_HANDLE_ENV"),
            HandleType::Stmt => Some("SQL_HANDLE_DBC"),
            HandleType::Desc => Some("SQL_HANDLE_DBC"),
        };

        self.writeln(&format!("// SQLAllocHandle - {}", ht.c_type_name()));
        self.writeln("{");
        self.indent += 1;
        self.writeln(&format!(
            "SQLRETURN ret = SQLAllocHandle({}, {parent_var}, &{var_name});",
            ht.sql_handle_type_constant()
        ));

        if let Some(dht) = diag_handle_type {
            self.emit_return_assertion(call.return_code, dht, &parent_var, true, false);
        } else {
            self.writeln("REQUIRE(ret == SQL_SUCCESS);");
        }
        self.writeln(&format!(
            "REQUIRE({var_name} != {});",
            ht.sql_null_constant()
        ));
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn emit_prepare(&mut self, call: &crate::model::Prepare) {
        let raw_sql = call.sql.as_deref().unwrap_or_default();
        let sql = escape_cpp_string_literal(&self.resolve_query(raw_sql));
        let stmt_var = self.stmt_var_for(&call.handle);

        self.writeln("// SQLPrepare");
        self.writeln("{");
        self.indent += 1;
        self.writeln(&format!(
            "SQLRETURN ret = SQLPrepare({stmt_var}, sqlchar(\"{sql}\"), SQL_NTS);"
        ));
        self.emit_return_assertion(call.return_code, "SQL_HANDLE_STMT", &stmt_var, true, false);
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn emit_execute(&mut self, call: &crate::model::Execute) {
        let stmt_var = self.stmt_var_for(&call.handle);

        self.writeln("// SQLExecute");
        self.writeln("{");
        self.indent += 1;
        self.writeln(&format!("SQLRETURN ret = SQLExecute({stmt_var});"));
        self.emit_return_assertion(call.return_code, "SQL_HANDLE_STMT", &stmt_var, true, false);
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn emit_exec_direct(&mut self, call: &crate::model::ExecDirect) {
        let raw_sql = call.sql.as_deref().unwrap_or_default();
        let sql = escape_cpp_string_literal(&self.resolve_query(raw_sql));
        let stmt_var = self.stmt_var_for(&call.handle);

        self.writeln("// SQLExecDirect");
        self.writeln("{");
        self.indent += 1;
        self.writeln(&format!(
            "SQLRETURN ret = SQLExecDirect({stmt_var}, sqlchar(\"{sql}\"), SQL_NTS);"
        ));
        self.emit_return_assertion(call.return_code, "SQL_HANDLE_STMT", &stmt_var, true, false);
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn emit_num_result_cols(&mut self, call: &crate::model::NumResultCols) {
        let stmt_var = self.stmt_var_for(&call.handle);

        self.writeln("// SQLNumResultCols");
        self.writeln("{");
        self.indent += 1;
        self.writeln("SQLSMALLINT numCols = 0;");
        self.writeln(&format!(
            "SQLRETURN ret = SQLNumResultCols({stmt_var}, &numCols);"
        ));
        self.emit_return_assertion(call.return_code, "SQL_HANDLE_STMT", &stmt_var, false, false);
        if let Some(n) = call.count {
            self.writeln(&format!("CHECK(numCols == {n});"));
        }
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn emit_row_count(&mut self, call: &crate::model::RowCount) {
        let stmt_var = self.stmt_var_for(&call.handle);

        self.writeln("// SQLRowCount");
        self.writeln("{");
        self.indent += 1;
        self.writeln("SQLLEN rowCount = 0;");
        self.writeln(&format!(
            "SQLRETURN ret = SQLRowCount({stmt_var}, &rowCount);"
        ));
        self.emit_return_assertion(call.return_code, "SQL_HANDLE_STMT", &stmt_var, false, false);
        if let Some(n) = call.count {
            self.writeln(&format!("CHECK(rowCount == {n});"));
        }
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn emit_describe_col(&mut self, call: &crate::model::DescribeCol) {
        let stmt_var = self.stmt_var_for(&call.handle);
        let col_num = call.column_number.unwrap_or(1);
        let buf_len = call.buffer_length.unwrap_or(50);

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
        self.emit_return_assertion(call.return_code, "SQL_HANDLE_STMT", &stmt_var, false, false);

        if let Some(name) = &call.column_name {
            let escaped = escape_cpp_string_literal(name);
            self.writeln(&format!("CHECK(std::string(colName) == \"{escaped}\");"));
        }
        if let Some(dt) = &call.data_type {
            self.writeln(&format!("CHECK(dataType == {dt});"));
        }
        if let Some(cs) = call.column_size {
            self.writeln(&format!("CHECK(colSize == {cs});"));
        }
        if let Some(s) = call.decimal_digits {
            self.writeln(&format!("CHECK(scale == {s});"));
        }
        if let Some(n) = &call.nullable {
            self.writeln(&format!("CHECK(nullable == {n});"));
        }

        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn emit_fetch_scroll(&mut self, call: &crate::model::FetchScroll) {
        let stmt_var = self.stmt_var_for(&call.handle);
        let fetch_orientation = call.orientation_name.as_deref().unwrap_or("SQL_FETCH_NEXT");
        let offset = call.offset.unwrap_or(1);

        self.writeln("// SQLFetchScroll");
        self.writeln("{");
        self.indent += 1;
        self.writeln(&format!(
            "SQLRETURN ret = SQLFetchScroll({stmt_var}, {fetch_orientation}, {offset});"
        ));
        self.emit_return_assertion(call.return_code, "SQL_HANDLE_STMT", &stmt_var, false, false);
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn emit_fetch(&mut self, call: &crate::model::Fetch) {
        let stmt_var = self.stmt_var_for(&call.handle);

        self.writeln("// SQLFetch");
        self.writeln("{");
        self.indent += 1;
        self.writeln(&format!("SQLRETURN ret = SQLFetch({stmt_var});"));
        self.emit_return_assertion(call.return_code, "SQL_HANDLE_STMT", &stmt_var, false, false);
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn emit_get_data(&mut self, call: &crate::model::GetData) {
        let stmt_var = self.stmt_var_for(&call.handle);
        let col_num = call.column_number.unwrap_or(1);
        let target_type = call.target_type_name.as_deref().unwrap_or("SQL_C_CHAR");
        let buf_len = call.buffer_length.unwrap_or(1024).min(4096);

        self.writeln(&format!("// SQLGetData col {col_num}"));
        self.writeln("{");
        self.indent += 1;

        if target_type == "SQL_C_CHAR" || target_type == "SQL_CHAR" {
            self.writeln(&format!("std::vector<char> buf({}, 0);", buf_len + 1));
            self.writeln("SQLLEN ind = 0;");
            self.writeln(&format!(
                "SQLRETURN ret = SQLGetData({stmt_var}, {col_num}, {target_type}, buf.data(), {buf_len}, &ind);"
            ));
            self.emit_return_assertion(
                call.return_code,
                "SQL_HANDLE_STMT",
                &stmt_var,
                false,
                false,
            );

            if let Some(ind_val) = call.indicator {
                if ind_val == -1 {
                    self.writeln("CHECK(ind == SQL_NULL_DATA);");
                } else {
                    if let Some(val) = &call.value {
                        let escaped = escape_cpp_string_literal(val);
                        self.writeln(&format!("CHECK(std::string(buf.data()) == \"{escaped}\");"));
                    }
                    self.writeln(&format!("CHECK(ind == {ind_val});"));
                }
            } else if let Some(val) = &call.value {
                let escaped = escape_cpp_string_literal(val);
                self.writeln(&format!("CHECK(std::string(buf.data()) == \"{escaped}\");"));
            }
        } else {
            self.writeln("SQLLEN ind = 0;");
            self.writeln(&format!("std::vector<char> buf({buf_len}, 0);"));
            self.writeln(&format!(
                "SQLRETURN ret = SQLGetData({stmt_var}, {col_num}, {target_type}, buf.data(), {buf_len}, &ind);"
            ));
            self.emit_return_assertion(
                call.return_code,
                "SQL_HANDLE_STMT",
                &stmt_var,
                false,
                false,
            );

            if let Some(ind_val) = call.indicator {
                if ind_val == -1 {
                    self.writeln("CHECK(ind == SQL_NULL_DATA);");
                } else {
                    self.writeln(&format!("CHECK(ind == {ind_val});"));
                }
            }
        }

        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn emit_more_results(&mut self, call: &crate::model::MoreResults) {
        let stmt_var = self.stmt_var_for(&call.handle);

        self.writeln("// SQLMoreResults");
        self.writeln("{");
        self.indent += 1;
        self.writeln(&format!("SQLRETURN ret = SQLMoreResults({stmt_var});"));
        self.emit_return_assertion(call.return_code, "SQL_HANDLE_STMT", &stmt_var, false, false);
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn emit_close_cursor(&mut self, call: &crate::model::CloseCursor) {
        let stmt_var = self.stmt_var_for(&call.handle);

        self.writeln("// SQLCloseCursor");
        self.writeln("{");
        self.indent += 1;
        self.writeln(&format!("SQLRETURN ret = SQLCloseCursor({stmt_var});"));
        self.emit_return_assertion(call.return_code, "SQL_HANDLE_STMT", &stmt_var, false, false);
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn emit_get_info(&mut self, call: &crate::model::GetInfo) {
        let info_type_name = call.info_type.as_deref().unwrap_or("0");
        let dbc_var = self.dbc_var_for(&call.handle);

        let is_driver_specific = DRIVER_SPECIFIC_INFO_TYPES.contains(&info_type_name);

        self.writeln(&format!("// SQLGetInfo - {info_type_name}"));
        self.writeln("{");
        self.indent += 1;
        self.writeln("char buf[256] = {};");
        self.writeln("SQLSMALLINT len = 0;");
        self.writeln(&format!(
            "SQLRETURN ret = SQLGetInfo({dbc_var}, {info_type_name}, buf, 255, &len);"
        ));

        if is_driver_specific {
            self.emit_return_assertion_relaxed("SQL_HANDLE_DBC", &dbc_var);
        } else {
            self.emit_return_assertion(call.return_code, "SQL_HANDLE_DBC", &dbc_var, false, false);
            if let Some(val) = &call.info_value {
                let escaped = escape_cpp_string_literal(val);
                self.writeln(&format!("CHECK(std::string(buf) == \"{escaped}\");"));
            }
        }

        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn emit_free_handle(&mut self, call: &crate::model::FreeHandle) {
        let Some(ht) = call.handle_type else { return };
        if ht == HandleType::Desc {
            return;
        }

        let var_name = call
            .handle
            .as_ref()
            .and_then(|a| self.handle_vars.get(a))
            .cloned()
            .unwrap_or_else(|| self.default_var_for(ht));

        if let Some(addr) = &call.handle {
            self.handle_vars.remove(addr);
        }

        self.writeln(&format!("// SQLFreeHandle - {}", ht.c_type_name()));
        self.writeln("{");
        self.indent += 1;
        self.writeln(&format!(
            "SQLRETURN ret = SQLFreeHandle({}, {var_name});",
            ht.sql_handle_type_constant()
        ));
        self.emit_return_assertion(
            call.return_code,
            ht.sql_handle_type_constant(),
            &var_name,
            false,
            false,
        );
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn emit_disconnect(&mut self, call: &crate::model::Disconnect) {
        let dbc_var = self.dbc_var_for(&call.handle);

        self.writeln("// SQLDisconnect");
        self.writeln("{");
        self.indent += 1;
        self.writeln(&format!("SQLRETURN ret = SQLDisconnect({dbc_var});"));
        self.emit_return_assertion_relaxed("SQL_HANDLE_DBC", &dbc_var);
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn emit_set_env_attr(&mut self, call: &crate::model::SetEnvAttr) {
        let attr_name = call.attribute.as_deref().unwrap_or_default();
        let env_var = self.env_var_for(&call.handle);

        if attr_name == "SQL_ATTR_ODBC_VERSION" && self.was_synthetically_set(&env_var) {
            return;
        }

        let value = call.value.unwrap_or(0);
        let value_expr = attr_value_cpp_expr(attr_name, value);
        let str_len = call.str_len.unwrap_or(0);

        self.writeln(&format!("// SQLSetEnvAttr - {attr_name}"));
        self.writeln("{");
        self.indent += 1;
        self.writeln(&format!(
            "SQLRETURN ret = SQLSetEnvAttr({env_var}, {attr_name},"
        ));
        self.writeln(&format!("    {value_expr}, {str_len});"));
        self.emit_return_assertion(call.return_code, "SQL_HANDLE_ENV", &env_var, false, false);
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn emit_set_connect_attr(&mut self, call: &crate::model::SetConnectAttr) {
        let attr_name = call.attribute.as_deref().unwrap_or_default();
        let value = call.value.unwrap_or(0);
        let value_expr = attr_value_cpp_expr(attr_name, value);
        let str_len = call.str_len.unwrap_or(0);

        let conn_var = self.dbc_var_for(&call.handle);

        self.writeln(&format!("// SQLSetConnectAttr - {attr_name}"));
        self.writeln("{");
        self.indent += 1;
        self.writeln(&format!(
            "SQLRETURN ret = SQLSetConnectAttr({conn_var}, {attr_name},"
        ));
        self.writeln(&format!("    {value_expr}, {str_len});"));
        self.emit_return_assertion(call.return_code, "SQL_HANDLE_DBC", &conn_var, false, false);
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn emit_return_assertion(
        &mut self,
        return_code: ReturnCode,
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
            format!("OdbcMatchers::{}()", return_code.matcher_name())
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

    fn resolve_query(&mut self, raw_sql: &str) -> String {
        let idx = self.query_counter;
        self.query_counter += 1;

        self.config
            .query_map
            .as_ref()
            .and_then(|qm| qm.get(idx))
            .map(|s| s.to_string())
            .unwrap_or_else(|| raw_sql.to_string())
    }

    fn next_env_var(&mut self) -> String {
        let name = format!("env{}", self.env_counter);
        self.env_counter += 1;
        name
    }

    fn next_dbc_var(&mut self) -> String {
        let name = format!("dbc{}", self.dbc_counter);
        self.dbc_counter += 1;
        name
    }

    fn next_stmt_var(&mut self) -> String {
        let name = format!("stmt{}", self.stmt_counter);
        self.stmt_counter += 1;
        name
    }

    fn declare_handle(&mut self, var_name: &str, ht: HandleType) {
        if !self.declared_handles.contains(var_name) {
            self.writeln(&format!(
                "{} {var_name} = {};",
                ht.c_type_name(),
                ht.sql_null_constant()
            ));
            self.declared_handles.insert(var_name.to_string());
        }
    }

    fn env_var_for(&self, handle: &Option<String>) -> String {
        handle
            .as_ref()
            .and_then(|addr| self.handle_vars.get(addr))
            .cloned()
            .unwrap_or_else(|| self.first_env_var())
    }

    fn dbc_var_for(&self, handle: &Option<String>) -> String {
        handle
            .as_ref()
            .and_then(|addr| self.handle_vars.get(addr))
            .cloned()
            .unwrap_or_else(|| self.first_dbc_var())
    }

    fn stmt_var_for(&self, handle: &Option<String>) -> String {
        handle
            .as_ref()
            .and_then(|addr| self.handle_vars.get(addr))
            .filter(|v| v.starts_with("stmt"))
            .cloned()
            .unwrap_or_else(|| "stmt0".to_string())
    }

    fn first_env_var(&self) -> String {
        self.handle_vars
            .values()
            .find(|v| v.starts_with("env"))
            .cloned()
            .unwrap_or_else(|| "env0".to_string())
    }

    fn first_dbc_var(&self) -> String {
        self.handle_vars
            .values()
            .find(|v| v.starts_with("dbc"))
            .cloned()
            .unwrap_or_else(|| "dbc0".to_string())
    }

    fn default_parent_var(&self, ht: HandleType) -> String {
        match ht {
            HandleType::Env => "SQL_NULL_HANDLE".to_string(),
            HandleType::Dbc => self.first_env_var(),
            HandleType::Stmt | HandleType::Desc => self.first_dbc_var(),
        }
    }

    fn default_var_for(&self, ht: HandleType) -> String {
        match ht {
            HandleType::Env => self.first_env_var(),
            HandleType::Dbc => self.first_dbc_var(),
            HandleType::Stmt | HandleType::Desc => "stmt0".to_string(),
        }
    }

    /// Returns true if the given env variable was created via synthetic
    /// (implicit) allocation, which already emits its own SetEnvAttr for
    /// SQL_ATTR_ODBC_VERSION. Used to avoid duplicating the call when the
    /// trace also contains one.
    fn was_synthetically_set(&self, env_var: &str) -> bool {
        self.output.contains(&format!(
            "SQLRETURN ret = SQLSetEnvAttr({env_var}, SQL_ATTR_ODBC_VERSION,"
        ))
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

fn attr_value_cpp_expr(attr_name: &str, value: i64) -> String {
    let symbolic = match (attr_name, value) {
        ("SQL_ATTR_AUTOCOMMIT", 0) => Some("SQL_AUTOCOMMIT_OFF"),
        ("SQL_ATTR_AUTOCOMMIT", 1) => Some("SQL_AUTOCOMMIT_ON"),
        ("SQL_ATTR_ODBC_VERSION", 2) => Some("SQL_OV_ODBC2"),
        ("SQL_ATTR_ODBC_VERSION", 3) => Some("SQL_OV_ODBC3"),
        ("SQL_ATTR_ODBC_VERSION", 380) => Some("SQL_OV_ODBC3_80"),
        ("SQL_ATTR_ACCESS_MODE", 0) => Some("SQL_MODE_DEFAULT"),
        ("SQL_ATTR_ACCESS_MODE", 1) => Some("SQL_MODE_READ_ONLY"),
        ("SQL_ATTR_ACCESS_MODE", 2) => Some("SQL_MODE_READ_WRITE"),
        _ => None,
    };

    if let Some(name) = symbolic {
        format!("(SQLPOINTER){name}")
    } else if value == 0 {
        "nullptr".to_string()
    } else {
        format!("(SQLPOINTER){value}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::iodbc;
    use crate::query_map::{QueryMap, QueryMapEntry};

    const SAMPLE_TRACE: &str =
        include_str!("../../../odbc_tests/tests/replay/iodbctest/select_1.log");

    #[test]
    fn test_generate_from_sample_trace() {
        let trace = iodbc::parse_str(SAMPLE_TRACE).expect("Failed to parse sample trace");
        let config = GeneratorConfig {
            test_name: "SELECT 1".to_string(),
            tag: "replay".to_string(),
            query_map: None,
        };

        let calls: Vec<_> = trace.calls.iter().map(|tc| tc.call.clone()).collect();
        let output = generate(&calls, &config);

        assert!(
            output.contains("TEST_CASE(\"Replay: SELECT 1\""),
            "plain TEST_CASE, no fixture"
        );
        assert!(
            !output.contains("TEST_CASE_METHOD"),
            "no fixture-based test"
        );
        assert!(output.contains("DataSourceConfig::Snowflake().install()"));
        assert!(output.contains("ODBCConfig.hpp"));
        assert!(!output.contains("ODBCFixtures.hpp"));
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
        assert!(output.contains("odbc_matchers.hpp"));
    }

    #[test]
    fn test_generate_snapshot() {
        let trace = iodbc::parse_str(SAMPLE_TRACE).expect("Failed to parse sample trace");
        let config = GeneratorConfig {
            test_name: "SELECT 1".to_string(),
            tag: "replay".to_string(),
            query_map: None,
        };

        let calls: Vec<_> = trace.calls.iter().map(|tc| tc.call.clone()).collect();
        let output = generate(&calls, &config);

        assert!(
            output.contains("SQLHENV env0 = SQL_NULL_HENV"),
            "explicit env allocation"
        );
        assert!(
            output.contains("SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env0)"),
            "env alloc"
        );
        assert!(
            output.contains("SQLHDBC dbc0 = SQL_NULL_HDBC"),
            "explicit dbc allocation"
        );
        assert!(
            output.contains("SQLAllocHandle(SQL_HANDLE_DBC, env0, &dbc0)"),
            "dbc alloc"
        );
        assert!(
            output.contains("SQLHSTMT stmt0 = SQL_NULL_HSTMT"),
            "stmt declaration"
        );
        assert!(
            output.contains("REQUIRE(stmt0 != SQL_NULL_HSTMT)"),
            "stmt non-null check"
        );
        assert!(
            output.contains("config.connection_string()"),
            "connection string from config"
        );
        assert!(output.contains("CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0),"));
        assert!(output.contains("OdbcMatchers::IsNoData()"));
        assert!(output.contains("OdbcMatchers::IsSuccess()"));
        assert!(
            output.contains("SQLRETURN ret = SQLDisconnect(dbc0)"),
            "disconnect uses var"
        );
        assert!(output.contains("SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt0)"));
        assert!(
            output.contains("SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_DBC, dbc0)"),
            "dbc freed"
        );
        assert!(
            output.contains("SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_ENV, env0)"),
            "env freed"
        );
    }

    #[test]
    fn test_generate_with_query_map() {
        let trace = iodbc::parse_str(SAMPLE_TRACE).expect("Failed to parse sample trace");

        let qm = QueryMap {
            queries: vec![QueryMapEntry {
                index: 0,
                original: "SELECT 1".to_string(),
                mapped: "SELECT 42 AS answer".to_string(),
            }],
        };

        let config = GeneratorConfig {
            test_name: "mapped query".to_string(),
            tag: "replay".to_string(),
            query_map: Some(qm),
        };

        let calls: Vec<_> = trace.calls.iter().map(|tc| tc.call.clone()).collect();
        let output = generate(&calls, &config);

        assert!(
            output.contains("SELECT 42 AS answer"),
            "mapped query should appear in output"
        );
        assert!(
            !output.contains("sqlchar(\"SELECT 1\")"),
            "original query should be replaced"
        );
    }
}
