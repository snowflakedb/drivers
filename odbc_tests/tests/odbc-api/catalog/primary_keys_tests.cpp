#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <algorithm>
#include <cctype>
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

namespace {
std::string to_lower_copy(const std::string& s) {
  std::string out = s;
  std::transform(out.begin(), out.end(), out.begin(), [](unsigned char c) { return std::tolower(c); });
  return out;
}
}  // namespace

// ============================================================================
// SQLPrimaryKeys - Result Set Structure
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLPrimaryKeys: Result set has correct number of columns",
                 "[odbc-api][primarykeys][catalog]") {
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

TEST_CASE_METHOD(ReadOnlyDbStmtFixture,
                 "SQLPrimaryKeys: KEY_SEQ and PK_NAME columns have ODBC 3.x types and nullability",
                 "[odbc-api][primarykeys][catalog]") {
  SQLRETURN ret = SQLPrimaryKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                 sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT keySeqType = 0;
  SQLSMALLINT keySeqNullable = -1;
  ret = SQLDescribeCol(stmt_handle(), 5, nullptr, 0, nullptr, &keySeqType, nullptr, nullptr, &keySeqNullable);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(keySeqType == SQL_SMALLINT);
  REQUIRE(keySeqNullable == SQL_NO_NULLS);

  SQLSMALLINT pkNameNullable = -1;
  ret = SQLDescribeCol(stmt_handle(), 6, nullptr, 0, nullptr, nullptr, nullptr, nullptr, &pkNameNullable);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(pkNameNullable == SQL_NULLABLE);
}

// ============================================================================
// SQLPrimaryKeys - Data Verification
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLPrimaryKeys: Returns primary key for single-column PK",
                 "[odbc-api][primarykeys][catalog]") {
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

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLPrimaryKeys: Returns named primary key constraint in PK_NAME",
                 "[odbc-api][primarykeys][catalog]") {
  SQLRETURN ret = SQLPrimaryKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                 sqlchar(readonly_db::NAMED_PK_TABLE), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  char pkName[256];
  std::memset(pkName, 0xFF, sizeof(pkName));
  SQLLEN indicator = 0;
  const SQLRETURN ret2 = SQLGetData(stmt_handle(), 6, SQL_C_CHAR, pkName, sizeof(pkName), &indicator);
  REQUIRE_THAT(OdbcResult(ret2, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(indicator != SQL_NULL_DATA);
  REQUIRE(std::string(pkName) == "PKNAMED");

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLPrimaryKeys: Table without primary key returns empty result set",
                 "[odbc-api][primarykeys][catalog]") {
  SQLRETURN ret = SQLPrimaryKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                 sqlchar(readonly_db::NO_PK_TABLE), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLPrimaryKeys: Non-existent table returns empty result set",
                 "[odbc-api][primarykeys][catalog]") {
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

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLPrimaryKeys: NULL catalog and schema resolve from connection context",
                 "[odbc-api][primarykeys][catalog]") {
  SQLRETURN ret = SQLPrimaryKeys(stmt_handle(), nullptr, 0, nullptr, 0, sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  char tableCat[256];
  char tableSchem[256];
  char tableName[256];
  char columnName[256];
  std::memset(tableCat, 0xFF, sizeof(tableCat));
  std::memset(tableSchem, 0xFF, sizeof(tableSchem));
  std::memset(tableName, 0xFF, sizeof(tableName));
  std::memset(columnName, 0xFF, sizeof(columnName));
  SQLSMALLINT keySeq = static_cast<SQLSMALLINT>(0xFFFE);

  SQLLEN tableCatInd = 0;
  SQLRETURN ret2 = SQLGetData(stmt_handle(), 1, SQL_C_CHAR, tableCat, sizeof(tableCat), &tableCatInd);
  REQUIRE_THAT(OdbcResult(ret2, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(tableCatInd != SQL_NULL_DATA);

  SQLLEN tableSchemInd = 0;
  ret2 = SQLGetData(stmt_handle(), 2, SQL_C_CHAR, tableSchem, sizeof(tableSchem), &tableSchemInd);
  REQUIRE_THAT(OdbcResult(ret2, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(tableSchemInd != SQL_NULL_DATA);

  SQLLEN tableNameInd = 0;
  ret2 = SQLGetData(stmt_handle(), 3, SQL_C_CHAR, tableName, sizeof(tableName), &tableNameInd);
  REQUIRE_THAT(OdbcResult(ret2, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(tableNameInd != SQL_NULL_DATA);

  SQLLEN columnNameInd = 0;
  ret2 = SQLGetData(stmt_handle(), 4, SQL_C_CHAR, columnName, sizeof(columnName), &columnNameInd);
  REQUIRE_THAT(OdbcResult(ret2, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(columnNameInd != SQL_NULL_DATA);

  SQLLEN keySeqInd = 0;
  ret2 = SQLGetData(stmt_handle(), 5, SQL_C_SSHORT, &keySeq, 0, &keySeqInd);
  REQUIRE_THAT(OdbcResult(ret2, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(keySeqInd != SQL_NULL_DATA);

  REQUIRE(std::string(tableCat) == database_name());
  REQUIRE(std::string(tableSchem) == schema_name());
  REQUIRE(std::string(tableName) == readonly_db::SINGLE_PK_TABLE);
  REQUIRE(std::string(columnName) == "ID");
  REQUIRE(keySeq == 1);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

// ============================================================================
// SQLPrimaryKeys - Statement Reuse
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLPrimaryKeys: Can call multiple times on same statement after close cursor",
                 "[odbc-api][primarykeys][catalog]") {
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPrimaryKeys: HY009 - NULL TableName pointer",
                 "[odbc-api][primarykeys][catalog][error]") {
  const SQLRETURN ret = SQLPrimaryKeys(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0);
  NON_IODBC { REQUIRE_EXPECTED_ERROR(ret, "HY009", stmt_handle(), SQL_HANDLE_STMT); }
  IODBC_ONLY { REQUIRE(ret == SQL_ERROR); }
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLPrimaryKeys: metadata_id=TRUE with NULL CatalogName returns HY009",
                 "[odbc-api][primarykeys][catalog][error]") {
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLPrimaryKeys(stmt_handle(), nullptr, 0, sqlchar(schema_name()), SQL_NTS,
                       sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "HY009", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLPrimaryKeys: metadata_id=TRUE with NULL SchemaName returns HY009",
                 "[odbc-api][primarykeys][catalog][error]") {
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLPrimaryKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, nullptr, 0,
                       sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "HY009", stmt_handle(), SQL_HANDLE_STMT);
}

// ============================================================================
// SQLPrimaryKeys - SQL_ATTR_METADATA_ID identifier matching (C3)
// ============================================================================

// In identifier mode, unquoted identifiers are case-insensitive (folded to
// uppercase), so a lowercase table name must still match the uppercase name
// Snowflake stores. The new driver folds unquoted identifiers (ODBC-spec
// compliant) so the row matches; the legacy driver compares case-sensitively
// and drops every row, yielding an empty result set (BD#87).
TEST_CASE_METHOD(ReadOnlyDbStmtFixture,
                 "SQLPrimaryKeys: metadata_id=TRUE matches unquoted identifiers case-insensitively",
                 "[odbc-api][primarykeys][catalog]") {
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  const std::string cat = to_lower_copy(database_name());
  const std::string sch = to_lower_copy(schema_name());
  const std::string tbl = to_lower_copy(readonly_db::SINGLE_PK_TABLE);

  ret = SQLPrimaryKeys(stmt_handle(), sqlchar(cat.c_str()), SQL_NTS, sqlchar(sch.c_str()), SQL_NTS,
                       sqlchar(tbl.c_str()), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  NEW_DRIVER_ONLY("BD#87") {
    REQUIRE(ret == SQL_SUCCESS);

    char tableName[256];
    std::memset(tableName, 0xFF, sizeof(tableName));
    SQLSMALLINT keySeq = static_cast<SQLSMALLINT>(0xFFFE);

    SQLLEN tableNameInd = 0;
    SQLRETURN dataRet = SQLGetData(stmt_handle(), 3, SQL_C_CHAR, tableName, sizeof(tableName), &tableNameInd);
    REQUIRE_THAT(OdbcResult(dataRet, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
    REQUIRE(tableNameInd != SQL_NULL_DATA);

    SQLLEN keySeqInd = 0;
    dataRet = SQLGetData(stmt_handle(), 5, SQL_C_SSHORT, &keySeq, 0, &keySeqInd);
    REQUIRE_THAT(OdbcResult(dataRet, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
    REQUIRE(keySeqInd != SQL_NULL_DATA);

    REQUIRE(std::string(tableName) == readonly_db::SINGLE_PK_TABLE);
    REQUIRE(keySeq == 1);

    ret = SQLFetch(stmt_handle());
    REQUIRE(ret == SQL_NO_DATA);
  }
  OLD_DRIVER_ONLY("BD#87") { REQUIRE(ret == SQL_NO_DATA); }
}

// In pattern mode (default), the arguments are ordinary case-sensitive values,
// so a lowercase table name must NOT match the uppercase stored name. Guards the
// C3 fix from over-reaching into pattern mode.
TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLPrimaryKeys: metadata_id=FALSE treats identifiers case-sensitively",
                 "[odbc-api][primarykeys][catalog]") {
  const std::string tbl = to_lower_copy(readonly_db::SINGLE_PK_TABLE);

  SQLRETURN ret = SQLPrimaryKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                 sqlchar(tbl.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

// In identifier mode a zero-length catalog/schema is treated as *absent*, not
// as an error: an empty string matches "tables that do not have catalogs",
// which does not exist in Snowflake, so the driver widens the SHOW scope and
// succeeds (matching the legacy driver). NULL still returns HY009 (covered
// above).
TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLPrimaryKeys: metadata_id=TRUE empty CatalogName succeeds",
                 "[odbc-api][primarykeys][catalog]") {
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLPrimaryKeys(stmt_handle(), sqlchar(""), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                       sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLPrimaryKeys: metadata_id=TRUE empty SchemaName succeeds",
                 "[odbc-api][primarykeys][catalog]") {
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLPrimaryKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(""), SQL_NTS,
                       sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPrimaryKeys: HY090 - Negative CatalogName length",
                 "[odbc-api][primarykeys][catalog][error]") {
  SQLRETURN ret = SQLPrimaryKeys(stmt_handle(), sqlchar("SNOWFLAKE"), -999, nullptr, 0, sqlchar("TABLE"), SQL_NTS);
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPrimaryKeys: HY090 - Negative SchemaName length",
                 "[odbc-api][primarykeys][catalog][error]") {
  SQLRETURN ret = SQLPrimaryKeys(stmt_handle(), nullptr, 0, sqlchar("SCHEMA"), -999, sqlchar("TABLE"), SQL_NTS);
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPrimaryKeys: HY090 - Negative TableName length",
                 "[odbc-api][primarykeys][catalog][error]") {
  SQLRETURN ret = SQLPrimaryKeys(stmt_handle(), nullptr, 0, nullptr, 0, sqlchar("TABLE"), -999);
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

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLPrimaryKeys: 24000 - Cursor already open",
                 "[odbc-api][primarykeys][catalog][error]") {
  SQLRETURN ret = SQLPrimaryKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                 sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // Second call without closing cursor
  ret = SQLPrimaryKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                       sqlchar(readonly_db::SINGLE_PK_TABLE), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(DbcFixture, "SQLPrimaryKeys: Requires active connection", "[odbc-api][primarykeys][catalog][error]") {
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  const SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);

  // Note: Reference driver refuses to allocate statement on disconnected handle
  REQUIRE(ret == SQL_ERROR);
}
