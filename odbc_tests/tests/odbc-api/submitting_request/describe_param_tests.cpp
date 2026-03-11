#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "ODBCFixtures.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

// ============================================================================
// SQLDescribeParam - Basic Functionality
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLDescribeParam: Describes parameter after SQLPrepare",
                 "[odbc-api][describeparam][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT dataType = 0;
  SQLULEN paramSize = 0;
  SQLSMALLINT decDigits = 0;
  SQLSMALLINT nullable = 0;

  ret = SQLDescribeParam(stmt_handle(), 1, &dataType, &paramSize, &decDigits, &nullable);
  REQUIRE(ret == SQL_SUCCESS);

  // Note: The reference driver reports all parameters as SQL_VARCHAR
  // with a large fixed size, regardless of context.
  REQUIRE(dataType == SQL_VARCHAR);
  REQUIRE(paramSize == 134217728);
  REQUIRE(nullable == SQL_NULLABLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLDescribeParam: Describes multiple parameters",
                 "[odbc-api][describeparam][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?, ?, ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  for (SQLUSMALLINT i = 1; i <= 3; i++) {
    SQLSMALLINT dataType = 0;
    SQLULEN paramSize = 0;
    SQLSMALLINT decDigits = 0;
    SQLSMALLINT nullable = 0;

    ret = SQLDescribeParam(stmt_handle(), i, &dataType, &paramSize, &decDigits, &nullable);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(dataType == SQL_VARCHAR);
    REQUIRE(paramSize == 134217728);
  }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLDescribeParam: Describes INSERT parameters against typed columns",
                 "[odbc-api][describeparam][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(
      stmt_handle(),
      sqlchar(
          ("CREATE OR REPLACE TABLE " + schema.name() + ".dp_typed_t(c1 INTEGER, c2 VARCHAR(100), c3 DOUBLE)").c_str()),
      SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  ret = SQLPrepare(stmt_handle(), sqlchar(("INSERT INTO " + schema.name() + ".dp_typed_t VALUES(?, ?, ?)").c_str()),
                   SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // Note: The reference driver reports SQL_VARCHAR with the same large fixed
  // paramSize for all parameters, even when target columns have specific types
  for (SQLUSMALLINT i = 1; i <= 3; i++) {
    SQLSMALLINT dataType = 0;
    SQLULEN paramSize = 0;
    SQLSMALLINT decDigits = 0;
    SQLSMALLINT nullable = 0;

    ret = SQLDescribeParam(stmt_handle(), i, &dataType, &paramSize, &decDigits, &nullable);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(dataType == SQL_VARCHAR);
    REQUIRE(paramSize == 134217728);
  }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLDescribeParam: Works after execute and close cursor",
                 "[odbc-api][describeparam][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER param_val = 42;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param_val, 0, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT dataType = 0;
  SQLULEN paramSize = 0;
  SQLSMALLINT decDigits = 0;
  SQLSMALLINT nullable = 0;
  ret = SQLDescribeParam(stmt_handle(), 1, &dataType, &paramSize, &decDigits, &nullable);
  REQUIRE(ret == SQL_SUCCESS);
  // IPD retains bound type after execution
  REQUIRE(dataType == SQL_INTEGER);
  REQUIRE(paramSize == 10);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLDescribeParam: Reflects bound parameter type in IPD",
                 "[odbc-api][describeparam][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // Before binding, describe returns default SQL_VARCHAR
  SQLSMALLINT dataType = 0;
  ret = SQLDescribeParam(stmt_handle(), 1, &dataType, nullptr, nullptr, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(dataType == SQL_VARCHAR);

  // After binding as SQL_INTEGER, IPD is updated
  SQLINTEGER param_val = 1;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param_val, 0, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLDescribeParam(stmt_handle(), 1, &dataType, nullptr, nullptr, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(dataType == SQL_INTEGER);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLDescribeParam: Re-prepare reflects new parameter count",
                 "[odbc-api][describeparam][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT dataType = 0;
  ret = SQLDescribeParam(stmt_handle(), 1, &dataType, nullptr, nullptr, nullptr);
  REQUIRE(ret == SQL_SUCCESS);

  // Re-prepare with a 2-parameter statement
  ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?, ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT numParams = 0;
  ret = SQLNumParams(stmt_handle(), &numParams);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(numParams == 2);

  ret = SQLDescribeParam(stmt_handle(), 2, &dataType, nullptr, nullptr, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(dataType == SQL_VARCHAR);

  // Parameter 3 no longer exists after re-prepare
  ret = SQLDescribeParam(stmt_handle(), 3, &dataType, nullptr, nullptr, nullptr);
  REQUIRE_EXPECTED_ERROR(ret, "07009", stmt_handle(), SQL_HANDLE_STMT);
}

// ============================================================================
// SQLDescribeParam - NULL Output Pointers
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLDescribeParam: All NULL output pointers accepted",
                 "[odbc-api][describeparam][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLDescribeParam(stmt_handle(), 1, nullptr, nullptr, nullptr, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLDescribeParam: Partial NULL output pointers accepted",
                 "[odbc-api][describeparam][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT dataType = 0;
  ret = SQLDescribeParam(stmt_handle(), 1, &dataType, nullptr, nullptr, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(dataType == SQL_VARCHAR);

  SQLULEN paramSize = 0;
  ret = SQLDescribeParam(stmt_handle(), 1, nullptr, &paramSize, nullptr, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(paramSize == 134217728);

  SQLSMALLINT nullable = 0;
  ret = SQLDescribeParam(stmt_handle(), 1, nullptr, nullptr, nullptr, &nullable);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(nullable == SQL_NULLABLE);
}

// ============================================================================
// SQLDescribeParam - Error Cases
// ============================================================================

TEST_CASE("SQLDescribeParam: SQL_INVALID_HANDLE for null statement handle",
          "[odbc-api][describeparam][submitting_request][error]") {
  SQLSMALLINT dataType = 0;
  SQLULEN paramSize = 0;
  SQLSMALLINT decDigits = 0;
  SQLSMALLINT nullable = 0;
  const SQLRETURN ret = SQLDescribeParam(SQL_NULL_HSTMT, 1, &dataType, &paramSize, &decDigits, &nullable);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLDescribeParam: 07009 for ParameterNumber 0",
                 "[odbc-api][describeparam][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT dataType = 0;
  SQLULEN paramSize = 0;
  SQLSMALLINT decDigits = 0;
  SQLSMALLINT nullable = 0;
  ret = SQLDescribeParam(stmt_handle(), 0, &dataType, &paramSize, &decDigits, &nullable);
  REQUIRE_EXPECTED_ERROR(ret, "07009", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLDescribeParam: 07009 for ParameterNumber beyond parameter count",
                 "[odbc-api][describeparam][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT dataType = 0;
  SQLULEN paramSize = 0;
  SQLSMALLINT decDigits = 0;
  SQLSMALLINT nullable = 0;
  ret = SQLDescribeParam(stmt_handle(), 99, &dataType, &paramSize, &decDigits, &nullable);
  REQUIRE_EXPECTED_ERROR(ret, "07009", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLDescribeParam: 07009 for prepared statement with no parameters",
                 "[odbc-api][describeparam][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 1 AS val"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT dataType = 0;
  SQLULEN paramSize = 0;
  SQLSMALLINT decDigits = 0;
  SQLSMALLINT nullable = 0;
  ret = SQLDescribeParam(stmt_handle(), 1, &dataType, &paramSize, &decDigits, &nullable);
  REQUIRE_EXPECTED_ERROR(ret, "07009", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLDescribeParam: HY010 for statement not prepared",
                 "[odbc-api][describeparam][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHSTMT fresh_stmt = SQL_NULL_HSTMT;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &fresh_stmt);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT dataType = 0;
  SQLULEN paramSize = 0;
  SQLSMALLINT decDigits = 0;
  SQLSMALLINT nullable = 0;
  ret = SQLDescribeParam(fresh_stmt, 1, &dataType, &paramSize, &decDigits, &nullable);
  REQUIRE_EXPECTED_ERROR(ret, "HY010", fresh_stmt, SQL_HANDLE_STMT);

  SQLFreeHandle(SQL_HANDLE_STMT, fresh_stmt);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLDescribeParam: HY010 during SQL_NEED_DATA",
                 "[odbc-api][describeparam][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 100, 0,
                         reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  SQLSMALLINT dataType = 0;
  SQLULEN paramSize = 0;
  SQLSMALLINT decDigits = 0;
  SQLSMALLINT nullable = 0;
  ret = SQLDescribeParam(stmt_handle(), 1, &dataType, &paramSize, &decDigits, &nullable);
  REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);

  SQLCancel(stmt_handle());
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLDescribeParam: HY010 after SQLExecDirect",
                 "[odbc-api][describeparam][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Note: The ODBC spec states HY010 is returned when called before
  // SQLPrepare or SQLExecDirect, return here should be 07009 (no parameters in the statement).
  // The reference driver incorrectly treats SQLExecDirect as not establishing the prepared state.
  SQLHSTMT fresh_stmt = SQL_NULL_HSTMT;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &fresh_stmt);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(fresh_stmt, sqlchar("SELECT 1 AS val"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT dataType = 0;
  SQLULEN paramSize = 0;
  SQLSMALLINT decDigits = 0;
  SQLSMALLINT nullable = 0;
  ret = SQLDescribeParam(fresh_stmt, 1, &dataType, &paramSize, &decDigits, &nullable);
  REQUIRE_EXPECTED_ERROR(ret, "HY010", fresh_stmt, SQL_HANDLE_STMT);

  SQLFreeHandle(SQL_HANDLE_STMT, fresh_stmt);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLDescribeParam: HY010 after execute and fetch",
                 "[odbc-api][describeparam][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Note: After SQLPrepare + SQLExecute + SQLFetch, none of the spec's HY010 conditions
  // apply (not before SQLPrepare, not async, not SQL_NEED_DATA). The reference
  // driver incorrectly returns HY010 when a cursor is open with rows fetched,
  // it should succeed.
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER param_val = 42;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param_val, 0, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT dataType = 0;
  SQLULEN paramSize = 0;
  SQLSMALLINT decDigits = 0;
  SQLSMALLINT nullable = 0;
  ret = SQLDescribeParam(stmt_handle(), 1, &dataType, &paramSize, &decDigits, &nullable);
  REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);
}
