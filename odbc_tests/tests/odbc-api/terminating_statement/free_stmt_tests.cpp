#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "ODBCFixtures.hpp"
#include "SessionParameterOverride.hpp"
#include "odbc_cast.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

// ============================================================================
// SQLFreeStmt - SQL_CLOSE Option
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: SQL_CLOSE and re-execute",
                 "[odbc-api][freestmt][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: SQL_CLOSE without open cursor",
                 "[odbc-api][freestmt][terminating_statement]") {
  // Unlike SQLCloseCursor, SQL_CLOSE does not error when no cursor is open.
  SQLRETURN ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: SQL_CLOSE preserves prepared statement",
                 "[odbc-api][freestmt][terminating_statement]") {
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
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 42);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: Fetch after SQL_CLOSE",
                 "[odbc-api][freestmt][terminating_statement]") {
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
  SQLRETURN ret = SQLFreeStmt(stmt_handle(), SQL_UNBIND);
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: SQL_UNBIND preserves prepared statement",
                 "[odbc-api][freestmt][terminating_statement]") {
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_UNBIND);
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
  SQLRETURN ret = SQLFreeStmt(stmt_handle(), SQL_RESET_PARAMS);
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: SQL_RESET_PARAMS preserves prepared statement",
                 "[odbc-api][freestmt][terminating_statement]") {
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_RESET_PARAMS);
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: SQL_CLOSE on prepared-but-not-executed",
                 "[odbc-api][freestmt][terminating_statement]") {
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: SQL_CLOSE preserves column bindings",
                 "[odbc-api][freestmt][terminating_statement]") {
  SQLINTEGER col_val = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &col_val, 0, &indicator);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 10"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  REQUIRE(col_val == 42);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: SQL_CLOSE preserves parameter bindings",
                 "[odbc-api][freestmt][terminating_statement]") {
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER param = 42;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param, 0, &ind);
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
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 42);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: SQL_CLOSE called multiple times",
                 "[odbc-api][freestmt][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: SQL_CLOSE after DML",
                 "[odbc-api][freestmt][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(),
                                sqlchar("CREATE OR REPLACE TEMPORARY TABLE test_freestmt_dml (id INTEGER)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("INSERT INTO test_freestmt_dml VALUES (1)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT id FROM test_freestmt_dml"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 1);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("DROP TABLE IF EXISTS test_freestmt_dml"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: SQLNumResultCols after SQL_CLOSE on prepared statement",
                 "[odbc-api][freestmt][terminating_statement]") {
  SKIP_OLD_DRIVER("BD#22", "Old driver does not preserve column metadata after SQL_CLOSE on prepared statement");
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 1, 2, 3"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: SQL_CLOSE from Done state",
                 "[odbc-api][freestmt][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: SQL_CLOSE from Fetching state",
                 "[odbc-api][freestmt][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1 UNION ALL SELECT 2"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: SQL_CLOSE resets used_extended_fetch flag",
                 "[odbc-api][freestmt][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLULEN row_count = 0;
  SQLUSMALLINT row_status = 0;
  ret = SQLExtendedFetch(stmt_handle(), SQL_FETCH_NEXT, 0, &row_count, &row_status);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(row_count == 1);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: SQL_CLOSE resets get_data_state",
                 "[odbc-api][freestmt][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 'abcdefghijklmnopqrstuvwxyz'"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // Read with a small buffer to trigger truncation (partial GetData)
  char small_buf[4] = {};
  SQLLEN ind = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_CHAR, small_buf, sizeof(small_buf), &ind);
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
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

TEST_CASE_METHOD(StmtDefaultDSNFixture,
                 "SQLFreeStmt: SQL_CLOSE from prepared NoResultSet transitions to Prepared (S4->S2)",
                 "[odbc-api][freestmt][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(),
                                sqlchar("CREATE OR REPLACE TEMPORARY TABLE test_freestmt_nrs (id INTEGER)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLPrepare(stmt_handle(), sqlchar("INSERT INTO test_freestmt_nrs VALUES (?)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER param = 1;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param, 0, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  // Re-execute should succeed: SQL_CLOSE transitions to Prepared, not Created.
  param = 2;
  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFreeStmt(stmt_handle(), SQL_RESET_PARAMS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT COUNT(*) FROM test_freestmt_nrs"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER count = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 2);
}

TEST_CASE_METHOD(TwoStmtDefaultDSNFixture, "SQLFreeStmt: SQL_CLOSE from Error state recovers to Created",
                 "[odbc-api][freestmt][terminating_statement]") {
  // The Error state occurs when the Arrow stream fails during fetch.
  // Force this by setting a short statement timeout on a large streaming query.
  SessionParameterOverride timeout(stmt2_handle(), "STATEMENT_TIMEOUT_IN_SECONDS", "1");
  REQUIRE(timeout.is_active());

  SQLRETURN ret =
      SQLExecDirect(stmt_handle(), sqlchar("SELECT SEQ8() FROM TABLE(GENERATOR(ROWCOUNT => 10000000000))"), SQL_NTS);
  if (ret != SQL_SUCCESS) {
    SKIP("Query timed out before streaming started; cannot reach Error state");
  }

  bool hit_error = false;
  while (!hit_error) {
    ret = SQLFetch(stmt_handle());
    if (ret == SQL_ERROR) {
      hit_error = true;
    } else if (ret != SQL_SUCCESS && ret != SQL_SUCCESS_WITH_INFO) {
      break;
    }
  }
  if (!hit_error) {
    SQLFreeStmt(stmt_handle(), SQL_CLOSE);
    SKIP("Streaming did not error within expected time; cannot reach Error state");
  }

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  // SQL_CLOSE from Error transitions to Created: SQLDescribeCol should return HY010.
  SQLCHAR col_name[128] = {};
  SQLSMALLINT name_len = 0;
  SQLSMALLINT data_type = 0;
  SQLULEN col_size = 0;
  SQLSMALLINT decimal_digits = 0;
  SQLSMALLINT nullable = 0;
  ret = SQLDescribeCol(stmt_handle(), 1, col_name, sizeof(col_name), &name_len, &data_type, &col_size, &decimal_digits,
                       &nullable);
  REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 42);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: prepared flag preserved through re-execute from NoResultSet",
                 "[odbc-api][freestmt][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLExecDirect(
      stmt_handle(), sqlchar("CREATE OR REPLACE TEMPORARY TABLE test_freestmt_reexec (id INTEGER)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLPrepare(stmt_handle(), sqlchar("INSERT INTO test_freestmt_reexec VALUES (1)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // Re-execute from NoResultSet; the prepared flag should propagate.
  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  // SQLExecute should still work: the preserved prepared flag means
  // SQL_CLOSE transitioned to Prepared, not Created.
  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT COUNT(*) FROM test_freestmt_reexec"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER count = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 3);
}

// ============================================================================
// SQLFreeStmt - Cross-API Interactions after SQL_CLOSE
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: SQLDescribeCol after SQL_CLOSE on prepared statement",
                 "[odbc-api][freestmt][terminating_statement]") {
  SKIP_OLD_DRIVER("BD#23", "Old driver does not preserve column metadata after SQL_CLOSE on prepared statement");
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 1 AS COL_A, 2 AS COL_B"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: Re-prepare with different query after SQL_CLOSE",
                 "[odbc-api][freestmt][terminating_statement]") {
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 1, 2, 3"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT col_count = 0;
  ret = SQLNumResultCols(stmt_handle(), &col_count);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(col_count == 3);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture,
                 "SQLFreeStmt: SQLRowCount returns HY010 after SQL_CLOSE on exec_direct statement",
                 "[odbc-api][freestmt][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN row_count = 0;
  ret = SQLRowCount(stmt_handle(), &row_count);
  REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture,
                 "SQLFreeStmt: SQLDescribeCol works after Prepare, fetch to Done, then SQL_CLOSE",
                 "[odbc-api][freestmt][terminating_statement]") {
  SKIP_OLD_DRIVER("BD#23", "Old driver does not preserve column metadata after SQL_CLOSE on prepared statement");
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 1 AS COL_X"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
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
  REQUIRE(std::string(reinterpret_cast<char*>(col_name)) == "COL_X");

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 1);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture,
                 "SQLFreeStmt: SQLDescribeCol fails with HY010 after SQL_CLOSE on exec_direct statement",
                 "[odbc-api][freestmt][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1 AS COL_Y"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: SQLDescribeCol after SQL_CLOSE from prepared Fetching state",
                 "[odbc-api][freestmt][terminating_statement]") {
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 1 AS COL_A UNION ALL SELECT 2"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
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

// ============================================================================
// SQLFreeStmt - SQL_UNBIND Cursor Independence
// These are separate TEST_CASE_METHODs instead of SECTIONs because Catch2
// destroys and re-creates the fixture between sections, and the
// UnixConfigInstallation destructor tears down the DSN config directory.
// The Driver Manager caches the old config, so SQLConnect fails on re-creation.
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: SQL_UNBIND does not close cursor",
                 "[odbc-api][freestmt][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_UNBIND);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 42);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: SQL_RESET_PARAMS does not close cursor",
                 "[odbc-api][freestmt][terminating_statement]") {
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_RESET_PARAMS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 42);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: SQL_UNBIND followed by re-bind",
                 "[odbc-api][freestmt][terminating_statement]") {
  SQLINTEGER buf_a = -1;
  SQLLEN ind_a = 0;
  SQLRETURN ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &buf_a, 0, &ind_a);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_UNBIND);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER buf_b = -1;
  SQLLEN ind_b = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &buf_b, 0, &ind_b);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  REQUIRE(buf_a == -1);
  REQUIRE(buf_b == 42);
}

// ============================================================================
// SQLFreeStmt - Error Cases (extended)
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeStmt: invalid option returns error with HY092",
                 "[odbc-api][freestmt][terminating_statement][error]") {
  // The Driver Manager may intercept invalid option values before they reach
  // the driver. Both the DM (HY092) and driver (HY092) map to the same SQLSTATE.
  const SQLRETURN ret = SQLFreeStmt(stmt_handle(), 99);
  REQUIRE_EXPECTED_ERROR(ret, "HY092", stmt_handle(), SQL_HANDLE_STMT);
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
    ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
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
  const SQLRETURN ret = SQLFreeStmt(SQL_NULL_HSTMT, SQL_CLOSE);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}
