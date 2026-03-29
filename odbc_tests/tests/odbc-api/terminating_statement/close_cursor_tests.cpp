#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "ODBCFixtures.hpp"
#include "compatibility.hpp"
#include "odbc_cast.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

// ============================================================================
// SQLCloseCursor - Basic Functionality
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCloseCursor: Close and re-execute",
                 "[odbc-api][closecursor][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 42);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCloseCursor: Fetch after close",
                 "[odbc-api][closecursor][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCloseCursor: Repeated close-execute cycles",
                 "[odbc-api][closecursor][terminating_statement]") {
  constexpr int values[] = {10, 20, 30};
  for (const int expected : values) {
    char sql[64];
    snprintf(sql, sizeof(sql), "SELECT %d", expected);

    SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar(sql), SQL_NTS);
    REQUIRE(ret == SQL_SUCCESS);

    ret = SQLFetch(stmt_handle());
    REQUIRE(ret == SQL_SUCCESS);

    SQLINTEGER val = 0;
    ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(val == expected);

    ret = SQLCloseCursor(stmt_handle());
    REQUIRE(ret == SQL_SUCCESS);
  }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCloseCursor: Close after partial fetch of multi-row result",
                 "[odbc-api][closecursor][terminating_statement]") {
  SQLRETURN ret =
      SQLExecDirect(stmt_handle(), sqlchar("SELECT * FROM (SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 99"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 99);
}

// ============================================================================
// SQLCloseCursor - Preserves Statement State
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCloseCursor: Preserves prepared statement",
                 "[odbc-api][closecursor][terminating_statement]") {
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 42);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCloseCursor: Preserves bound parameters",
                 "[odbc-api][closecursor][terminating_statement]") {
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER param = 99;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param, 0, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  param = 77;
  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 77);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCloseCursor: Preserves bound columns",
                 "[odbc-api][closecursor][terminating_statement]") {
  SQLINTEGER col_val = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &col_val, 0, &indicator);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(col_val == 42);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 99"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(col_val == 99);
}

// ============================================================================
// SQLCloseCursor - Error Cases
// ============================================================================

TEST_CASE("SQLCloseCursor: SQL_INVALID_HANDLE for null statement handle",
          "[odbc-api][closecursor][terminating_statement][error]") {
  const SQLRETURN ret = SQLCloseCursor(SQL_NULL_HSTMT);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCloseCursor: 24000 - No cursor open",
                 "[odbc-api][closecursor][terminating_statement][error]") {
  const SQLRETURN ret = SQLCloseCursor(stmt_handle());
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCloseCursor: HY010 - Called during SQL_NEED_DATA",
                 "[odbc-api][closecursor][terminating_statement][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN dae_indicator = SQL_DATA_AT_EXEC;
  constexpr SQLINTEGER param_id = 1;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0,
                         reinterpret_cast<SQLPOINTER>(static_cast<SQLLEN>(param_id)), 0, &dae_indicator);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCloseCursor: 24000 - Double close",
                 "[odbc-api][closecursor][terminating_statement][error]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
}

// ============================================================================
// SQLCloseCursor - 24000 Error Cases (no cursor open)
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCloseCursor: 24000 - No cursor after DML",
                 "[odbc-api][closecursor][terminating_statement][error]") {
  SQLRETURN ret = SQLExecDirect(
      stmt_handle(), sqlchar("CREATE OR REPLACE TEMPORARY TABLE test_closecursor_dml (id INTEGER)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);

  ret = SQLExecDirect(stmt_handle(), sqlchar("INSERT INTO test_closecursor_dml VALUES (1)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCloseCursor: 24000 - No cursor after DDL",
                 "[odbc-api][closecursor][terminating_statement][error]") {
  SQLRETURN ret = SQLExecDirect(
      stmt_handle(), sqlchar("CREATE OR REPLACE TEMPORARY TABLE test_closecursor_ddl (id INTEGER)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCloseCursor: 24000 - Prepared but not executed",
                 "[odbc-api][closecursor][terminating_statement][error]") {
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
}

// ============================================================================
// SQLCloseCursor - State Transitions (cursor is open, close succeeds)
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCloseCursor: Close from Done state",
                 "[odbc-api][closecursor][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 42);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCloseCursor: Resets used_extended_fetch flag",
                 "[odbc-api][closecursor][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLULEN row_count = 0;
  SQLUSMALLINT row_status = 0;
  ret = SQLExtendedFetch(stmt_handle(), SQL_FETCH_NEXT, 0, &row_count, &row_status);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(row_count == 1);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 42);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCloseCursor: Resets get_data_state",
                 "[odbc-api][closecursor][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 'abcdefghijklmnopqrstuvwxyz'"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  char small_buf[4] = {};
  SQLLEN ind = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_CHAR, small_buf, sizeof(small_buf), &ind);
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 42);
}

// ============================================================================
// SQLCloseCursor - Cross-API Interactions after Close
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCloseCursor: SQLNumResultCols after close on prepared statement",
                 "[odbc-api][closecursor][terminating_statement]") {
  SKIP_OLD_DRIVER("BD#22", "Old driver does not preserve column metadata after SQL_CLOSE on prepared statement");
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 1, 2, 3"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT col_count = 0;
  ret = SQLNumResultCols(stmt_handle(), &col_count);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(col_count == 3);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 1);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCloseCursor: SQLDescribeCol after close on prepared statement",
                 "[odbc-api][closecursor][terminating_statement]") {
  SKIP_OLD_DRIVER("BD#23", "Old driver does not preserve column metadata after SQL_CLOSE on prepared statement");
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 1 AS COL_A, 2 AS COL_B"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR col_name[128] = {};
  SQLSMALLINT name_len = 0;
  SQLSMALLINT data_type = 0;
  SQLULEN col_size = 0;
  SQLSMALLINT decimal_digits = 0;
  SQLSMALLINT nullable = 0;
  ret = SQLDescribeCol(stmt_handle(), 1, col_name, sizeof(col_name), &name_len, &data_type, &col_size, &decimal_digits,
                       &nullable);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(reinterpret_cast<char*>(col_name)) == "COL_A");

  ret = SQLDescribeCol(stmt_handle(), 2, col_name, sizeof(col_name), &name_len, &data_type, &col_size, &decimal_digits,
                       &nullable);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(reinterpret_cast<char*>(col_name)) == "COL_B");

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val1 = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val1, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val1 == 1);

  SQLINTEGER val2 = 0;
  ret = SQLGetData(stmt_handle(), 2, SQL_C_SLONG, &val2, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val2 == 2);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCloseCursor: SQLDescribeCol fails with HY010 after close on exec_direct",
                 "[odbc-api][closecursor][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1 AS COL_Y"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR col_name[128] = {};
  SQLSMALLINT name_len = 0;
  SQLSMALLINT data_type = 0;
  SQLULEN col_size = 0;
  SQLSMALLINT decimal_digits = 0;
  SQLSMALLINT nullable = 0;
  ret = SQLDescribeCol(stmt_handle(), 1, col_name, sizeof(col_name), &name_len, &data_type, &col_size, &decimal_digits,
                       &nullable);
  REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCloseCursor: SQLRowCount returns HY010 after close on exec_direct",
                 "[odbc-api][closecursor][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN row_count = 0;
  ret = SQLRowCount(stmt_handle(), &row_count);
  REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCloseCursor: Re-prepare with different query after close",
                 "[odbc-api][closecursor][terminating_statement]") {
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 1, 2, 3"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT col_count = 0;
  ret = SQLNumResultCols(stmt_handle(), &col_count);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(col_count == 3);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLCloseCursor: SQLDescribeCol after close from prepared Fetching state",
                 "[odbc-api][closecursor][terminating_statement]") {
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 1 AS COL_A UNION ALL SELECT 2"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR col_name[128] = {};
  SQLSMALLINT name_len = 0;
  SQLSMALLINT data_type = 0;
  SQLULEN col_size = 0;
  SQLSMALLINT decimal_digits = 0;
  SQLSMALLINT nullable = 0;
  ret = SQLDescribeCol(stmt_handle(), 1, col_name, sizeof(col_name), &name_len, &data_type, &col_size, &decimal_digits,
                       &nullable);
  OLD_DRIVER_ONLY("BD#23") { REQUIRE_EXPECTED_ERROR(ret, "07009", stmt_handle(), SQL_HANDLE_STMT); }
  NEW_DRIVER_ONLY("BD#23") {
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(std::string(reinterpret_cast<char*>(col_name)) == "COL_A");
  }

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(val == 1);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(val == 2);

  ret = SQLFetch(stmt_handle());
  CHECK(ret == SQL_NO_DATA);
}
