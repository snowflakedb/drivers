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

// ============================================================================
// SQLTables - Result Set Structure
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: Result set has correct number of columns",
                 "[odbc-api][catalog][tables]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                            sqlchar(readonly_db::BASIC_TABLE), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // ODBC spec defines 5 columns
  SQLSMALLINT numCols = 0;
  ret = SQLNumResultCols(stmt_handle(), &numCols);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(numCols == 5);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: Result set column names match ODBC 3.x spec",
                 "[odbc-api][catalog][tables]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                            sqlchar(readonly_db::BASIC_TABLE), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  const char* expectedColNames[] = {"TABLE_CAT", "TABLE_SCHEM", "TABLE_NAME", "TABLE_TYPE", "REMARKS"};

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
// SQLTables - Data Verification
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: Returns known table with correct metadata",
                 "[odbc-api][catalog][tables]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                            sqlchar(readonly_db::BASIC_TABLE), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  char tableCat[256] = {};
  char tableSchem[256] = {};
  char tableName[256] = {};
  char tableType[256] = {};

  SQLGetData(stmt_handle(), 1, SQL_C_CHAR, tableCat, sizeof(tableCat), nullptr);
  SQLGetData(stmt_handle(), 2, SQL_C_CHAR, tableSchem, sizeof(tableSchem), nullptr);
  SQLGetData(stmt_handle(), 3, SQL_C_CHAR, tableName, sizeof(tableName), nullptr);
  SQLGetData(stmt_handle(), 4, SQL_C_CHAR, tableType, sizeof(tableType), nullptr);

  REQUIRE(std::string(tableCat) == database_name());
  REQUIRE(std::string(tableSchem) == schema_name());
  REQUIRE(std::string(tableName) == readonly_db::BASIC_TABLE);
  REQUIRE(std::string(tableType) == "TABLE");

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: Returns view with TABLE_TYPE VIEW", "[odbc-api][catalog][tables]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                            sqlchar(readonly_db::BASIC_VIEW), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  char tableType[256] = {};
  SQLGetData(stmt_handle(), 4, SQL_C_CHAR, tableType, sizeof(tableType), nullptr);
  REQUIRE(std::string(tableType) == "VIEW");

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: Non-existent table returns empty result set",
                 "[odbc-api][catalog][tables]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                            sqlchar("NONEXISTENTTABLEXYZ99999"), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: TABLE_TYPE filter restricts results",
                 "[odbc-api][catalog][tables]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // No filter - wildcard BASIC% matches both BASIC_TABLE and BASIC_VIEW
  SQLRETURN ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                            sqlchar("BASIC%"), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);
  int totalCount = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    totalCount++;
  REQUIRE(totalCount == 2);
  SQLCloseCursor(stmt_handle());

  // Filter for TABLE - should return only BASIC_TABLE
  ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                  sqlchar(readonly_db::BASIC_TABLE), SQL_NTS, sqlchar("TABLE"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  char name1[256] = {};
  char type1[256] = {};
  SQLGetData(stmt_handle(), 3, SQL_C_CHAR, name1, sizeof(name1), nullptr);
  SQLGetData(stmt_handle(), 4, SQL_C_CHAR, type1, sizeof(type1), nullptr);
  REQUIRE(std::string(name1) == readonly_db::BASIC_TABLE);
  REQUIRE(std::string(type1) == "TABLE");
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
  SQLCloseCursor(stmt_handle());

  // Filter for VIEW - should return only BASIC_VIEW
  ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                  sqlchar(readonly_db::BASIC_VIEW), SQL_NTS, sqlchar("VIEW"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  char name2[256] = {};
  char type2[256] = {};
  SQLGetData(stmt_handle(), 3, SQL_C_CHAR, name2, sizeof(name2), nullptr);
  SQLGetData(stmt_handle(), 4, SQL_C_CHAR, type2, sizeof(type2), nullptr);
  REQUIRE(std::string(name2) == readonly_db::BASIC_VIEW);
  REQUIRE(std::string(type2) == "VIEW");
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: Wildcard search finds table", "[odbc-api][catalog][tables]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                            sqlchar("BASICTAB%"), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  char tableName[256] = {};
  SQLGetData(stmt_handle(), 3, SQL_C_CHAR, tableName, sizeof(tableName), nullptr);
  REQUIRE(std::string(tableName) == readonly_db::BASIC_TABLE);
}

// ============================================================================
// SQLTables - Parameter Variations
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: Various parameter combinations are accepted",
                 "[odbc-api][catalog][tables]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  const char* db = database_name();
  const char* schema = schema_name();
  const char* tbl = readonly_db::BASIC_TABLE;

  // SQL_NTS lengths
  SQLRETURN ret =
      SQLTables(stmt_handle(), sqlchar(db), SQL_NTS, sqlchar(schema), SQL_NTS, sqlchar(tbl), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);
  int count1 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count1++;
  REQUIRE(count1 == 1);
  SQLCloseCursor(stmt_handle());

  // Explicit string lengths
  ret = SQLTables(stmt_handle(), sqlchar(db), static_cast<SQLSMALLINT>(std::strlen(db)), sqlchar(schema),
                  static_cast<SQLSMALLINT>(std::strlen(schema)), sqlchar(tbl),
                  static_cast<SQLSMALLINT>(std::strlen(tbl)), nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);
  int count2 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count2++;
  REQUIRE(count2 == 1);
}

// ============================================================================
// SQLTables - Statement Reuse & SQLRowCount
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: Can call multiple times after close cursor",
                 "[odbc-api][catalog][tables]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                            sqlchar(readonly_db::BASIC_TABLE), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);
  int count1 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count1++;
  REQUIRE(count1 == 1);
  SQLCloseCursor(stmt_handle());

  ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                  sqlchar(readonly_db::BASIC_TABLE), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);
  int count2 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count2++;
  REQUIRE(count2 == 1);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: SQLRowCount returns -1", "[odbc-api][catalog][tables]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                            sqlchar(readonly_db::BASIC_TABLE), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN rowCount = 0;
  ret = SQLRowCount(stmt_handle(), &rowCount);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(rowCount == -1);
}

// ============================================================================
// SQLTables - Error Cases
// ============================================================================

TEST_CASE("SQLTables: SQL_INVALID_HANDLE for null statement handle", "[odbc-api][catalog][tables][error]") {
  const SQLRETURN ret = SQLTables(SQL_NULL_HSTMT, nullptr, 0, nullptr, 0, sqlchar("T"), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLTables: HY090 - Negative CatalogName length",
                 "[odbc-api][catalog][tables][error]") {
  const SQLRETURN ret = SQLTables(stmt_handle(), sqlchar("DB"), -999, nullptr, 0, sqlchar("T"), SQL_NTS, nullptr, 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLTables: HY090 - Negative SchemaName length",
                 "[odbc-api][catalog][tables][error]") {
  const SQLRETURN ret = SQLTables(stmt_handle(), nullptr, 0, sqlchar("S"), -999, sqlchar("T"), SQL_NTS, nullptr, 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLTables: HY090 - Negative TableName length",
                 "[odbc-api][catalog][tables][error]") {
  const SQLRETURN ret = SQLTables(stmt_handle(), nullptr, 0, nullptr, 0, sqlchar("T"), -999, nullptr, 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLTables: 24000 - Cursor already open",
                 "[odbc-api][catalog][tables][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SQLRETURN ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                            sqlchar(readonly_db::BASIC_TABLE), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Second call without closing cursor
  ret = SQLTables(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                  sqlchar(readonly_db::BASIC_TABLE), SQL_NTS, nullptr, 0);
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(DbcFixture, "SQLTables: Requires active connection", "[odbc-api][catalog][tables][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHSTMT stmt = SQL_NULL_HSTMT;
  const SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);

  // Note: Reference driver refuses to allocate statement on disconnected handle
  REQUIRE(ret == SQL_ERROR);
}
