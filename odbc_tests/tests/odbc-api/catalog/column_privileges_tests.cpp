#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "ODBCFixtures.hpp"
#include "ReadOnlyDbFixture.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

// Note: SQLColumnPrivileges is not fully supported by the Snowflake reference driver.
// It always returns an empty result set (SQL_NO_DATA on first fetch) regardless of
// the table or parameters used. These tests document this behavior.

// ============================================================================
// SQLColumnPrivileges - Result Set Structure
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumnPrivileges: Result set has correct number of columns",
                 "[odbc-api][catalog][columnprivileges]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLColumnPrivileges(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                      sqlchar(readonly_db::BASIC_TABLE), SQL_NTS, sqlchar("%"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT numCols = 0;
  ret = SQLNumResultCols(stmt_handle(), &numCols);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(numCols == 8);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumnPrivileges: Result set column names match ODBC 3.x spec",
                 "[odbc-api][catalog][columnprivileges]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLColumnPrivileges(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                      sqlchar(readonly_db::BASIC_TABLE), SQL_NTS, sqlchar("%"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  const char* expectedColNames[] = {"TABLE_CAT", "TABLE_SCHEM", "TABLE_NAME", "COLUMN_NAME",
                                    "GRANTOR",   "GRANTEE",     "PRIVILEGE",  "IS_GRANTABLE"};

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
// SQLColumnPrivileges - Empty Result Set (Snowflake limitation)
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumnPrivileges: Returns empty result set for existing table",
                 "[odbc-api][catalog][columnprivileges]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Note: Snowflake does NOT support traditional SQL column-level GRANT privileges
  // (e.g., GRANT SELECT(col)). SQLColumnPrivileges always returns an empty result set.

  SQLRETURN ret = SQLColumnPrivileges(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                      sqlchar(readonly_db::BASIC_TABLE), SQL_NTS, sqlchar("%"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumnPrivileges: Various parameter combinations return empty",
                 "[odbc-api][catalog][columnprivileges]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Note: Cannot verify actual search pattern/parameter behavior since Snowflake doesn't
  // support column privileges. These tests only verify that various parameter combinations
  // are accepted without error and return empty result sets.

  // Wildcard % for ColumnName
  SQLRETURN ret = SQLColumnPrivileges(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                      sqlchar(readonly_db::BASIC_TABLE), SQL_NTS, sqlchar("%"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
  SQLCloseCursor(stmt_handle());

  // NULL ColumnName (treated as wildcard)
  ret = SQLColumnPrivileges(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                            sqlchar(readonly_db::BASIC_TABLE), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
  SQLCloseCursor(stmt_handle());

  // Explicit string lengths instead of SQL_NTS
  const char* tbl = readonly_db::BASIC_TABLE;
  const char* col = "%";
  ret = SQLColumnPrivileges(stmt_handle(), sqlchar(database_name()), static_cast<SQLSMALLINT>(strlen(database_name())),
                            sqlchar(schema_name()), static_cast<SQLSMALLINT>(strlen(schema_name())), sqlchar(tbl),
                            static_cast<SQLSMALLINT>(strlen(tbl)), sqlchar(col), static_cast<SQLSMALLINT>(strlen(col)));
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumnPrivileges: Non-existent table returns empty result set",
                 "[odbc-api][catalog][columnprivileges]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Note: Cannot distinguish between "table doesn't exist" and "no privileges exist"
  // since Snowflake doesn't support column privileges - both return empty result sets.

  SQLRETURN ret = SQLColumnPrivileges(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                      sqlchar("NONEXISTENTTABLEXYZ99999"), SQL_NTS, sqlchar("%"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

// ============================================================================
// SQLColumnPrivileges - Statement Reuse & SQLRowCount
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumnPrivileges: Can call multiple times after close cursor",
                 "[odbc-api][catalog][columnprivileges]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLColumnPrivileges(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                      sqlchar(readonly_db::BASIC_TABLE), SQL_NTS, sqlchar("%"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
  SQLCloseCursor(stmt_handle());

  ret = SQLColumnPrivileges(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                            sqlchar(readonly_db::BASIC_TABLE), SQL_NTS, sqlchar("%"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumnPrivileges: SQLRowCount returns -1",
                 "[odbc-api][catalog][columnprivileges]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLColumnPrivileges(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                      sqlchar(readonly_db::BASIC_TABLE), SQL_NTS, sqlchar("%"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN rowCount = 0;
  ret = SQLRowCount(stmt_handle(), &rowCount);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(rowCount == -1);
}

// ============================================================================
// SQLColumnPrivileges - Error Cases
// ============================================================================

TEST_CASE("SQLColumnPrivileges: SQL_INVALID_HANDLE for null statement handle",
          "[odbc-api][catalog][columnprivileges][error]") {
  const SQLRETURN ret =
      SQLColumnPrivileges(SQL_NULL_HSTMT, nullptr, 0, nullptr, 0, sqlchar("TABLE"), SQL_NTS, sqlchar("%"), SQL_NTS);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLColumnPrivileges: HY009 - NULL TableName pointer",
                 "[odbc-api][catalog][columnprivileges][error]") {
  const SQLRETURN ret = SQLColumnPrivileges(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, SQL_NTS, nullptr, 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY009", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLColumnPrivileges: HY090 - Negative CatalogName length",
                 "[odbc-api][catalog][columnprivileges][error]") {
  const SQLRETURN ret =
      SQLColumnPrivileges(stmt_handle(), sqlchar("DB"), -999, nullptr, 0, sqlchar("TABLE"), SQL_NTS, nullptr, 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLColumnPrivileges: HY090 - Negative SchemaName length",
                 "[odbc-api][catalog][columnprivileges][error]") {
  const SQLRETURN ret =
      SQLColumnPrivileges(stmt_handle(), nullptr, 0, sqlchar("SCHEMA"), -999, sqlchar("TABLE"), SQL_NTS, nullptr, 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLColumnPrivileges: HY090 - Negative TableName length",
                 "[odbc-api][catalog][columnprivileges][error]") {
  const SQLRETURN ret = SQLColumnPrivileges(stmt_handle(), nullptr, 0, nullptr, 0, sqlchar("TABLE"), -999, nullptr, 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLColumnPrivileges: HY090 - Negative ColumnName length",
                 "[odbc-api][catalog][columnprivileges][error]") {
  const SQLRETURN ret =
      SQLColumnPrivileges(stmt_handle(), nullptr, 0, nullptr, 0, sqlchar("TABLE"), SQL_NTS, sqlchar("COLUMN"), -999);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumnPrivileges: 24000 - Cursor already open",
                 "[odbc-api][catalog][columnprivileges][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLColumnPrivileges(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                      sqlchar(readonly_db::BASIC_TABLE), SQL_NTS, sqlchar("%"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // Second call without closing cursor
  ret = SQLColumnPrivileges(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                            sqlchar(readonly_db::BASIC_TABLE), SQL_NTS, sqlchar("%"), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(DbcFixture, "SQLColumnPrivileges: Requires active connection",
                 "[odbc-api][catalog][columnprivileges][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHSTMT stmt = SQL_NULL_HSTMT;
  const SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);

  // Note: Reference driver refuses to allocate statement on disconnected handle
  REQUIRE(ret == SQL_ERROR);
}
