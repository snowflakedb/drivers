#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <set>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "ODBCFixtures.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "test_macros.hpp"

// ============================================================================
// Comprehensive Type Information - All Supported Types
// ============================================================================

struct TypeInfoExpected {
  SQLSMALLINT sqlType;
  const char* typeName;
  SQLINTEGER columnSize;
  const char* literalPrefix;
  SQLLEN literalPrefixIndicator;
  const char* literalSuffix;
  SQLLEN literalSuffixIndicator;
  const char* createParams;
  SQLLEN createParamsIndicator;
  SQLSMALLINT nullable;
  SQLSMALLINT caseSensitive;
  SQLSMALLINT searchable;
  SQLSMALLINT unsignedAttr;
  SQLLEN unsignedAttrIndicator;
  SQLSMALLINT fixedPrecScale;
  SQLSMALLINT autoUniqueValue;
  SQLLEN autoUniqueValueIndicator;
  const char* localTypeName;
  SQLLEN localTypeNameIndicator;
  SQLSMALLINT minimumScale;
  SQLLEN minimumScaleIndicator;
  SQLSMALLINT maximumScale;
  SQLLEN maximumScaleIndicator;
  SQLSMALLINT sqlDataType;
  SQLSMALLINT sqlDatetimeSub;
  SQLLEN sqlDatetimeSubIndicator;
  SQLINTEGER numPrecRadix;
  SQLLEN numPrecRadixIndicator;
  SQLINTEGER intervalPrecision;
  SQLLEN intervalPrecisionIndicator;
  SQLSMALLINT userDataType;
  SQLLEN userDataTypeIndicator;
};

static const TypeInfoExpected ALL_TYPE_INFO[] = {
    // Character Types
    {SQL_CHAR,
     "CHAR",
     134217728,
     "'",
     1,
     "'",
     1,
     "LENGTH",
     6,
     SQL_NULLABLE,
     SQL_TRUE,
     SQL_SEARCHABLE,
     0,
     SQL_NULL_DATA,
     SQL_FALSE,
     0,
     SQL_NULL_DATA,
     "CHAR",
     4,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     SQL_CHAR,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     0,
     2},
    {SQL_VARCHAR,
     "VARCHAR",
     134217728,
     "'",
     1,
     "'",
     1,
     "max length",
     10,
     SQL_NULLABLE,
     SQL_TRUE,
     SQL_SEARCHABLE,
     0,
     SQL_NULL_DATA,
     SQL_FALSE,
     0,
     SQL_NULL_DATA,
     "VARCHAR",
     7,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     SQL_VARCHAR,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     0,
     2},
    {SQL_WCHAR,
     "CHAR",
     134217728,
     "'",
     1,
     "'",
     1,
     "LENGTH",
     6,
     SQL_NULLABLE,
     SQL_TRUE,
     SQL_SEARCHABLE,
     0,
     SQL_NULL_DATA,
     SQL_FALSE,
     0,
     SQL_NULL_DATA,
     "WCHAR",
     5,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     SQL_WCHAR,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     0,
     2},
    {SQL_WVARCHAR,
     "VARCHAR",
     134217728,
     "'",
     1,
     "'",
     1,
     "LENGTH",
     6,
     SQL_NULLABLE,
     SQL_TRUE,
     SQL_SEARCHABLE,
     0,
     SQL_NULL_DATA,
     SQL_FALSE,
     0,
     SQL_NULL_DATA,
     "WVARCHAR",
     8,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     SQL_WVARCHAR,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     0,
     2},

    // Exact Numeric Types (Note: buffer contents undefined when indicator is SQL_NULL_DATA)
    {SQL_DECIMAL,
     "DECIMAL",
     38,
     "",
     SQL_NULL_DATA,
     "",
     SQL_NULL_DATA,
     "precision,scale",
     15,
     SQL_NULLABLE,
     SQL_FALSE,
     SQL_PRED_BASIC,
     SQL_FALSE,
     2,
     SQL_FALSE,
     2,
     SQL_NULL_DATA,
     "DECIMAL",
     7,
     0,
     2,
     38,
     2,
     SQL_DECIMAL,
     0,
     SQL_NULL_DATA,
     10,
     4,
     0,
     SQL_NULL_DATA,
     0,
     2},
    {SQL_NUMERIC,
     "NUMERIC",
     38,
     "",
     SQL_NULL_DATA,
     "",
     SQL_NULL_DATA,
     "precision,scale",
     15,
     SQL_NULLABLE,
     SQL_FALSE,
     SQL_PRED_BASIC,
     SQL_FALSE,
     2,
     SQL_FALSE,
     2,
     SQL_NULL_DATA,
     "NUMERIC",
     7,
     0,
     2,
     38,
     2,
     SQL_NUMERIC,
     0,
     SQL_NULL_DATA,
     10,
     4,
     0,
     SQL_NULL_DATA,
     0,
     2},
    {SQL_INTEGER,
     "INTEGER",
     10,
     "",
     SQL_NULL_DATA,
     "",
     SQL_NULL_DATA,
     "",
     SQL_NULL_DATA,
     SQL_NULLABLE,
     SQL_FALSE,
     SQL_PRED_BASIC,
     SQL_FALSE,
     2,
     SQL_FALSE,
     2,
     SQL_NULL_DATA,
     "INTEGER",
     7,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     SQL_INTEGER,
     0,
     SQL_NULL_DATA,
     2,
     4,
     0,
     SQL_NULL_DATA,
     0,
     2},
    {SQL_BIGINT,
     "BIGINT",
     19,
     "",
     SQL_NULL_DATA,
     "",
     SQL_NULL_DATA,
     "",
     SQL_NULL_DATA,
     SQL_NULLABLE,
     SQL_FALSE,
     SQL_PRED_BASIC,
     SQL_FALSE,
     2,
     SQL_FALSE,
     2,
     SQL_NULL_DATA,
     "BIGINT",
     6,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     SQL_BIGINT,
     0,
     SQL_NULL_DATA,
     2,
     4,
     0,
     SQL_NULL_DATA,
     0,
     2},

    // Approximate Numeric Types (Note: buffer contents undefined when indicator is SQL_NULL_DATA)
    {SQL_REAL,
     "REAL",
     7,
     "",
     SQL_NULL_DATA,
     "",
     SQL_NULL_DATA,
     "",
     SQL_NULL_DATA,
     SQL_NULLABLE,
     SQL_FALSE,
     SQL_PRED_BASIC,
     SQL_FALSE,
     2,
     SQL_FALSE,
     2,
     SQL_NULL_DATA,
     "REAL",
     4,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     SQL_REAL,
     0,
     SQL_NULL_DATA,
     2,
     4,
     0,
     SQL_NULL_DATA,
     0,
     2},
    {SQL_FLOAT,
     "FLOAT",
     15,
     "",
     SQL_NULL_DATA,
     "",
     SQL_NULL_DATA,
     "",
     SQL_NULL_DATA,
     SQL_NULLABLE,
     SQL_FALSE,
     SQL_PRED_BASIC,
     SQL_FALSE,
     2,
     SQL_FALSE,
     2,
     SQL_NULL_DATA,
     "FLOAT",
     5,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     SQL_FLOAT,
     0,
     SQL_NULL_DATA,
     2,
     4,
     0,
     SQL_NULL_DATA,
     0,
     2},
    {SQL_DOUBLE,
     "DOUBLE",
     15,
     "",
     SQL_NULL_DATA,
     "",
     SQL_NULL_DATA,
     "",
     SQL_NULL_DATA,
     SQL_NULLABLE,
     SQL_FALSE,
     SQL_PRED_BASIC,
     SQL_FALSE,
     2,
     SQL_FALSE,
     2,
     SQL_NULL_DATA,
     "DOUBLE",
     6,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     SQL_DOUBLE,
     0,
     SQL_NULL_DATA,
     2,
     4,
     0,
     SQL_NULL_DATA,
     0,
     2},

    // Boolean Type (Note: buffer contents undefined when indicator is SQL_NULL_DATA)
    {SQL_BIT,
     "BOOLEAN",
     1,
     "",
     SQL_NULL_DATA,
     "",
     SQL_NULL_DATA,
     "",
     SQL_NULL_DATA,
     SQL_NULLABLE,
     SQL_FALSE,
     SQL_PRED_BASIC,
     0,
     SQL_NULL_DATA,
     SQL_FALSE,
     2,
     SQL_NULL_DATA,
     "BIT",
     3,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     SQL_BIT,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     0,
     2},

    // Binary Types
    {SQL_BINARY,
     "BINARY",
     67108864,
     "0x",
     2,
     "",
     0,
     "LENGTH",
     6,
     SQL_NULLABLE,
     SQL_TRUE,
     SQL_SEARCHABLE,
     0,
     SQL_NULL_DATA,
     SQL_FALSE,
     2,
     SQL_NULL_DATA,
     "BINARY",
     6,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     SQL_BINARY,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     0,
     2},
    {SQL_VARBINARY,
     "VARBINARY",
     67108864,
     "0x",
     2,
     "",
     0,
     "max length",
     10,
     SQL_NULLABLE,
     SQL_TRUE,
     SQL_SEARCHABLE,
     0,
     SQL_NULL_DATA,
     SQL_FALSE,
     2,
     SQL_NULL_DATA,
     "VARBINARY",
     9,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     SQL_VARBINARY,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     0,
     2},

    // Date/Time Types (Note: buffer contents undefined when indicator is SQL_NULL_DATA)
    {SQL_TYPE_DATE,
     "DATE",
     10,
     "'",
     1,
     "'",
     1,
     "",
     SQL_NULL_DATA,
     SQL_NULLABLE,
     SQL_FALSE,
     SQL_SEARCHABLE,
     0,
     SQL_NULL_DATA,
     SQL_FALSE,
     2,
     SQL_NULL_DATA,
     "TYPE_DATE",
     9,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     SQL_DATETIME,
     SQL_CODE_DATE,
     2,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     0,
     2},
    {SQL_TYPE_TIME,
     "TIME",
     18,
     "'",
     1,
     "'",
     1,
     "",
     SQL_NULL_DATA,
     SQL_NULLABLE,
     SQL_FALSE,
     SQL_SEARCHABLE,
     0,
     SQL_NULL_DATA,
     SQL_FALSE,
     2,
     SQL_NULL_DATA,
     "TYPE_TIME",
     9,
     0,
     2,
     0,
     2,
     SQL_DATETIME,
     SQL_CODE_TIME,
     2,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     0,
     2},
    {SQL_TYPE_TIMESTAMP,
     "TIMESTAMP",
     35,
     "'",
     1,
     "'",
     1,
     "",
     SQL_NULL_DATA,
     SQL_NULLABLE,
     SQL_FALSE,
     SQL_SEARCHABLE,
     0,
     SQL_NULL_DATA,
     SQL_FALSE,
     2,
     SQL_NULL_DATA,
     "TYPE_TIMESTAMP",
     14,
     0,
     2,
     0,
     2,
     SQL_DATETIME,
     SQL_CODE_TIMESTAMP,
     2,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     0,
     2},

    // Vendor-Specific Types (Note: buffer contents undefined when indicator is SQL_NULL_DATA)
    {2000,
     "TIMESTAMP_LTZ",
     35,
     "'",
     1,
     "'",
     1,
     "",
     SQL_NULL_DATA,
     SQL_NULLABLE,
     SQL_FALSE,
     SQL_SEARCHABLE,
     0,
     SQL_NULL_DATA,
     SQL_FALSE,
     2,
     SQL_NULL_DATA,
     "STAMP_LTZ",
     9,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     2000,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     0,
     2},
    {2001,
     "TIMESTAMP_TZ",
     35,
     "'",
     1,
     "'",
     1,
     "",
     SQL_NULL_DATA,
     SQL_NULLABLE,
     SQL_FALSE,
     SQL_SEARCHABLE,
     0,
     SQL_NULL_DATA,
     SQL_FALSE,
     2,
     SQL_NULL_DATA,
     "STAMP_TZ",
     8,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     2001,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     0,
     2},
    {2002,
     "TIMESTAMP_NTZ",
     35,
     "'",
     1,
     "'",
     1,
     "",
     SQL_NULL_DATA,
     SQL_NULLABLE,
     SQL_FALSE,
     SQL_SEARCHABLE,
     0,
     SQL_NULL_DATA,
     SQL_FALSE,
     2,
     SQL_NULL_DATA,
     "STAMP_NTZ",
     9,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     2002,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     0,
     2},
    {2003,
     "ARRAY",
     134217728,
     "'",
     1,
     "'",
     1,
     "max length",
     10,
     SQL_NULLABLE,
     SQL_TRUE,
     SQL_SEARCHABLE,
     0,
     SQL_NULL_DATA,
     SQL_FALSE,
     2,
     SQL_NULL_DATA,
     "OWN",
     3,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     2003,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     0,
     2},
    {2004,
     "OBJECT",
     134217728,
     "'",
     1,
     "'",
     1,
     "max length",
     10,
     SQL_NULLABLE,
     SQL_TRUE,
     SQL_SEARCHABLE,
     0,
     SQL_NULL_DATA,
     SQL_FALSE,
     2,
     SQL_NULL_DATA,
     "OWN",
     3,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     2004,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     0,
     2},
    {2005,
     "VARIANT",
     134217728,
     "'",
     1,
     "'",
     1,
     "max length",
     10,
     SQL_NULLABLE,
     SQL_TRUE,
     SQL_SEARCHABLE,
     0,
     SQL_NULL_DATA,
     SQL_FALSE,
     2,
     SQL_NULL_DATA,
     "OWN",
     3,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     2005,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     0,
     SQL_NULL_DATA,
     0,
     2},
};

// ============================================================================
// SQLGetTypeInfo - Basic Functionality
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetTypeInfo: Result set ordering when using SQL_ALL_TYPES",
                 "[odbc-api][gettypeinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLGetTypeInfo(stmt_handle(), SQL_ALL_TYPES);
  REQUIRE(ret == SQL_SUCCESS);

  std::vector<std::pair<SQLSMALLINT, std::string>> types;

  while ((ret = SQLFetch(stmt_handle())) == SQL_SUCCESS) {
    SQLSMALLINT dataType;
    char typeName[256];
    SQLLEN indicator;

    ret = SQLGetData(stmt_handle(), 2, SQL_C_SSHORT, &dataType, sizeof(dataType), &indicator);
    REQUIRE(ret == SQL_SUCCESS);

    ret = SQLGetData(stmt_handle(), 1, SQL_C_CHAR, typeName, sizeof(typeName), &indicator);
    REQUIRE(ret == SQL_SUCCESS);

    types.emplace_back(dataType, typeName);
  }
  REQUIRE(ret == SQL_NO_DATA);

  // Note: The reference driver does not sort types by data type, unlike in the ODBC spec
  REQUIRE(types.size() == 23);
  REQUIRE(types[0].first == SQL_CHAR);
  REQUIRE(types[1].first == SQL_NUMERIC);
  REQUIRE(types[2].first == SQL_DECIMAL);
  REQUIRE(types[3].first == SQL_INTEGER);
  REQUIRE(types[4].first == SQL_BIGINT);
  REQUIRE(types[5].first == SQL_FLOAT);
  REQUIRE(types[6].first == SQL_REAL);
  REQUIRE(types[7].first == SQL_DOUBLE);
  REQUIRE(types[8].first == SQL_VARCHAR);
  REQUIRE(types[9].first == SQL_BINARY);
  REQUIRE(types[10].first == SQL_VARBINARY);
  REQUIRE(types[11].first == SQL_TYPE_DATE);
  REQUIRE(types[12].first == SQL_TYPE_TIME);
  REQUIRE(types[13].first == 2000);  // TIMESTAMP_LTZ
  REQUIRE(types[14].first == 2002);  // TIMESTAMP_NTZ
  REQUIRE(types[15].first == 2001);  // TIMESTAMP_TZ
  REQUIRE(types[16].first == SQL_TYPE_TIMESTAMP);
  REQUIRE(types[17].first == 2003);  // ARRAY
  REQUIRE(types[18].first == 2004);  // OBJECT
  REQUIRE(types[19].first == 2005);  // VARIANT
  REQUIRE(types[20].first == SQL_WCHAR);
  REQUIRE(types[21].first == SQL_WVARCHAR);
  REQUIRE(types[22].first == SQL_BIT);
}

// ============================================================================
// SQLGetTypeInfo - Error Cases: Invalid Handle
// ============================================================================

TEST_CASE("SQLGetTypeInfo: SQL_INVALID_HANDLE - NULL statement handle", "[odbc-api][gettypeinfo][driver_info][error]") {
  const SQLRETURN ret = SQLGetTypeInfo(SQL_NULL_HSTMT, SQL_ALL_TYPES);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetTypeInfo: SQL_INVALID_HANDLE - Invalid handle type",
                 "[odbc-api][gettypeinfo][driver_info][error]") {
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(dsn_name().c_str())), SQL_NTS,
                             nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLGetTypeInfo(dbc_handle(), SQL_ALL_TYPES);
  REQUIRE(ret == SQL_INVALID_HANDLE);

  SQLDisconnect(dbc_handle());
}

// ============================================================================
// SQLGetTypeInfo - Error Cases: Invalid Parameters
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetTypeInfo: Returns empty result for invalid SQL data type",
                 "[odbc-api][gettypeinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLGetTypeInfo(stmt_handle(), 9999);

  // Note: Reference driver returns SUCCESS with empty result set (differs from ODBC spec)
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

// ============================================================================
// SQLGetTypeInfo - State Transition Tests
// ============================================================================

TEST_CASE_METHOD(DbcFixture, "SQLGetTypeInfo: Requires active connection",
                 "[odbc-api][gettypeinfo][driver_info][error]") {
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);

  // Note: Reference driver requires connection to allocate statement (differs from ODBC spec)
  if (ret == SQL_ERROR) {
    const auto records = get_diag_rec(SQL_HANDLE_DBC, dbc_handle());
    REQUIRE(!records.empty());
    return;
  }

  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLGetTypeInfo(stmt, SQL_ALL_TYPES);
  REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt, SQL_HANDLE_STMT);

  SQLFreeHandle(SQL_HANDLE_STMT, stmt);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetTypeInfo: Can be called multiple times on same statement",
                 "[odbc-api][gettypeinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLGetTypeInfo(stmt_handle(), SQL_VARCHAR);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLGetTypeInfo(stmt_handle(), SQL_INTEGER);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

// ============================================================================
// SQLGetTypeInfo - Result Set Column Tests
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetTypeInfo: Result set has correct columns",
                 "[odbc-api][gettypeinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLGetTypeInfo(stmt_handle(), SQL_ALL_TYPES);
  REQUIRE(ret == SQL_SUCCESS);

  // Note: Reference driver returns 20 columns instead of standard 19
  SQLSMALLINT numCols = 0;
  ret = SQLNumResultCols(stmt_handle(), &numCols);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(numCols == 20);

  char colName[256];
  SQLSMALLINT nameLen;
  SQLSMALLINT dataType;
  SQLULEN colSize;
  SQLSMALLINT decDigits;
  SQLSMALLINT nullable;

  const char* expectedColNames[] = {"TYPE_NAME",        "DATA_TYPE",          "COLUMN_SIZE",        "LITERAL_PREFIX",
                                    "LITERAL_SUFFIX",   "CREATE_PARAMS",      "NULLABLE",           "CASE_SENSITIVE",
                                    "SEARCHABLE",       "UNSIGNED_ATTRIBUTE", "FIXED_PREC_SCALE",   "AUTO_UNIQUE_VALUE",
                                    "LOCAL_TYPE_NAME",  "MINIMUM_SCALE",      "MAXIMUM_SCALE",      "SQL_DATA_TYPE",
                                    "SQL_DATETIME_SUB", "NUM_PREC_RADIX",     "INTERVAL_PRECISION", "USER_DATA_TYPE"};

  for (SQLSMALLINT col = 1; col <= numCols; col++) {
    ret = SQLDescribeCol(stmt_handle(), col, reinterpret_cast<SQLCHAR*>(colName), sizeof(colName), &nameLen, &dataType,
                         &colSize, &decDigits, &nullable);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(std::string(colName) == expectedColNames[col - 1]);
  }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetTypeInfo: Can bind columns and fetch data",
                 "[odbc-api][gettypeinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  char typeName[256];
  SQLLEN typeNameInd;
  SQLSMALLINT dataType;
  SQLLEN dataTypeInd;
  SQLINTEGER columnSize;
  SQLLEN columnSizeInd;

  SQLRETURN ret = SQLGetTypeInfo(stmt_handle(), SQL_ALL_TYPES);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLBindCol(stmt_handle(), 1, SQL_C_CHAR, typeName, sizeof(typeName), &typeNameInd);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLBindCol(stmt_handle(), 2, SQL_C_SSHORT, &dataType, sizeof(dataType), &dataTypeInd);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLBindCol(stmt_handle(), 3, SQL_C_SLONG, &columnSize, sizeof(columnSize), &columnSizeInd);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  REQUIRE(typeNameInd != SQL_NULL_DATA);
  REQUIRE(dataTypeInd != SQL_NULL_DATA);
  REQUIRE(columnSizeInd != SQL_NULL_DATA);

  REQUIRE(strlen(typeName) > 0);
  REQUIRE((dataType == SQL_CHAR || dataType == SQL_VARCHAR || dataType == SQL_WCHAR || dataType == SQL_WVARCHAR ||
           dataType == SQL_DECIMAL || dataType == SQL_NUMERIC || dataType == SQL_INTEGER || dataType == SQL_BIGINT ||
           dataType == SQL_REAL || dataType == SQL_FLOAT || dataType == SQL_DOUBLE || dataType == SQL_BIT ||
           dataType == SQL_BINARY || dataType == SQL_VARBINARY || dataType == SQL_TYPE_DATE ||
           dataType == SQL_TYPE_TIME || dataType == SQL_TYPE_TIMESTAMP));
  REQUIRE(columnSize > 0);
}

// ============================================================================
// SQLGetTypeInfo - Comprehensive Deep Validation for All Types
// ============================================================================

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetTypeInfo: Documents all supported types with exact column values",
                 "[odbc-api][gettypeinfo][driver_info]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(dsn_name().c_str())), SQL_NTS,
                             nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  for (const auto& expected : ALL_TYPE_INFO) {
    SQLHSTMT stmt = SQL_NULL_HSTMT;
    ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
    REQUIRE(ret == SQL_SUCCESS);

    ret = SQLGetTypeInfo(stmt, expected.sqlType);
    REQUIRE(ret == SQL_SUCCESS);

    ret = SQLFetch(stmt);
    REQUIRE(ret == SQL_SUCCESS);

    char typeName[256];
    SQLSMALLINT dataType;
    SQLINTEGER columnSize;
    char literalPrefix[32];
    char literalSuffix[32];
    char createParams[256];
    SQLSMALLINT nullable;
    SQLSMALLINT caseSensitive;
    SQLSMALLINT searchable;
    SQLSMALLINT unsignedAttr;
    SQLSMALLINT fixedPrecScale;
    SQLSMALLINT autoUniqueValue;
    char localTypeName[256];
    SQLSMALLINT minimumScale;
    SQLSMALLINT maximumScale;
    SQLSMALLINT sqlDataType;
    SQLSMALLINT sqlDatetimeSub;
    SQLINTEGER numPrecRadix;
    SQLINTEGER intervalPrecision;
    SQLSMALLINT userDataType;
    SQLLEN indicator;

    // TYPE_NAME
    ret = SQLGetData(stmt, 1, SQL_C_CHAR, typeName, sizeof(typeName), &indicator);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(std::string(typeName) == expected.typeName);

    // DATA_TYPE
    ret = SQLGetData(stmt, 2, SQL_C_SSHORT, &dataType, sizeof(dataType), &indicator);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(dataType == expected.sqlType);

    // COLUMN_SIZE
    ret = SQLGetData(stmt, 3, SQL_C_SLONG, &columnSize, sizeof(columnSize), &indicator);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(columnSize == expected.columnSize);

    // LITERAL_PREFIX
    ret = SQLGetData(stmt, 4, SQL_C_CHAR, literalPrefix, sizeof(literalPrefix), &indicator);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(indicator == expected.literalPrefixIndicator);
    // Buffer contents are undefined when SQL_NULL_DATA
    if (expected.literalPrefixIndicator != SQL_NULL_DATA) {
      REQUIRE(std::string(literalPrefix) == expected.literalPrefix);
    }

    // LITERAL_SUFFIX
    ret = SQLGetData(stmt, 5, SQL_C_CHAR, literalSuffix, sizeof(literalSuffix), &indicator);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(indicator == expected.literalSuffixIndicator);
    if (expected.literalSuffixIndicator != SQL_NULL_DATA) {
      REQUIRE(std::string(literalSuffix) == expected.literalSuffix);
    }

    // CREATE_PARAMS
    ret = SQLGetData(stmt, 6, SQL_C_CHAR, createParams, sizeof(createParams), &indicator);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(indicator == expected.createParamsIndicator);
    if (expected.createParamsIndicator != SQL_NULL_DATA) {
      REQUIRE(std::string(createParams) == expected.createParams);
    }

    // NULLABLE
    ret = SQLGetData(stmt, 7, SQL_C_SSHORT, &nullable, sizeof(nullable), &indicator);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(nullable == expected.nullable);

    // CASE_SENSITIVE
    ret = SQLGetData(stmt, 8, SQL_C_SSHORT, &caseSensitive, sizeof(caseSensitive), &indicator);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(caseSensitive == expected.caseSensitive);

    // SEARCHABLE
    ret = SQLGetData(stmt, 9, SQL_C_SSHORT, &searchable, sizeof(searchable), &indicator);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(searchable == expected.searchable);

    // UNSIGNED_ATTRIBUTE
    ret = SQLGetData(stmt, 10, SQL_C_SSHORT, &unsignedAttr, sizeof(unsignedAttr), &indicator);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(indicator == expected.unsignedAttrIndicator);
    if (expected.unsignedAttrIndicator != SQL_NULL_DATA) {
      REQUIRE(unsignedAttr == expected.unsignedAttr);
    }

    // FIXED_PREC_SCALE
    ret = SQLGetData(stmt, 11, SQL_C_SSHORT, &fixedPrecScale, sizeof(fixedPrecScale), &indicator);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(fixedPrecScale == expected.fixedPrecScale);

    // AUTO_UNIQUE_VALUE
    ret = SQLGetData(stmt, 12, SQL_C_SSHORT, &autoUniqueValue, sizeof(autoUniqueValue), &indicator);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(indicator == expected.autoUniqueValueIndicator);
    if (expected.autoUniqueValueIndicator != SQL_NULL_DATA) {
      REQUIRE(autoUniqueValue == expected.autoUniqueValue);
    }

    // LOCAL_TYPE_NAME
    ret = SQLGetData(stmt, 13, SQL_C_CHAR, localTypeName, sizeof(localTypeName), &indicator);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(indicator == expected.localTypeNameIndicator);
    if (expected.localTypeNameIndicator != SQL_NULL_DATA) {
      REQUIRE(std::string(localTypeName) == expected.localTypeName);
    }

    // MINIMUM_SCALE
    ret = SQLGetData(stmt, 14, SQL_C_SSHORT, &minimumScale, sizeof(minimumScale), &indicator);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(indicator == expected.minimumScaleIndicator);
    if (expected.minimumScaleIndicator != SQL_NULL_DATA) {
      REQUIRE(minimumScale == expected.minimumScale);
    }

    // MAXIMUM_SCALE
    ret = SQLGetData(stmt, 15, SQL_C_SSHORT, &maximumScale, sizeof(maximumScale), &indicator);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(indicator == expected.maximumScaleIndicator);
    if (expected.maximumScaleIndicator != SQL_NULL_DATA) {
      REQUIRE(maximumScale == expected.maximumScale);
    }

    // SQL_DATA_TYPE
    ret = SQLGetData(stmt, 16, SQL_C_SSHORT, &sqlDataType, sizeof(sqlDataType), &indicator);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(sqlDataType == expected.sqlDataType);

    // SQL_DATETIME_SUB
    ret = SQLGetData(stmt, 17, SQL_C_SSHORT, &sqlDatetimeSub, sizeof(sqlDatetimeSub), &indicator);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(indicator == expected.sqlDatetimeSubIndicator);
    if (expected.sqlDatetimeSubIndicator != SQL_NULL_DATA) {
      REQUIRE(sqlDatetimeSub == expected.sqlDatetimeSub);
    }

    // NUM_PREC_RADIX
    ret = SQLGetData(stmt, 18, SQL_C_SLONG, &numPrecRadix, sizeof(numPrecRadix), &indicator);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(indicator == expected.numPrecRadixIndicator);
    if (expected.numPrecRadixIndicator != SQL_NULL_DATA) {
      REQUIRE(numPrecRadix == expected.numPrecRadix);
    }

    // INTERVAL_PRECISION
    ret = SQLGetData(stmt, 19, SQL_C_SLONG, &intervalPrecision, sizeof(intervalPrecision), &indicator);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(indicator == expected.intervalPrecisionIndicator);
    if (expected.intervalPrecisionIndicator != SQL_NULL_DATA) {
      REQUIRE(intervalPrecision == expected.intervalPrecision);
    }

    // USER_DATA_TYPE
    ret = SQLGetData(stmt, 20, SQL_C_SSHORT, &userDataType, sizeof(userDataType), &indicator);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(indicator == expected.userDataTypeIndicator);
    if (expected.userDataTypeIndicator != SQL_NULL_DATA) {
      REQUIRE(userDataType == expected.userDataType);
    }

    SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  }

  SQLDisconnect(dbc_handle());
}
