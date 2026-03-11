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
// SQLFreeStmt - SQL_CLOSE Option
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: SQL_CLOSE and re-execute",
                 "[odbc-api][freestmt][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: SQL_CLOSE without open cursor",
                 "[odbc-api][freestmt][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Unlike SQLCloseCursor, SQL_CLOSE does not error when no cursor is open.
  SQLRETURN ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(val == 42);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: SQL_CLOSE preserves prepared statement",
                 "[odbc-api][freestmt][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(val == 42);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: Fetch after SQL_CLOSE",
                 "[odbc-api][freestmt][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);
}

// ============================================================================
// SQLFreeStmt - SQL_UNBIND Option
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: SQL_UNBIND unbinds all columns",
                 "[odbc-api][freestmt][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLINTEGER col_val = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &col_val, 0, &indicator);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_UNBIND);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  REQUIRE(col_val == 0);

  SQLINTEGER val = 0;
  SQLLEN ind2 = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, &ind2);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 42);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: SQL_UNBIND without bindings",
                 "[odbc-api][freestmt][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLFreeStmt(stmt_handle(), SQL_UNBIND);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(val == 42);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: SQL_UNBIND preserves prepared statement",
                 "[odbc-api][freestmt][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_UNBIND);
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
// SQLFreeStmt - SQL_RESET_PARAMS Option
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: SQL_RESET_PARAMS resets bound parameters",
                 "[odbc-api][freestmt][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER param = 1;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param, 0, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_RESET_PARAMS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE_EXPECTED_ERROR(ret, "07002", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: SQL_RESET_PARAMS without parameters",
                 "[odbc-api][freestmt][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLFreeStmt(stmt_handle(), SQL_RESET_PARAMS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(val == 42);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: SQL_RESET_PARAMS preserves prepared statement",
                 "[odbc-api][freestmt][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_RESET_PARAMS);
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
// SQLFreeStmt - SQL_DROP Option (Deprecated)
// ============================================================================

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLFreeStmt: SQL_DROP frees statement handle",
                 "[odbc-api][freestmt][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt, SQL_DROP);
  REQUIRE(ret == SQL_SUCCESS);

  // Note: Using a freed handle is undefined behavior per ODBC spec. The reference
  // driver returns SQL_INVALID_HANDLE for statement handles but crashes for
  // connection handles.
  ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  REQUIRE(ret == SQL_INVALID_HANDLE);

  SQLDisconnect(dbc_handle());
}

// ============================================================================
// SQLFreeStmt - Combining Options
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: All permutations of SQL_CLOSE, SQL_UNBIND, SQL_RESET_PARAMS",
                 "[odbc-api][freestmt][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const SQLUSMALLINT options[][3] = {
      {SQL_CLOSE, SQL_UNBIND, SQL_RESET_PARAMS}, {SQL_CLOSE, SQL_RESET_PARAMS, SQL_UNBIND},
      {SQL_UNBIND, SQL_CLOSE, SQL_RESET_PARAMS}, {SQL_UNBIND, SQL_RESET_PARAMS, SQL_CLOSE},
      {SQL_RESET_PARAMS, SQL_CLOSE, SQL_UNBIND}, {SQL_RESET_PARAMS, SQL_UNBIND, SQL_CLOSE},
  };

  for (const auto& perm : options) {
    SQLINTEGER col_val = 0;
    SQLRETURN ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &col_val, 0, nullptr);
    REQUIRE(ret == SQL_SUCCESS);

    SQLINTEGER param = 1;
    SQLLEN ind = 0;
    ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param, 0, &ind);
    REQUIRE(ret == SQL_SUCCESS);

    ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
    REQUIRE(ret == SQL_SUCCESS);

    for (const auto i : perm) {
      ret = SQLFreeStmt(stmt_handle(), i);
      REQUIRE(ret == SQL_SUCCESS);
    }

    ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
    REQUIRE(ret == SQL_SUCCESS);

    ret = SQLFetch(stmt_handle());
    REQUIRE(ret == SQL_SUCCESS);

    REQUIRE(col_val == 0);

    SQLINTEGER val = 0;
    SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
    REQUIRE(val == 42);

    ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
    REQUIRE(ret == SQL_SUCCESS);

    ret = SQLFreeStmt(stmt_handle(), SQL_UNBIND);
    REQUIRE(ret == SQL_SUCCESS);
  }
}

// ============================================================================
// SQLFreeStmt - Error Cases
// ============================================================================

TEST_CASE("SQLFreeStmt: SQL_INVALID_HANDLE for null statement handle",
          "[odbc-api][freestmt][terminating_statement][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const SQLRETURN ret = SQLFreeStmt(SQL_NULL_HSTMT, SQL_CLOSE);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}
