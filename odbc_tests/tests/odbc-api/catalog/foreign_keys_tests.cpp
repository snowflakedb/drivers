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

void require_sqlgetdata_char(SQLHSTMT stmt, SQLUSMALLINT column, char* buf, const size_t buf_len, SQLLEN* indicator) {
  std::memset(buf, 0xFF, buf_len);
  *indicator = 0;
  const SQLRETURN ret = SQLGetData(stmt, column, SQL_C_CHAR, buf, buf_len, indicator);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt), OdbcMatchers::Succeeded());
  REQUIRE(*indicator != SQL_NULL_DATA);
}

void require_sqlgetdata_sshort(SQLHSTMT stmt, SQLUSMALLINT column, SQLSMALLINT* value, SQLLEN* indicator) {
  *value = static_cast<SQLSMALLINT>(0xFFFE);
  *indicator = 0;
  const SQLRETURN ret = SQLGetData(stmt, column, SQL_C_SSHORT, value, 0, indicator);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt), OdbcMatchers::Succeeded());
  // A non-null SQL_C_SSHORT value must report the fixed C-type width, not merely
  // "not null" — this also catches a driver that leaves the indicator at 0.
  REQUIRE(*indicator == sizeof(SQLSMALLINT));
}

}  // namespace

// ============================================================================
// SQLForeignKeys - Result Set Structure
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: Result set has correct number of columns",
                 "[odbc-api][foreignkeys][catalog]") {
  // Query FK table to get foreign keys
  SQLRETURN ret = SQLForeignKeys(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0, sqlchar(database_name()), SQL_NTS,
                                 sqlchar(schema_name()), SQL_NTS, sqlchar(readonly_db::FK_CHILD), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // ODBC 3.x spec defines 14 columns for SQLForeignKeys
  SQLSMALLINT numCols = 0;
  ret = SQLNumResultCols(stmt_handle(), &numCols);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(numCols == 14);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: Result set column names match ODBC 3.x spec",
                 "[odbc-api][foreignkeys][catalog]") {
  SQLRETURN ret = SQLForeignKeys(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0, sqlchar(database_name()), SQL_NTS,
                                 sqlchar(schema_name()), SQL_NTS, sqlchar(readonly_db::FK_CHILD), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  const char* expectedColNames[] = {"PKTABLE_CAT",   "PKTABLE_SCHEM", "PKTABLE_NAME",  "PKCOLUMN_NAME", "FKTABLE_CAT",
                                    "FKTABLE_SCHEM", "FKTABLE_NAME",  "FKCOLUMN_NAME", "KEY_SEQ",       "UPDATE_RULE",
                                    "DELETE_RULE",   "FK_NAME",       "PK_NAME",       "DEFERRABILITY"};

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

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: KEY_SEQ and rule columns have ODBC 3.x types and nullability",
                 "[odbc-api][foreignkeys][catalog]") {
  SQLRETURN ret = SQLForeignKeys(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0, sqlchar(database_name()), SQL_NTS,
                                 sqlchar(schema_name()), SQL_NTS, sqlchar(readonly_db::FK_CHILD), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT keySeqType = 0;
  SQLSMALLINT keySeqNullable = -1;
  ret = SQLDescribeCol(stmt_handle(), 9, nullptr, 0, nullptr, &keySeqType, nullptr, nullptr, &keySeqNullable);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(keySeqType == SQL_SMALLINT);
  REQUIRE(keySeqNullable == SQL_NO_NULLS);

  SQLSMALLINT updateRuleType = 0;
  SQLSMALLINT updateRuleNullable = -1;
  ret = SQLDescribeCol(stmt_handle(), 10, nullptr, 0, nullptr, &updateRuleType, nullptr, nullptr, &updateRuleNullable);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(updateRuleType == SQL_SMALLINT);
  REQUIRE(updateRuleNullable == SQL_NULLABLE);

  SQLSMALLINT deleteRuleType = 0;
  SQLSMALLINT deleteRuleNullable = -1;
  ret = SQLDescribeCol(stmt_handle(), 11, nullptr, 0, nullptr, &deleteRuleType, nullptr, nullptr, &deleteRuleNullable);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(deleteRuleType == SQL_SMALLINT);
  REQUIRE(deleteRuleNullable == SQL_NULLABLE);

  SQLSMALLINT fkNameNullable = -1;
  ret = SQLDescribeCol(stmt_handle(), 12, nullptr, 0, nullptr, nullptr, nullptr, nullptr, &fkNameNullable);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(fkNameNullable == SQL_NULLABLE);

  SQLSMALLINT pkNameNullable = -1;
  ret = SQLDescribeCol(stmt_handle(), 13, nullptr, 0, nullptr, nullptr, nullptr, nullptr, &pkNameNullable);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(pkNameNullable == SQL_NULLABLE);

  SQLSMALLINT deferrabilityType = 0;
  SQLSMALLINT deferrabilityNullable = -1;
  ret = SQLDescribeCol(stmt_handle(), 14, nullptr, 0, nullptr, &deferrabilityType, nullptr, nullptr,
                       &deferrabilityNullable);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(deferrabilityType == SQL_SMALLINT);
  REQUIRE(deferrabilityNullable == SQL_NULLABLE);
}

// ============================================================================
// SQLForeignKeys - Data Verification
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: FK table returns foreign key referencing PK table",
                 "[odbc-api][foreignkeys][catalog]") {
  // Query by FK table: what foreign keys does FK_CHILD have?
  SQLRETURN ret = SQLForeignKeys(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0, sqlchar(database_name()), SQL_NTS,
                                 sqlchar(schema_name()), SQL_NTS, sqlchar(readonly_db::FK_CHILD), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  char pkTableName[256];
  char pkColumnName[256];
  char fkTableName[256];
  char fkColumnName[256];
  auto keySeq = static_cast<SQLSMALLINT>(0xFFFE);

  SQLLEN pkTableNameInd = 0;
  require_sqlgetdata_char(stmt_handle(), 3, pkTableName, sizeof(pkTableName), &pkTableNameInd);
  SQLLEN pkColumnNameInd = 0;
  require_sqlgetdata_char(stmt_handle(), 4, pkColumnName, sizeof(pkColumnName), &pkColumnNameInd);
  SQLLEN fkTableNameInd = 0;
  require_sqlgetdata_char(stmt_handle(), 7, fkTableName, sizeof(fkTableName), &fkTableNameInd);
  SQLLEN fkColumnNameInd = 0;
  require_sqlgetdata_char(stmt_handle(), 8, fkColumnName, sizeof(fkColumnName), &fkColumnNameInd);
  SQLLEN keySeqInd = 0;
  require_sqlgetdata_sshort(stmt_handle(), 9, &keySeq, &keySeqInd);

  REQUIRE(std::string(pkTableName) == readonly_db::FK_PARENT);
  REQUIRE(std::string(pkColumnName) == "ID");
  REQUIRE(std::string(fkTableName) == readonly_db::FK_CHILD);
  REQUIRE(std::string(fkColumnName) == "PARENTID");
  REQUIRE(keySeq == 1);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: PK table returns foreign keys referencing it",
                 "[odbc-api][foreignkeys][catalog]") {
  // Query by PK table: what tables reference FK_PARENT's primary key?
  SQLRETURN ret = SQLForeignKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                 sqlchar(readonly_db::FK_PARENT), SQL_NTS, nullptr, 0, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  char pkTableName[256];
  char pkColumnName[256];
  char fkTableName[256];
  char fkColumnName[256];

  SQLLEN pkTableNameInd = 0;
  require_sqlgetdata_char(stmt_handle(), 3, pkTableName, sizeof(pkTableName), &pkTableNameInd);
  SQLLEN pkColumnNameInd = 0;
  require_sqlgetdata_char(stmt_handle(), 4, pkColumnName, sizeof(pkColumnName), &pkColumnNameInd);
  SQLLEN fkTableNameInd = 0;
  require_sqlgetdata_char(stmt_handle(), 7, fkTableName, sizeof(fkTableName), &fkTableNameInd);
  SQLLEN fkColumnNameInd = 0;
  require_sqlgetdata_char(stmt_handle(), 8, fkColumnName, sizeof(fkColumnName), &fkColumnNameInd);

  REQUIRE(std::string(pkTableName) == readonly_db::FK_PARENT);
  REQUIRE(std::string(pkColumnName) == "ID");
  REQUIRE(std::string(fkTableName) == readonly_db::FK_CHILD);
  REQUIRE(std::string(fkColumnName) == "PARENTID");

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: Both PK and FK table specified returns matching relationship",
                 "[odbc-api][foreignkeys][catalog]") {
  SQLRETURN ret = SQLForeignKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                 sqlchar(readonly_db::FK_PARENT), SQL_NTS, sqlchar(database_name()), SQL_NTS,
                                 sqlchar(schema_name()), SQL_NTS, sqlchar(readonly_db::FK_CHILD), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  char pkTableName[256];
  char fkTableName[256];
  SQLLEN pkTableNameInd = 0;
  require_sqlgetdata_char(stmt_handle(), 3, pkTableName, sizeof(pkTableName), &pkTableNameInd);
  SQLLEN fkTableNameInd = 0;
  require_sqlgetdata_char(stmt_handle(), 7, fkTableName, sizeof(fkTableName), &fkTableNameInd);

  REQUIRE(std::string(pkTableName) == readonly_db::FK_PARENT);
  REQUIRE(std::string(fkTableName) == readonly_db::FK_CHILD);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture,
                 "SQLForeignKeys: PK table referenced by multiple children returns all relationships",
                 "[odbc-api][foreignkeys][catalog]") {
  // Query by PK table: both children should appear
  SQLRETURN ret = SQLForeignKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                 sqlchar(readonly_db::FK_MULTI_PARENT), SQL_NTS, nullptr, 0, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  int rowCount = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS) {
    char fkTable[256];
    SQLLEN fkTableInd = 0;
    require_sqlgetdata_char(stmt_handle(), 7, fkTable, sizeof(fkTable), &fkTableInd);
    rowCount++;
  }
  REQUIRE(rowCount == 2);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: Table without foreign keys returns empty result set",
                 "[odbc-api][foreignkeys][catalog]") {
  SQLRETURN ret = SQLForeignKeys(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0, sqlchar(database_name()), SQL_NTS,
                                 sqlchar(schema_name()), SQL_NTS, sqlchar(readonly_db::NO_PK_TABLE), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: Non-existent table returns empty result set",
                 "[odbc-api][foreignkeys][catalog]") {
  SQLRETURN ret = SQLForeignKeys(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0, sqlchar(database_name()), SQL_NTS,
                                 sqlchar(schema_name()), SQL_NTS, sqlchar("NONEXISTENTTABLEXYZ99999"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture,
                 "SQLForeignKeys: UPDATE_RULE DELETE_RULE and DEFERRABILITY have expected values",
                 "[odbc-api][foreignkeys][catalog]") {
  SQLRETURN ret = SQLForeignKeys(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0, sqlchar(database_name()), SQL_NTS,
                                 sqlchar(schema_name()), SQL_NTS, sqlchar(readonly_db::FK_CHILD), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT updateRule = static_cast<SQLSMALLINT>(0xFFFE);
  SQLSMALLINT deleteRule = static_cast<SQLSMALLINT>(0xFFFE);
  SQLSMALLINT deferrability = static_cast<SQLSMALLINT>(0xFFFE);

  SQLLEN updateRuleInd = 0;
  SQLRETURN ret2 = SQLGetData(stmt_handle(), 10, SQL_C_SSHORT, &updateRule, 0, &updateRuleInd);
  REQUIRE_THAT(OdbcResult(ret2, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(updateRuleInd == sizeof(SQLSMALLINT));
  REQUIRE(updateRule == SQL_NO_ACTION);

  SQLLEN deleteRuleInd = 0;
  ret2 = SQLGetData(stmt_handle(), 11, SQL_C_SSHORT, &deleteRule, 0, &deleteRuleInd);
  REQUIRE_THAT(OdbcResult(ret2, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(deleteRuleInd == sizeof(SQLSMALLINT));
  REQUIRE(deleteRule == SQL_NO_ACTION);

  SQLLEN deferrabilityInd = 0;
  ret2 = SQLGetData(stmt_handle(), 14, SQL_C_SSHORT, &deferrability, 0, &deferrabilityInd);
  REQUIRE_THAT(OdbcResult(ret2, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(deferrabilityInd == sizeof(SQLSMALLINT));
  REQUIRE(deferrability == SQL_NOT_DEFERRABLE);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: Returns non-empty FK_NAME and PK_NAME",
                 "[odbc-api][foreignkeys][catalog]") {
  SQLRETURN ret = SQLForeignKeys(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0, sqlchar(database_name()), SQL_NTS,
                                 sqlchar(schema_name()), SQL_NTS, sqlchar(readonly_db::FK_CHILD), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  char fkName[256];
  char pkName[256];
  std::memset(fkName, 0xFF, sizeof(fkName));
  std::memset(pkName, 0xFF, sizeof(pkName));

  SQLLEN fkNameInd = 0;
  SQLRETURN ret2 = SQLGetData(stmt_handle(), 12, SQL_C_CHAR, fkName, sizeof(fkName), &fkNameInd);
  REQUIRE_THAT(OdbcResult(ret2, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(fkNameInd != SQL_NULL_DATA);
  REQUIRE(!std::string(fkName).empty());

  SQLLEN pkNameInd = 0;
  ret2 = SQLGetData(stmt_handle(), 13, SQL_C_CHAR, pkName, sizeof(pkName), &pkNameInd);
  REQUIRE_THAT(OdbcResult(ret2, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  REQUIRE(pkNameInd != SQL_NULL_DATA);
  REQUIRE(!std::string(pkName).empty());

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: NULL catalog and schema resolve from connection context",
                 "[odbc-api][foreignkeys][catalog]") {
  SQLRETURN ret = SQLForeignKeys(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0, nullptr, 0, nullptr, 0,
                                 sqlchar(readonly_db::FK_CHILD), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  NEW_DRIVER_ONLY("BD#88") {
    ret = SQLFetch(stmt_handle());
    REQUIRE(ret == SQL_SUCCESS);

    char fkTableCat[256];
    char fkTableSchem[256];
    char fkTableName[256];
    char fkColumnName[256];
    SQLSMALLINT keySeq = static_cast<SQLSMALLINT>(0xFFFE);
    std::memset(fkTableCat, 0xFF, sizeof(fkTableCat));
    std::memset(fkTableSchem, 0xFF, sizeof(fkTableSchem));
    std::memset(fkTableName, 0xFF, sizeof(fkTableName));
    std::memset(fkColumnName, 0xFF, sizeof(fkColumnName));

    SQLLEN fkTableCatInd = 0;
    SQLRETURN ret2 = SQLGetData(stmt_handle(), 5, SQL_C_CHAR, fkTableCat, sizeof(fkTableCat), &fkTableCatInd);
    REQUIRE_THAT(OdbcResult(ret2, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
    REQUIRE(fkTableCatInd != SQL_NULL_DATA);

    SQLLEN fkTableSchemInd = 0;
    ret2 = SQLGetData(stmt_handle(), 6, SQL_C_CHAR, fkTableSchem, sizeof(fkTableSchem), &fkTableSchemInd);
    REQUIRE_THAT(OdbcResult(ret2, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
    REQUIRE(fkTableSchemInd != SQL_NULL_DATA);

    SQLLEN fkTableNameInd = 0;
    ret2 = SQLGetData(stmt_handle(), 7, SQL_C_CHAR, fkTableName, sizeof(fkTableName), &fkTableNameInd);
    REQUIRE_THAT(OdbcResult(ret2, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
    REQUIRE(fkTableNameInd != SQL_NULL_DATA);

    SQLLEN fkColumnNameInd = 0;
    ret2 = SQLGetData(stmt_handle(), 8, SQL_C_CHAR, fkColumnName, sizeof(fkColumnName), &fkColumnNameInd);
    REQUIRE_THAT(OdbcResult(ret2, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
    REQUIRE(fkColumnNameInd != SQL_NULL_DATA);

    SQLLEN keySeqInd = 0;
    ret2 = SQLGetData(stmt_handle(), 9, SQL_C_SSHORT, &keySeq, 0, &keySeqInd);
    REQUIRE_THAT(OdbcResult(ret2, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
    REQUIRE(keySeqInd == sizeof(SQLSMALLINT));

    REQUIRE(std::string(fkTableCat) == database_name());
    REQUIRE(std::string(fkTableSchem) == schema_name());
    REQUIRE(std::string(fkTableName) == readonly_db::FK_CHILD);
    REQUIRE(std::string(fkColumnName) == "PARENTID");
    REQUIRE(keySeq == 1);

    ret = SQLFetch(stmt_handle());
    REQUIRE(ret == SQL_NO_DATA);
  }
  OLD_DRIVER_ONLY("BD#88") {
    // The reference driver's outcome depends on the account's
    // CLIENT_METADATA_REQUEST_USE_CONNECTION_CTX session parameter: when it is
    // enabled the FK-side identifiers resolve from the connection context and the
    // FK_CHILD row is returned; when it is disabled the call returns an empty
    // result set. Accept either so the test is not account-dependent.
    ret = SQLFetch(stmt_handle());
    REQUIRE((ret == SQL_SUCCESS || ret == SQL_NO_DATA));
  }
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture,
                 "SQLForeignKeys: Both PK and FK table specified with no relationship returns empty result set",
                 "[odbc-api][foreignkeys][catalog]") {
  // FKCHILD references FKPARENT, not FKMULTIPARENT.
  SQLRETURN ret = SQLForeignKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                 sqlchar(readonly_db::FK_MULTI_PARENT), SQL_NTS, sqlchar(database_name()), SQL_NTS,
                                 sqlchar(schema_name()), SQL_NTS, sqlchar(readonly_db::FK_CHILD), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

// ============================================================================
// SQLForeignKeys - Statement Reuse
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: Can call multiple times on same statement after close cursor",
                 "[odbc-api][foreignkeys][catalog]") {
  SQLRETURN ret = SQLForeignKeys(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0, sqlchar(database_name()), SQL_NTS,
                                 sqlchar(schema_name()), SQL_NTS, sqlchar(readonly_db::FK_CHILD), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  int count1 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count1++;
  REQUIRE(count1 == 1);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLForeignKeys(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0, sqlchar(database_name()), SQL_NTS,
                       sqlchar(schema_name()), SQL_NTS, sqlchar(readonly_db::FK_CHILD), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  int count2 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count2++;
  REQUIRE(count2 == 1);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: SQLRowCount after catalog function call",
                 "[odbc-api][foreignkeys][catalog]") {
  SQLRETURN ret = SQLForeignKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                 sqlchar(readonly_db::FK_PARENT), SQL_NTS, nullptr, 0, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN rowCount = 0;
  ret = SQLRowCount(stmt_handle(), &rowCount);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(rowCount == -1);
}

// ============================================================================
// SQLForeignKeys - Error Cases
// ============================================================================

TEST_CASE("SQLForeignKeys: SQL_INVALID_HANDLE for null statement handle", "[odbc-api][foreignkeys][catalog][error]") {
  const SQLRETURN ret = SQLForeignKeys(SQL_NULL_HSTMT, nullptr, 0, nullptr, 0, nullptr, 0, nullptr, 0, nullptr, 0,
                                       sqlchar("TABLE"), SQL_NTS);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLForeignKeys: HY009 - Both PKTableName and FKTableName are null",
                 "[odbc-api][foreignkeys][catalog][error]") {
  SQLRETURN ret = SQLForeignKeys(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0, nullptr, 0, nullptr, 0, nullptr, 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY009", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: metadata_id=TRUE with NULL PKCatalogName returns HY009",
                 "[odbc-api][foreignkeys][catalog][error]") {
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLForeignKeys(stmt_handle(), nullptr, 0, sqlchar(schema_name()), SQL_NTS, sqlchar(readonly_db::FK_PARENT),
                       SQL_NTS, nullptr, 0, nullptr, 0, nullptr, 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY009", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: metadata_id=TRUE with NULL PKSchemaName returns HY009",
                 "[odbc-api][foreignkeys][catalog][error]") {
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLForeignKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, nullptr, 0, sqlchar(readonly_db::FK_PARENT),
                       SQL_NTS, nullptr, 0, nullptr, 0, nullptr, 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY009", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: metadata_id=TRUE with NULL FKCatalogName returns HY009",
                 "[odbc-api][foreignkeys][catalog][error]") {
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLForeignKeys(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0, nullptr, 0, sqlchar(schema_name()), SQL_NTS,
                       sqlchar(readonly_db::FK_CHILD), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "HY009", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: metadata_id=TRUE with NULL FKSchemaName returns HY009",
                 "[odbc-api][foreignkeys][catalog][error]") {
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLForeignKeys(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0, sqlchar(database_name()), SQL_NTS, nullptr, 0,
                       sqlchar(readonly_db::FK_CHILD), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "HY009", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture,
                 "SQLForeignKeys: metadata_id=TRUE single-sided PK query with NULL FK side succeeds",
                 "[odbc-api][foreignkeys][catalog]") {
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLForeignKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                       sqlchar(readonly_db::FK_PARENT), SQL_NTS, nullptr, 0, nullptr, 0, nullptr, 0);

  NEW_DRIVER_ONLY("BD#89") {
    WINDOWS_ONLY {
      // With SQL_ATTR_METADATA_ID=SQL_TRUE the Windows Driver Manager validates the
      // identifier arguments itself and rejects the NULL FK-side pointers with HY009
      // before the call reaches the driver (the MS ODBC reference marks this HY009 as
      // posted by the "(DM)"). The single-sided query therefore cannot succeed on
      // Windows regardless of driver behavior. unixODBC and iODBC do not perform this
      // check, so the driver runs and the query succeeds (asserted below).
      REQUIRE_EXPECTED_ERROR(ret, "HY009", stmt_handle(), SQL_HANDLE_STMT);
    }
    else {
      REQUIRE(ret == SQL_SUCCESS);

      ret = SQLFetch(stmt_handle());
      REQUIRE(ret == SQL_SUCCESS);

      char pkTableName[256];
      char fkTableName[256];
      SQLLEN pkTableNameInd = 0;
      require_sqlgetdata_char(stmt_handle(), 3, pkTableName, sizeof(pkTableName), &pkTableNameInd);
      SQLLEN fkTableNameInd = 0;
      require_sqlgetdata_char(stmt_handle(), 7, fkTableName, sizeof(fkTableName), &fkTableNameInd);

      REQUIRE(std::string(pkTableName) == readonly_db::FK_PARENT);
      REQUIRE(std::string(fkTableName) == readonly_db::FK_CHILD);

      ret = SQLFetch(stmt_handle());
      REQUIRE(ret == SQL_NO_DATA);
    }
  }
  OLD_DRIVER_ONLY("BD#89") { REQUIRE_EXPECTED_ERROR(ret, "HY009", stmt_handle(), SQL_HANDLE_STMT); }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLForeignKeys: HY090 - Negative PKCatalogName length",
                 "[odbc-api][foreignkeys][catalog][error]") {
  SQLRETURN ret = SQLForeignKeys(stmt_handle(), sqlchar("SNOWFLAKE"), -999, nullptr, 0, sqlchar("TABLE"), SQL_NTS,
                                 nullptr, 0, nullptr, 0, nullptr, 0);
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLForeignKeys: HY090 - Negative FKTableName length",
                 "[odbc-api][foreignkeys][catalog][error]") {
  SQLRETURN ret =
      SQLForeignKeys(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0, nullptr, 0, nullptr, 0, sqlchar("TABLE"), -999);
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

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: 24000 - Cursor already open",
                 "[odbc-api][foreignkeys][catalog][error]") {
  SQLRETURN ret = SQLForeignKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                 sqlchar(readonly_db::FK_PARENT), SQL_NTS, nullptr, 0, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Second call without closing cursor
  ret = SQLForeignKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                       sqlchar(readonly_db::FK_PARENT), SQL_NTS, nullptr, 0, nullptr, 0, nullptr, 0);
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(DbcFixture, "SQLForeignKeys: Requires active connection", "[odbc-api][foreignkeys][catalog][error]") {
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  const SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);

  // Note: Reference driver refuses to allocate statement on disconnected handle
  REQUIRE(ret == SQL_ERROR);
}

// ============================================================================
// SQLForeignKeys - SQL_ATTR_METADATA_ID identifier matching (C6)
// ============================================================================

// Identifier mode folds unquoted identifiers to uppercase, so lowercase PK/FK
// identifiers must still match the uppercase names Snowflake stores. Both sides
// are supplied (all six args non-NULL) so the Windows DM NULL check does not
// fire. The new driver folds unquoted identifiers (ODBC-spec) and matches; the
// legacy driver re-filters case-sensitively and returns nothing (BD#89).
TEST_CASE_METHOD(ReadOnlyDbStmtFixture,
                 "SQLForeignKeys: metadata_id=TRUE matches unquoted identifiers case-insensitively",
                 "[odbc-api][foreignkeys][catalog]") {
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  const std::string cat = to_lower_copy(database_name());
  const std::string sch = to_lower_copy(schema_name());
  const std::string pk = to_lower_copy(readonly_db::FK_PARENT);
  const std::string fk = to_lower_copy(readonly_db::FK_CHILD);

  ret = SQLForeignKeys(stmt_handle(), sqlchar(cat.c_str()), SQL_NTS, sqlchar(sch.c_str()), SQL_NTS, sqlchar(pk.c_str()),
                       SQL_NTS, sqlchar(cat.c_str()), SQL_NTS, sqlchar(sch.c_str()), SQL_NTS, sqlchar(fk.c_str()),
                       SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLFetch(stmt_handle());
  NEW_DRIVER_ONLY("BD#89") {
    REQUIRE(ret == SQL_SUCCESS);

    char pkTableName[256];
    char fkTableName[256];
    SQLLEN ind = 0;
    require_sqlgetdata_char(stmt_handle(), 3, pkTableName, sizeof(pkTableName), &ind);
    require_sqlgetdata_char(stmt_handle(), 7, fkTableName, sizeof(fkTableName), &ind);
    REQUIRE(std::string(pkTableName) == readonly_db::FK_PARENT);
    REQUIRE(std::string(fkTableName) == readonly_db::FK_CHILD);

    ret = SQLFetch(stmt_handle());
    REQUIRE(ret == SQL_NO_DATA);
  }
  OLD_DRIVER_ONLY("BD#89") { REQUIRE(ret == SQL_NO_DATA); }
}

// Pattern mode (default) is case-sensitive: a lowercase table name must not
// match the uppercase stored name. Catalog/schema stay valid (uppercase) so the
// SHOW scope resolves; only the lowercase table drives the empty result.
TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: metadata_id=FALSE treats identifiers case-sensitively",
                 "[odbc-api][foreignkeys][catalog]") {
  const std::string fk = to_lower_copy(readonly_db::FK_CHILD);

  SQLRETURN ret = SQLForeignKeys(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0, sqlchar(database_name()), SQL_NTS,
                                 sqlchar(schema_name()), SQL_NTS, sqlchar(fk.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

// ============================================================================
// SQLForeignKeys - Asymmetric filter corner cases (CC1-CC3)
// ============================================================================

// CC1: PK table + FK schema-only filter -> EXPORTED scoped to PK table, FK
// schema re-applied client-side.
TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: PK table with FK schema-only filter returns relationship",
                 "[odbc-api][foreignkeys][catalog]") {
  SQLRETURN ret =
      SQLForeignKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                     sqlchar(readonly_db::FK_PARENT), SQL_NTS, nullptr, 0, sqlchar(schema_name()), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  char pkTableName[256];
  char fkTableName[256];
  SQLLEN ind = 0;
  require_sqlgetdata_char(stmt_handle(), 3, pkTableName, sizeof(pkTableName), &ind);
  require_sqlgetdata_char(stmt_handle(), 7, fkTableName, sizeof(fkTableName), &ind);
  REQUIRE(std::string(pkTableName) == readonly_db::FK_PARENT);
  REQUIRE(std::string(fkTableName) == readonly_db::FK_CHILD);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

// CC2: FK table + PK schema-only filter -> IMPORTED scoped to FK table.
TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: FK table with PK schema-only filter returns relationship",
                 "[odbc-api][foreignkeys][catalog]") {
  SQLRETURN ret =
      SQLForeignKeys(stmt_handle(), nullptr, 0, sqlchar(schema_name()), SQL_NTS, nullptr, 0, sqlchar(database_name()),
                     SQL_NTS, sqlchar(schema_name()), SQL_NTS, sqlchar(readonly_db::FK_CHILD), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  char pkTableName[256];
  char fkTableName[256];
  SQLLEN ind = 0;
  require_sqlgetdata_char(stmt_handle(), 3, pkTableName, sizeof(pkTableName), &ind);
  require_sqlgetdata_char(stmt_handle(), 7, fkTableName, sizeof(fkTableName), &ind);
  REQUIRE(std::string(pkTableName) == readonly_db::FK_PARENT);
  REQUIRE(std::string(fkTableName) == readonly_db::FK_CHILD);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

// CC3: catalog-only filter on the non-table side, matching then non-matching
// (the non-matching case doubles as the cross-DB filter-mismatch scenario).
TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: FK table with PK catalog-only filter (match and mismatch)",
                 "[odbc-api][foreignkeys][catalog]") {
  SQLRETURN ret =
      SQLForeignKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, nullptr, 0, nullptr, 0, sqlchar(database_name()),
                     SQL_NTS, sqlchar(schema_name()), SQL_NTS, sqlchar(readonly_db::FK_CHILD), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);

  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  // Non-matching PK catalog -> empty (cross-DB filter mismatch).
  ret = SQLForeignKeys(stmt_handle(), sqlchar("OTHERDBXYZ"), SQL_NTS, nullptr, 0, nullptr, 0, sqlchar(database_name()),
                       SQL_NTS, sqlchar(schema_name()), SQL_NTS, sqlchar(readonly_db::FK_CHILD), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

// CC4: metadata_id=TRUE single-sided FK query (mirror of the PK-side BD#88 case).
TEST_CASE_METHOD(ReadOnlyDbStmtFixture,
                 "SQLForeignKeys: metadata_id=TRUE single-sided FK query with NULL PK side succeeds",
                 "[odbc-api][foreignkeys][catalog]") {
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLForeignKeys(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0, sqlchar(database_name()), SQL_NTS,
                       sqlchar(schema_name()), SQL_NTS, sqlchar(readonly_db::FK_CHILD), SQL_NTS);
  NEW_DRIVER_ONLY("BD#88") {
    WINDOWS_ONLY {
      // The Windows Driver Manager rejects the NULL PK-side identifier pointers
      // with HY009 before the driver runs (see BD#88); the single-sided query
      // cannot succeed on Windows.
      REQUIRE_EXPECTED_ERROR(ret, "HY009", stmt_handle(), SQL_HANDLE_STMT);
    }
    else {
      REQUIRE(ret == SQL_SUCCESS);
      ret = SQLFetch(stmt_handle());
      REQUIRE(ret == SQL_SUCCESS);
      char pkTableName[256];
      char fkTableName[256];
      SQLLEN ind = 0;
      require_sqlgetdata_char(stmt_handle(), 3, pkTableName, sizeof(pkTableName), &ind);
      require_sqlgetdata_char(stmt_handle(), 7, fkTableName, sizeof(fkTableName), &ind);
      REQUIRE(std::string(pkTableName) == readonly_db::FK_PARENT);
      REQUIRE(std::string(fkTableName) == readonly_db::FK_CHILD);
      ret = SQLFetch(stmt_handle());
      REQUIRE(ret == SQL_NO_DATA);
    }
  }
  OLD_DRIVER_ONLY("BD#88") { REQUIRE_EXPECTED_ERROR(ret, "HY009", stmt_handle(), SQL_HANDLE_STMT); }
}

// ============================================================================
// SQLForeignKeys - Cross-schema corner cases (CC5-CC7) [require XSPARENT/XSCHILD]
// ============================================================================

// CC5: cross-schema, query by PK table -> EXPORTED; child lives in FKREMOTESCHEMA.
TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: cross-schema query by PK table returns remote-schema child",
                 "[odbc-api][foreignkeys][catalog]") {
  SQLRETURN ret = SQLForeignKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(READONLY_SCHEMA_NAME),
                                 SQL_NTS, sqlchar(readonly_db::XS_PARENT), SQL_NTS, nullptr, 0, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  char fkTableSchem[256];
  char fkTableName[256];
  SQLLEN ind = 0;
  require_sqlgetdata_char(stmt_handle(), 6, fkTableSchem, sizeof(fkTableSchem), &ind);
  require_sqlgetdata_char(stmt_handle(), 7, fkTableName, sizeof(fkTableName), &ind);
  REQUIRE(std::string(fkTableSchem) == READONLY_FK_REMOTE_SCHEMA_NAME);
  REQUIRE(std::string(fkTableName) == readonly_db::XS_CHILD);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

// CC6: cross-schema, query by FK table -> IMPORTED; parent lives in CATALOGTESTS.
TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: cross-schema query by FK table returns remote-schema parent",
                 "[odbc-api][foreignkeys][catalog]") {
  SQLRETURN ret =
      SQLForeignKeys(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0, sqlchar(database_name()), SQL_NTS,
                     sqlchar(READONLY_FK_REMOTE_SCHEMA_NAME), SQL_NTS, sqlchar(readonly_db::XS_CHILD), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  char pkTableSchem[256];
  char pkTableName[256];
  SQLLEN ind = 0;
  require_sqlgetdata_char(stmt_handle(), 2, pkTableSchem, sizeof(pkTableSchem), &ind);
  require_sqlgetdata_char(stmt_handle(), 3, pkTableName, sizeof(pkTableName), &ind);
  REQUIRE(std::string(pkTableSchem) == READONLY_SCHEMA_NAME);
  REQUIRE(std::string(pkTableName) == readonly_db::XS_PARENT);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

// CC7: cross-schema asymmetric (PK table + FK schema-only in the remote schema).
TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: cross-schema asymmetric PK table + FK schema filter",
                 "[odbc-api][foreignkeys][catalog]") {
  SQLRETURN ret = SQLForeignKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(READONLY_SCHEMA_NAME),
                                 SQL_NTS, sqlchar(readonly_db::XS_PARENT), SQL_NTS, nullptr, 0,
                                 sqlchar(READONLY_FK_REMOTE_SCHEMA_NAME), SQL_NTS, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  char fkTableSchem[256];
  char fkTableName[256];
  SQLLEN ind = 0;
  require_sqlgetdata_char(stmt_handle(), 6, fkTableSchem, sizeof(fkTableSchem), &ind);
  require_sqlgetdata_char(stmt_handle(), 7, fkTableName, sizeof(fkTableName), &ind);
  REQUIRE(std::string(fkTableSchem) == READONLY_FK_REMOTE_SCHEMA_NAME);
  REQUIRE(std::string(fkTableName) == readonly_db::XS_CHILD);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

// ============================================================================
// SQLForeignKeys - Composite FK KEY_SEQ ordering (CC8) [requires CFKPARENT/CFKCHILD]
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: composite FK returns ordered KEY_SEQ column pairs",
                 "[odbc-api][foreignkeys][catalog]") {
  SQLRETURN ret = SQLForeignKeys(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0, sqlchar(database_name()), SQL_NTS,
                                 sqlchar(schema_name()), SQL_NTS, sqlchar(readonly_db::COMPOSITE_FK_CHILD), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  char pkColumnName[256];
  char fkColumnName[256];
  SQLSMALLINT keySeq = 0;
  SQLLEN ind = 0;

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  require_sqlgetdata_char(stmt_handle(), 4, pkColumnName, sizeof(pkColumnName), &ind);
  require_sqlgetdata_char(stmt_handle(), 8, fkColumnName, sizeof(fkColumnName), &ind);
  require_sqlgetdata_sshort(stmt_handle(), 9, &keySeq, &ind);
  REQUIRE(keySeq == 1);
  REQUIRE(std::string(pkColumnName) == "A");
  REQUIRE(std::string(fkColumnName) == "X");

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  require_sqlgetdata_char(stmt_handle(), 4, pkColumnName, sizeof(pkColumnName), &ind);
  require_sqlgetdata_char(stmt_handle(), 8, fkColumnName, sizeof(fkColumnName), &ind);
  require_sqlgetdata_sshort(stmt_handle(), 9, &keySeq, &ind);
  REQUIRE(keySeq == 2);
  REQUIRE(std::string(pkColumnName) == "B");
  REQUIRE(std::string(fkColumnName) == "Y");

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

// ============================================================================
// SQLForeignKeys - Empty-string / quoted identifier corner cases (CC9-CC11)
// ============================================================================

// CC9: empty-string schema on a table-bearing side in identifier mode is
// treated as *absent* (not HY090), matching the legacy driver and
// SQLPrimaryKeys. Both sides are supplied so the Windows DM NULL check does not
// fire; the empty FK schema simply widens the FK-side scope and the call
// succeeds.
TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: metadata_id=TRUE empty FK schema succeeds",
                 "[odbc-api][foreignkeys][catalog]") {
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLForeignKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                       sqlchar(readonly_db::FK_PARENT), SQL_NTS, sqlchar(database_name()), SQL_NTS, sqlchar(""),
                       SQL_NTS, sqlchar(readonly_db::FK_CHILD), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
}

// CC10: quoted identifier under metadata_id=TRUE is case-sensitive. A correctly
// cased quoted name matches; a wrong-case quoted name returns no rows.
TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: metadata_id=TRUE quoted identifier is case-sensitive",
                 "[odbc-api][foreignkeys][catalog]") {
  SQLRETURN ret = SQLSetStmtAttr(stmt_handle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Correctly-cased quoted FK table (both sides supplied to avoid the DM NULL check).
  const std::string quotedFk = std::string("\"") + readonly_db::FK_CHILD + "\"";
  const std::string quotedPk = std::string("\"") + readonly_db::FK_PARENT + "\"";
  ret = SQLForeignKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                       sqlchar(quotedPk.c_str()), SQL_NTS, sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()),
                       SQL_NTS, sqlchar(quotedFk.c_str()), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);

  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  // Wrong-case quoted FK table -> no match (case-sensitive).
  const std::string quotedFkLower = std::string("\"") + to_lower_copy(readonly_db::FK_CHILD) + "\"";
  const std::string quotedPkLower = std::string("\"") + to_lower_copy(readonly_db::FK_PARENT) + "\"";
  ret = SQLForeignKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                       sqlchar(quotedPkLower.c_str()), SQL_NTS, sqlchar(database_name()), SQL_NTS,
                       sqlchar(schema_name()), SQL_NTS, sqlchar(quotedFkLower.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

// CC11: empty string on one side only, pattern mode. The new driver treats the
// empty PK table as absent for side selection, so the query resolves to the FK
// side and returns the relationship. The legacy driver does not, returning an
// empty result set (BD#90).
TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: empty PK table with FK table resolves to FK side",
                 "[odbc-api][foreignkeys][catalog]") {
  SQLRETURN ret = SQLForeignKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                 sqlchar(""), SQL_NTS, sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()),
                                 SQL_NTS, sqlchar(readonly_db::FK_CHILD), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  NEW_DRIVER_ONLY("BD#90") {
    REQUIRE(ret == SQL_SUCCESS);
    char pkTableName[256];
    char fkTableName[256];
    SQLLEN ind = 0;
    require_sqlgetdata_char(stmt_handle(), 3, pkTableName, sizeof(pkTableName), &ind);
    require_sqlgetdata_char(stmt_handle(), 7, fkTableName, sizeof(fkTableName), &ind);
    REQUIRE(std::string(pkTableName) == readonly_db::FK_PARENT);
    REQUIRE(std::string(fkTableName) == readonly_db::FK_CHILD);

    ret = SQLFetch(stmt_handle());
    REQUIRE(ret == SQL_NO_DATA);
  }
  OLD_DRIVER_ONLY("BD#90") { REQUIRE(ret == SQL_NO_DATA); }
}
