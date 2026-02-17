#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "ODBCFixtures.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

// ============================================================================
// SQLPrepare - Basic Functionality
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPrepare: Successfully prepares a simple SELECT statement",
                 "[odbc-api][prepare][preparing]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT 1")), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPrepare: Prepared statement can be executed with SQLExecute",
                 "[odbc-api][prepare][preparing]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT 1")), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &value, 0, &indicator);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(value == 1);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPrepare: Can be executed multiple times after prepare",
                 "[odbc-api][prepare][preparing]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT 42")), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &value, 0, &indicator);
  REQUIRE(ret == SQL_SUCCESS);

  // First execution
  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(value == 42);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // Second execution - same prepared statement
  value = 0;
  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(value == 42);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPrepare: Replaces previous prepared statement on same handle",
                 "[odbc-api][prepare][preparing]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT 1")), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLPrepare(stmt_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT 99")), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &value, 0, &indicator);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(value == 99);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPrepare: With explicit text length instead of SQL_NTS",
                 "[odbc-api][prepare][preparing]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto sql = "SELECT 1";
  SQLRETURN ret = SQLPrepare(stmt_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(sql)),
                             static_cast<SQLINTEGER>(strlen(sql)));
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &value, 0, &indicator);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(value == 1);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPrepare: Explicit length shorter than string uses partial SQL",
                 "[odbc-api][prepare][preparing]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Pass length 8 for "SELECT 1 AS col" -> only "SELECT 1" is used
  SQLRETURN ret = SQLPrepare(stmt_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT 1 AS col")), 8);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT num_cols = 0;
  ret = SQLNumResultCols(stmt_handle(), &num_cols);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(num_cols == 1);

  // Verify column name to ensure "AS col" was NOT processed
  SQLCHAR col_name[128] = {};
  SQLSMALLINT name_len = 0;
  SQLSMALLINT data_type = 0;
  SQLULEN column_size = 0;
  SQLSMALLINT decimal_digits = 0;
  SQLSMALLINT nullable = 0;
  ret = SQLDescribeCol(stmt_handle(), 1, col_name, sizeof(col_name), &name_len, &data_type, &column_size,
                       &decimal_digits, &nullable);
  REQUIRE(ret == SQL_SUCCESS);
  std::string actual_col_name(reinterpret_cast<char*>(col_name));
  // The truncated SQL "SELECT 1" results in column name that is NOT "col"
  REQUIRE(actual_col_name == "1");

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &value, 0, &indicator);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(value == 1);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPrepare: SQLNumResultCols available after prepare",
                 "[odbc-api][prepare][preparing]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT 1, 2, 3")), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT num_cols = 0;
  ret = SQLNumResultCols(stmt_handle(), &num_cols);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(num_cols == 3);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPrepare: Prepares and executes statement with parameter markers",
                 "[odbc-api][prepare][preparing]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT ? AS val")), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER param_value = 77;
  SQLLEN param_indicator = 0;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param_value, 0,
                         &param_indicator);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER result = 0;
  SQLLEN result_indicator = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &result, 0, &result_indicator);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(result == 77);
}

// ============================================================================
// SQLPrepare - Error Cases
// ============================================================================

TEST_CASE("SQLPrepare: SQL_INVALID_HANDLE for null statement handle", "[odbc-api][prepare][preparing][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const SQLRETURN ret = SQLPrepare(SQL_NULL_HSTMT, reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT 1")), SQL_NTS);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPrepare: 42000 for invalid SQL syntax",
                 "[odbc-api][prepare][preparing][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret =
      SQLPrepare(stmt_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("THIS IS NOT VALID SQL")), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "42000", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPrepare: HY009 for null SQL text pointer",
                 "[odbc-api][prepare][preparing][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // HY009: Invalid use of null pointer
  SQLRETURN ret = SQLPrepare(stmt_handle(), nullptr, SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "HY009", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPrepare: HY090 for empty SQL string",
                 "[odbc-api][prepare][preparing][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Note: Reference driver treats empty string as invalid buffer length (HY090)
  SQLRETURN ret = SQLPrepare(stmt_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("")), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPrepare: HY090 for negative TextLength",
                 "[odbc-api][prepare][preparing][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // HY090: Invalid string or buffer length
  SQLRETURN ret = SQLPrepare(stmt_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT 1")), -5);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPrepare: HY090 for TextLength of zero",
                 "[odbc-api][prepare][preparing][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // HY090: Invalid string or buffer length
  SQLRETURN ret = SQLPrepare(stmt_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT 1")), 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPrepare: 24000 when cursor is already open",
                 "[odbc-api][prepare][preparing][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Open a cursor by executing a query
  SQLRETURN ret = SQLExecDirect(stmt_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT 1")), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // 24000: Invalid cursor state - attempt to prepare while cursor is open
  ret = SQLPrepare(stmt_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT 2")), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
}
