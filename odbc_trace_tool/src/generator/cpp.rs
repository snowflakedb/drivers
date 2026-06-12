use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Write as FmtWrite;

use crate::generator::getdata_codec;
use crate::generator::validate::{validate_call, MissingRequired};
use crate::model::{HandleType, OdbcCall, ReturnCode};
use crate::query_map::QueryMap;

pub struct GeneratorConfig {
    pub test_name: String,
    pub tag: String,
    pub query_map: Option<QueryMap>,
    pub allow_unsupported: bool,
    /// When true, emit a capture harness that records obscured GetData values
    /// to JSON instead of asserting them.
    pub capture_mode: bool,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            test_name: "Replay trace".to_string(),
            tag: "replay".to_string(),
            query_map: None,
            allow_unsupported: false,
            capture_mode: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UnsupportedCall {
    pub name: String,
    pub count: usize,
}

#[derive(Debug)]
pub enum GenerateError {
    Unsupported(Vec<UnsupportedCall>),
    /// One or more `OdbcCall`s in the IR are missing a required field that
    /// the C++ emitter would otherwise paper over with a `unwrap_or(...)`
    /// substitution. Each substituted default is a *real* (different) ODBC
    /// call, so we fail-fast and let the caller fix the upstream parser /
    /// trace instead of writing a silently-wrong test. Surfaced to the CLI
    /// as an `error: SQLX at trace line N is missing required field F`
    /// message and a non-zero exit code.
    MissingRequired(Vec<MissingRequired>),
}

// Used by tests and library consumers; the binary itself goes through
// `generate_with_lines` so it can attribute validator errors to trace lines.
#[allow(dead_code)]
pub fn generate(calls: &[OdbcCall], config: &GeneratorConfig) -> Result<String, GenerateError> {
    generate_with_lines(calls, &[], config)
}

/// Like [`generate`], but with per-call source line numbers. When supplied,
/// the validator surfaces them in `GenerateError::MissingRequired` so the
/// CLI can render `SQLGetInfo at trace line 4221 is missing required field
/// ...` instead of the line-less form. Pass an empty slice (or shorter than
/// `calls`) when line info isn't available — the validator silently falls
/// back to `None`.
pub fn generate_with_lines(
    calls: &[OdbcCall],
    entry_lines: &[Option<usize>],
    config: &GeneratorConfig,
) -> Result<String, GenerateError> {
    let mut ctx = GenContext::new(calls, entry_lines, config);
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

/// `SQLGetInfo` info types whose returned strings vary by driver/DBMS build,
/// platform, or DM. The exact bytes are environment-specific (e.g.
/// `SQL_DBMS_VER` shifts each Snowflake release; `SQL_DRIVER_NAME` carries an
/// install-time path on the new driver), so we deliberately skip the
/// `CHECK(std::string(buf) == ...)` comparison for these and assert only that
/// the call returned the same code as the trace.
///
/// **Every other** `SQLGetInfo` call gets a strict value check by default —
/// i.e. the captured trace value is the contract. If a particular info type
/// turns out to be unstable in practice (different between the Windows
/// reference driver and a Unix replay environment, or between driver builds),
/// add it here rather than reverting the strict-by-default policy.
///
/// See `.cursor/rules/odbc-trace-replay-sampling.mdc` for the rule.
const INFO_TYPES_WITH_UNSTABLE_VALUES: &[&str] = &[
    "SQL_DRIVER_VER",
    "SQL_DRIVER_NAME",
    "SQL_DRIVER_ODBC_VER",
    "SQL_DM_VER",
    "SQL_DBMS_VER",
    "SQL_DBMS_NAME",
];

struct GenContext<'a> {
    calls: &'a [OdbcCall],
    /// Per-call source trace line, parallel to [`calls`]. Indices past the
    /// end of this slice yield `None` (legacy call sites that don't carry
    /// line info). Used purely for diagnostic attribution in
    /// [`GenerateError::MissingRequired`].
    entry_lines: &'a [Option<usize>],
    config: &'a GeneratorConfig,
    output: String,
    indent: usize,
    handle_vars: HashMap<String, String>,
    env_counter: usize,
    dbc_counter: usize,
    stmt_counter: usize,
    declared_handles: HashSet<String>,
    query_counter: usize,
    unsupported: BTreeMap<String, usize>,
    skipped_col_attr_undocumented: usize,
    /// Env variable names allocated during this test that the captured trace
    /// never freed. ODBC-consuming applications routinely leak their env
    /// handles on shutdown because (a) `SQL_ATTR_CONNECTION_POOLING`-enabled
    /// envs are intentionally kept alive as the pool root, and (b) any
    /// teardown that happens during the host's `DllMain(DLL_PROCESS_DETACH)`
    /// is invisible to the trace facility. Our replay binary doesn't have the
    /// process-exit luxury, so we emit explicit `SQLFreeHandle` calls in a
    /// closing epilogue. Order preserved so the emitted output is stable.
    unfreed_envs: Vec<String>,
}

impl<'a> GenContext<'a> {
    fn new(
        calls: &'a [OdbcCall],
        entry_lines: &'a [Option<usize>],
        config: &'a GeneratorConfig,
    ) -> Self {
        Self {
            calls,
            entry_lines,
            config,
            output: String::new(),
            indent: 1,
            handle_vars: HashMap::new(),
            env_counter: 0,
            dbc_counter: 0,
            stmt_counter: 0,
            declared_handles: HashSet::new(),
            query_counter: 0,
            unsupported: BTreeMap::new(),
            skipped_col_attr_undocumented: 0,
            unfreed_envs: Vec::new(),
        }
    }

    fn entry_line_at(&self, idx: usize) -> Option<usize> {
        self.entry_lines.get(idx).copied().flatten()
    }

    fn generate(&mut self) -> Result<String, GenerateError> {
        // Validate the entire call list up-front so missing-required-field
        // diagnostics are reported as a batch (one error per offending
        // call) rather than aborting at the first one. The generator's
        // emitters assume validation has already passed and drop their
        // historical `unwrap_or(...)` defaults for required fields.
        let mut missing: Vec<MissingRequired> = Vec::new();
        for (idx, call) in self.calls.iter().enumerate() {
            if let Err(err) = validate_call(call, self.entry_line_at(idx)) {
                missing.push(err);
            }
        }
        if !missing.is_empty() {
            return Err(GenerateError::MissingRequired(missing));
        }

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

        if !self.unsupported.is_empty() && !self.config.allow_unsupported {
            let names: Vec<UnsupportedCall> = self
                .unsupported
                .iter()
                .map(|(name, count)| UnsupportedCall {
                    name: name.clone(),
                    count: *count,
                })
                .collect();
            return Err(GenerateError::Unsupported(names));
        }

        self.emit_test_close();
        Ok(self.output.clone())
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
                HandleType::Env if !implicit_envs.contains(&addr.to_string()) => {
                    implicit_envs.push(addr.to_string());
                }
                HandleType::Dbc if !implicit_dbcs.contains(&addr.to_string()) => {
                    implicit_dbcs.push(addr.to_string());
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
                OdbcCall::SetStmtAttr(c) => {
                    if let Some(a) = &c.handle {
                        record_implicit(a, HandleType::Stmt);
                    }
                }
                OdbcCall::ColAttribute(c) => {
                    if let Some(a) = &c.handle {
                        record_implicit(a, HandleType::Stmt);
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
            self.unfreed_envs.push(var.clone());
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
        self.writeln("#include <algorithm>");
        self.writeln("#include <cstring>");
        self.writeln("#include <string>");
        self.writeln("#include <vector>");
        if self.config.capture_mode {
            self.writeln("#include <cmath>");
            self.writeln("#include <cstdio>");
            self.writeln("#include <fstream>");
            self.writeln("#include \"picojson.h\"");
        }
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
        if self.config.capture_mode {
            self.writeln("picojson::object captured_values;");
        }
        self.writeln("");
    }

    fn emit_test_close(&mut self) {
        self.emit_env_cleanup_epilogue();
        if self.config.capture_mode {
            self.writeln("// Write captured GetData values (wrapper gates apply on ctest exit 0)");
            self.writeln("{");
            self.indent += 1;
            self.writeln("const char* out_path = std::getenv(\"CAPTURE_OUTPUT_PATH\");");
            self.writeln("REQUIRE(out_path != nullptr);");
            self.writeln("std::ofstream out(out_path);");
            self.writeln("REQUIRE(out.good());");
            self.writeln("out << picojson::value(captured_values).serialize() << std::endl;");
            self.indent -= 1;
            self.writeln("}");
            self.writeln("");
        }
        if self.skipped_col_attr_undocumented > 0 {
            self.writeln(&format!(
                "// skipped {} SQLColAttribute call(s) with undocumented field id",
                self.skipped_col_attr_undocumented,
            ));
        }
        let saved = self.indent;
        self.indent = 0;
        self.writeln("}");
        self.indent = saved;
    }

    /// Emit `SQLFreeHandle(SQL_HANDLE_ENV, …)` for every env this replay
    /// allocated but never explicitly freed. See [`Self::unfreed_envs`] for
    /// the rationale — this is a deliberate divergence from the captured
    /// trace, so we surround it with an explanatory comment.
    fn emit_env_cleanup_epilogue(&mut self) {
        if self.unfreed_envs.is_empty() {
            return;
        }
        let envs = std::mem::take(&mut self.unfreed_envs);
        self.writeln("// --- Replay-only env cleanup (not present in the original trace) ---");
        self.writeln("// ODBC-consuming hosts (Excel, Power Query, ...) deliberately leave their");
        self.writeln("// SQL_HANDLE_ENV handles allocated at shutdown — the pool root for");
        self.writeln("// `SQL_ATTR_CONNECTION_POOLING` is anchored on the env, and any teardown");
        self.writeln(
            "// done during DllMain(DLL_PROCESS_DETACH) is invisible to the trace logger.",
        );
        self.writeln("// Our replay binary runs many tests in one process, so we explicitly free");
        self.writeln("// each leaked env here to avoid leaking pooled connections across tests.");
        for env in envs {
            self.writeln(&format!("// SQLFreeHandle - SQLHENV ({env}, replay-only)"));
            self.writeln("{");
            self.indent += 1;
            self.writeln(&format!(
                "SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_ENV, {env});"
            ));
            self.writeln(&format!(
                "CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, {env}),"
            ));
            self.writeln("           OdbcMatchers::IsSuccess());");
            self.indent -= 1;
            self.writeln("}");
            self.writeln("");
        }
    }

    fn emit_call(&mut self, call: &OdbcCall) {
        match call {
            OdbcCall::DriverConnect(c) => self.emit_driver_connect(c),
            OdbcCall::AllocHandle(c) => self.emit_alloc_handle(c),
            OdbcCall::SetEnvAttr(c) => self.emit_set_env_attr(c),
            OdbcCall::SetConnectAttr(c) => self.emit_set_connect_attr(c),
            OdbcCall::SetStmtAttr(c) => self.emit_set_stmt_attr(c),
            OdbcCall::ColAttribute(c) => self.emit_col_attribute(c),
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
            OdbcCall::Unsupported(c) => {
                *self.unsupported.entry(c.function_name.clone()).or_insert(0) += 1;
                if self.config.allow_unsupported {
                    self.writeln(&format!(
                        "// TODO: unsupported ODBC call {}",
                        c.function_name
                    ));
                }
            }
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
        if ht == HandleType::Env {
            self.unfreed_envs.push(var_name.clone());
        }

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
        // `column_number` / `buffer_length` are required - the validator
        // refuses to emit when they're missing, so the `expect` here is
        // purely a static assertion that we ran the validator first.
        let col_num = call
            .column_number
            .expect("validate_call enforces SQLDescribeCol.column_number");
        let buf_len = call
            .buffer_length
            .expect("validate_call enforces SQLDescribeCol.buffer_length");

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
        let fetch_orientation = call
            .orientation_name
            .as_deref()
            .expect("validate_call enforces SQLFetchScroll.orientation_name");
        let offset = call
            .offset
            .expect("validate_call enforces SQLFetchScroll.offset");

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
        let col_num = call
            .column_number
            .expect("validate_call enforces SQLGetData.column_number");
        let target_type = call
            .target_type_name
            .as_deref()
            .expect("validate_call enforces SQLGetData.target_type_name");
        let buf_len = call
            .buffer_length
            .expect("validate_call enforces SQLGetData.buffer_length")
            .max(0);

        self.writeln(&format!("// SQLGetData col {col_num}"));
        self.writeln("{");
        self.indent += 1;

        if target_type == "SQL_C_CHAR" || target_type == "SQL_CHAR" {
            // Sentinel-fill (0xFF) so a driver that silently fails to write
            // is detectable: a zero-filled buffer would look indistinguishable
            // from a legitimate empty/zero payload, but 0xFF is never a valid
            // ASCII character and never produced by a successful narrow read.
            self.writeln(&format!(
                "std::vector<char> buf({}, static_cast<char>(0xFF));",
                buf_len + 1
            ));
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
                match ind_val {
                    -1 => self.writeln("CHECK(ind == SQL_NULL_DATA);"),
                    -4 => self.writeln("CHECK(ind == SQL_NO_TOTAL);"),
                    _ => {
                        if let Some(val) = &call.value {
                            let escaped = escape_cpp_string_literal(val);
                            // Length-bounded comparison: SQL_C_CHAR's NUL
                            // terminator is normally honoured, but with a
                            // 0xFF sentinel fill we can't rely on it if the
                            // driver writes a partial buffer without a NUL.
                            // `ind` is the byte length (excluding NUL), capped
                            // to the buffer size to defend against drivers
                            // that report a larger untruncated length on
                            // SQL_SUCCESS_WITH_INFO.
                            self.writeln(
                                "const size_t n = std::min<size_t>(static_cast<size_t>(ind), buf.size());",
                            );
                            self.writeln(&format!(
                                "CHECK(std::string(buf.data(), n) == \"{escaped}\");"
                            ));
                        }
                        self.writeln(&format!("CHECK(ind == {ind_val});"));
                    }
                }
            } else if let Some(val) = &call.value {
                let escaped = escape_cpp_string_literal(val);
                self.writeln(
                    "const size_t n = std::min<size_t>(static_cast<size_t>(ind), buf.size());",
                );
                self.writeln(&format!(
                    "CHECK(std::string(buf.data(), n) == \"{escaped}\");"
                ));
            }
        } else {
            self.writeln("SQLLEN ind = 0;");
            self.writeln(&format!(
                "std::vector<char> buf({buf_len}, static_cast<char>(0xFF));"
            ));
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
                match ind_val {
                    -1 => self.writeln("CHECK(ind == SQL_NULL_DATA);"),
                    -4 => self.writeln("CHECK(ind == SQL_NO_TOTAL);"),
                    _ => {
                        // SQL_C_WCHAR is specified by ODBC as UTF-16, with the
                        // indicator giving the payload byte count (excluding
                        // the trailing NUL pair). On unixODBC / Windows DM
                        // (everything our test infra targets), SQLWCHAR is a
                        // 2-byte code unit. We reinterpret the byte buffer as
                        // `char16_t*` and compare against a `u"..."` literal
                        // built from the trace's UTF-8 capture — the compiler
                        // re-encodes the source UTF-8 to UTF-16 at compile
                        // time, so we never need a runtime converter.
                        //
                        // NOTE: this assumes 2-byte SQLWCHAR. iODBC builds
                        // that use a 4-byte SQLWCHAR (UTF-32) would need a
                        // separate code path; we'd add it the day we run the
                        // replay tests against such a build.
                        // The WinODBC Driver Manager renders SQL_C_WCHAR
                        // payloads to the trace file via
                        // `WideCharToMultiByte(CP_ACP, …)`, which replaces
                        // every codepoint unmappable in the Windows ANSI
                        // codepage (CJK, emoji, RTL, anything outside the
                        // active codepage's range) with a literal '?'. Once
                        // a captured value contains '?', we cannot tell
                        // whether it was a real question mark in the data
                        // or a replacement marker for a non-ANSI codepoint
                        // — so we must skip the value assertion. iODBC's
                        // trace formatter is UTF-8 lossless, so this only
                        // false-negatives on iODBC traces that happen to
                        // contain a genuine '?', which we accept as the
                        // safer tradeoff vs. the WinODBC false-positive
                        // assertion failures it would otherwise cause.
                        if let Some(val) = call
                            .value
                            .as_ref()
                            .filter(|_| target_type == "SQL_C_WCHAR")
                            .filter(|v| !v.contains('?'))
                        {
                            let escaped = escape_cpp_string_literal(val);
                            self.writeln("const size_t code_units = std::min<size_t>(");
                            self.writeln(
                                "    static_cast<size_t>(ind) / sizeof(char16_t), buf.size() / sizeof(char16_t));",
                            );
                            self.writeln(
                                "std::u16string actual(reinterpret_cast<const char16_t*>(buf.data()), code_units);",
                            );
                            self.writeln(&format!("CHECK(actual == u\"{escaped}\");"));
                        } else if target_type == "SQL_C_WCHAR"
                            && call.value.as_deref().is_some_and(|v| v.contains('?'))
                        {
                            // Surface the skip in the generated test so
                            // readers don't wonder where the value check
                            // went — without this line the assertion gap
                            // is invisible.
                            self.writeln(
                                "// SQL_C_WCHAR value not pinned: trace rendering used CP_ACP",
                            );
                            self.writeln(
                                "// and may have replaced unmappable codepoints with '?'.",
                            );
                        }
                        self.writeln(&format!("CHECK(ind == {ind_val});"));
                        if !self.config.capture_mode {
                            if let Some(ref captured) = call.captured {
                                for line in
                                    getdata_codec::captured_assert_lines(target_type, captured)
                                {
                                    self.writeln(&line);
                                }
                            }
                        }
                    }
                }
            } else if !self.config.capture_mode {
                if let Some(ref captured) = call.captured {
                    for line in getdata_codec::captured_assert_lines(target_type, captured) {
                        self.writeln(&line);
                    }
                }
            }

            if self.config.capture_mode && getdata_codec::is_obscured_target(target_type) {
                if let Some(seq) = call.seq {
                    for line in getdata_codec::capture_record_lines(seq, target_type) {
                        self.writeln(&line);
                    }
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
        // Validator guarantees at least one of `info_type` (symbolic name)
        // or `info_type_value` (raw integer) is populated. Prefer the
        // symbolic name when available so the emitted call reads as
        // `SQLGetInfo(dbc0, SQL_OWNER_USAGE, ...)`; fall back to the raw
        // integer for InfoTypes that the Windows DM trace renders as
        // `<unknown>` (e.g. `UWORD 169 <unknown>` for
        // `SQL_AGGREGATE_FUNCTIONS`).
        let info_type_owned: String = call
            .info_type
            .clone()
            .or_else(|| call.info_type_value.map(|v| v.to_string()))
            .expect(
                "validate_call ensures at least one of info_type / info_type_value is populated",
            );
        let info_type_name: &str = info_type_owned.as_str();
        let dbc_var = self.dbc_var_for(&call.handle);

        // Two orthogonal `SQLGetInfo` policies, decided independently:
        //
        //   1. **Return code**: strict by default — the assertion mirrors the
        //      captured `ReturnCode` from the trace. Non-symbolic info types
        //      (raw integers like `SQLGetInfo(0)` that the trace captured but
        //      that neither driver is obligated to implement) are relaxed to
        //      `Succeeded()` because their return codes legitimately diverge
        //      across DM/driver/platform combinations.
        //
        //   2. **Value**: strict by default. The shape of the assertion is
        //      chosen by which field the parser populated:
        //        * `info_value: Some(_)` → string-typed info type, emit
        //          `CHECK(std::string(buf) == "<captured>")`.
        //        * `info_value_numeric: Some(_)` → numeric/bitmask info type,
        //          emit `memcpy` into a `SQLUINTEGER` and compare.
        //      Skip the value check only for info types in
        //      [`INFO_TYPES_WITH_UNSTABLE_VALUES`] (driver/DBMS/DM version
        //      strings whose bytes shift between runs) or when the trace
        //      didn't capture either form (some legacy traces).
        //
        // The relaxation only kicks in when the trace captured a *success*
        // code — when the trace captured an error (e.g. WinODBC's
        // `<unknown>` InfoTypes like 180 that the Snowflake driver rejects
        // with HY000), the reference driver also returns an error and the
        // relaxed `Succeeded()` matcher would inappropriately demand
        // success. Mirroring the captured failure code keeps the
        // assertion honest in that case.
        let is_symbolic = info_type_name.starts_with("SQL_");
        let return_code_relaxed = !is_symbolic && call.return_code.is_success();
        let value_is_unstable = INFO_TYPES_WITH_UNSTABLE_VALUES.contains(&info_type_name);

        self.writeln(&format!("// SQLGetInfo - {info_type_name}"));
        self.writeln("{");
        self.indent += 1;
        self.writeln("char buf[256] = {};");
        self.writeln("SQLSMALLINT len = 0;");
        self.writeln(&format!(
            "SQLRETURN ret = SQLGetInfo({dbc_var}, {info_type_name}, buf, 255, &len);"
        ));

        if return_code_relaxed {
            self.emit_return_assertion_relaxed("SQL_HANDLE_DBC", &dbc_var);
        } else {
            self.emit_return_assertion(call.return_code, "SQL_HANDLE_DBC", &dbc_var, false, false);
        }

        // Emit a value check whenever the call succeeded — comparing buffer
        // contents after an error response would just be reading uninitialised
        // memory. The parser populates exactly one of `info_value` (string)
        // or `info_value_numeric` (integer/bitmask) per the InfoType's ODBC
        // category; we match accordingly:
        //   * String types → `CHECK(std::string(buf) == "<captured>")`.
        //   * Numeric/bitmask types → `memcpy` into a `SQLUINTEGER` and
        //     compare. We zero-initialise `buf` above, so this is safe even
        //     when the driver wrote a 16-bit `SQLUSMALLINT` (the high bytes
        //     stay zero and the read still yields the captured value on the
        //     little-endian targets we run).
        if !value_is_unstable && call.return_code.is_success() {
            if let Some(val) = &call.info_value {
                let escaped = escape_cpp_string_literal(val);
                self.writeln(&format!("CHECK(std::string(buf) == \"{escaped}\");"));
            } else if let Some(numeric) = call.info_value_numeric {
                let as_u32 = numeric as u32;
                self.writeln("SQLUINTEGER numericValue = 0;");
                self.writeln("std::memcpy(&numericValue, buf, sizeof(numericValue));");
                self.writeln(&format!("CHECK(numericValue == 0x{as_u32:X}u);"));
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
        if ht == HandleType::Env {
            self.unfreed_envs.retain(|v| v != &var_name);
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
        let attr_name = call
            .attribute
            .as_deref()
            .expect("validate_call enforces SQLSetEnvAttr.attribute");
        let env_var = self.env_var_for(&call.handle);

        if attr_name == "SQL_ATTR_ODBC_VERSION" && self.was_synthetically_set(&env_var) {
            return;
        }

        let value = call
            .value
            .expect("validate_call enforces SQLSetEnvAttr.value");
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
        let attr_name = call
            .attribute
            .as_deref()
            .expect("validate_call enforces SQLSetConnectAttr.attribute");
        let value = call
            .value
            .expect("validate_call enforces SQLSetConnectAttr.value");
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

    fn emit_set_stmt_attr(&mut self, call: &crate::model::SetStmtAttr) {
        let attr_name = call
            .attribute
            .as_deref()
            .expect("validate_call enforces SQLSetStmtAttr.attribute");
        let value = call
            .value
            .expect("validate_call enforces SQLSetStmtAttr.value");
        let value_expr = attr_value_cpp_expr(attr_name, value);
        let str_len = call.str_len.unwrap_or(0);
        let stmt_var = self.stmt_var_for(&call.handle);

        self.writeln(&format!("// SQLSetStmtAttr - {attr_name}"));
        self.writeln("{");
        self.indent += 1;
        self.writeln(&format!(
            "SQLRETURN ret = SQLSetStmtAttr({stmt_var}, {attr_name},"
        ));
        self.writeln(&format!("    {value_expr}, {str_len});"));
        self.emit_return_assertion(call.return_code, "SQL_HANDLE_STMT", &stmt_var, false, false);
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn emit_col_attribute(&mut self, call: &crate::model::ColAttribute) {
        // Excel probes undocumented descriptor fields (e.g. attribute 32) that
        // the reference driver rejects with HY091; tally them and skip replay.
        //
        // After the `<unknown>` parser fix, every captured row has at least
        // a `field_identifier_value` even when the symbolic name is missing,
        // so the validator no longer rejects this case. The skip here is
        // therefore an *intentional* emit-policy choice, not silent data
        // loss — we drop calls whose only identifier is the raw integer
        // because the reference driver will return HY091 for any field id
        // the documented descriptor table doesn't list.
        let Some(field_id) = call.field_identifier.as_deref() else {
            self.skipped_col_attr_undocumented += 1;
            return;
        };

        let stmt_var = self.stmt_var_for(&call.handle);
        let col_num = call
            .column_number
            .expect("validate_call enforces SQLColAttribute.column_number");

        self.writeln(&format!("// SQLColAttribute - {field_id}"));
        self.writeln("{");
        self.indent += 1;
        self.writeln(&format!("SQLUSMALLINT col = {col_num};"));
        self.writeln("SQLLEN numAttr = 0;");
        self.writeln("SQLSMALLINT strLen = 0;");

        if call.character_value.is_some() || call.buffer_length.unwrap_or(0) > 0 {
            // Use the captured buffer length verbatim so the replay reproduces
            // any truncation behavior the original call hit. Clamp to i16::MAX
            // because SQLColAttribute's BufferLength is SQLSMALLINT (narrowing
            // cast would silently wrap), and floor at 1 because C++ does not
            // permit zero-sized arrays for the buf declaration below.
            let buf_size = call
                .buffer_length
                .unwrap_or(256)
                .min(i16::MAX as i64)
                .max(1);
            self.writeln(&format!("char buf[{buf_size}] = {{}};"));
            self.writeln(&format!(
                "SQLRETURN ret = SQLColAttribute({stmt_var}, col, {field_id},"
            ));
            self.writeln(&format!("    buf, {buf_size}, &strLen, &numAttr);"));
        } else {
            self.writeln(&format!(
                "SQLRETURN ret = SQLColAttribute({stmt_var}, col, {field_id},"
            ));
            self.writeln("    nullptr, 0, &strLen, &numAttr);");
        }

        self.emit_return_assertion(call.return_code, "SQL_HANDLE_STMT", &stmt_var, false, false);
        // Pin the returned numeric attribute. The Windows DM logs it either as a
        // bare integer (`numeric_attribute`) or as a symbolic ODBC constant
        // (`numeric_attribute_name`, e.g. `SQL_DESC_CONCISE_TYPE` -> `SQL_VARCHAR`).
        // Emit a value check from whichever the parser captured; the symbolic
        // name is a header-defined constant so it compiles directly. Only emit
        // for `SQL_`-prefixed names to stay compile-safe against names the target
        // headers might not define.
        if let Some(val) = call.numeric_attribute {
            self.writeln(&format!("CHECK(numAttr == {val});"));
        } else if let Some(name) = call
            .numeric_attribute_name
            .as_deref()
            .filter(|n| n.starts_with("SQL_"))
        {
            self.writeln(&format!("CHECK(numAttr == {name});"));
        }

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
            ..Default::default()
        };

        let calls: Vec<_> = trace.calls.iter().map(|tc| tc.call.clone()).collect();
        let output = generate(&calls, &config).expect("generate");

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
            ..Default::default()
        };

        let calls: Vec<_> = trace.calls.iter().map(|tc| tc.call.clone()).collect();
        let output = generate(&calls, &config).expect("generate");

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
    fn test_get_data_emits_sql_no_total_and_keeps_large_buffer() {
        use crate::model::{GetData, OdbcCall};

        let calls = vec![
            OdbcCall::GetData(GetData {
                return_code: crate::model::ReturnCode::SuccessWithInfo,
                handle: Some("0xstmt".to_string()),
                column_number: Some(1),
                target_type: Some(-8),
                target_type_name: Some("SQL_C_WCHAR".to_string()),
                buffer_length: Some(131074),
                indicator: Some(-4),
                ..Default::default()
            }),
            OdbcCall::GetData(GetData {
                return_code: crate::model::ReturnCode::Success,
                handle: Some("0xstmt".to_string()),
                column_number: Some(1),
                target_type: Some(-8),
                target_type_name: Some("SQL_C_WCHAR".to_string()),
                buffer_length: Some(262146),
                indicator: Some(2),
                ..Default::default()
            }),
        ];

        let config = GeneratorConfig {
            test_name: "lob streaming".to_string(),
            allow_unsupported: true,
            ..Default::default()
        };

        let output = generate(&calls, &config).expect("generate");

        assert!(
            output.contains("std::vector<char> buf(131074, static_cast<char>(0xFF));"),
            "131074-byte buffer must be preserved (no 4096 clamp) and sentinel-filled; output:\n{output}",
        );
        assert!(
            output.contains("std::vector<char> buf(262146, static_cast<char>(0xFF));"),
            "262146-byte buffer must be preserved and sentinel-filled; output:\n{output}",
        );
        assert!(
            output.contains("CHECK(ind == SQL_NO_TOTAL);"),
            "ind == -4 must render as SQL_NO_TOTAL; output:\n{output}",
        );
        // Without a captured `value`, the second SQL_C_WCHAR call (positive
        // ind, value=None) gets a strict indicator check but no string
        // comparison — there is no expected payload to assert against.
        assert!(
            output.contains("CHECK(ind == 2);"),
            "positive indicator should be pinned to the captured value; output:\n{output}",
        );
        assert!(
            !output.contains("CHECK(ind > 0);"),
            "must not emit the legacy presence-only check now that indicator is strict; output:\n{output}",
        );
        assert!(
            !output.contains("CHECK(ind == -4)"),
            "must not emit raw -4 literal; output:\n{output}",
        );
    }

    #[test]
    fn col_attribute_pins_symbolic_numeric_attribute_name() {
        use crate::model::{ColAttribute, OdbcCall};

        // Windows DM rendered the returned attribute as a symbolic constant, so
        // the parser populated `numeric_attribute_name` and left the integer
        // `numeric_attribute` null. The generator must still pin the value.
        let calls = vec![OdbcCall::ColAttribute(ColAttribute {
            return_code: crate::model::ReturnCode::Success,
            handle: Some("0xstmt".to_string()),
            column_number: Some(1),
            field_identifier: Some("SQL_DESC_CONCISE_TYPE".to_string()),
            field_identifier_value: Some(2),
            buffer_length: None,
            string_length: None,
            numeric_attribute: None,
            numeric_attribute_name: Some("SQL_DECIMAL".to_string()),
            character_value: None,
        })];

        let config = GeneratorConfig {
            test_name: "colattr".to_string(),
            allow_unsupported: true,
            ..Default::default()
        };
        let output = generate(&calls, &config).expect("generate");

        assert!(
            output.contains("CHECK(numAttr == SQL_DECIMAL);"),
            "must pin the symbolic numeric attribute when only the name is captured; output:\n{output}",
        );
    }

    #[test]
    fn col_attribute_skips_non_sql_numeric_attribute_name() {
        use crate::model::{ColAttribute, OdbcCall};

        // A name the target headers might not define must not be emitted, to
        // keep the generated test compile-safe (mirrors the SQLGetInfo policy).
        let calls = vec![OdbcCall::ColAttribute(ColAttribute {
            return_code: crate::model::ReturnCode::Success,
            handle: Some("0xstmt".to_string()),
            column_number: Some(1),
            field_identifier: Some("SQL_DESC_CONCISE_TYPE".to_string()),
            field_identifier_value: Some(2),
            buffer_length: None,
            string_length: None,
            numeric_attribute: None,
            numeric_attribute_name: Some("<unknown>".to_string()),
            character_value: None,
        })];

        let config = GeneratorConfig {
            test_name: "colattr".to_string(),
            allow_unsupported: true,
            ..Default::default()
        };
        let output = generate(&calls, &config).expect("generate");

        assert!(
            !output.contains("CHECK(numAttr =="),
            "must not pin a non-SQL_ symbolic name; output:\n{output}",
        );
    }

    #[test]
    fn col_attribute_pins_integer_numeric_attribute() {
        use crate::model::{ColAttribute, OdbcCall};

        // The original path: Windows DM logged the returned attribute as a raw
        // integer (numeric_attribute is set, numeric_attribute_name is absent).
        // The generator must emit CHECK(numAttr == <value>).
        let calls = vec![OdbcCall::ColAttribute(ColAttribute {
            return_code: crate::model::ReturnCode::Success,
            handle: Some("0xstmt".to_string()),
            column_number: Some(1),
            field_identifier: Some("SQL_DESC_CONCISE_TYPE".to_string()),
            field_identifier_value: Some(2),
            buffer_length: None,
            string_length: None,
            numeric_attribute: Some(12),
            numeric_attribute_name: None,
            character_value: None,
        })];

        let config = GeneratorConfig {
            test_name: "colattr".to_string(),
            allow_unsupported: true,
            ..Default::default()
        };
        let output = generate(&calls, &config).expect("generate");

        assert!(
            output.contains("CHECK(numAttr == 12);"),
            "must pin the raw integer numeric attribute; output:\n{output}",
        );
    }

    #[test]
    fn col_attribute_emits_no_check_when_numeric_attribute_absent() {
        use crate::model::{ColAttribute, OdbcCall};

        // When neither numeric_attribute nor numeric_attribute_name is present
        // (e.g. the DM only recorded a string value for this field), no numAttr
        // CHECK should be emitted.
        let calls = vec![OdbcCall::ColAttribute(ColAttribute {
            return_code: crate::model::ReturnCode::Success,
            handle: Some("0xstmt".to_string()),
            column_number: Some(1),
            field_identifier: Some("SQL_DESC_TYPE_NAME".to_string()),
            field_identifier_value: Some(14),
            buffer_length: None,
            string_length: None,
            numeric_attribute: None,
            numeric_attribute_name: None,
            character_value: None,
        })];

        let config = GeneratorConfig {
            test_name: "colattr".to_string(),
            allow_unsupported: true,
            ..Default::default()
        };
        let output = generate(&calls, &config).expect("generate");

        assert!(
            !output.contains("CHECK(numAttr =="),
            "must not emit a numAttr check when neither field is present; output:\n{output}",
        );
    }

    #[test]
    fn get_info_emits_value_check_for_stable_info_types() {
        use crate::model::{GetInfo, OdbcCall};

        let calls = vec![OdbcCall::GetInfo(GetInfo {
            return_code: crate::model::ReturnCode::Success,
            handle: Some("0xdbc".to_string()),
            info_type: Some("SQL_IDENTIFIER_QUOTE_CHAR".to_string()),
            info_type_value: Some(29),
            info_value: Some("\"".to_string()),
            info_value_numeric: None,
        })];

        let config = GeneratorConfig {
            test_name: "getinfo".to_string(),
            allow_unsupported: true,
            ..Default::default()
        };
        let output = generate(&calls, &config).expect("generate");

        assert!(
            output.contains("SQLGetInfo(dbc0, SQL_IDENTIFIER_QUOTE_CHAR, buf, 255, &len);"),
            "call should be emitted; output:\n{output}",
        );
        assert!(
            output.contains("OdbcMatchers::IsSuccess()"),
            "stable info type should keep the strict return-code matcher; output:\n{output}",
        );
        assert!(
            output.contains("CHECK(std::string(buf) == \"\\\"\");"),
            "stable info type should emit an exact-value check (with the captured `\"` escaped); output:\n{output}",
        );
    }

    #[test]
    fn get_info_skips_value_check_for_unstable_info_types() {
        use crate::model::{GetInfo, OdbcCall};

        let calls = vec![
            OdbcCall::GetInfo(GetInfo {
                return_code: crate::model::ReturnCode::Success,
                handle: Some("0xdbc".to_string()),
                info_type: Some("SQL_DRIVER_NAME".to_string()),
                info_type_value: Some(6),
                info_value: Some("snowflake.so".to_string()),
                info_value_numeric: None,
            }),
            OdbcCall::GetInfo(GetInfo {
                return_code: crate::model::ReturnCode::Success,
                handle: Some("0xdbc".to_string()),
                info_type: Some("SQL_DBMS_VER".to_string()),
                info_type_value: Some(18),
                info_value: Some("10.19.100".to_string()),
                info_value_numeric: None,
            }),
        ];

        let config = GeneratorConfig {
            test_name: "unstable".to_string(),
            allow_unsupported: true,
            ..Default::default()
        };
        let output = generate(&calls, &config).expect("generate");

        // Return code stays strict (these always succeed in practice) …
        assert!(
            output.contains("OdbcMatchers::IsSuccess()"),
            "strict return-code matcher even for unstable values; output:\n{output}",
        );
        // … but the captured value strings are NOT emitted as assertions.
        assert!(
            !output.contains("snowflake.so"),
            "unstable SQL_DRIVER_NAME value must not be pinned; output:\n{output}",
        );
        assert!(
            !output.contains("10.19.100"),
            "unstable SQL_DBMS_VER value must not be pinned; output:\n{output}",
        );
    }

    #[test]
    fn get_info_emits_numeric_check_for_bitmask_info_types() {
        use crate::model::{GetInfo, OdbcCall};

        // Numeric/bitmask info types like SQL_CATALOG_USAGE carry their value
        // in a SQLUINTEGER pointer; the parser captures it as
        // `info_value_numeric` and we emit a memcpy + integer comparison so
        // a driver returning the wrong flag bits trips the test.
        let calls = vec![OdbcCall::GetInfo(GetInfo {
            return_code: crate::model::ReturnCode::Success,
            handle: Some("0xdbc".to_string()),
            info_type: Some("SQL_CATALOG_USAGE".to_string()),
            info_type_value: Some(92),
            info_value: None,
            info_value_numeric: Some(0x15),
        })];

        let config = GeneratorConfig {
            test_name: "numeric".to_string(),
            allow_unsupported: true,
            ..Default::default()
        };
        let output = generate(&calls, &config).expect("generate");

        assert!(
            output.contains("OdbcMatchers::IsSuccess()"),
            "strict return-code matcher; output:\n{output}",
        );
        assert!(
            !output.contains("CHECK(std::string(buf) =="),
            "numeric info types must not get a string comparison; output:\n{output}",
        );
        assert!(
            output.contains("SQLUINTEGER numericValue = 0;"),
            "numeric value scratch should be emitted; output:\n{output}",
        );
        assert!(
            output.contains("std::memcpy(&numericValue, buf, sizeof(numericValue));"),
            "memcpy lift should be emitted; output:\n{output}",
        );
        assert!(
            output.contains("CHECK(numericValue == 0x15u);"),
            "captured bitmask value should be checked verbatim; output:\n{output}",
        );
    }

    #[test]
    fn get_info_skips_value_check_when_trace_captured_nothing() {
        use crate::model::{GetInfo, OdbcCall};

        // Some legacy iodbc traces don't render the InfoValue at all — neither
        // string nor integer. In that case we still assert the return code
        // but have nothing to compare the buffer against.
        let calls = vec![OdbcCall::GetInfo(GetInfo {
            return_code: crate::model::ReturnCode::Success,
            handle: Some("0xdbc".to_string()),
            info_type: Some("SQL_CATALOG_USAGE".to_string()),
            info_type_value: Some(92),
            info_value: None,
            info_value_numeric: None,
        })];

        let config = GeneratorConfig {
            test_name: "missing".to_string(),
            allow_unsupported: true,
            ..Default::default()
        };
        let output = generate(&calls, &config).expect("generate");

        assert!(
            output.contains("OdbcMatchers::IsSuccess()"),
            "strict return-code matcher; output:\n{output}",
        );
        assert!(
            !output.contains("CHECK(std::string(buf) =="),
            "no string check; output:\n{output}",
        );
        assert!(
            !output.contains("std::memcpy"),
            "no numeric check when the trace captured no value; output:\n{output}",
        );
    }

    #[test]
    fn get_info_keeps_strict_return_code_for_non_symbolic_when_trace_errored() {
        use crate::model::{GetInfo, OdbcCall};

        // The non-symbolic-InfoType relaxation only applies when the
        // trace captured a *success*. When the trace captured an error
        // (e.g. Excel/PQ's `SQLGetInfo - 180` which Snowflake rejects
        // with HY000), the reference driver returns the same error, so
        // we must keep the strict `IsError()` matcher — `Succeeded()`
        // would invert the assertion's truthiness.
        let calls = vec![OdbcCall::GetInfo(GetInfo {
            return_code: crate::model::ReturnCode::Error,
            handle: Some("0xdbc".to_string()),
            info_type: None,
            info_type_value: Some(180),
            info_value: None,
            info_value_numeric: None,
        })];

        let config = GeneratorConfig {
            test_name: "raw int error".to_string(),
            allow_unsupported: true,
            ..Default::default()
        };
        let output = generate(&calls, &config).expect("generate");

        assert!(
            output.contains("SQLGetInfo(dbc0, 180, buf, 255, &len);"),
            "integer info type rendered verbatim; output:\n{output}",
        );
        assert!(
            output.contains("OdbcMatchers::IsError()"),
            "non-symbolic info type with recorded SQL_ERROR must keep the strict matcher; output:\n{output}",
        );
        assert!(
            !output.contains("OdbcMatchers::Succeeded()"),
            "must not relax to Succeeded() when the trace recorded an error; output:\n{output}",
        );
    }

    #[test]
    fn get_info_falls_back_to_integer_when_symbolic_name_is_missing() {
        use crate::model::{GetInfo, OdbcCall};

        // Mirrors the WinODBC `UWORD 169 <unknown>` case after the parser
        // fix: `info_type` is `None` because the trace had no symbolic
        // name, but `info_type_value: Some(169)` carries the recovered
        // integer. The emitter must surface the integer rather than
        // silently substituting `0` (= `SQL_INFO_FIRST`, a different
        // real call).
        let calls = vec![OdbcCall::GetInfo(GetInfo {
            return_code: crate::model::ReturnCode::Success,
            handle: Some("0xdbc".to_string()),
            info_type: None,
            info_type_value: Some(169),
            info_value: None,
            info_value_numeric: Some(0x7F),
        })];

        let config = GeneratorConfig {
            test_name: "unknown info type".to_string(),
            allow_unsupported: true,
            ..Default::default()
        };
        let output = generate(&calls, &config).expect("generate");

        assert!(
            output.contains("SQLGetInfo(dbc0, 169, buf, 255, &len);"),
            "integer info type must be emitted verbatim; output:\n{output}",
        );
        assert!(
            output.contains("// SQLGetInfo - 169"),
            "comment line should reflect the integer InfoType; output:\n{output}",
        );
        assert!(
            !output.contains("SQLGetInfo(dbc0, 0,"),
            "must not silently fall back to `0`; output:\n{output}",
        );
        // The integer-only path goes through the relaxed (Succeeded()) matcher
        // because `starts_with(\"SQL_\")` is false for the bare integer.
        assert!(
            output.contains("OdbcMatchers::Succeeded()"),
            "integer InfoType uses the relaxed matcher; output:\n{output}",
        );
    }

    #[test]
    fn generate_rejects_get_info_with_no_info_type_at_all() {
        use crate::model::{GetInfo, OdbcCall};

        // Validator boundary: when BOTH `info_type` (symbolic) and
        // `info_type_value` (numeric) are missing, we must not emit a
        // `SQLGetInfo(dbc0, 0, ...)` substitute. Generation fails-fast.
        let calls = vec![OdbcCall::GetInfo(GetInfo {
            return_code: crate::model::ReturnCode::Success,
            handle: Some("0xdbc".to_string()),
            info_type: None,
            info_type_value: None,
            info_value: None,
            info_value_numeric: None,
        })];

        let config = GeneratorConfig {
            test_name: "no info type".to_string(),
            allow_unsupported: true,
            ..Default::default()
        };

        let err = generate(&calls, &config)
            .expect_err("generator must reject an IR with no info_type / info_type_value");
        match err {
            GenerateError::MissingRequired(missing) => {
                assert_eq!(missing.len(), 1);
                assert_eq!(missing[0].call, "SQLGetInfo");
                assert_eq!(missing[0].field, "info_type|info_type_value");
            }
            other => panic!("expected MissingRequired, got {other:?}"),
        }
    }

    #[test]
    fn generate_rejects_get_data_with_missing_target_type() {
        use crate::model::{GetData, OdbcCall};

        // Without the validator the emitter would silently substitute
        // `SQL_C_CHAR` — a completely different wire layout from
        // `SQL_C_WCHAR`. Refuse to generate so the trace must be re-
        // captured or the parser fixed.
        let calls = vec![OdbcCall::GetData(GetData {
            return_code: crate::model::ReturnCode::Success,
            handle: Some("0xstmt".to_string()),
            column_number: Some(1),
            target_type: Some(-8),
            target_type_name: None,
            buffer_length: Some(256),
            ..Default::default()
        })];

        let config = GeneratorConfig {
            test_name: "no target type".to_string(),
            allow_unsupported: true,
            ..Default::default()
        };
        let err = generate(&calls, &config)
            .expect_err("generator must reject a SQLGetData with no target_type_name");
        match err {
            GenerateError::MissingRequired(missing) => {
                assert_eq!(missing[0].field, "target_type_name");
            }
            other => panic!("expected MissingRequired, got {other:?}"),
        }
    }

    #[test]
    fn get_info_relaxes_return_code_for_non_symbolic_info_types() {
        use crate::model::{GetInfo, OdbcCall};

        // Raw-integer info types (e.g. `SQLGetInfo(0)`) are commonly absent
        // from one of the drivers under test; relax the return code so the
        // replay test does not pin a (driver, platform) tuple.
        let calls = vec![OdbcCall::GetInfo(GetInfo {
            return_code: crate::model::ReturnCode::Success,
            handle: Some("0xdbc".to_string()),
            info_type: Some("0".to_string()),
            info_type_value: Some(0),
            info_value: None,
            info_value_numeric: None,
        })];

        let config = GeneratorConfig {
            test_name: "raw int".to_string(),
            allow_unsupported: true,
            ..Default::default()
        };
        let output = generate(&calls, &config).expect("generate");

        assert!(
            output.contains("SQLGetInfo(dbc0, 0, buf, 255, &len);"),
            "raw integer info type rendered verbatim; output:\n{output}",
        );
        assert!(
            output.contains("OdbcMatchers::Succeeded()"),
            "non-symbolic info type uses relaxed (Succeeded) matcher; output:\n{output}",
        );
    }

    #[test]
    fn get_info_skips_value_check_on_error_return() {
        use crate::model::{GetInfo, OdbcCall};

        // When the trace captured an error, the InfoValue buffer is
        // effectively undefined — even for stable info types we must not
        // pin its contents.
        let calls = vec![OdbcCall::GetInfo(GetInfo {
            return_code: crate::model::ReturnCode::Error,
            handle: Some("0xdbc".to_string()),
            info_type: Some("SQL_IDENTIFIER_QUOTE_CHAR".to_string()),
            info_type_value: Some(29),
            info_value: Some("garbage".to_string()),
            info_value_numeric: None,
        })];

        let config = GeneratorConfig {
            test_name: "err".to_string(),
            allow_unsupported: true,
            ..Default::default()
        };
        let output = generate(&calls, &config).expect("generate");

        assert!(
            output.contains("OdbcMatchers::IsError()"),
            "strict matcher mirrors the captured SQL_ERROR; output:\n{output}",
        );
        assert!(
            !output.contains("garbage"),
            "must not assert buffer contents when the call returned an error; output:\n{output}",
        );
    }

    #[test]
    fn env_cleanup_epilogue_emitted_for_unfreed_envs() {
        use crate::model::{AllocHandle, DriverConnect, OdbcCall};

        // Mirrors the Excel/PQ shape: allocate two envs explicitly, never free
        // them, and end the trace. The generator must close the test body
        // with explicit SQLFreeHandle(SQL_HANDLE_ENV, …) calls plus a
        // comment block explaining the divergence.
        let calls = vec![
            OdbcCall::AllocHandle(AllocHandle {
                return_code: ReturnCode::Success,
                handle_type: Some(HandleType::Env),
                parent_handle: None,
                child_handle: Some("0xenv0".to_string()),
            }),
            OdbcCall::AllocHandle(AllocHandle {
                return_code: ReturnCode::Success,
                handle_type: Some(HandleType::Env),
                parent_handle: None,
                child_handle: Some("0xenv1".to_string()),
            }),
            // A no-op call so the body is non-empty; otherwise we'd just be
            // testing alloc+epilogue, which is fine but unrealistic.
            OdbcCall::DriverConnect(DriverConnect {
                return_code: ReturnCode::Success,
                handle: Some("0xdbc0".to_string()),
            }),
        ];

        let config = GeneratorConfig {
            test_name: "env leaks".to_string(),
            allow_unsupported: true,
            ..Default::default()
        };
        let output = generate(&calls, &config).expect("generate");

        assert!(
            output.contains("Replay-only env cleanup (not present in the original trace)"),
            "epilogue header comment must be present; output:\n{output}",
        );
        assert!(
            output.contains("SQLFreeHandle(SQL_HANDLE_ENV, env0)"),
            "env0 must be freed in the epilogue; output:\n{output}",
        );
        assert!(
            output.contains("SQLFreeHandle(SQL_HANDLE_ENV, env1)"),
            "env1 must be freed in the epilogue; output:\n{output}",
        );

        // The two epilogue frees must appear in declaration order (env0 first).
        let env0_pos = output
            .rfind("SQLFreeHandle(SQL_HANDLE_ENV, env0)")
            .expect("env0 free present");
        let env1_pos = output
            .rfind("SQLFreeHandle(SQL_HANDLE_ENV, env1)")
            .expect("env1 free present");
        assert!(env0_pos < env1_pos, "epilogue order should be stable");
    }

    #[test]
    fn env_cleanup_epilogue_skips_explicitly_freed_envs() {
        use crate::model::{AllocHandle, FreeHandle, OdbcCall};

        // If the trace already freed an env (as the iodbctest sample does),
        // the epilogue must not emit a duplicate SQLFreeHandle — that would
        // be a use-after-free for the second call.
        let calls = vec![
            OdbcCall::AllocHandle(AllocHandle {
                return_code: ReturnCode::Success,
                handle_type: Some(HandleType::Env),
                parent_handle: None,
                child_handle: Some("0xenv0".to_string()),
            }),
            OdbcCall::FreeHandle(FreeHandle {
                return_code: ReturnCode::Success,
                handle_type: Some(HandleType::Env),
                handle: Some("0xenv0".to_string()),
            }),
        ];

        let config = GeneratorConfig {
            test_name: "env freed".to_string(),
            allow_unsupported: true,
            ..Default::default()
        };
        let output = generate(&calls, &config).expect("generate");

        assert!(
            !output.contains("Replay-only env cleanup"),
            "no epilogue when the trace freed every env; output:\n{output}",
        );
        // The trace's own free must still be emitted exactly once.
        assert_eq!(
            output
                .matches("SQLFreeHandle(SQL_HANDLE_ENV, env0)")
                .count(),
            1,
            "trace's explicit env free must not be duplicated; output:\n{output}",
        );
    }

    #[test]
    fn get_data_narrow_uses_sentinel_buffer_and_length_bounded_comparison() {
        use crate::model::{GetData, OdbcCall};

        // Captures the new shape: 0xFF sentinel fill so silent driver
        // no-writes surface, plus `std::string(buf.data(), n)` instead of the
        // NUL-dependent `std::string(buf.data())` form.
        let calls = vec![OdbcCall::GetData(GetData {
            return_code: ReturnCode::Success,
            handle: Some("0xstmt".to_string()),
            column_number: Some(1),
            target_type: Some(1),
            target_type_name: Some("SQL_C_CHAR".to_string()),
            buffer_length: Some(1024),
            value: Some("hello".to_string()),
            indicator: Some(5),
            ..Default::default()
        })];

        let config = GeneratorConfig {
            test_name: "narrow getdata".to_string(),
            allow_unsupported: true,
            ..Default::default()
        };
        let output = generate(&calls, &config).expect("generate");

        assert!(
            output.contains("std::vector<char> buf(1025, static_cast<char>(0xFF));"),
            "narrow path must sentinel-fill buf (allocated buf_len + 1); output:\n{output}",
        );
        assert!(
            output.contains(
                "const size_t n = std::min<size_t>(static_cast<size_t>(ind), buf.size());"
            ),
            "narrow path must compute a buffer-bounded read length; output:\n{output}",
        );
        assert!(
            output.contains("CHECK(std::string(buf.data(), n) == \"hello\");"),
            "narrow path must use length-bounded comparison; output:\n{output}",
        );
        assert!(
            !output.contains("CHECK(std::string(buf.data()) == \"hello\");"),
            "must not use the legacy NUL-dependent comparison; output:\n{output}",
        );
    }

    #[test]
    fn get_data_wide_emits_u16_value_assertion_when_value_present() {
        use crate::model::{GetData, OdbcCall};

        // The classic Excel/PQ shape: SELECT 1 fetched as SQL_C_WCHAR with
        // the literal "1" in 2-byte UTF-16 (ind = 2 bytes).
        let calls = vec![OdbcCall::GetData(GetData {
            return_code: ReturnCode::Success,
            handle: Some("0xstmt".to_string()),
            column_number: Some(1),
            target_type: Some(-8),
            target_type_name: Some("SQL_C_WCHAR".to_string()),
            buffer_length: Some(2048),
            value: Some("1".to_string()),
            indicator: Some(2),
            ..Default::default()
        })];

        let config = GeneratorConfig {
            test_name: "wide getdata".to_string(),
            allow_unsupported: true,
            ..Default::default()
        };
        let output = generate(&calls, &config).expect("generate");

        assert!(
            output.contains("std::vector<char> buf(2048, static_cast<char>(0xFF));"),
            "wide path must sentinel-fill; output:\n{output}",
        );
        assert!(
            output.contains("reinterpret_cast<const char16_t*>(buf.data())"),
            "wide path must reinterpret the byte buffer as char16_t*; output:\n{output}",
        );
        assert!(
            output.contains("std::u16string actual"),
            "wide path must build a u16string from the captured byte count; output:\n{output}",
        );
        assert!(
            output.contains("CHECK(actual == u\"1\");"),
            "wide path must compare against a u\"...\" literal; output:\n{output}",
        );
        assert!(
            output.contains("CHECK(ind == 2);"),
            "wide path must keep a strict indicator check; output:\n{output}",
        );
    }

    #[test]
    fn get_data_wide_skips_value_assertion_when_capture_has_question_marks() {
        use crate::model::{GetData, OdbcCall};

        // WinODBC traces render SQL_C_WCHAR payloads via CP_ACP, which
        // produces '?' for any non-ANSI codepoint (CJK, emoji, RTL, …).
        // We can't tell after the fact whether a '?' in the captured value
        // was a real question mark or a replacement marker, so the
        // generator must conservatively skip the value assertion.
        let calls = vec![OdbcCall::GetData(GetData {
            return_code: ReturnCode::Success,
            handle: Some("0xstmt".to_string()),
            column_number: Some(1),
            target_type: Some(-8),
            target_type_name: Some("SQL_C_WCHAR".to_string()),
            buffer_length: Some(2048),
            value: Some("CJK ??? emoji ??".to_string()),
            indicator: Some(32),
            ..Default::default()
        })];

        let config = GeneratorConfig {
            test_name: "wide getdata lossy".to_string(),
            allow_unsupported: true,
            ..Default::default()
        };
        let output = generate(&calls, &config).expect("generate");

        assert!(
            !output.contains("std::u16string actual"),
            "must not assert a possibly-lossy CP_ACP capture; output:\n{output}",
        );
        assert!(
            !output.contains("CHECK(actual"),
            "no actual comparison emitted; output:\n{output}",
        );
        assert!(
            output.contains("// SQL_C_WCHAR value not pinned: trace rendering used CP_ACP"),
            "must explain the deliberate skip in the generated test; output:\n{output}",
        );
        assert!(
            output.contains("CHECK(ind == 32);"),
            "indicator check still emitted; output:\n{output}",
        );
    }

    #[test]
    fn get_data_wide_skips_value_assertion_when_value_absent() {
        use crate::model::{GetData, OdbcCall};

        // Some Power Query traces fetch a wide column for which the WinODBC
        // trace formatter omitted the value (no surrounding double-quotes,
        // just the pointer + indicator). We must not invent a u"…" literal
        // out of thin air — just keep the indicator pin.
        let calls = vec![OdbcCall::GetData(GetData {
            return_code: ReturnCode::Success,
            handle: Some("0xstmt".to_string()),
            column_number: Some(1),
            target_type: Some(-8),
            target_type_name: Some("SQL_C_WCHAR".to_string()),
            buffer_length: Some(2048),
            indicator: Some(42),
            ..Default::default()
        })];

        let config = GeneratorConfig {
            test_name: "wide getdata none".to_string(),
            allow_unsupported: true,
            ..Default::default()
        };
        let output = generate(&calls, &config).expect("generate");

        assert!(
            !output.contains("std::u16string"),
            "no u16string when the trace captured no value; output:\n{output}",
        );
        assert!(
            !output.contains("CHECK(actual"),
            "no value assertion when the trace captured no value; output:\n{output}",
        );
        assert!(
            output.contains("CHECK(ind == 42);"),
            "indicator check stays; output:\n{output}",
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
            query_map: Some(qm),
            ..Default::default()
        };

        let calls: Vec<_> = trace.calls.iter().map(|tc| tc.call.clone()).collect();
        let output = generate(&calls, &config).expect("generate");

        assert!(
            output.contains("SELECT 42 AS answer"),
            "mapped query should appear in output"
        );
        assert!(
            !output.contains("sqlchar(\"SELECT 1\")"),
            "original query should be replaced"
        );
    }

    #[test]
    fn capture_mode_records_obscured_getdata_by_seq() {
        use crate::model::{GetData, OdbcCall, ReturnCode};

        let calls = vec![OdbcCall::GetData(GetData {
            return_code: ReturnCode::Success,
            handle: Some("0xstmt".to_string()),
            column_number: Some(8),
            target_type: Some(8),
            target_type_name: Some("SQL_C_DOUBLE".to_string()),
            buffer_length: Some(8),
            value: None,
            indicator: Some(8),
            seq: Some(42),
            captured: None,
        })];

        let config = GeneratorConfig {
            test_name: "capture".to_string(),
            tag: "capture".to_string(),
            query_map: None,
            allow_unsupported: true,
            capture_mode: true,
        };
        let output = generate(&calls, &config).expect("generate");

        assert!(
            output.contains("picojson"),
            "capture harness includes picojson; output:\n{output}"
        );
        assert!(
            output.contains("\"42\""),
            "capture harness keys by seq; output:\n{output}"
        );
        assert!(
            output.contains("captured_values"),
            "capture harness declares accumulator; output:\n{output}"
        );
        assert!(
            output.contains("CAPTURE_OUTPUT_PATH"),
            "capture harness writes JSON at end; output:\n{output}"
        );
        assert!(
            !output.contains("CHECK(*reinterpret_cast<double*>"),
            "capture mode must not assert obscured values; output:\n{output}"
        );
    }

    #[test]
    fn generate_emits_captured_double_assert() {
        use crate::captured_value::{CapturedValue, DoubleVal};
        use crate::model::{GetData, OdbcCall, ReturnCode};

        let calls = vec![OdbcCall::GetData(GetData {
            return_code: ReturnCode::Success,
            handle: Some("0xstmt".to_string()),
            column_number: Some(8),
            target_type: Some(8),
            target_type_name: Some("SQL_C_DOUBLE".to_string()),
            buffer_length: Some(8),
            value: None,
            indicator: Some(8),
            captured: Some(CapturedValue::Double(DoubleVal::Finite(2.5))),
            seq: None,
        })];

        let config = GeneratorConfig {
            test_name: "double assert".to_string(),
            allow_unsupported: true,
            ..Default::default()
        };
        let output = generate(&calls, &config).expect("generate");

        assert!(
            output.contains("CHECK((*reinterpret_cast<double*>(buf.data())) == 2.5);"),
            "exact double assert from captured; output:\n{output}"
        );
    }

    #[test]
    fn generate_emits_captured_bytes_with_min_ind() {
        use crate::captured_value::CapturedValue;
        use crate::model::{GetData, OdbcCall, ReturnCode};

        let calls = vec![OdbcCall::GetData(GetData {
            return_code: ReturnCode::Success,
            handle: Some("0xstmt".to_string()),
            column_number: Some(9),
            target_type: Some(-2),
            target_type_name: Some("SQL_C_BINARY".to_string()),
            buffer_length: Some(16),
            value: None,
            indicator: Some(4),
            captured: Some(CapturedValue::Bytes("deadbeef".to_string())),
            seq: None,
        })];

        let config = GeneratorConfig {
            test_name: "bytes assert".to_string(),
            allow_unsupported: true,
            ..Default::default()
        };
        let output = generate(&calls, &config).expect("generate");

        assert!(
            output.contains("std::min<size_t>(static_cast<size_t>(ind), buf.size())"),
            "binary assert caps by buffer size; output:\n{output}"
        );
        assert!(
            output.contains("0xde, 0xad, 0xbe, 0xef"),
            "binary assert uses hex literal; output:\n{output}"
        );
    }
}
