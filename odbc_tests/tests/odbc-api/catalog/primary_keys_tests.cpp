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
// SQLPrimaryKeys - Result Set Structure
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLPrimaryKeys: Result set has correct number of columns",
                 "[odbc-api][primarykeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrimaryKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                 sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT numCols = 0;
  ret = SQLNumResultCols(stmt_handle(), &numCols);
  REQUIRE(ret == SQL_SUCCESS);
  // ODBC 3.x spec defines 6 columns for SQLPrimaryKeys
  REQUIRE(numCols == 6);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLPrimaryKeys: Result set column names match ODBC 3.x spec",
                 "[odbc-api][primarykeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrimaryKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                 sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  const char* expectedColNames[] = {"TABLE_CAT", "TABLE_SCHEM", "TABLE_NAME", "COLUMN_NAME", "KEY_SEQ", "PK_NAME"};

  SQLSMALLINT numCols = 0;
  ret = SQLNumResultCols(stmt_handle(), &numCols);
  REQUIRE(ret == SQL_SUCCESS);

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
// SQLPrimaryKeys - Data Verification
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLPrimaryKeys: Returns primary key for single-column PK",
                 "[odbc-api][primarykeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrimaryKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                 sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  char tableCat[256] = {};
  char tableSchem[256] = {};
  char tableName[256] = {};
  char columnName[256] = {};
  SQLSMALLINT keySeq = 0;

  SQLGetData(stmt_handle(), 1, SQL_C_CHAR, tableCat, sizeof(tableCat), nullptr);
  SQLGetData(stmt_handle(), 2, SQL_C_CHAR, tableSchem, sizeof(tableSchem), nullptr);
  SQLGetData(stmt_handle(), 3, SQL_C_CHAR, tableName, sizeof(tableName), nullptr);
  SQLGetData(stmt_handle(), 4, SQL_C_CHAR, columnName, sizeof(columnName), nullptr);
  SQLGetData(stmt_handle(), 5, SQL_C_SSHORT, &keySeq, 0, nullptr);

  REQUIRE(std::string(tableCat) == database_name());
  REQUIRE(std::string(tableSchem) == schema_name());
  REQUIRE(std::string(tableName) == readonly_db::SINGLE_PK_TABLE);
  REQUIRE(std::string(columnName) == "ID");
  REQUIRE(keySeq == 1);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLPrimaryKeys: Returns composite primary key with correct KEY_SEQ",
                 "[odbc-api][primarykeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrimaryKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                 sqlchar(readonly_db::COMPOSITE_PK_TABLE), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  char columnName[256] = {};
  SQLSMALLINT keySeq = 0;

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  SQLGetData(stmt_handle(), 4, SQL_C_CHAR, columnName, sizeof(columnName), nullptr);
  SQLGetData(stmt_handle(), 5, SQL_C_SSHORT, &keySeq, 0, nullptr);
  REQUIRE(std::string(columnName) == "REGIONID");
  REQUIRE(keySeq == 1);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  SQLGetData(stmt_handle(), 4, SQL_C_CHAR, columnName, sizeof(columnName), nullptr);
  SQLGetData(stmt_handle(), 5, SQL_C_SSHORT, &keySeq, 0, nullptr);
  REQUIRE(std::string(columnName) == "STOREID");
  REQUIRE(keySeq == 2);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLPrimaryKeys: Table without primary key returns empty result set",
                 "[odbc-api][primarykeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrimaryKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                 sqlchar(readonly_db::NO_PK_TABLE), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLPrimaryKeys: Non-existent table returns empty result set",
                 "[odbc-api][primarykeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrimaryKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                 sqlchar("NONEXISTENTTABLEXYZ99999"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

// ============================================================================
// SQLPrimaryKeys - Parameter Variations
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLPrimaryKeys: Various parameter combinations are accepted",
                 "[odbc-api][primarykeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Explicit catalog, schema, table with SQL_NTS
  SQLRETURN ret = SQLPrimaryKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                 sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  int count1 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count1++;
  REQUIRE(count1 == 1);
  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // Explicit string lengths instead of SQL_NTS
  ret = SQLPrimaryKeys(stmt_handle(), sqlchar(database_name()), static_cast<SQLSMALLINT>(std::strlen(database_name())),
                       sqlchar(schema_name()), static_cast<SQLSMALLINT>(std::strlen(schema_name())),
                       sqlchar(readonly_db::SINGLE_PK_TABLE),
                       static_cast<SQLSMALLINT>(std::strlen(readonly_db::SINGLE_PK_TABLE)));
  REQUIRE(ret == SQL_SUCCESS);
  int count2 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count2++;
  REQUIRE(count2 == 1);
}

// ============================================================================
// SQLPrimaryKeys - Statement Reuse
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLPrimaryKeys: Can call multiple times on same statement after close cursor",
                 "[odbc-api][primarykeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrimaryKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                 sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  int count1 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count1++;
  REQUIRE(count1 == 1);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLPrimaryKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                       sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  int count2 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count2++;
  REQUIRE(count2 == 1);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLPrimaryKeys: SQLRowCount after catalog function call",
                 "[odbc-api][primarykeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrimaryKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                 sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN rowCount = 0;
  ret = SQLRowCount(stmt_handle(), &rowCount);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(rowCount == -1);
}

// ============================================================================
// SQLPrimaryKeys - Error Cases
// ============================================================================

TEST_CASE("SQLPrimaryKeys: SQL_INVALID_HANDLE for null statement handle", "[odbc-api][primarykeys][catalog][error]") {
  const SQLRETURN ret = SQLPrimaryKeys(SQL_NULL_HSTMT, nullptr, 0, nullptr, 0, sqlchar("TABLE"), SQL_NTS);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPrimaryKeys: HY090 - Negative CatalogName length",
                 "[odbc-api][primarykeys][catalog][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrimaryKeys(stmt_handle(), sqlchar("SNOWFLAKE"), -999, nullptr, 0, sqlchar("TABLE"), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPrimaryKeys: HY090 - Negative SchemaName length",
                 "[odbc-api][primarykeys][catalog][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrimaryKeys(stmt_handle(), nullptr, 0, sqlchar("SCHEMA"), -999, sqlchar("TABLE"), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPrimaryKeys: HY090 - Negative TableName length",
                 "[odbc-api][primarykeys][catalog][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrimaryKeys(stmt_handle(), nullptr, 0, nullptr, 0, sqlchar("TABLE"), -999);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLPrimaryKeys: 24000 - Cursor already open",
                 "[odbc-api][primarykeys][catalog][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrimaryKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                 sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // Second call without closing cursor
  ret = SQLPrimaryKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                       sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(DbcFixture, "SQLPrimaryKeys: Requires active connection", "[odbc-api][primarykeys][catalog][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHSTMT stmt = SQL_NULL_HSTMT;
  const SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);

  // Note: Reference driver refuses to allocate statement on disconnected handle
  REQUIRE(ret == SQL_ERROR);
}
