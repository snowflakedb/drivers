#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "ODBCFixtures.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "query_helpers.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

// ============================================================================
// SQLPrimaryKeys - Result Set Structure
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPrimaryKeys: Result set has correct number of columns",
                 "[odbc-api][primarykeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("CREATE TABLE test_pk_numcols (id INT PRIMARY KEY)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());

  ret = SQLPrimaryKeys(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()), SQL_NTS,
                       sqlchar("TEST_PK_NUMCOLS"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT numCols = 0;
  ret = SQLNumResultCols(stmt_handle(), &numCols);
  REQUIRE(ret == SQL_SUCCESS);
  // ODBC 3.x spec defines 6 columns for SQLPrimaryKeys
  REQUIRE(numCols == 6);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPrimaryKeys: Result set column names match ODBC 3.x spec",
                 "[odbc-api][primarykeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("CREATE TABLE test_pk_colnames (id INT PRIMARY KEY)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());

  ret = SQLPrimaryKeys(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()), SQL_NTS,
                       sqlchar("TEST_PK_COLNAMES"), SQL_NTS);
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPrimaryKeys: Returns primary key for single-column PK",
                 "[odbc-api][primarykeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(stmt_handle(),
                                sqlchar("CREATE TABLE test_pk_single (id INT PRIMARY KEY, name VARCHAR(50))"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());

  ret = SQLPrimaryKeys(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()), SQL_NTS,
                       sqlchar("TEST_PK_SINGLE"), SQL_NTS);
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

  REQUIRE(std::string(tableCat) == currentDb);
  REQUIRE(std::string(tableSchem) == schema.name());
  REQUIRE(std::string(tableName) == "TEST_PK_SINGLE");
  REQUIRE(std::string(columnName) == "ID");
  REQUIRE(keySeq == 1);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPrimaryKeys: Returns composite primary key with correct KEY_SEQ",
                 "[odbc-api][primarykeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(stmt_handle(),
                                sqlchar("CREATE TABLE test_pk_composite (region_id INT, store_id INT, name "
                                        "VARCHAR(50), PRIMARY KEY (region_id, store_id))"),
                                SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());

  ret = SQLPrimaryKeys(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()), SQL_NTS,
                       sqlchar("TEST_PK_COMPOSITE"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  char columnName[256] = {};
  SQLSMALLINT keySeq = 0;

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  SQLGetData(stmt_handle(), 4, SQL_C_CHAR, columnName, sizeof(columnName), nullptr);
  SQLGetData(stmt_handle(), 5, SQL_C_SSHORT, &keySeq, 0, nullptr);
  REQUIRE(std::string(columnName) == "REGION_ID");
  REQUIRE(keySeq == 1);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  SQLGetData(stmt_handle(), 4, SQL_C_CHAR, columnName, sizeof(columnName), nullptr);
  SQLGetData(stmt_handle(), 5, SQL_C_SSHORT, &keySeq, 0, nullptr);
  REQUIRE(std::string(columnName) == "STORE_ID");
  REQUIRE(keySeq == 2);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPrimaryKeys: Table without primary key returns empty result set",
                 "[odbc-api][primarykeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret =
      SQLExecDirect(stmt_handle(), sqlchar("CREATE TABLE test_pk_none (id INT, name VARCHAR(50))"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());

  ret = SQLPrimaryKeys(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()), SQL_NTS,
                       sqlchar("TEST_PK_NONE"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPrimaryKeys: Non-existent table returns empty result set",
                 "[odbc-api][primarykeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  const std::string currentDb = get_current_database(dbc_handle());

  SQLRETURN ret = SQLPrimaryKeys(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()),
                                 SQL_NTS, sqlchar("NONEXISTENT_TABLE_XYZ_99999"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

// ============================================================================
// SQLPrimaryKeys - Parameter Variations
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPrimaryKeys: Various parameter combinations are accepted",
                 "[odbc-api][primarykeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("CREATE TABLE test_pk_params (id INT PRIMARY KEY)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());
  const std::string& schemaName = schema.name();

  // Explicit catalog, schema, table with SQL_NTS
  ret = SQLPrimaryKeys(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schemaName.c_str()), SQL_NTS,
                       sqlchar("TEST_PK_PARAMS"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  int count1 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count1++;
  REQUIRE(count1 == 1);
  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // Explicit string lengths instead of SQL_NTS
  const std::string table = "TEST_PK_PARAMS";
  ret = SQLPrimaryKeys(stmt_handle(), sqlchar(currentDb.c_str()), static_cast<SQLSMALLINT>(currentDb.length()),
                       sqlchar(schemaName.c_str()), static_cast<SQLSMALLINT>(schemaName.length()),
                       sqlchar(table.c_str()), static_cast<SQLSMALLINT>(table.length()));
  REQUIRE(ret == SQL_SUCCESS);
  int count2 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count2++;
  REQUIRE(count2 == 1);
}

// ============================================================================
// SQLPrimaryKeys - Statement Reuse
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPrimaryKeys: Can call multiple times on same statement after close cursor",
                 "[odbc-api][primarykeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("CREATE TABLE test_pk_reuse (id INT PRIMARY KEY)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());

  ret = SQLPrimaryKeys(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()), SQL_NTS,
                       sqlchar("TEST_PK_REUSE"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  int count1 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count1++;
  REQUIRE(count1 == 1);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLPrimaryKeys(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()), SQL_NTS,
                       sqlchar("TEST_PK_REUSE"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  int count2 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count2++;
  REQUIRE(count2 == 1);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPrimaryKeys: SQLRowCount after catalog function call",
                 "[odbc-api][primarykeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("CREATE TABLE test_pk_rowcount (id INT PRIMARY KEY)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());

  ret = SQLPrimaryKeys(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()), SQL_NTS,
                       sqlchar("TEST_PK_ROWCOUNT"), SQL_NTS);
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLPrimaryKeys: 24000 - Cursor already open",
                 "[odbc-api][primarykeys][catalog][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("CREATE TABLE test_pk_cursor (id INT PRIMARY KEY)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());

  ret = SQLPrimaryKeys(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()), SQL_NTS,
                       sqlchar("TEST_PK_CURSOR"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // Second call without closing cursor
  ret = SQLPrimaryKeys(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()), SQL_NTS,
                       sqlchar("TEST_PK_CURSOR"), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(DbcFixture, "SQLPrimaryKeys: Requires active connection", "[odbc-api][primarykeys][catalog][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHSTMT stmt = SQL_NULL_HSTMT;
  const SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);

  // Note: Reference driver refuses to allocate statement on disconnected handle
  REQUIRE(ret == SQL_ERROR);
}
