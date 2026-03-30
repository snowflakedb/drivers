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

// NOTE: SQLCancelHandle(SQL_HANDLE_STMT, ...) behaves identically to SQLCancel
// per the ODBC 3.8 spec. The Driver Manager maps it to SQLCancel when the
// driver does not export SQLCancelHandle. The same Unix DM cursor-closing
// behavior described in cancel_tests.cpp applies here.

namespace {
constexpr int kMaxPollIterations = 300;
}  // namespace

// ============================================================================
// SQLCancelHandle - Statement Handle: Basic Functionality
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Cancel on idle statement",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Cancel after query execution",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Cancel after fetch",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Cancel on prepared but not executed statement",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
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
// SQLCancelHandle - Statement Handle: State After Cancel
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: After cancel on executed prepared statement",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecute(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Statement recoverable via SQLFreeStmt SQL_CLOSE after cancel",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);

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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: SQLCloseCursor fails after cancel",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  WINDOWS_ONLY {
    ret = SQLCloseCursor(stmt_handle());
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  }
  UNIX_ONLY {
    ret = SQLCloseCursor(stmt_handle());
    REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
  }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Cancel after error recovery and re-execution",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT * FROM nonexistent_table_xyz_999"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::IsError());

  ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(val == 42);

  ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Cancel on never-executed statement then use and free",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHSTMT fresh_stmt = SQL_NULL_HSTMT;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &fresh_stmt);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc_handle()), OdbcMatchers::Succeeded());

  ret = SQLCancelHandle(SQL_HANDLE_STMT, fresh_stmt);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, fresh_stmt), OdbcMatchers::Succeeded());

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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Multiple cancels on idle statement",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  for (int i = 0; i < 3; i++) {
    const SQLRETURN ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
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
// SQLCancelHandle - Statement Handle: Interaction with Bindings
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Preserves bound columns after cancel",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLINTEGER col_val = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &col_val, 0, &indicator);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 99"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(col_val == 99);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Preserves bound parameters after cancel",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLINTEGER param = 55;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param, 0, &ind);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecute(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Parameter bindings preserved after cancel with open cursor",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

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

  ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

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
// SQLCancelHandle - Statement Handle: Data-at-Execution
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Cancels data-at-execution operation",
                 "[odbc-api][cancelhandle][terminating_statement]") {
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

  ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Re-execute immediately after canceling data-at-execution",
                 "[odbc-api][cancelhandle][terminating_statement]") {
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

  ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
}

// ============================================================================
// SQLCancelHandle - Statement Handle: State Coverage
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Cancel after all rows fetched",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::IsNoData());

  ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Cancel after DDL execution",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  Schema::use_temp_session_schema(dbc_handle());

  SQLRETURN ret =
      SQLExecDirect(stmt_handle(), sqlchar("CREATE TEMPORARY TABLE cancel_handle_test_tmp (id INT)"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Cancel on statement in Error state",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  Schema::use_temp_session_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT * FROM nonexistent_table"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::IsError());

  ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
}

// ============================================================================
// SQLCancelHandle - Statement Handle: Cursor Preservation
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Cursor remains usable after cancel on multi-row result",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT column1 FROM VALUES (1),(2),(3) ORDER BY 1"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLINTEGER val = 0;

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(val == 1);

  ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  WINDOWS_ONLY {
    ret = SQLFetch(stmt_handle());
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
    ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
    REQUIRE(val == 2);

    ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
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
// SQLCancelHandle - Statement Handle: Isolation
// ============================================================================

TEST_CASE_METHOD(TwoStmtDefaultDSNFixture, "SQLCancelHandle: Does not affect other statements on same connection",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecDirect(stmt2_handle(), sqlchar("SELECT 2"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2_handle()), OdbcMatchers::Succeeded());

  ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt2_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2_handle()), OdbcMatchers::Succeeded());

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt2_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2_handle()), OdbcMatchers::Succeeded());
  REQUIRE(val == 2);
}

// ============================================================================
// SQLCancelHandle - Statement Handle: Attribute Preservation
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Preserves statement attributes after cancel",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLULEN max_length = 1024;
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_MAX_LENGTH, reinterpret_cast<SQLPOINTER>(max_length), 0);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
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
// SQLCancelHandle - Statement Handle: Diagnostic Behavior
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Diagnostics after cancel error state",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 2"), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Does not post diagnostic records on no-op cancel",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLCHAR sql_state[6] = {};
  SQLINTEGER native_error = 0;
  SQLCHAR message[256] = {};
  SQLSMALLINT msg_len = 0;
  ret = SQLGetDiagRec(SQL_HANDLE_STMT, stmt_handle(), 1, sql_state, &native_error, message, sizeof(message), &msg_len);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::IsNoData());
}

// ============================================================================
// SQLCancelHandle - Statement Handle: Async Cancel
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Async cancel interrupts execution with HY008",
                 "[odbc-api][cancelhandle][terminating_statement][async]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SKIP_OLD_DRIVER("BD#32", "Async cancel does not interrupt in-progress operations on reference driver");

  SQLRETURN ret =
      SQLSetStmtAttr(stmt_handle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT COUNT(*) FROM TABLE(GENERATOR(TIMELIMIT => 30))"), SQL_NTS);
  REQUIRE(ret == SQL_STILL_EXECUTING);

  SQLRETURN cancel_ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Async cancel clears diagnostics and posts its own",
                 "[odbc-api][cancelhandle][terminating_statement][async]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SKIP_OLD_DRIVER("BD#32", "Async cancel does not interrupt in-progress operations on reference driver");

  SQLRETURN ret =
      SQLSetStmtAttr(stmt_handle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT COUNT(*) FROM TABLE(GENERATOR(TIMELIMIT => 30))"), SQL_NTS);
  REQUIRE(ret == SQL_STILL_EXECUTING);

  SQLRETURN cancel_ret = SQLCancelHandle(SQL_HANDLE_STMT, stmt_handle());
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

// ============================================================================
// SQLCancelHandle - Statement Handle: Cross-Thread Cancel
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Cross-thread cancel interrupts execution with HY008",
                 "[odbc-api][cancelhandle][terminating_statement][cross_thread]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHSTMT stmt = stmt_handle();
  odbc_test::CrossThreadCancel ctx;
  ctx.run(stmt, "SELECT COUNT(*) FROM TABLE(GENERATOR(TIMELIMIT => 60))", std::chrono::seconds(5),
          [](SQLHSTMT s) { return SQLCancelHandle(SQL_HANDLE_STMT, s); });

  REQUIRE_THAT(OdbcResult(ctx.cancel_result, SQL_HANDLE_STMT, stmt), OdbcMatchers::Succeeded());

  SQLRETURN exec_ret = ctx.exec_result.load();
  REQUIRE_EXPECTED_ERROR(exec_ret, "HY008", stmt, SQL_HANDLE_STMT);

  auto records = get_diag_rec(SQL_HANDLE_STMT, stmt);
  REQUIRE(records.size() == 1);
  REQUIRE(records[0].sqlState == "HY008");
}

// ============================================================================
// SQLCancelHandle - Connection Handle
// ============================================================================

// The ODBC 3.8 spec intends SQLCancelHandle(SQL_HANDLE_DBC) to cancel
// in-progress connection operations (e.g. a blocking SQLDriverConnect from
// another thread). Connection-level cancel is a no-op for now because no
// async or cross-thread connection operations are supported.

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Cancel on idle connection",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLCancelHandle(SQL_HANDLE_DBC, dbc_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 123"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(val == 123);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Cancel connection with open cursor on statement",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLCancelHandle(SQL_HANDLE_DBC, dbc_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(val == 42);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Cancel connection between prepare and execute",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLCancelHandle(SQL_HANDLE_DBC, dbc_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecute(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(val == 42);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: Multiple cancels on idle connection",
                 "[odbc-api][cancelhandle][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  for (int i = 0; i < 3; i++) {
    const SQLRETURN ret = SQLCancelHandle(SQL_HANDLE_DBC, dbc_handle());
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc_handle()), OdbcMatchers::Succeeded());
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

TEST_CASE_METHOD(EnvFixture, "SQLCancelHandle: SQL_ERROR with HY092 for environment handle type",
                 "[odbc-api][cancelhandle][terminating_statement][error]") {
  const SQLRETURN ret = SQLCancelHandle(SQL_HANDLE_ENV, env_handle());
  // The DM typically intercepts this before reaching the driver.
  // Windows DM returns SQL_ERROR with HY092 (per spec).
  // Unix DM (unixODBC) may return SQL_INVALID_HANDLE instead.
  WINDOWS_ONLY {
    REQUIRE(ret == SQL_ERROR);
    REQUIRE(get_sqlstate(SQL_HANDLE_ENV, env_handle()) == "HY092");
  }
  UNIX_ONLY {
    // unixODBC returns SQL_INVALID_HANDLE for unsupported handle types.
    REQUIRE(ret == SQL_INVALID_HANDLE);
  }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCancelHandle: SQL_ERROR with HY092 for descriptor handle type",
                 "[odbc-api][cancelhandle][terminating_statement][error]") {
  // Obtain an implicit descriptor handle (ARD) from the statement.
  SQLHDESC ard = SQL_NULL_HDESC;
  SQLRETURN ret = SQLGetStmtAttr(stmt_handle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, nullptr);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(ard != SQL_NULL_HDESC);

  ret = SQLCancelHandle(SQL_HANDLE_DESC, static_cast<SQLHANDLE>(ard));
  // The DM typically intercepts this before reaching the driver.
  // Windows DM returns SQL_ERROR with HY092 (per spec).
  // Unix DM (unixODBC) may return SQL_INVALID_HANDLE instead.
  WINDOWS_ONLY {
    REQUIRE(ret == SQL_ERROR);
    // TODO(SNOW-3307200): verify HY092 SQLSTATE once descriptor diagnostic
    // infrastructure lands.
  }
  UNIX_ONLY {
    // unixODBC returns SQL_INVALID_HANDLE for unsupported handle types.
    REQUIRE(ret == SQL_INVALID_HANDLE);
  }
}
