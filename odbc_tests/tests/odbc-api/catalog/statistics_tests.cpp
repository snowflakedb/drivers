#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "ODBCFixtures.hpp"
#include "ReadOnlyDbFixture.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

// Note: SQLStatistics is not fully supported by the Snowflake reference driver.
// It always returns an empty result set (SQL_NO_DATA on first fetch) regardless of
// the table or index configuration. These tests document this behavior.

// ============================================================================
// SQLStatistics - Result Set Structure
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLStatistics: Result set has correct number of columns",
                 "[odbc-api][catalog][statistics]") {
  SQLRETURN ret = SQLStatistics(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS, SQL_INDEX_ALL, SQL_QUICK);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT numCols = 0;
  ret = SQLNumResultCols(stmt_handle(), &numCols);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(numCols == 13);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLStatistics: Result set column names match ODBC 3.x spec",
                 "[odbc-api][catalog][statistics]") {
  SQLRETURN ret = SQLStatistics(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS, SQL_INDEX_ALL, SQL_QUICK);
  REQUIRE(ret == SQL_SUCCESS);

  const char* expectedColNames[] = {"TABLE_CAT",   "TABLE_SCHEM", "TABLE_NAME",       "NON_UNIQUE",  "INDEX_QUALIFIER",
                                    "INDEX_NAME",  "TYPE",        "ORDINAL_POSITION", "COLUMN_NAME", "ASC_OR_DESC",
                                    "CARDINALITY", "PAGES",       "FILTER_CONDITION"};

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
// SQLStatistics - Empty Result Set (Snowflake limitation)
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLStatistics: Returns empty result set for table with primary key",
                 "[odbc-api][catalog][statistics]") {
  // Note: Snowflake does not expose index/statistics metadata through ODBC.
  // SQLStatistics always returns an empty result set.

  SQLRETURN ret = SQLStatistics(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS, SQL_INDEX_ALL, SQL_QUICK);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLStatistics: SQL_INDEX_UNIQUE returns empty",
                 "[odbc-api][catalog][statistics]") {
  SQLRETURN ret = SQLStatistics(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS, SQL_INDEX_UNIQUE, SQL_ENSURE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

// ============================================================================
// SQLStatistics - Statement Reuse & SQLRowCount
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLStatistics: Can call multiple times after close cursor",
                 "[odbc-api][catalog][statistics]") {
  SQLRETURN ret = SQLStatistics(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS, SQL_INDEX_ALL, SQL_QUICK);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
  SQLCloseCursor(stmt_handle());

  ret = SQLStatistics(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                      sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS, SQL_INDEX_ALL, SQL_QUICK);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLStatistics: SQLRowCount returns -1", "[odbc-api][catalog][statistics]") {
  SQLRETURN ret = SQLStatistics(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS, SQL_INDEX_ALL, SQL_QUICK);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN rowCount = 0;
  ret = SQLRowCount(stmt_handle(), &rowCount);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(rowCount == -1);
}

// ============================================================================
// SQLStatistics - Error Cases
// ============================================================================

TEST_CASE("SQLStatistics: SQL_INVALID_HANDLE for null statement handle", "[odbc-api][catalog][statistics][error]") {
  const SQLRETURN ret =
      SQLStatistics(SQL_NULL_HSTMT, nullptr, 0, nullptr, 0, sqlchar("T"), SQL_NTS, SQL_INDEX_ALL, SQL_QUICK);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLStatistics: HY090 - Negative TableName length",
                 "[odbc-api][catalog][statistics][error]") {
  const SQLRETURN ret =
      SQLStatistics(stmt_handle(), nullptr, 0, nullptr, 0, sqlchar("TABLE"), -999, SQL_INDEX_ALL, SQL_QUICK);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLStatistics: 24000 - Cursor already open",
                 "[odbc-api][catalog][statistics][error]") {
  SQLRETURN ret = SQLStatistics(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS, SQL_INDEX_ALL, SQL_QUICK);
  REQUIRE(ret == SQL_SUCCESS);

  // Second call without closing cursor
  ret = SQLStatistics(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                      sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS, SQL_INDEX_ALL, SQL_QUICK);
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(DbcFixture, "SQLStatistics: Requires active connection", "[odbc-api][catalog][statistics][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHSTMT stmt = SQL_NULL_HSTMT;
  const SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);

  // Note: Reference driver refuses to allocate statement on disconnected handle
  REQUIRE(ret == SQL_ERROR);
}
