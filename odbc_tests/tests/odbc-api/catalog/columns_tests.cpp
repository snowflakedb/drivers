#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "ODBCFixtures.hpp"
#include "ReadOnlyDbFixture.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

// ============================================================================
// SQLColumns - Result Set Structure
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLColumns: Result set has correct number of columns",
                 "[odbc-api][columns][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLColumns(stmt_handle(), nullptr, 0, nullptr, 0, sqlchar("DATABASES"), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Note: Reference driver returns 19 columns (ODBC 3.x spec defines 18, driver adds 1 extra)
  SQLSMALLINT numCols = 0;
  ret = SQLNumResultCols(stmt_handle(), &numCols);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(numCols == 19);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLColumns: Result set column names match ODBC 3.x spec",
                 "[odbc-api][columns][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLColumns(stmt_handle(), nullptr, 0, nullptr, 0, sqlchar("DATABASES"), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Note: Reference driver returns 19 columns
  const char* expectedColNames[] = {"TABLE_CAT",        "TABLE_SCHEM",    "TABLE_NAME",       "COLUMN_NAME",
                                    "DATA_TYPE",        "TYPE_NAME",      "COLUMN_SIZE",      "BUFFER_LENGTH",
                                    "DECIMAL_DIGITS",   "NUM_PREC_RADIX", "NULLABLE",         "REMARKS",
                                    "COLUMN_DEF",       "SQL_DATA_TYPE",  "SQL_DATETIME_SUB", "CHAR_OCTET_LENGTH",
                                    "ORDINAL_POSITION", "IS_NULLABLE",    "USER_DATA_TYPE"};

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
// SQLColumns - Data Verification
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumns: Returns correct column metadata for known table",
                 "[odbc-api][columns][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                             sqlchar(readonly_db::MULTI_TYPE_TABLE), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  std::vector<std::string> columnNames;
  while (true) {
    ret = SQLFetch(stmt_handle());
    if (ret == SQL_NO_DATA) break;
    REQUIRE(ret == SQL_SUCCESS);

    char tableCat[256] = {};
    char tableSchem[256] = {};
    char tableName[256] = {};
    char columnName[256] = {};

    SQLGetData(stmt_handle(), 1, SQL_C_CHAR, tableCat, sizeof(tableCat), nullptr);
    SQLGetData(stmt_handle(), 2, SQL_C_CHAR, tableSchem, sizeof(tableSchem), nullptr);
    SQLGetData(stmt_handle(), 3, SQL_C_CHAR, tableName, sizeof(tableName), nullptr);
    SQLGetData(stmt_handle(), 4, SQL_C_CHAR, columnName, sizeof(columnName), nullptr);

    REQUIRE(std::string(tableCat) == database_name());
    REQUIRE(std::string(tableSchem) == schema_name());
    REQUIRE(std::string(tableName) == readonly_db::MULTI_TYPE_TABLE);

    columnNames.emplace_back(columnName);
  }

  REQUIRE(columnNames.size() == 4);
  REQUIRE(columnNames[0] == "ID");
  REQUIRE(columnNames[1] == "NAME");
  REQUIRE(columnNames[2] == "PRICE");
  REQUIRE(columnNames[3] == "ACTIVE");
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumns: Returns correct data types for known columns",
                 "[odbc-api][columns][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                             sqlchar(readonly_db::BASIC_TABLE), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  int rowCount = 0;
  while (true) {
    ret = SQLFetch(stmt_handle());
    if (ret == SQL_NO_DATA) break;
    REQUIRE(ret == SQL_SUCCESS);

    char columnName[256] = {};
    SQLSMALLINT dataType = 0;
    char typeName[256] = {};
    SQLLEN dataTypeInd = 0;

    SQLGetData(stmt_handle(), 4, SQL_C_CHAR, columnName, sizeof(columnName), nullptr);
    SQLGetData(stmt_handle(), 5, SQL_C_SSHORT, &dataType, 0, &dataTypeInd);
    SQLGetData(stmt_handle(), 6, SQL_C_CHAR, typeName, sizeof(typeName), nullptr);

    if (rowCount == 0) {
      REQUIRE(std::string(columnName) == "ID");
      REQUIRE(dataType == 3);
      REQUIRE(std::string(typeName) == "DECIMAL");
    } else if (rowCount == 1) {
      REQUIRE(std::string(columnName) == "NAME");
      REQUIRE(dataType == 12);
      REQUIRE(std::string(typeName) == "VARCHAR");
    }

    rowCount++;
  }

  REQUIRE(rowCount == 2);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumns: ORDINAL_POSITION is sequential starting from 1",
                 "[odbc-api][columns][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                             sqlchar(readonly_db::THREE_COL_TABLE), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  for (int i = 1; i <= 3; i++) {
    ret = SQLFetch(stmt_handle());
    REQUIRE(ret == SQL_SUCCESS);

    SQLINTEGER ordinalPos = 0;
    SQLGetData(stmt_handle(), 17, SQL_C_SLONG, &ordinalPos, 0, nullptr);
    REQUIRE(ordinalPos == i);
  }

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumns: NULLABLE column reports correct nullability",
                 "[odbc-api][columns][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                             sqlchar(readonly_db::NULLABILITY_TABLE), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // id INTEGER NOT NULL
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  SQLSMALLINT nullable1 = -1;
  SQLGetData(stmt_handle(), 11, SQL_C_SSHORT, &nullable1, 0, nullptr);
  REQUIRE(nullable1 == SQL_NO_NULLS);

  // name VARCHAR(100) - nullable by default
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  SQLSMALLINT nullable2 = -1;
  SQLGetData(stmt_handle(), 11, SQL_C_SSHORT, &nullable2, 0, nullptr);
  REQUIRE(nullable2 == SQL_NULLABLE);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

// ============================================================================
// SQLColumns - Search Patterns
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumns: ColumnName wildcard % returns all columns",
                 "[odbc-api][columns][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                             sqlchar(readonly_db::THREE_COL_TABLE), SQL_NTS, sqlchar("%"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  int rowCount = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS) {
    rowCount++;
  }
  REQUIRE(rowCount == 3);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumns: NULL ColumnName returns all columns",
                 "[odbc-api][columns][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                             sqlchar(readonly_db::BASIC_TABLE), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  int rowCount = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS) {
    rowCount++;
  }
  REQUIRE(rowCount == 2);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumns: Specific ColumnName filters results",
                 "[odbc-api][columns][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                             sqlchar(readonly_db::MULTI_TYPE_TABLE), SQL_NTS, sqlchar("NAME"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  char columnName[256] = {};
  SQLGetData(stmt_handle(), 4, SQL_C_CHAR, columnName, sizeof(columnName), nullptr);
  REQUIRE(std::string(columnName) == "NAME");

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumns: Underscore _ wildcard matches single character",
                 "[odbc-api][columns][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                             sqlchar(readonly_db::WILDCARD_COL_TABLE), SQL_NTS, sqlchar("C_"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // C_ matches CA and CB but not DDD
  char colName[256] = {};

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  SQLGetData(stmt_handle(), 4, SQL_C_CHAR, colName, sizeof(colName), nullptr);
  REQUIRE(std::string(colName) == "CA");

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  SQLGetData(stmt_handle(), 4, SQL_C_CHAR, colName, sizeof(colName), nullptr);
  REQUIRE(std::string(colName) == "CB");

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumns: Non-existent table returns empty result set",
                 "[odbc-api][columns][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                             sqlchar("NONEXISTENTTABLEXYZ12345"), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

// ============================================================================
// SQLColumns - Parameter Variations
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumns: Various parameter combinations are accepted",
                 "[odbc-api][columns][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const char* db = database_name();
  const char* schema = schema_name();
  const char* table = readonly_db::BASIC_TABLE;

  // Explicit catalog and schema with SQL_NTS
  SQLRETURN ret =
      SQLColumns(stmt_handle(), sqlchar(db), SQL_NTS, sqlchar(schema), SQL_NTS, sqlchar(table), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);
  int count1 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count1++;
  REQUIRE(count1 == 2);
  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // Explicit string lengths instead of SQL_NTS
  ret = SQLColumns(stmt_handle(), sqlchar(db), static_cast<SQLSMALLINT>(std::strlen(db)), sqlchar(schema),
                   static_cast<SQLSMALLINT>(std::strlen(schema)), sqlchar(table),
                   static_cast<SQLSMALLINT>(std::strlen(table)), nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);
  int count2 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count2++;
  REQUIRE(count2 == 2);
}

// ============================================================================
// SQLColumns - Statement Reuse
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumns: Can call multiple times on same statement after close cursor",
                 "[odbc-api][columns][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                             sqlchar(readonly_db::NAMED_PK_TABLE), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  int count1 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count1++;
  REQUIRE(count1 == 1);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                   sqlchar(readonly_db::NAMED_PK_TABLE), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  int count2 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count2++;
  REQUIRE(count2 == 1);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLColumns: SQLRowCount after catalog function call",
                 "[odbc-api][columns][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLColumns(stmt_handle(), nullptr, 0, nullptr, 0, sqlchar("DATABASES"), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // SQLRowCount is undefined for catalog functions, reference driver returns -1
  SQLLEN rowCount = 0;
  ret = SQLRowCount(stmt_handle(), &rowCount);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(rowCount == -1);
}

// ============================================================================
// SQLColumns - Error Cases
// ============================================================================

TEST_CASE("SQLColumns: SQL_INVALID_HANDLE for null statement handle", "[odbc-api][columns][catalog][error]") {
  const SQLRETURN ret = SQLColumns(SQL_NULL_HSTMT, nullptr, 0, nullptr, 0, sqlchar("TABLE"), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLColumns: HY090 - Negative CatalogName length",
                 "[odbc-api][columns][catalog][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret =
      SQLColumns(stmt_handle(), sqlchar("SNOWFLAKE"), -999, nullptr, 0, sqlchar("TABLE"), SQL_NTS, nullptr, 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLColumns: HY090 - Negative SchemaName length",
                 "[odbc-api][columns][catalog][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLColumns(stmt_handle(), nullptr, 0, sqlchar("SCHEMA"), -999, sqlchar("TABLE"), SQL_NTS, nullptr, 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLColumns: HY090 - Negative TableName length",
                 "[odbc-api][columns][catalog][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLColumns(stmt_handle(), nullptr, 0, nullptr, 0, sqlchar("TABLE"), -999, nullptr, 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLColumns: HY090 - Negative ColumnName length",
                 "[odbc-api][columns][catalog][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLColumns(stmt_handle(), nullptr, 0, nullptr, 0, sqlchar("TABLE"), SQL_NTS, sqlchar("COLUMN"), -999);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLColumns: 24000 - Cursor already open",
                 "[odbc-api][columns][catalog][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLColumns(stmt_handle(), nullptr, 0, nullptr, 0, sqlchar("DATABASES"), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Second call without closing cursor
  ret = SQLColumns(stmt_handle(), nullptr, 0, nullptr, 0, sqlchar("DATABASES"), SQL_NTS, nullptr, 0);
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(DbcFixture, "SQLColumns: Requires active connection", "[odbc-api][columns][catalog][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHSTMT stmt = SQL_NULL_HSTMT;
  const SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);

  // Note: Reference driver refuses to allocate statement on disconnected handle
  REQUIRE(ret == SQL_ERROR);
}
