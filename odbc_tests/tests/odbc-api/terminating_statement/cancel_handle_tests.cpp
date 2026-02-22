#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "ODBCFixtures.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

// ============================================================================
// SQLCancelHandle - Statement Handle: Basic Functionality
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Cancel on idle statement",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(val == 42);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Cancel after query execution",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // Note: Per ODBC 3.5 spec, cancel when no async processing is in progress
  // should have no effect keeping the cursor open and usable. The reference
  // driver unconditionally closes cursors during cancel.
  ret = SQLFetch(stmt_handle());
  REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 2"), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Cancel after fetch",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // Note: Same reference driver bug as above
  ret = SQLFetch(stmt_handle());
  REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Cancel on prepared but not executed statement",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(val == 42);
}

// ============================================================================
// SQLCancelHandle - Statement Handle: State After Cancel
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: After cancel on executed prepared statement",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // Note: Same reference driver bug
  ret = SQLFetch(stmt_handle());
  REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);

  ret = SQLExecute(stmt_handle());
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Statement recoverable via SQLFreeStmt SQL_CLOSE after cancel",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(val == 42);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: SQLCloseCursor fails after cancel",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // Note: SQLCloseCursor fails because the reference driver already invalidated
  // the cursor during cancel.
  ret = SQLCloseCursor(stmt_handle());
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Multiple cancels on idle statement",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  for (int i = 0; i < 3; i++) {
    const SQLRETURN ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
    REQUIRE(ret == SQL_SUCCESS);
  }

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 99"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(val == 99);
}

// ============================================================================
// SQLCancelHandle - Statement Handle: Interaction with Bindings
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Preserves bound columns after cancel",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLINTEGER col_val = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &col_val, 0, &indicator);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 99"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(col_val == 99);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Preserves bound parameters after cancel",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER param = 55;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param, 0, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  param = 88;
  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(val == 88);
}

// ============================================================================
// SQLCancelHandle - Statement Handle: Data-at-Execution
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Cancels data-at-execution operation",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN dae_indicator = SQL_DATA_AT_EXEC;
  SQLINTEGER param_id = 1;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0,
                         reinterpret_cast<SQLPOINTER>(static_cast<SQLLEN>(param_id)), 0, &dae_indicator);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_RESET_PARAMS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 77"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(val == 77);
}

// ============================================================================
// SQLCancelHandle - Statement Handle: Diagnostic Information
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Diagnostics after cancel error state",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // Note: 24000 because of the reference driver bug, re-execution should succeed.
  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 2"), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
}

// ============================================================================
// SQLCancelHandle - Connection Handle
// ============================================================================

// Note: The reference driver does not support connection-level cancel
// (SFConnection::OnCancel is a no-op that only reports unsupported telemetry).
// The ODBC 3.8 spec intends SQLCancelHandle(SQL_HANDLE_DBC) to cancel
// in-progress connection operations (e.g. a blocking SQLDriverConnect from
// another thread). Since this is unsupported, all connection-handle cancel
// calls silently succeed without any observable effect.

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Cancel on idle connection",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLCancelHandle(SQL_HANDLE_DBC, dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 123"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(val == 123);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Cancel connection with open cursor on statement",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCancelHandle(SQL_HANDLE_DBC, dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(val == 42);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Cancel connection between prepare and execute",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Issuing a connection-level cancel between prepare and execute has no effect
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCancelHandle(SQL_HANDLE_DBC, dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(val == 42);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Multiple cancels on idle connection",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  for (int i = 0; i < 3; i++) {
    const SQLRETURN ret = SQLCancelHandle(SQL_HANDLE_DBC, dbc_handle());
    REQUIRE(ret == SQL_SUCCESS);
  }

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 99"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(val == 99);
}

// ============================================================================
// SQLCancelHandle - Error Cases
// ============================================================================

TEST_CASE("SQLCancelHandle: SQL_INVALID_HANDLE for null statement handle",
          "[odbc-api][cancelhandle][terminating_statement][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const SQLRETURN ret = SQLCancelHandle(SQL_HANDLE_STMT, SQL_NULL_HSTMT);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE("SQLCancelHandle: SQL_INVALID_HANDLE for null connection handle",
          "[odbc-api][cancelhandle][terminating_statement][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const SQLRETURN ret = SQLCancelHandle(SQL_HANDLE_DBC, SQL_NULL_HDBC);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(EnvFixture, "SQLCancelHandle: SQL_INVALID_HANDLE for environment handle type",
                 "[odbc-api][cancelhandle][terminating_statement][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const SQLRETURN ret = SQLCancelHandle(SQL_HANDLE_ENV, env_handle());
  REQUIRE(ret == SQL_INVALID_HANDLE);
}
