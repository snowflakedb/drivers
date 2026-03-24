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
// SQLForeignKeys - Result Set Structure
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: Result set has correct number of columns",
                 "[odbc-api][foreignkeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

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
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

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

// ============================================================================
// SQLForeignKeys - Data Verification
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: FK table returns foreign key referencing PK table",
                 "[odbc-api][foreignkeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Query by FK table: what foreign keys does FK_CHILD have?
  SQLRETURN ret = SQLForeignKeys(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0, sqlchar(database_name()), SQL_NTS,
                                 sqlchar(schema_name()), SQL_NTS, sqlchar(readonly_db::FK_CHILD), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  char pkTableName[256] = {};
  char pkColumnName[256] = {};
  char fkTableName[256] = {};
  char fkColumnName[256] = {};
  SQLSMALLINT keySeq = 0;

  SQLGetData(stmt_handle(), 3, SQL_C_CHAR, pkTableName, sizeof(pkTableName), nullptr);
  SQLGetData(stmt_handle(), 4, SQL_C_CHAR, pkColumnName, sizeof(pkColumnName), nullptr);
  SQLGetData(stmt_handle(), 7, SQL_C_CHAR, fkTableName, sizeof(fkTableName), nullptr);
  SQLGetData(stmt_handle(), 8, SQL_C_CHAR, fkColumnName, sizeof(fkColumnName), nullptr);
  SQLGetData(stmt_handle(), 9, SQL_C_SSHORT, &keySeq, 0, nullptr);

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
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Query by PK table: what tables reference FK_PARENT's primary key?
  SQLRETURN ret = SQLForeignKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                 sqlchar(readonly_db::FK_PARENT), SQL_NTS, nullptr, 0, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  char pkTableName[256] = {};
  char pkColumnName[256] = {};
  char fkTableName[256] = {};
  char fkColumnName[256] = {};

  SQLGetData(stmt_handle(), 3, SQL_C_CHAR, pkTableName, sizeof(pkTableName), nullptr);
  SQLGetData(stmt_handle(), 4, SQL_C_CHAR, pkColumnName, sizeof(pkColumnName), nullptr);
  SQLGetData(stmt_handle(), 7, SQL_C_CHAR, fkTableName, sizeof(fkTableName), nullptr);
  SQLGetData(stmt_handle(), 8, SQL_C_CHAR, fkColumnName, sizeof(fkColumnName), nullptr);

  REQUIRE(std::string(pkTableName) == readonly_db::FK_PARENT);
  REQUIRE(std::string(pkColumnName) == "ID");
  REQUIRE(std::string(fkTableName) == readonly_db::FK_CHILD);
  REQUIRE(std::string(fkColumnName) == "PARENTID");

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: Both PK and FK table specified returns matching relationship",
                 "[odbc-api][foreignkeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLForeignKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                 sqlchar(readonly_db::FK_PARENT), SQL_NTS, sqlchar(database_name()), SQL_NTS,
                                 sqlchar(schema_name()), SQL_NTS, sqlchar(readonly_db::FK_CHILD), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  char pkTableName[256] = {};
  char fkTableName[256] = {};
  SQLGetData(stmt_handle(), 3, SQL_C_CHAR, pkTableName, sizeof(pkTableName), nullptr);
  SQLGetData(stmt_handle(), 7, SQL_C_CHAR, fkTableName, sizeof(fkTableName), nullptr);

  REQUIRE(std::string(pkTableName) == readonly_db::FK_PARENT);
  REQUIRE(std::string(fkTableName) == readonly_db::FK_CHILD);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture,
                 "SQLForeignKeys: PK table referenced by multiple children returns all relationships",
                 "[odbc-api][foreignkeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Query by PK table: both children should appear
  SQLRETURN ret = SQLForeignKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                 sqlchar(readonly_db::FK_MULTI_PARENT), SQL_NTS, nullptr, 0, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  int rowCount = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS) {
    char fkTable[256] = {};
    SQLGetData(stmt_handle(), 7, SQL_C_CHAR, fkTable, sizeof(fkTable), nullptr);
    rowCount++;
  }
  REQUIRE(rowCount == 2);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: Table without foreign keys returns empty result set",
                 "[odbc-api][foreignkeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLForeignKeys(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0, sqlchar(database_name()), SQL_NTS,
                                 sqlchar(schema_name()), SQL_NTS, sqlchar(readonly_db::NO_PK_TABLE), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: Non-existent table returns empty result set",
                 "[odbc-api][foreignkeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLForeignKeys(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0, sqlchar(database_name()), SQL_NTS,
                                 sqlchar(schema_name()), SQL_NTS, sqlchar("NONEXISTENTTABLEXYZ99999"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

// ============================================================================
// SQLForeignKeys - Statement Reuse
// ============================================================================

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: Can call multiple times on same statement after close cursor",
                 "[odbc-api][foreignkeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

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
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

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
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLForeignKeys(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0, nullptr, 0, nullptr, 0, nullptr, 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY009", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLForeignKeys: HY090 - Negative PKCatalogName length",
                 "[odbc-api][foreignkeys][catalog][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLForeignKeys(stmt_handle(), sqlchar("SNOWFLAKE"), -999, nullptr, 0, sqlchar("TABLE"), SQL_NTS,
                                 nullptr, 0, nullptr, 0, nullptr, 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLForeignKeys: HY090 - Negative FKTableName length",
                 "[odbc-api][foreignkeys][catalog][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret =
      SQLForeignKeys(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0, nullptr, 0, nullptr, 0, sqlchar("TABLE"), -999);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(ReadOnlyDbStmtFixture, "SQLForeignKeys: 24000 - Cursor already open",
                 "[odbc-api][foreignkeys][catalog][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLForeignKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                                 sqlchar(readonly_db::FK_PARENT), SQL_NTS, nullptr, 0, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Second call without closing cursor
  ret = SQLForeignKeys(stmt_handle(), sqlchar(database_name()), SQL_NTS, sqlchar(schema_name()), SQL_NTS,
                       sqlchar(readonly_db::FK_PARENT), SQL_NTS, nullptr, 0, nullptr, 0, nullptr, 0);
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(DbcFixture, "SQLForeignKeys: Requires active connection", "[odbc-api][foreignkeys][catalog][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHSTMT stmt = SQL_NULL_HSTMT;
  const SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);

  // Note: Reference driver refuses to allocate statement on disconnected handle
  REQUIRE(ret == SQL_ERROR);
}
