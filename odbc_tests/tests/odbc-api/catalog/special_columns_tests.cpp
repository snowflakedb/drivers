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

// Note: SQLSpecialColumns is not fully supported by the Snowflake reference driver.
// It always returns an empty result set (SQL_NO_DATA on first fetch) regardless of
// the table or parameters used. These tests document this behavior.

// ============================================================================
// SQLSpecialColumns - Result Set Structure
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLSpecialColumns: Result set has correct number of columns",
                 "[odbc-api][catalog][specialcolumns]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret =
      SQLSpecialColumns(stmt_handle(), SQL_BEST_ROWID, sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()),
                        SQL_NTS, sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS, SQL_SCOPE_SESSION, SQL_NULLABLE);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT numCols = 0;
  ret = SQLNumResultCols(stmt_handle(), &numCols);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(numCols == 8);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLSpecialColumns: Result set column names match ODBC 3.x spec",
                 "[odbc-api][catalog][specialcolumns]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret =
      SQLSpecialColumns(stmt_handle(), SQL_BEST_ROWID, sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()),
                        SQL_NTS, sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS, SQL_SCOPE_SESSION, SQL_NULLABLE);
  REQUIRE(ret == SQL_SUCCESS);

  const char* expectedColNames[] = {"SCOPE",       "COLUMN_NAME",   "DATA_TYPE",      "TYPE_NAME",
                                    "COLUMN_SIZE", "BUFFER_LENGTH", "DECIMAL_DIGITS", "PSEUDO_COLUMN"};

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
// SQLSpecialColumns - Empty Result Set (Snowflake limitation)
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLSpecialColumns: SQL_BEST_ROWID returns empty result set",
                 "[odbc-api][catalog][specialcolumns]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Note: Snowflake does not support row identifiers, so SQLSpecialColumns
  // always returns an empty result set for SQL_BEST_ROWID.

  SQLRETURN ret =
      SQLSpecialColumns(stmt_handle(), SQL_BEST_ROWID, sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()),
                        SQL_NTS, sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS, SQL_SCOPE_SESSION, SQL_NULLABLE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLSpecialColumns: SQL_ROWVER returns empty result set",
                 "[odbc-api][catalog][specialcolumns]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Note: Snowflake does not have auto-updated version columns, so
  // SQLSpecialColumns always returns an empty result set for SQL_ROWVER.

  SQLRETURN ret =
      SQLSpecialColumns(stmt_handle(), SQL_ROWVER, sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                        sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS, SQL_SCOPE_SESSION, SQL_NULLABLE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLSpecialColumns: Various scope and nullable combinations return empty",
                 "[odbc-api][catalog][specialcolumns]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // SQL_SCOPE_CURROW + SQL_NO_NULLS
  SQLRETURN ret =
      SQLSpecialColumns(stmt_handle(), SQL_BEST_ROWID, sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()),
                        SQL_NTS, sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS, SQL_SCOPE_CURROW, SQL_NO_NULLS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
  SQLCloseCursor(stmt_handle());

  // SQL_SCOPE_TRANSACTION + SQL_NULLABLE
  ret = SQLSpecialColumns(stmt_handle(), SQL_BEST_ROWID, sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()),
                          SQL_NTS, sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS, SQL_SCOPE_TRANSACTION, SQL_NULLABLE);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

// ============================================================================
// SQLSpecialColumns - Statement Reuse & SQLRowCount
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLSpecialColumns: Can call multiple times after close cursor",
                 "[odbc-api][catalog][specialcolumns]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret =
      SQLSpecialColumns(stmt_handle(), SQL_BEST_ROWID, sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()),
                        SQL_NTS, sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS, SQL_SCOPE_SESSION, SQL_NULLABLE);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
  SQLCloseCursor(stmt_handle());

  ret = SQLSpecialColumns(stmt_handle(), SQL_ROWVER, sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                          sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS, SQL_SCOPE_SESSION, SQL_NULLABLE);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLSpecialColumns: SQLRowCount returns -1",
                 "[odbc-api][catalog][specialcolumns]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret =
      SQLSpecialColumns(stmt_handle(), SQL_BEST_ROWID, sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()),
                        SQL_NTS, sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS, SQL_SCOPE_SESSION, SQL_NULLABLE);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN rowCount = 0;
  ret = SQLRowCount(stmt_handle(), &rowCount);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(rowCount == -1);
}

// ============================================================================
// SQLSpecialColumns - Error Cases
// ============================================================================

TEST_CASE("SQLSpecialColumns: SQL_INVALID_HANDLE for null statement handle",
          "[odbc-api][catalog][specialcolumns][error]") {
  const SQLRETURN ret = SQLSpecialColumns(SQL_NULL_HSTMT, SQL_BEST_ROWID, nullptr, 0, nullptr, 0, sqlchar("T"), SQL_NTS,
                                          SQL_SCOPE_SESSION, SQL_NULLABLE);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSpecialColumns: HY090 - Negative TableName length",
                 "[odbc-api][catalog][specialcolumns][error]") {
  const SQLRETURN ret = SQLSpecialColumns(stmt_handle(), SQL_BEST_ROWID, nullptr, 0, nullptr, 0, sqlchar("TABLE"), -999,
                                          SQL_SCOPE_SESSION, SQL_NULLABLE);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLSpecialColumns: 24000 - Cursor already open",
                 "[odbc-api][catalog][specialcolumns][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret =
      SQLSpecialColumns(stmt_handle(), SQL_BEST_ROWID, sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()),
                        SQL_NTS, sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS, SQL_SCOPE_SESSION, SQL_NULLABLE);
  REQUIRE(ret == SQL_SUCCESS);

  // Second call without closing cursor
  ret = SQLSpecialColumns(stmt_handle(), SQL_BEST_ROWID, sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()),
                          SQL_NTS, sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS, SQL_SCOPE_SESSION, SQL_NULLABLE);
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(DbcFixture, "SQLSpecialColumns: Requires active connection",
                 "[odbc-api][catalog][specialcolumns][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHSTMT stmt = SQL_NULL_HSTMT;
  const SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);

  // Note: Reference driver refuses to allocate statement on disconnected handle
  REQUIRE(ret == SQL_ERROR);
}
