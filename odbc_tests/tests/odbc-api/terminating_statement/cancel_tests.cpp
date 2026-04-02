#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <chrono>
#include <thread>

#include <catch2/catch_test_macros.hpp>

#include "ODBCFixtures.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "cross_thread_cancel.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"
#include "test_setup.hpp"

// NOTE: Unix ODBC Driver Managers (both unixODBC and iODBC) unconditionally
// close the cursor in their internal state machine when SQLCancel succeeds,
// regardless of the SQL_ATTR_ODBC_VERSION setting. This is ODBC 2.x behavior;
// the ODBC 3.5 spec says SQLCancel on a synchronous idle statement should be
// a no-op (state transition table shows "--" for S5-S7).
//
// Only the Windows ODBC DM correctly implements the 3.5 no-op semantics.
// Both our driver and the reference driver return SQL_SUCCESS from SQLCancel
// without touching the cursor, but the Unix DMs mark the cursor as closed
// before the next call reaches the driver.
//
// Source code references:
//   unixODBC: https://github.com/lurcher/unixODBC/blob/master/DriverManager/SQLCancel.c
//     (else branch: "Same action as SQLFreeStmt( SQL_CLOSE )")
//   iODBC: https://github.com/openlink/iODBC/blob/develop/iodbc/hstmt.c
//     (case en_stmt_cursoropen/en_stmt_fetched/en_stmt_xfetched)
//   ODBC spec: https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlcancel-function
//     ("In ODBC 3.5, a call to SQLCancel when no processing is being done
//      on the statement is not treated as SQLFreeStmt with the SQL_CLOSE
//      option, but has no effect at all.")

namespace {
constexpr int kMaxPollIterations = 300;
}  // namespace

// ============================================================================
// SQLCancel - Basic Functionality
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Cancel on idle statement",
                 "[odbc-api][cancel][terminating_statement]") {
  SQLRETURN ret = SQLCancel(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(val == 42);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Cancel after query execution",
                 "[odbc-api][cancel][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLCancel(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  // Unix DMs close the cursor on SQLCancel.
  WINDOWS_ONLY {
    ret = SQLFetch(stmt_handle());
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  }
  UNIX_ONLY {
    ret = SQLFetch(stmt_handle());
    REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);

    ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 2"), SQL_NTS);
    REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
  }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Cancel after fetch", "[odbc-api][cancel][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLCancel(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  // Unix DMs close the cursor on SQLCancel.
  WINDOWS_ONLY {
    ret = SQLFetch(stmt_handle());
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::IsNoData());
  }
  UNIX_ONLY {
    ret = SQLFetch(stmt_handle());
    REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);

    ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
    REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
  }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Cancel on prepared but not executed statement",
                 "[odbc-api][cancel][terminating_statement]") {
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLCancel(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecute(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(val == 42);
}

// ============================================================================
// SQLCancel - Statement State After Cancel
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: After cancel on executed prepared statement",
                 "[odbc-api][cancel][terminating_statement]") {
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecute(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLCancel(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  // Unix DMs close the cursor on SQLCancel.
  WINDOWS_ONLY {
    ret = SQLFetch(stmt_handle());
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  }
  UNIX_ONLY {
    ret = SQLFetch(stmt_handle());
    REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);

    ret = SQLExecute(stmt_handle());
    REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
  }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Statement recoverable via SQLFreeStmt SQL_CLOSE after cancel",
                 "[odbc-api][cancel][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLCancel(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  // Unix DMs close the cursor on SQLCancel.
  WINDOWS_ONLY {
    ret = SQLFetch(stmt_handle());
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

    // Re-execution fails because cursor is still open (BD#33).
    ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
    REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
  }
  UNIX_ONLY {
    ret = SQLFetch(stmt_handle());
    REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);

    ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
    REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
  }

  // Both paths recover via SQL_CLOSE.
  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(val == 42);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: SQLCloseCursor after cancel",
                 "[odbc-api][cancel][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLCancel(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  // Unix DMs close the cursor on SQLCancel.
  WINDOWS_ONLY {
    ret = SQLCloseCursor(stmt_handle());
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  }
  UNIX_ONLY {
    ret = SQLCloseCursor(stmt_handle());
    REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
  }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Cancel after error recovery and re-execution",
                 "[odbc-api][cancel][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT * FROM nonexistent_table_xyz_999"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::IsError());

  ret = SQLCancel(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(val == 42);

  ret = SQLCancel(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 99"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(val == 99);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Cancel on never-executed statement then use and free",
                 "[odbc-api][cancel][terminating_statement]") {
  SQLHSTMT fresh_stmt = SQL_NULL_HSTMT;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &fresh_stmt);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc_handle()), OdbcMatchers::Succeeded());

  ret = SQLCancel(fresh_stmt);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, fresh_stmt), OdbcMatchers::Succeeded());

  // Verify the handle is still usable after cancel
  ret = SQLExecDirect(fresh_stmt, sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, fresh_stmt), OdbcMatchers::Succeeded());

  ret = SQLFetch(fresh_stmt);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, fresh_stmt), OdbcMatchers::Succeeded());

  SQLINTEGER val = 0;
  ret = SQLGetData(fresh_stmt, 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, fresh_stmt), OdbcMatchers::Succeeded());
  REQUIRE(val == 42);

  ret = SQLFreeHandle(SQL_HANDLE_STMT, fresh_stmt);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, fresh_stmt), OdbcMatchers::Succeeded());
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Multiple cancels on idle statement",
                 "[odbc-api][cancel][terminating_statement]") {
  for (int i = 0; i < 3; i++) {
    const SQLRETURN ret = SQLCancel(stmt_handle());
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  }

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 99"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(val == 99);
}

// ============================================================================
// SQLCancel - Interaction with Bindings
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Preserves bound columns after cancel",
                 "[odbc-api][cancel][terminating_statement]") {
  SQLINTEGER col_val = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &col_val, 0, &indicator);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLCancel(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 99"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(col_val == 99);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Preserves bound parameters after cancel",
                 "[odbc-api][cancel][terminating_statement]") {
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLINTEGER param = 55;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param, 0, &ind);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecute(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLCancel(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  param = 88;
  ret = SQLExecute(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(val == 88);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Parameter bindings preserved after cancel with open cursor",
                 "[odbc-api][cancel][terminating_statement]") {
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLINTEGER param = 55;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param, 0, &ind);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecute(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(val == 55);

  // Cancel while cursor is open (no-op on new driver, closes cursor on old)
  ret = SQLCancel(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  // Re-execute with updated parameter value — bindings should be intact
  param = 123;
  ret = SQLExecute(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(val == 123);
}

// ============================================================================
// SQLCancel - Data-at-Execution
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Cancels data-at-execution operation",
                 "[odbc-api][cancel][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLLEN dae_indicator = SQL_DATA_AT_EXEC;
  SQLINTEGER param_id = 1;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0,
                         reinterpret_cast<SQLPOINTER>(static_cast<SQLLEN>(param_id)), 0, &dae_indicator);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  ret = SQLCancel(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFreeStmt(stmt_handle(), SQL_RESET_PARAMS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 77"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(val == 77);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Re-execute immediately after canceling data-at-execution",
                 "[odbc-api][cancel][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLLEN dae_indicator = SQL_DATA_AT_EXEC;
  SQLINTEGER param_id = 1;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0,
                         reinterpret_cast<SQLPOINTER>(static_cast<SQLLEN>(param_id)), 0, &dae_indicator);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  ret = SQLCancel(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  // Verify re-execution works directly without an intervening SQLFreeStmt
  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  ret = SQLCancel(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
}

// ============================================================================
// SQLCancel - Error Cases
// ============================================================================

TEST_CASE("SQLCancel: SQL_INVALID_HANDLE for null statement handle",
          "[odbc-api][cancel][terminating_statement][error]") {
  const SQLRETURN ret = SQLCancel(SQL_NULL_HSTMT);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

// ============================================================================
// SQLCancel - State Coverage
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Cancel after all rows fetched",
                 "[odbc-api][cancel][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::IsNoData());

  ret = SQLCancel(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(val == 42);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Cancel after DDL execution",
                 "[odbc-api][cancel][terminating_statement]") {
  auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("CREATE OR REPLACE TABLE cancel_test_tmp (id INT)"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLCancel(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(val == 1);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Cancel on statement in Error state",
                 "[odbc-api][cancel][terminating_statement]") {
  auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT * FROM nonexistent_table"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::IsError());

  ret = SQLCancel(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
}

// ============================================================================
// SQLCancel - Cursor Preservation (Windows only; Unix DMs close the cursor)
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Cursor remains usable after cancel on multi-row result",
                 "[odbc-api][cancel][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT column1 FROM VALUES (1),(2),(3) ORDER BY 1"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLINTEGER val = 0;

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(val == 1);

  ret = SQLCancel(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  // Unix DMs close the cursor on SQLCancel.
  WINDOWS_ONLY {
    ret = SQLFetch(stmt_handle());
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
    ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
    REQUIRE(val == 2);

    ret = SQLCancel(stmt_handle());
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

    ret = SQLFetch(stmt_handle());
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
    ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
    REQUIRE(val == 3);

    ret = SQLFetch(stmt_handle());
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::IsNoData());
  }
  UNIX_ONLY {
    ret = SQLFetch(stmt_handle());
    REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);
  }
}

// ============================================================================
// SQLCancel - Isolation
// ============================================================================

TEST_CASE_METHOD(TwoStmtDefaultDSNFixture, "SQLCancel: Does not affect other statements on same connection",
                 "[odbc-api][cancel][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecDirect(stmt2_handle(), sqlchar("SELECT 2"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2_handle()), OdbcMatchers::Succeeded());

  ret = SQLCancel(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt2_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2_handle()), OdbcMatchers::Succeeded());

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt2_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2_handle()), OdbcMatchers::Succeeded());
  REQUIRE(val == 2);
}

// ============================================================================
// SQLCancel - Attribute Preservation
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Preserves statement attributes after cancel",
                 "[odbc-api][cancel][terminating_statement]") {
  SQLULEN max_length = 1024;
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_MAX_LENGTH, reinterpret_cast<SQLPOINTER>(max_length), 0);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLCancel(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLULEN retrieved_max_length = 0;
  ret = SQLGetStmtAttr(stmt_handle(), SQL_ATTR_MAX_LENGTH, &retrieved_max_length, 0, nullptr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(retrieved_max_length == max_length);

  SQLULEN cursor_type = 0;
  ret = SQLGetStmtAttr(stmt_handle(), SQL_ATTR_CURSOR_TYPE, &cursor_type, 0, nullptr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(cursor_type == SQL_CURSOR_FORWARD_ONLY);
}

// ============================================================================
// SQLCancel - Diagnostic Behavior
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Does not post diagnostic records on no-op cancel",
                 "[odbc-api][cancel][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLCancel(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLCHAR sql_state[6] = {};
  SQLINTEGER native_error = 0;
  SQLCHAR message[256] = {};
  SQLSMALLINT msg_len = 0;
  ret = SQLGetDiagRec(SQL_HANDLE_STMT, stmt_handle(), 1, sql_state, &native_error, message, sizeof(message), &msg_len);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::IsNoData());
}

// ============================================================================
// SQLCancel - Async Cancel
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Async cancel interrupts execution with HY008",
                 "[odbc-api][cancel][terminating_statement][async]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SKIP_OLD_DRIVER("BD#34", "Async cancel does not interrupt in-progress operations on reference driver");

  SQLRETURN ret =
      SQLSetStmtAttr(stmt_handle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  // Use a long TIMELIMIT so the query cannot complete before the cancel.
  // If the poll returns before 30s, it must be because the cancel worked.
  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT COUNT(*) FROM TABLE(GENERATOR(TIMELIMIT => 30))"), SQL_NTS);
  REQUIRE(ret == SQL_STILL_EXECUTING);

  SQLRETURN cancel_ret = SQLCancel(stmt_handle());
  REQUIRE_THAT(OdbcResult(cancel_ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  int polls = 0;
  SQLRETURN poll_ret = SQL_STILL_EXECUTING;
  while (poll_ret == SQL_STILL_EXECUTING && ++polls < kMaxPollIterations) {
    std::this_thread::sleep_for(std::chrono::milliseconds(100));
    poll_ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT COUNT(*) FROM TABLE(GENERATOR(TIMELIMIT => 30))"), SQL_NTS);
  }
  REQUIRE(poll_ret != SQL_STILL_EXECUTING);

  REQUIRE_EXPECTED_ERROR(poll_ret, "HY008", stmt_handle(), SQL_HANDLE_STMT);

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_ASYNC_ENABLE, SQL_ASYNC_ENABLE_OFF, 0);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Async cancel clears diagnostics and posts its own",
                 "[odbc-api][cancel][terminating_statement][async]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SKIP_OLD_DRIVER("BD#34", "Async cancel does not interrupt in-progress operations on reference driver");

  SQLRETURN ret =
      SQLSetStmtAttr(stmt_handle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT COUNT(*) FROM TABLE(GENERATOR(TIMELIMIT => 30))"), SQL_NTS);
  REQUIRE(ret == SQL_STILL_EXECUTING);

  SQLRETURN cancel_ret = SQLCancel(stmt_handle());
  REQUIRE_THAT(OdbcResult(cancel_ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  int polls = 0;
  SQLRETURN poll_ret = SQL_STILL_EXECUTING;
  while (poll_ret == SQL_STILL_EXECUTING && ++polls < kMaxPollIterations) {
    std::this_thread::sleep_for(std::chrono::milliseconds(100));
    poll_ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT COUNT(*) FROM TABLE(GENERATOR(TIMELIMIT => 30))"), SQL_NTS);
  }
  REQUIRE(poll_ret != SQL_STILL_EXECUTING);

  REQUIRE_EXPECTED_ERROR(poll_ret, "HY008", stmt_handle(), SQL_HANDLE_STMT);

  auto records = get_diag_rec(SQL_HANDLE_STMT, stmt_handle());
  REQUIRE(!records.empty());
  REQUIRE(records.size() == 1);
  REQUIRE(records[0].sqlState == "HY008");

  ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_ASYNC_ENABLE, SQL_ASYNC_ENABLE_OFF, 0);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
}

// The ODBC spec allows function completion despite the cancel instruction.
// This case is non-deterministic as we cannot distinguish "cancel was a no-op"
// from "cancel tried but the query finished first".

// ============================================================================
// SQLCancel - Cross-Thread Cancel
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Cross-thread cancel interrupts execution with HY008",
                 "[odbc-api][cancel][terminating_statement][cross_thread]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHSTMT stmt = stmt_handle();
  odbc_test::CrossThreadCancel ctx;
  // 5-second delay lets the query reach the server before cancel fires.
  // Without it, SQLCancel can arrive before SQLExecDirect has sent the query,
  // leaving nothing to cancel and causing the query to run to completion.
  ctx.run(stmt, "SELECT COUNT(*) FROM TABLE(GENERATOR(TIMELIMIT => 60))", std::chrono::seconds(5));

  REQUIRE_THAT(OdbcResult(ctx.cancel_result, SQL_HANDLE_STMT, stmt), OdbcMatchers::Succeeded());

  SQLRETURN exec_ret = ctx.exec_result.load();
  REQUIRE_EXPECTED_ERROR(exec_ret, "HY008", stmt, SQL_HANDLE_STMT);

  // Per ODBC spec, cross-thread cancel does NOT clear the diagnostic
  // records of the canceled function, and does NOT post its own.
  // The HY008 from the canceled SQLExecDirect should be the only record.
  auto records = get_diag_rec(SQL_HANDLE_STMT, stmt);
  REQUIRE(records.size() == 1);
  REQUIRE(records[0].sqlState == "HY008");
}

// HY018 (server declines cancel) is not tested: Backend always accepts
// cancel requests

// The ODBC spec describes a race where both SQLCancel and the original
// function return SQL_SUCCESS. In that case the Driver Manager assumes the
// cursor is closed by the cancel, so the application cannot use the cursor.
// This requires SQLCancel to arrive after SQLExecDirect enters the driver but
// before the query reaches the server. This is non-deterministic and untestable.

// ============================================================================
// SQLCancel - Connection-Level Async
// ============================================================================
//
// Per ODBC spec, SQLCancel returns HY010 if a connection-level async function
// is still executing on the parent connection. Testing this requires enabling
// SQL_ATTR_ASYNC_DBC_FUNCTIONS_ENABLE on the connection handle.

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancel: Driver rejects enabling connection-level async with HY092",
                 "[odbc-api][cancel][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const SQLRETURN ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_ASYNC_DBC_FUNCTIONS_ENABLE,
                                          reinterpret_cast<SQLPOINTER>(SQL_ASYNC_DBC_ENABLE_ON), 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY092", dbc_handle(), SQL_HANDLE_DBC);
}
