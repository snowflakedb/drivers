#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "ODBCFixtures.hpp"
#include "compatibility.hpp"
#include "get_descriptor.hpp"
#include "odbc_cast.hpp"
#include "terminating_statement_helpers.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

// ============================================================================
// Descriptor Swap via SQLSetStmtAttr — SQL_ATTR_APP_ROW_DESC
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "Descriptor swap: Set explicit ARD on statement",
                 "[odbc-api][descriptor][swap]") {
  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetDescField(explicit_desc, 0, SQL_DESC_COUNT, reinterpret_cast<SQLPOINTER>(3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_ROW_DESC, explicit_desc, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Verify the active ARD is the explicit one via its state
  SQLHDESC current_ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);
  SQLSMALLINT alloc_type = -1;
  ret = SQLGetDescField(current_ard, 0, SQL_DESC_ALLOC_TYPE, &alloc_type, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(alloc_type == SQL_DESC_ALLOC_USER);

  SQLSMALLINT count = -1;
  ret = SQLGetDescField(current_ard, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 3);

  SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "Descriptor swap: Explicit ARD reflects bindings independently",
                 "[odbc-api][descriptor][swap]") {
  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetDescField(explicit_desc, 0, SQL_DESC_COUNT, reinterpret_cast<SQLPOINTER>(2), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_ROW_DESC, explicit_desc, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC active_ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);
  SQLSMALLINT count = -1;
  ret = SQLGetDescField(active_ard, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 2);

  SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
}

// ============================================================================
// Descriptor Swap via SQLSetStmtAttr — SQL_ATTR_APP_PARAM_DESC
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "Descriptor swap: Set explicit APD on statement",
                 "[odbc-api][descriptor][swap]") {
  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetDescField(explicit_desc, 0, SQL_DESC_COUNT, reinterpret_cast<SQLPOINTER>(2), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_PARAM_DESC, explicit_desc, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Verify the active APD is the explicit one via its state
  SQLHDESC current_apd = get_descriptor(stmt_handle(), SQL_ATTR_APP_PARAM_DESC);
  SQLSMALLINT alloc_type = -1;
  ret = SQLGetDescField(current_apd, 0, SQL_DESC_ALLOC_TYPE, &alloc_type, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(alloc_type == SQL_DESC_ALLOC_USER);

  SQLSMALLINT count = -1;
  ret = SQLGetDescField(current_apd, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 2);

  SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
}

// ============================================================================
// Descriptor Swap — Sharing Across Statements
// ============================================================================

TEST_CASE_METHOD(TwoStmtDefaultDSNFixture, "Descriptor swap: Share explicit ARD across two statements",
                 "[odbc-api][descriptor][swap]") {
  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetDescField(explicit_desc, 0, SQL_DESC_COUNT, reinterpret_cast<SQLPOINTER>(9), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_ROW_DESC, explicit_desc, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetStmtAttr(stmt2_handle(), SQL_ATTR_APP_ROW_DESC, explicit_desc, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Both statements should see the explicit descriptor's state (COUNT == 9)
  SQLHDESC ard1 = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);
  SQLSMALLINT count1 = -1;
  ret = SQLGetDescField(ard1, 0, SQL_DESC_COUNT, &count1, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count1 == 9);

  SQLHDESC ard2 = get_descriptor(stmt2_handle(), SQL_ATTR_APP_ROW_DESC);
  SQLSMALLINT count2 = -1;
  ret = SQLGetDescField(ard2, 0, SQL_DESC_COUNT, &count2, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count2 == 9);

  // Both report ALLOC_USER since the shared descriptor is explicit
  SQLSMALLINT alloc1 = -1, alloc2 = -1;
  ret = SQLGetDescField(ard1, 0, SQL_DESC_ALLOC_TYPE, &alloc1, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLGetDescField(ard2, 0, SQL_DESC_ALLOC_TYPE, &alloc2, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(alloc1 == SQL_DESC_ALLOC_USER);
  REQUIRE(alloc2 == SQL_DESC_ALLOC_USER);

  // Explicitly revert both statements to implicit before freeing the descriptor.
  // The Windows DM crashes if SQLFreeHandle(SQL_HANDLE_DESC) auto-reverts multiple
  // statements — its internal per-statement descriptor tracking holds stale refs.
  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_ROW_DESC, SQL_NULL_HDESC, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetStmtAttr(stmt2_handle(), SQL_ATTR_APP_ROW_DESC, SQL_NULL_HDESC, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
}

// ============================================================================
// Descriptor Swap — Free Reverts to Implicit
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "Descriptor swap: Free explicit desc reverts stmt to implicit ARD",
                 "[odbc-api][descriptor][swap]") {
  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetDescField(explicit_desc, 0, SQL_DESC_COUNT, reinterpret_cast<SQLPOINTER>(7), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_ROW_DESC, explicit_desc, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  // After free, the statement should revert to its implicit ARD (ALLOC_AUTO, COUNT 0)
  SQLHDESC current_ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);
  SQLSMALLINT alloc_type = -1;
  ret = SQLGetDescField(current_ard, 0, SQL_DESC_ALLOC_TYPE, &alloc_type, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(alloc_type == SQL_DESC_ALLOC_AUTO);

  SQLSMALLINT count = -1;
  ret = SQLGetDescField(current_ard, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 0);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "Descriptor swap: Free explicit desc reverts stmt to implicit APD",
                 "[odbc-api][descriptor][swap]") {
  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetDescField(explicit_desc, 0, SQL_DESC_COUNT, reinterpret_cast<SQLPOINTER>(4), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_PARAM_DESC, explicit_desc, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  // After free, the statement should revert to its implicit APD (ALLOC_AUTO, COUNT 0)
  SQLHDESC current_apd = get_descriptor(stmt_handle(), SQL_ATTR_APP_PARAM_DESC);
  SQLSMALLINT alloc_type = -1;
  ret = SQLGetDescField(current_apd, 0, SQL_DESC_ALLOC_TYPE, &alloc_type, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(alloc_type == SQL_DESC_ALLOC_AUTO);

  SQLSMALLINT count = -1;
  ret = SQLGetDescField(current_apd, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 0);
}

// ============================================================================
// Descriptor Swap — Error Cases
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "Descriptor swap: Cannot set IRD via SQLSetStmtAttr",
                 "[odbc-api][descriptor][swap][error]") {
  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_IMP_ROW_DESC, explicit_desc, 0);
  REQUIRE(ret == SQL_ERROR);

  SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "Descriptor swap: Cannot set IPD via SQLSetStmtAttr",
                 "[odbc-api][descriptor][swap][error]") {
  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_IMP_PARAM_DESC, explicit_desc, 0);
  REQUIRE(ret == SQL_ERROR);

  SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
}

TEST_CASE_METHOD(TwoStmtDefaultDSNFixture, "Descriptor swap: HY017 when setting foreign implicit ARD as APP_ROW_DESC",
                 "[odbc-api][descriptor][swap][error]") {
  SQLHDESC foreign_ard = get_descriptor(stmt2_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_ROW_DESC, foreign_ard, 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY017", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(TwoStmtDefaultDSNFixture, "Descriptor swap: HY017 when setting foreign implicit APD as APP_PARAM_DESC",
                 "[odbc-api][descriptor][swap][error]") {
  SQLHDESC foreign_apd = get_descriptor(stmt2_handle(), SQL_ATTR_APP_PARAM_DESC);

  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_PARAM_DESC, foreign_apd, 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY017", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(TwoStmtDefaultDSNFixture, "Descriptor swap: HY017 when setting foreign implicit ARD as APP_PARAM_DESC",
                 "[odbc-api][descriptor][swap][error]") {
  SQLHDESC foreign_ard = get_descriptor(stmt2_handle(), SQL_ATTR_APP_ROW_DESC);

  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_PARAM_DESC, foreign_ard, 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY017", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(TwoStmtDefaultDSNFixture, "Descriptor swap: HY017 when setting foreign implicit APD as APP_ROW_DESC",
                 "[odbc-api][descriptor][swap][error]") {
  SQLHDESC foreign_apd = get_descriptor(stmt2_handle(), SQL_ATTR_APP_PARAM_DESC);

  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_ROW_DESC, foreign_apd, 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY017", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "Descriptor swap: Cannot use explicit descriptor from different connection",
                 "[odbc-api][descriptor][swap][error]") {
  // A second connection on the same environment; RAII-disconnects and frees on
  // scope exit.
  ExtraConnectedDbc other(env_handle(), dsn_name());

  // Allocate an explicit descriptor on the second connection.
  SQLHDESC foreign_explicit = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, other.dbc(), &foreign_explicit);
  REQUIRE(ret == SQL_SUCCESS);

  // Free the descriptor before `other` tears down its parent connection. Declared
  // after `other` so it destructs first (reverse declaration order).
  struct DescCleanup {
    SQLHDESC desc{SQL_NULL_HDESC};
    ~DescCleanup() {
      if (desc != SQL_NULL_HDESC) (void)SQLFreeHandle(SQL_HANDLE_DESC, desc);
    }
  } desc_cleanup{foreign_explicit};

  // Attempt to set it on stmt from the first connection — must fail.
  // iODBC's DM intercepts this and returns HY017 (it treats the foreign handle
  // as automatically allocated); unixODBC/Windows DM pass through to the driver
  // which returns HY024.
  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_ROW_DESC, foreign_explicit, 0);
  IODBC_ONLY { REQUIRE_EXPECTED_ERROR(ret, "HY017", stmt_handle(), SQL_HANDLE_STMT); }
  NON_IODBC { REQUIRE_EXPECTED_ERROR(ret, "HY024", stmt_handle(), SQL_HANDLE_STMT); }
}

// ============================================================================
// Descriptor Swap — End-to-End: Fetch through explicit ARD
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "Descriptor swap: Fetch uses bindings from explicit ARD",
                 "[odbc-api][descriptor][swap]") {
  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_ROW_DESC, explicit_desc, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR sql[] = "SELECT 42 AS val, 'hello' AS msg";
  ret = SQLExecDirect(stmt_handle(), sql, SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // SQLBindCol writes to the active (explicit) ARD
  SQLINTEGER int_val = 0;
  SQLLEN int_ind = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_LONG, &int_val, sizeof(int_val), &int_ind);
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR str_val[64] = {};
  SQLLEN str_ind = 0;
  ret = SQLBindCol(stmt_handle(), 2, SQL_C_CHAR, str_val, sizeof(str_val), &str_ind);
  REQUIRE(ret == SQL_SUCCESS);

  // The driver must read bindings from the explicit ARD during fetch
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(int_val == 42);
  REQUIRE(int_ind == sizeof(SQLINTEGER));
  REQUIRE(std::string(reinterpret_cast<char*>(str_val)) == "hello");
  REQUIRE(str_ind == 5);

  SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
}

TEST_CASE_METHOD(TwoStmtDefaultDSNFixture,
                 "Descriptor swap: Two statements sharing explicit ARD fetch into same buffers",
                 "[odbc-api][descriptor][swap]") {
  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_ROW_DESC, explicit_desc, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetStmtAttr(stmt2_handle(), SQL_ATTR_APP_ROW_DESC, explicit_desc, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Bind a column via stmt1 — writes to the shared explicit ARD
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  REQUIRE(ret == SQL_SUCCESS);

  // Execute and fetch on stmt2 — shares the ARD, should write to the same buffer
  SQLCHAR sql[] = "SELECT 99 AS num";
  ret = SQLExecDirect(stmt2_handle(), sql, SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt2_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(value == 99);
  REQUIRE(indicator == sizeof(SQLINTEGER));

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_ROW_DESC, SQL_NULL_HDESC, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetStmtAttr(stmt2_handle(), SQL_ATTR_APP_ROW_DESC, SQL_NULL_HDESC, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
}

// ============================================================================
// Descriptor Swap — End-to-End: Parameter binding through explicit APD
// ============================================================================
//
// Mirrors the "Fetch uses bindings from explicit ARD" tests above, on the
// parameter side. A naive bind→execute→check-result test would pass even when
// `active_apd` is not wired, because SQLBindParameter and execution both touch
// the implicit APD in lockstep. To actually exercise the active APD, each test
// additionally asserts the binding landed on the *explicit* descriptor (its
// SQL_DESC_COUNT reflects the bound params) — spec-correct behaviour the
// reference driver satisfies and that fails when binding leaks to the implicit
// APD.

TEST_CASE_METHOD(StmtDefaultDSNFixture, "Descriptor swap: Execute uses single param binding from explicit APD",
                 "[odbc-api][descriptor][swap]") {
  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_PARAM_DESC, explicit_desc, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // SQLBindParameter writes the binding to the active (explicit) APD.
  SQLINTEGER value = 42;
  SQLLEN value_ind = 0;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_LONG, SQL_INTEGER, 0, 0, &value, 0, &value_ind);
  REQUIRE(ret == SQL_SUCCESS);

  // The binding must land on the explicit descriptor, not the implicit APD.
  SQLSMALLINT count = -1;
  ret = SQLGetDescField(explicit_desc, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 1);

  // The driver must read the parameter binding from the explicit APD during execution.
  SQLCHAR sql[] = "SELECT ? AS val";
  ret = SQLExecDirect(stmt_handle(), sql, SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER result = 0;
  SQLLEN result_ind = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_LONG, &result, sizeof(result), &result_ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(result == 42);

  ret = SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "Descriptor swap: Execute uses multiple param bindings from explicit APD",
                 "[odbc-api][descriptor][swap]") {
  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_PARAM_DESC, explicit_desc, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER lhs = 20;
  SQLLEN lhs_ind = 0;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_LONG, SQL_INTEGER, 0, 0, &lhs, 0, &lhs_ind);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER rhs = 22;
  SQLLEN rhs_ind = 0;
  ret = SQLBindParameter(stmt_handle(), 2, SQL_PARAM_INPUT, SQL_C_LONG, SQL_INTEGER, 0, 0, &rhs, 0, &rhs_ind);
  REQUIRE(ret == SQL_SUCCESS);

  // Both bindings must land on the explicit descriptor.
  SQLSMALLINT count = -1;
  ret = SQLGetDescField(explicit_desc, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 2);

  SQLCHAR sql[] = "SELECT ? + ? AS val";
  ret = SQLExecDirect(stmt_handle(), sql, SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER result = 0;
  SQLLEN result_ind = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_LONG, &result, sizeof(result), &result_ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(result == 42);

  ret = SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "Descriptor swap: Revert to implicit APD restores implicit param binding",
                 "[odbc-api][descriptor][swap]") {
  // Bind a parameter on the implicit APD first (no explicit descriptor active).
  SQLINTEGER implicit_val = 7;
  SQLLEN implicit_ind = 0;
  SQLRETURN ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_LONG, SQL_INTEGER, 0, 0, &implicit_val, 0,
                                   &implicit_ind);
  REQUIRE(ret == SQL_SUCCESS);

  // Swap in an explicit APD and bind a different value on it.
  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_PARAM_DESC, explicit_desc, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER explicit_val = 99;
  SQLLEN explicit_ind = 0;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_LONG, SQL_INTEGER, 0, 0, &explicit_val, 0,
                         &explicit_ind);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER result = 0;
  SQLLEN result_ind = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_LONG, &result, sizeof(result), &result_ind);
  REQUIRE(ret == SQL_SUCCESS);

  // With the explicit APD active, execution reads its binding (99).
  SQLCHAR sql[] = "SELECT ? AS val";
  ret = SQLExecDirect(stmt_handle(), sql, SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(result == 99);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  // Revert to the implicit APD — its original binding (7) must be restored,
  // proving the explicit binding never overwrote the implicit descriptor.
  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_PARAM_DESC, SQL_NULL_HDESC, 0);
  REQUIRE(ret == SQL_SUCCESS);

  result = 0;
  ret = SQLExecDirect(stmt_handle(), sql, SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(result == 7);

  ret = SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(TwoStmtDefaultDSNFixture, "Descriptor swap: Two statements share param buffer through explicit APD",
                 "[odbc-api][descriptor][swap]") {
  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_PARAM_DESC, explicit_desc, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetStmtAttr(stmt2_handle(), SQL_ATTR_APP_PARAM_DESC, explicit_desc, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Each statement must describe its own parameter: the IPD is per-statement and
  // is NOT shared by SQLSetStmtAttr (only the APD is). Both statements bind
  // parameter 1 on the shared explicit APD, so each populates its own IPD while
  // the single shared APD record is rewritten — stmt2's bind lands last, pointing
  // the record at stmt2's buffer (55) rather than stmt1's (11).
  SQLINTEGER stmt1_value = 11;
  SQLLEN stmt1_ind = 0;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_LONG, SQL_INTEGER, 0, 0, &stmt1_value, 0, &stmt1_ind);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER shared_value = 55;
  SQLLEN shared_ind = 0;
  ret = SQLBindParameter(stmt2_handle(), 1, SQL_PARAM_INPUT, SQL_C_LONG, SQL_INTEGER, 0, 0, &shared_value, 0,
                         &shared_ind);
  REQUIRE(ret == SQL_SUCCESS);

  // stmt1 executes and reads the shared APD, whose record now points at the buffer
  // bound through stmt2 (55) — proving the descriptor (not just the handle) is shared.
  SQLINTEGER result = 0;
  SQLLEN result_ind = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_LONG, &result, sizeof(result), &result_ind);
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR sql[] = "SELECT ? AS val";
  ret = SQLExecDirect(stmt_handle(), sql, SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(result == 55);

  // Revert both statements before freeing the shared descriptor (Windows DM safety).
  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_PARAM_DESC, SQL_NULL_HDESC, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetStmtAttr(stmt2_handle(), SQL_ATTR_APP_PARAM_DESC, SQL_NULL_HDESC, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "Descriptor swap: Execution reads SQL_DESC_ARRAY_SIZE from explicit APD",
                 "[odbc-api][descriptor][swap]") {
  // Step 2b routes the active APD's ARRAY_SIZE header correctly and the multi-set
  // execution path runs, but column-wise array binding strides by the APD record's
  // buffer_length (param_binding.rs binding_for_row), which is 0 here because
  // SQLBindParameter is called with BufferLength 0 for a fixed-size C type — so every
  // row reads element 0. The driver should derive the column stride from the C-type
  // size when buffer_length is 0. See SNOW-3720841 (same gap gates the batch-bind
  // tests in int/boolean/float.cpp). Reference-validated here; unskip when the fix lands.
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLCHAR create_sql[] = "CREATE OR REPLACE TEMPORARY TABLE desc_explicit_apd_array (col INTEGER)";
  SQLRETURN ret = SQLExecDirect(stmt_handle(), create_sql, SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_PARAM_DESC, explicit_desc, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Set the parameter-set size as a header field on the *explicit* APD. If
  // execution reads the implicit APD instead, it sees array size 1 and inserts
  // only the first row.
  constexpr SQLULEN num_rows = 3;
  ret = SQLSetDescField(explicit_desc, 0, SQL_DESC_ARRAY_SIZE, reinterpret_cast<SQLPOINTER>(num_rows), 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER values[num_rows] = {10, 20, 30};
  SQLLEN indicators[num_rows] = {0, 0, 0};
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_LONG, SQL_INTEGER, 0, 0, values, 0, indicators);
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR insert_sql[] = "INSERT INTO desc_explicit_apd_array VALUES (?)";
  ret = SQLExecDirect(stmt_handle(), insert_sql, SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // Revert to implicit and clear the binding before reusing the statement to read back.
  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_PARAM_DESC, SQL_NULL_HDESC, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFreeStmt(stmt_handle(), SQL_RESET_PARAMS);
  REQUIRE(ret == SQL_SUCCESS);

  // All three parameter sets must have been inserted.
  SQLCHAR select_sql[] = "SELECT col FROM desc_explicit_apd_array ORDER BY col";
  ret = SQLExecDirect(stmt_handle(), select_sql, SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER result = 0;
  SQLLEN result_ind = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_LONG, &result, sizeof(result), &result_ind);
  REQUIRE(ret == SQL_SUCCESS);

  for (SQLINTEGER expected : {10, 20, 30}) {
    INFO("expected row value: " << expected);
    ret = SQLFetch(stmt_handle());
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(result == expected);
  }
  REQUIRE(SQLFetch(stmt_handle()) == SQL_NO_DATA);

  ret = SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "Descriptor swap: Data-at-execution param flows through explicit APD",
                 "[odbc-api][descriptor][swap]") {
  SQLHDESC explicit_desc = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_PARAM_DESC, explicit_desc, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // Bind a data-at-execution parameter on the explicit APD. The token passed as
  // ParameterValuePtr must round-trip back out of the explicit APD via SQLParamData.
  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 100, 0,
                         reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE(ret == SQL_SUCCESS);

  // The binding must land on the explicit descriptor.
  SQLSMALLINT count = -1;
  ret = SQLGetDescField(explicit_desc, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 1);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  SQLPOINTER value_ptr = nullptr;
  ret = SQLParamData(stmt_handle(), &value_ptr);
  REQUIRE(ret == SQL_NEED_DATA);
  REQUIRE(value_ptr == reinterpret_cast<SQLPOINTER>(1));

  char put_data[] = "hello";
  ret = SQLPutData(stmt_handle(), put_data, 5);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLParamData(stmt_handle(), &value_ptr);
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR buf[64] = {};
  SQLLEN buf_ind = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_CHAR, buf, sizeof(buf), &buf_ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(reinterpret_cast<char*>(buf)) == "hello");

  ret = SQLFreeHandle(SQL_HANDLE_DESC, explicit_desc);
  REQUIRE(ret == SQL_SUCCESS);
}

// Pure-descriptor binding: configure a parameter by writing APD record fields
// directly on the explicit descriptor (no SQLBindParameter). Because nothing
// populates the IPD in this flow, the parameter's SQL type is described on the
// implicit IPD via SQLSetDescField — still the descriptor API, no bind call.
// Exercises the record-field reads (TYPE / OCTET_LENGTH / DATA_PTR /
// OCTET_LENGTH_PTR / INDICATOR_PTR) that 2b must route through the active APD.
TEST_CASE_METHOD(StmtDefaultDSNFixture,
                 "Descriptor swap: Bind param via SQLSetDescField on explicit APD (no SQLBindParameter)",
                 "[odbc-api][descriptor][swap]") {
  // The active APD reads manually-set descriptor fields (step 2b).  The IPD
  // is described via SQLSetDescField (SQL_DESC_LENGTH / SQL_DESC_CONCISE_TYPE /
  // SQL_DESC_PARAMETER_TYPE), which is now supported.

  SQLHDESC explicit_apd = SQL_NULL_HDESC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &explicit_apd);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_PARAM_DESC, explicit_apd, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Describe the parameter's SQL type on the (implicit, per-statement) IPD.
  SQLHDESC ipd = SQL_NULL_HDESC;
  ret = SQLGetStmtAttr(stmt_handle(), SQL_ATTR_IMP_PARAM_DESC, &ipd, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(ipd, 1, SQL_DESC_CONCISE_TYPE, reinterpret_cast<SQLPOINTER>(SQL_VARCHAR), 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(ipd, 1, SQL_DESC_LENGTH, reinterpret_cast<SQLPOINTER>(100), 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(ipd, 1, SQL_DESC_PARAMETER_TYPE, reinterpret_cast<SQLPOINTER>(SQL_PARAM_INPUT), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Application side: write the APD record fields directly on the explicit
  // descriptor. CONCISE_TYPE is set first (it resets the deferred fields); DATA_PTR
  // is set last so the record is complete when the consistency check fires.
  SQLCHAR text[] = "desc-field-bound";
  SQLLEN text_len = static_cast<SQLLEN>(sizeof(text) - 1);
  ret = SQLSetDescField(explicit_apd, 1, SQL_DESC_CONCISE_TYPE, reinterpret_cast<SQLPOINTER>(SQL_C_CHAR), 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(explicit_apd, 1, SQL_DESC_OCTET_LENGTH, reinterpret_cast<SQLPOINTER>(sizeof(text)), 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(explicit_apd, 1, SQL_DESC_OCTET_LENGTH_PTR, &text_len, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(explicit_apd, 1, SQL_DESC_INDICATOR_PTR, &text_len, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetDescField(explicit_apd, 1, SQL_DESC_DATA_PTR, text, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // The manual field writes must register parameter 1 on the explicit descriptor.
  SQLSMALLINT count = -1;
  ret = SQLGetDescField(explicit_apd, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 1);

  SQLCHAR sql[] = "SELECT ? AS val";
  ret = SQLExecDirect(stmt_handle(), sql, SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR result[64] = {};
  SQLLEN result_ind = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_CHAR, result, sizeof(result), &result_ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(reinterpret_cast<char*>(result)) == "desc-field-bound");

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_APP_PARAM_DESC, SQL_NULL_HDESC, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFreeHandle(SQL_HANDLE_DESC, explicit_apd);
  REQUIRE(ret == SQL_SUCCESS);
}
