#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <algorithm>
#include <cctype>
#include <cstring>
#include <map>
#include <string>
#include <utility>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "ODBCFixtures.hpp"
#include "ReadOnlyDbFixture.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

// ---------------------------------------------------------------------------
// Helper: read a CHAR column from a SQLColumns result set with proper hygiene:
//   0xFF sentinel fill, named indicator, hard assertion on SQLGetData return.
// ---------------------------------------------------------------------------
struct ColumnValue {
  std::string text;
  SQLLEN indicator = 0;
  bool is_null() const { return indicator == SQL_NULL_DATA; }
  bool is_present() const { return indicator > 0 || indicator == SQL_NTS; }
};

static ColumnValue sqlcolumns_get_column(SQLHSTMT stmt, SQLUSMALLINT column) {
  char buf[1024];
  std::memset(buf, 0xFF, sizeof(buf));
  SQLLEN indicator = 0;
  const SQLRETURN ret = SQLGetData(stmt, column, SQL_C_CHAR, buf, sizeof(buf), &indicator);
  REQUIRE(ret == SQL_SUCCESS);
  if (indicator == SQL_NULL_DATA) {
    return {std::string(), indicator};
  }
  return {std::string(buf), indicator};
}

static std::string to_lower_copy(const std::string& s) {
  std::string out = s;
  std::transform(out.begin(), out.end(), out.begin(), [](unsigned char c) { return std::tolower(c); });
  return out;
}

// ============================================================================
// SQLColumns - Result Set Structure
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLColumns: Result set has correct number of columns",
                 "[odbc-api][columns][catalog]") {
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

// SNOW-3897864 / BD#117: IRD concise types of the SQLColumns result set itself
// (not the DATA_TYPE / TYPE_NAME cell values describing user table columns).
// Match the reference driver catalog IRD: string cols = SQL_WVARCHAR; numerics =
// SMALLINT / INTEGER (NUM_PREC_RADIX is INTEGER on the reference driver).
TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLColumns: result-set IRD concise types match reference driver",
                 "[odbc-api][columns][catalog]") {
  SQLRETURN ret = SQLColumns(stmt_handle(), nullptr, 0, nullptr, 0, sqlchar("DATABASES"), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // 1-based column index → expected SQL_DESC_CONCISE_TYPE.
  const std::pair<SQLSMALLINT, SQLSMALLINT> expectedTypes[] = {
      {1, SQL_WVARCHAR},   // TABLE_CAT
      {2, SQL_WVARCHAR},   // TABLE_SCHEM
      {3, SQL_WVARCHAR},   // TABLE_NAME
      {4, SQL_WVARCHAR},   // COLUMN_NAME
      {5, SQL_SMALLINT},   // DATA_TYPE
      {6, SQL_WVARCHAR},   // TYPE_NAME
      {7, SQL_INTEGER},    // COLUMN_SIZE
      {8, SQL_INTEGER},    // BUFFER_LENGTH
      {9, SQL_SMALLINT},   // DECIMAL_DIGITS
      {10, SQL_INTEGER},   // NUM_PREC_RADIX (reference driver INTEGER)
      {11, SQL_SMALLINT},  // NULLABLE
      {12, SQL_WVARCHAR},  // REMARKS
      {13, SQL_WVARCHAR},  // COLUMN_DEF
      {14, SQL_SMALLINT},  // SQL_DATA_TYPE
      {15, SQL_SMALLINT},  // SQL_DATETIME_SUB
      {16, SQL_INTEGER},   // CHAR_OCTET_LENGTH
      {17, SQL_INTEGER},   // ORDINAL_POSITION
      {18, SQL_WVARCHAR},  // IS_NULLABLE
      {19, SQL_SMALLINT},  // USER_DATA_TYPE
  };

  for (const auto& [col, expected] : expectedTypes) {
    INFO("col " << col << " expected=" << expected);
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    ret = SQLColAttribute(stmt_handle(), col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(numAttr == expected);

    char colName[256] = {};
    SQLSMALLINT nameLen = 0;
    SQLSMALLINT dataType = 0x7FFF;
    SQLULEN colSize = 0;
    SQLSMALLINT decDigits = 0;
    SQLSMALLINT nullable = 0;
    ret = SQLDescribeCol(stmt_handle(), col, reinterpret_cast<SQLCHAR*>(colName), sizeof(colName), &nameLen, &dataType,
                         &colSize, &decDigits, &nullable);
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(dataType == expected);
  }
}

// ============================================================================
// SQLColumns - Data Verification
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumns: Returns correct column metadata for known table",
                 "[odbc-api][columns][catalog]") {
  SQLRETURN ret = SQLColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                             sqlchar(readonly_db::MULTI_TYPE_TABLE), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  std::vector<std::string> columnNames;
  while (true) {
    ret = SQLFetch(stmt_handle());
    if (ret == SQL_NO_DATA) break;
    REQUIRE(ret == SQL_SUCCESS);

    const auto tableCat = sqlcolumns_get_column(stmt_handle(), 1);
    const auto tableSchem = sqlcolumns_get_column(stmt_handle(), 2);
    const auto tableName = sqlcolumns_get_column(stmt_handle(), 3);
    const auto columnName = sqlcolumns_get_column(stmt_handle(), 4);

    REQUIRE(tableCat.text == database_name());
    REQUIRE(tableSchem.text == schema_name());
    REQUIRE(tableName.text == readonly_db::MULTI_TYPE_TABLE);

    columnNames.emplace_back(columnName.text);
  }

  REQUIRE(columnNames.size() == 4);
  REQUIRE(columnNames[0] == "ID");
  REQUIRE(columnNames[1] == "NAME");
  REQUIRE(columnNames[2] == "PRICE");
  REQUIRE(columnNames[3] == "ACTIVE");
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumns: Returns correct data types for known columns",
                 "[odbc-api][columns][catalog]") {
  SQLRETURN ret = SQLColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                             sqlchar(readonly_db::BASIC_TABLE), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  int rowCount = 0;
  while (true) {
    ret = SQLFetch(stmt_handle());
    if (ret == SQL_NO_DATA) break;
    REQUIRE(ret == SQL_SUCCESS);

    const auto columnName = sqlcolumns_get_column(stmt_handle(), 4);
    SQLSMALLINT dataType = 0;
    SQLLEN dataTypeInd = 0;
    REQUIRE(SQLGetData(stmt_handle(), 5, SQL_C_SSHORT, &dataType, 0, &dataTypeInd) == SQL_SUCCESS);
    REQUIRE(dataTypeInd == sizeof(SQLSMALLINT));
    const auto typeName = sqlcolumns_get_column(stmt_handle(), 6);

    if (rowCount == 0) {
      REQUIRE(columnName.text == "ID");
      REQUIRE(dataType == SQL_DECIMAL);
      REQUIRE(typeName.text == "DECIMAL");
    } else if (rowCount == 1) {
      REQUIRE(columnName.text == "NAME");
      REQUIRE(dataType == SQL_VARCHAR);
      REQUIRE(typeName.text == "VARCHAR");
    }

    rowCount++;
  }

  REQUIRE(rowCount == 2);
}

// SNOW-3899531: SQLColumns col 6 (TYPE_NAME) is a catalog cell value that must
// report Snowflake external / friendly names (BOOLEAN, TIMESTAMP, VARIANT,
// STRUCT, ARRAY, GEOGRAPHY, …), matching the reference driver — NOT the SDK
// labels (BIT / TYPE_DATE / TYPE_TIMESTAMP) that SQLColAttribute reports on
// query columns (covered by e2e/query/sql_col_attribute.cpp; must stay as-is).
// Regression guard: semi-structured types must not collapse to VARCHAR, and
// GEOGRAPHY must not come back NULL.
TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumns: TYPE_NAME matches reference driver for each Snowflake type",
                 "[odbc-api][columns][catalog]") {
  // ALLDATATYPES lives in the second schema (DATATYPETESTS), not the
  // connection's default schema, so pass it explicitly.
  SQLRETURN ret = SQLColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(READONLY_SECOND_SCHEMA_NAME),
                             SQL_NTS, sqlchar(readonly_db::SECOND_SCHEMA_TABLE), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // COLUMN_NAME -> external TYPE_NAME expected on the NEW driver, mirroring OLD.
  const std::map<std::string, std::string> expected = {
      {"ROWKIND", "VARCHAR"},    {"INTVAL", "DECIMAL"},     {"BIGINTVAL", "DECIMAL"},   {"SMALLINTVAL", "DECIMAL"},
      {"TINYINTVAL", "DECIMAL"}, {"NUM38", "DECIMAL"},      {"NUM18S6", "DECIMAL"},     {"FLOATVAL", "DOUBLE"},
      {"DOUBLEVAL", "DOUBLE"},   {"REALVAL", "DOUBLE"},     {"VARCHARVAL", "VARCHAR"},  {"TEXTVAL", "VARCHAR"},
      {"CHARVAL", "VARCHAR"},    {"BINARYVAL", "BINARY"},   {"VARBINARYVAL", "BINARY"}, {"BOOLVAL", "BOOLEAN"},
      {"DATEVAL", "DATE"},       {"TIMEVAL", "TIME"},       {"TSNTZ", "TIMESTAMP"},     {"TSLTZ", "TIMESTAMP"},
      {"TSTZ", "TIMESTAMP"},     {"VARIANTVAL", "VARIANT"}, {"OBJECTVAL", "STRUCT"},    {"ARRAYVAL", "ARRAY"},
      {"GEOVAL", "GEOGRAPHY"},
  };

  std::map<std::string, std::string> actual;
  while (true) {
    ret = SQLFetch(stmt_handle());
    if (ret == SQL_NO_DATA) break;
    REQUIRE(ret == SQL_SUCCESS);

    const auto columnName = sqlcolumns_get_column(stmt_handle(), 4);
    const auto typeName = sqlcolumns_get_column(stmt_handle(), 6);
    // Every column has a data-source type name; none should be NULL (the
    // GEOGRAPHY regression this test guards against).
    REQUIRE_FALSE(typeName.is_null());
    actual.emplace(columnName.text, typeName.text);
  }

  for (const auto& [column, wantType] : expected) {
    const auto it = actual.find(column);
    REQUIRE(it != actual.end());
    INFO("column " << column);
    CHECK(it->second == wantType);
  }
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumns: ORDINAL_POSITION is sequential starting from 1",
                 "[odbc-api][columns][catalog]") {
  SQLRETURN ret = SQLColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                             sqlchar(readonly_db::THREE_COL_TABLE), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  for (int i = 1; i <= 3; i++) {
    ret = SQLFetch(stmt_handle());
    REQUIRE(ret == SQL_SUCCESS);

    SQLINTEGER ordinalPos = -1;
    SQLLEN ordInd = 0;
    REQUIRE(SQLGetData(stmt_handle(), 17, SQL_C_SLONG, &ordinalPos, 0, &ordInd) == SQL_SUCCESS);
    REQUIRE(ordInd == sizeof(SQLINTEGER));
    REQUIRE(ordinalPos == i);
  }

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumns: NULLABLE column reports correct nullability",
                 "[odbc-api][columns][catalog]") {
  SQLRETURN ret = SQLColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                             sqlchar(readonly_db::NULLABILITY_TABLE), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // id INTEGER NOT NULL
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  SQLSMALLINT nullable1 = -1;
  SQLLEN nullable1Ind = 0;
  REQUIRE(SQLGetData(stmt_handle(), 11, SQL_C_SSHORT, &nullable1, 0, &nullable1Ind) == SQL_SUCCESS);
  REQUIRE(nullable1Ind == sizeof(SQLSMALLINT));
  REQUIRE(nullable1 == SQL_NO_NULLS);

  // name VARCHAR(100) - nullable by default
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  SQLSMALLINT nullable2 = -1;
  SQLLEN nullable2Ind = 0;
  REQUIRE(SQLGetData(stmt_handle(), 11, SQL_C_SSHORT, &nullable2, 0, &nullable2Ind) == SQL_SUCCESS);
  REQUIRE(nullable2Ind == sizeof(SQLSMALLINT));
  REQUIRE(nullable2 == SQL_NULLABLE);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

// ============================================================================
// SQLColumns - Search Patterns
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumns: ColumnName wildcard % returns all columns",
                 "[odbc-api][columns][catalog]") {
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
  SQLRETURN ret = SQLColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                             sqlchar(readonly_db::MULTI_TYPE_TABLE), SQL_NTS, sqlchar("NAME"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  const auto columnName = sqlcolumns_get_column(stmt_handle(), 4);
  REQUIRE(columnName.text == "NAME");

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumns: Underscore _ wildcard matches single character",
                 "[odbc-api][columns][catalog]") {
  SQLRETURN ret = SQLColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                             sqlchar(readonly_db::WILDCARD_COL_TABLE), SQL_NTS, sqlchar("C_"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // C_ matches CA and CB but not DDD
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(sqlcolumns_get_column(stmt_handle(), 4).text == "CA");

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(sqlcolumns_get_column(stmt_handle(), 4).text == "CB");

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumns: Non-existent table returns empty result set",
                 "[odbc-api][columns][catalog]") {
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
  SQLRETURN ret =
      SQLColumns(stmt_handle(), sqlchar("SNOWFLAKE"), -999, nullptr, 0, sqlchar("TABLE"), SQL_NTS, nullptr, 0);
  IODBC_ONLY {
    // iODBC's DM-side length validator rejects the negative length with the
    //   ODBC 2.x form of HY090 ("S1090") before the call reaches the driver.
    //   Exactly one record is posted on the SQL_HANDLE_STMT handle.
    REQUIRE(ret == SQL_ERROR);
    auto records = get_diag_rec(SQL_HANDLE_STMT, stmt_handle());
    REQUIRE(records.size() == 1);
    REQUIRE(records[0].sqlState == "S1090");
  }
  else {
    REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
  }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLColumns: HY090 - Negative SchemaName length",
                 "[odbc-api][columns][catalog][error]") {
  SQLRETURN ret = SQLColumns(stmt_handle(), nullptr, 0, sqlchar("SCHEMA"), -999, sqlchar("TABLE"), SQL_NTS, nullptr, 0);
  IODBC_ONLY {
    // iODBC's DM-side length validator rejects the negative length with the
    //   ODBC 2.x form of HY090 ("S1090") before the call reaches the driver.
    //   Exactly one record is posted on the SQL_HANDLE_STMT handle.
    REQUIRE(ret == SQL_ERROR);
    auto records = get_diag_rec(SQL_HANDLE_STMT, stmt_handle());
    REQUIRE(records.size() == 1);
    REQUIRE(records[0].sqlState == "S1090");
  }
  else {
    REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
  }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLColumns: HY090 - Negative TableName length",
                 "[odbc-api][columns][catalog][error]") {
  SQLRETURN ret = SQLColumns(stmt_handle(), nullptr, 0, nullptr, 0, sqlchar("TABLE"), -999, nullptr, 0);
  IODBC_ONLY {
    // iODBC's DM-side length validator rejects the negative length with the
    //   ODBC 2.x form of HY090 ("S1090") before the call reaches the driver.
    //   Exactly one record is posted on the SQL_HANDLE_STMT handle.
    REQUIRE(ret == SQL_ERROR);
    auto records = get_diag_rec(SQL_HANDLE_STMT, stmt_handle());
    REQUIRE(records.size() == 1);
    REQUIRE(records[0].sqlState == "S1090");
  }
  else {
    REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
  }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLColumns: HY090 - Negative ColumnName length",
                 "[odbc-api][columns][catalog][error]") {
  SQLRETURN ret = SQLColumns(stmt_handle(), nullptr, 0, nullptr, 0, sqlchar("TABLE"), SQL_NTS, sqlchar("COLUMN"), -999);
  IODBC_ONLY {
    // iODBC's DM-side length validator rejects the negative length with the
    //   ODBC 2.x form of HY090 ("S1090") before the call reaches the driver.
    //   Exactly one record is posted on the SQL_HANDLE_STMT handle.
    REQUIRE(ret == SQL_ERROR);
    auto records = get_diag_rec(SQL_HANDLE_STMT, stmt_handle());
    REQUIRE(records.size() == 1);
    REQUIRE(records[0].sqlState == "S1090");
  }
  else {
    REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
  }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLColumns: 24000 - Cursor already open",
                 "[odbc-api][columns][catalog][error]") {
  SQLRETURN ret = SQLColumns(stmt_handle(), nullptr, 0, nullptr, 0, sqlchar("DATABASES"), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Second call without closing cursor
  ret = SQLColumns(stmt_handle(), nullptr, 0, nullptr, 0, sqlchar("DATABASES"), SQL_NTS, nullptr, 0);
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(DbcFixture, "SQLColumns: Requires active connection", "[odbc-api][columns][catalog][error]") {
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  const SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);

  // Note: Reference driver refuses to allocate statement on disconnected handle
  REQUIRE(ret == SQL_ERROR);
}

// ============================================================================
// SQLColumns - SQL_ATTR_METADATA_ID (identifier vs pattern mode)
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumns: metadata_id=TRUE treats _ and % as literals in column name",
                 "[odbc-api][columns][catalog]") {
  // Given SQL_ATTR_METADATA_ID is enabled (identifier mode)
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQLColumns is called with the exact column name
  ret = SQLColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                   sqlchar(readonly_db::MULTI_TYPE_TABLE), SQL_NTS, sqlchar("ID"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // Then the exact column is returned (metadata_id forces exact match)
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(sqlcolumns_get_column(stmt_handle(), 4).text == "ID");

  ret = SQLFetch(stmt_handle());
  CHECK(ret == SQL_NO_DATA);
  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // And a % that would match ID in pattern mode is treated as a literal → no rows
  ret = SQLColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                   sqlchar(readonly_db::MULTI_TYPE_TABLE), SQL_NTS, sqlchar("I%"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt_handle());
  CHECK(ret == SQL_NO_DATA);
  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // And a _ that would match ID in pattern mode is treated as a literal → no rows
  ret = SQLColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                   sqlchar(readonly_db::MULTI_TYPE_TABLE), SQL_NTS, sqlchar("_D"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt_handle());
  CHECK(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumns: metadata_id=TRUE with NULL CatalogName returns HY009",
                 "[odbc-api][columns][catalog][error]") {
  // Given SQL_ATTR_METADATA_ID is enabled
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQLColumns is called with NULL CatalogName (identifier required)
  ret = SQLColumns(stmt_handle(), nullptr, 0, sqlchar(schema_name()), SQL_NTS, sqlchar(readonly_db::MULTI_TYPE_TABLE),
                   SQL_NTS, sqlchar("ID"), SQL_NTS);

  // Then HY009 (Invalid use of null pointer) is returned
  REQUIRE_EXPECTED_ERROR(ret, "HY009", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumns: metadata_id=TRUE with NULL SchemaName returns HY009",
                 "[odbc-api][columns][catalog][error]") {
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, nullptr, 0, sqlchar(readonly_db::MULTI_TYPE_TABLE),
                   SQL_NTS, sqlchar("ID"), SQL_NTS);

  REQUIRE_EXPECTED_ERROR(ret, "HY009", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumns: metadata_id=TRUE with NULL TableName returns HY009",
                 "[odbc-api][columns][catalog][error]") {
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS, nullptr, 0,
                   sqlchar("ID"), SQL_NTS);

  REQUIRE_EXPECTED_ERROR(ret, "HY009", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumns: metadata_id=TRUE with NULL ColumnName returns HY009",
                 "[odbc-api][columns][catalog][error]") {
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                   sqlchar(readonly_db::MULTI_TYPE_TABLE), SQL_NTS, nullptr, 0);

  REQUIRE_EXPECTED_ERROR(ret, "HY009", stmt_handle(), SQL_HANDLE_STMT);
}

// Identifier mode folds unquoted identifiers to uppercase, so a lowercase
// ColumnName must still match the uppercase stored name. Catalog/schema/table
// stay canonical uppercase so this isolates the column-fold path. Both the
// universal driver and the legacy driver fold ColumnName here (unlike
// SQLTables TableName, where legacy stays case-sensitive — see BD#113).
TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumns: metadata_id=TRUE matches unquoted ColumnName case-insensitively",
                 "[odbc-api][columns][catalog]") {
  // Given SQL_ATTR_METADATA_ID is enabled (identifier mode)
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  const std::string col = to_lower_copy("ID");

  // When SQLColumns is called with a lowercase unquoted ColumnName only
  ret = SQLColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                   sqlchar(readonly_db::MULTI_TYPE_TABLE), SQL_NTS, sqlchar(col.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // Then the uppercase column is returned on both drivers
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(sqlcolumns_get_column(stmt_handle(), 4).text == "ID");

  ret = SQLFetch(stmt_handle());
  CHECK(ret == SQL_NO_DATA);
}

// In pattern mode (default) a lowercase ColumnName must NOT match the uppercase name.
TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumns: metadata_id=FALSE treats ColumnName case-sensitively",
                 "[odbc-api][columns][catalog]") {
  // Given a lowercase ColumnName pattern while METADATA_ID is SQL_FALSE (default)
  const std::string col = to_lower_copy("ID");

  // When SQLColumns is called in pattern mode
  SQLRETURN ret = SQLColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                             sqlchar(readonly_db::MULTI_TYPE_TABLE), SQL_NTS, sqlchar(col.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // Then no rows are returned (case is significant)
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

// In pattern mode (default) a lowercase TableName must NOT match the uppercase name.
TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLColumns: metadata_id=FALSE treats TableName case-sensitively",
                 "[odbc-api][columns][catalog]") {
  // Given a lowercase TableName pattern while METADATA_ID is SQL_FALSE (default)
  const std::string tbl = to_lower_copy(readonly_db::MULTI_TYPE_TABLE);

  // When SQLColumns is called in pattern mode
  SQLRETURN ret = SQLColumns(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                             sqlchar(tbl.c_str()), SQL_NTS, sqlchar("ID"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // Then no rows are returned (case is significant)
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}
