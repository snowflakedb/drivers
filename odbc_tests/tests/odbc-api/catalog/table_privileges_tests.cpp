#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "ODBCFixtures.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "query_helpers.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

// Note: SQLTablePrivileges is not fully supported by the Snowflake reference driver.
// It always returns an empty result set (SQL_NO_DATA on first fetch) regardless of
// the table or privilege configuration. These tests document this behavior.

// ============================================================================
// SQLTablePrivileges - Result Set Structure
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLTablePrivileges: Result set has correct number of columns",
                 "[odbc-api][catalog][tableprivileges]") {
  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret =
      SQLExecDirect(stmt_handle(), sqlchar("CREATE TABLE test_tp_numcols (id INT, name VARCHAR(100))"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());

  ret = SQLTablePrivileges(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()), SQL_NTS,
                           sqlchar("TEST_TP_NUMCOLS"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT numCols = 0;
  ret = SQLNumResultCols(stmt_handle(), &numCols);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(numCols == 7);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLTablePrivileges: Result set column names match ODBC 3.x spec",
                 "[odbc-api][catalog][tableprivileges]") {
  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("CREATE TABLE test_tp_colnames (id INT)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());

  ret = SQLTablePrivileges(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()), SQL_NTS,
                           sqlchar("TEST_TP_COLNAMES"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  const char* expectedColNames[] = {"TABLE_CAT", "TABLE_SCHEM", "TABLE_NAME",  "GRANTOR",
                                    "GRANTEE",   "PRIVILEGE",   "IS_GRANTABLE"};

  for (SQLSMALLINT col = 1; col <= static_cast<SQLSMALLINT>(std::size(expectedColNames)); col++) {
    char colName[256] = {};
    SQLSMALLINT nameLen = 0;
    SQLSMALLINT dataType = 0;
    SQLULEN colSize = 0;
    SQLSMALLINT decDigits = 0;
    SQLSMALLINT nullable = 0;

    ret = SQLDescribeCol(stmt_handle(), col, reinterpret_cast<SQLCHAR*>(colName), sizeof(colName), &nameLen, &dataType,
                         &colSize, &decDigits, &nullable);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(std::string(colName) == expectedColNames[col - 1]);
  }
}

// ============================================================================
// SQLTablePrivileges - Empty Result Set (Snowflake limitation)
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLTablePrivileges: Returns empty result set for existing table",
                 "[odbc-api][catalog][tableprivileges]") {
  // Note: Snowflake does not expose table-level privilege metadata through ODBC.
  // SQLTablePrivileges always returns an empty result set.

  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret =
      SQLExecDirect(stmt_handle(), sqlchar("CREATE TABLE test_tp_empty (id INT, name VARCHAR(100))"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());

  ret = SQLTablePrivileges(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()), SQL_NTS,
                           sqlchar("TEST_TP_EMPTY"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLTablePrivileges: Wildcard and NULL parameters return empty",
                 "[odbc-api][catalog][tableprivileges]") {
  const SQLRETURN ret = SQLTablePrivileges(stmt_handle(), nullptr, 0, nullptr, 0, sqlchar("%"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  const SQLRETURN fetchRet = SQLFetch(stmt_handle());
  REQUIRE(fetchRet == SQL_NO_DATA);
}

// ============================================================================
// SQLTablePrivileges - Statement Reuse & SQLRowCount
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLTablePrivileges: Can call multiple times after close cursor",
                 "[odbc-api][catalog][tableprivileges]") {
  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("CREATE TABLE test_tp_reuse (id INT)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());

  ret = SQLTablePrivileges(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()), SQL_NTS,
                           sqlchar("TEST_TP_REUSE"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
  SQLCloseCursor(stmt_handle());

  ret = SQLTablePrivileges(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()), SQL_NTS,
                           sqlchar("TEST_TP_REUSE"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLTablePrivileges: SQLRowCount returns -1",
                 "[odbc-api][catalog][tableprivileges]") {
  const SQLRETURN ret = SQLTablePrivileges(stmt_handle(), nullptr, 0, nullptr, 0, sqlchar("ANY_TABLE"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN rowCount = 0;
  const SQLRETURN rcRet = SQLRowCount(stmt_handle(), &rowCount);
  REQUIRE(rcRet == SQL_SUCCESS);
  REQUIRE(rowCount == -1);
}

// ============================================================================
// SQLTablePrivileges - Error Cases
// ============================================================================

TEST_CASE("SQLTablePrivileges: SQL_INVALID_HANDLE for null statement handle",
          "[odbc-api][catalog][tableprivileges][error]") {
  const SQLRETURN ret = SQLTablePrivileges(SQL_NULL_HSTMT, nullptr, 0, nullptr, 0, sqlchar("T"), SQL_NTS);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLTablePrivileges: HY090 - Negative TableName length",
                 "[odbc-api][catalog][tableprivileges][error]") {
  const SQLRETURN ret = SQLTablePrivileges(stmt_handle(), nullptr, 0, nullptr, 0, sqlchar("TABLE"), -999);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLTablePrivileges: HY090 - Negative SchemaName length",
                 "[odbc-api][catalog][tableprivileges][error]") {
  const SQLRETURN ret =
      SQLTablePrivileges(stmt_handle(), nullptr, 0, sqlchar("SCHEMA"), -999, sqlchar("TABLE"), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLTablePrivileges: 24000 - Cursor already open",
                 "[odbc-api][catalog][tableprivileges][error]") {
  SQLRETURN ret = SQLTablePrivileges(stmt_handle(), nullptr, 0, nullptr, 0, sqlchar("ANY_TABLE"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // Second call without closing cursor
  ret = SQLTablePrivileges(stmt_handle(), nullptr, 0, nullptr, 0, sqlchar("ANY_TABLE"), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(DbcFixture, "SQLTablePrivileges: Requires active connection",
                 "[odbc-api][catalog][tableprivileges][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHSTMT stmt = SQL_NULL_HSTMT;
  const SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);

  // Note: Reference driver refuses to allocate statement on disconnected handle
  REQUIRE(ret == SQL_ERROR);
}
