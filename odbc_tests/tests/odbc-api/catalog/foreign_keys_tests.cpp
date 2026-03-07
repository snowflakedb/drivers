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
// SQLForeignKeys - Result Set Structure
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLForeignKeys: Result set has correct number of columns",
                 "[odbc-api][foreignkeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("CREATE TABLE test_fk_parent (id INT PRIMARY KEY)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  ret = SQLExecDirect(
      stmt_handle(),
      sqlchar(
          "CREATE TABLE test_fk_child (id INT, parent_id INT, FOREIGN KEY (parent_id) REFERENCES test_fk_parent(id))"),
      SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());

  // Query FK table to get foreign keys
  ret = SQLForeignKeys(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0, sqlchar(currentDb.c_str()), SQL_NTS,
                       sqlchar(schema.name().c_str()), SQL_NTS, sqlchar("TEST_FK_CHILD"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // ODBC 3.x spec defines 14 columns for SQLForeignKeys
  SQLSMALLINT numCols = 0;
  ret = SQLNumResultCols(stmt_handle(), &numCols);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(numCols == 14);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLForeignKeys: Result set column names match ODBC 3.x spec",
                 "[odbc-api][foreignkeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("CREATE TABLE test_fk_p_names (id INT PRIMARY KEY)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  ret = SQLExecDirect(
      stmt_handle(),
      sqlchar("CREATE TABLE test_fk_c_names (id INT, pid INT, FOREIGN KEY (pid) REFERENCES test_fk_p_names(id))"),
      SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());

  ret = SQLForeignKeys(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0, sqlchar(currentDb.c_str()), SQL_NTS,
                       sqlchar(schema.name().c_str()), SQL_NTS, sqlchar("TEST_FK_C_NAMES"), SQL_NTS);
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLForeignKeys: FK table returns foreign key referencing PK table",
                 "[odbc-api][foreignkeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret =
      SQLExecDirect(stmt_handle(), sqlchar("CREATE TABLE test_fk_orders_parent (order_id INT PRIMARY KEY)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  ret = SQLExecDirect(stmt_handle(),
                      sqlchar("CREATE TABLE test_fk_lines (line_id INT, order_id INT, FOREIGN KEY "
                              "(order_id) REFERENCES test_fk_orders_parent(order_id))"),
                      SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());

  // Query by FK table: what foreign keys does test_fk_lines have?
  ret = SQLForeignKeys(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0, sqlchar(currentDb.c_str()), SQL_NTS,
                       sqlchar(schema.name().c_str()), SQL_NTS, sqlchar("TEST_FK_LINES"), SQL_NTS);
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

  REQUIRE(std::string(pkTableName) == "TEST_FK_ORDERS_PARENT");
  REQUIRE(std::string(pkColumnName) == "ORDER_ID");
  REQUIRE(std::string(fkTableName) == "TEST_FK_LINES");
  REQUIRE(std::string(fkColumnName) == "ORDER_ID");
  REQUIRE(keySeq == 1);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLForeignKeys: PK table returns foreign keys referencing it",
                 "[odbc-api][foreignkeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("CREATE TABLE test_fk_pk_parent (id INT PRIMARY KEY)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  ret = SQLExecDirect(stmt_handle(),
                      sqlchar("CREATE TABLE test_fk_pk_child (id INT, parent_id INT, FOREIGN KEY "
                              "(parent_id) REFERENCES test_fk_pk_parent(id))"),
                      SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());

  // Query by PK table: what tables reference test_fk_pk_parent's primary key?
  ret = SQLForeignKeys(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()), SQL_NTS,
                       sqlchar("TEST_FK_PK_PARENT"), SQL_NTS, nullptr, 0, nullptr, 0, nullptr, 0);
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

  REQUIRE(std::string(pkTableName) == "TEST_FK_PK_PARENT");
  REQUIRE(std::string(pkColumnName) == "ID");
  REQUIRE(std::string(fkTableName) == "TEST_FK_PK_CHILD");
  REQUIRE(std::string(fkColumnName) == "PARENT_ID");

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLForeignKeys: Both PK and FK table specified returns matching relationship",
                 "[odbc-api][foreignkeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("CREATE TABLE test_fk_both_pk (id INT PRIMARY KEY)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  ret = SQLExecDirect(
      stmt_handle(),
      sqlchar("CREATE TABLE test_fk_both_fk (id INT, ref_id INT, FOREIGN KEY (ref_id) REFERENCES test_fk_both_pk(id))"),
      SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());

  ret = SQLForeignKeys(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()), SQL_NTS,
                       sqlchar("TEST_FK_BOTH_PK"), SQL_NTS, sqlchar(currentDb.c_str()), SQL_NTS,
                       sqlchar(schema.name().c_str()), SQL_NTS, sqlchar("TEST_FK_BOTH_FK"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  char pkTableName[256] = {};
  char fkTableName[256] = {};
  SQLGetData(stmt_handle(), 3, SQL_C_CHAR, pkTableName, sizeof(pkTableName), nullptr);
  SQLGetData(stmt_handle(), 7, SQL_C_CHAR, fkTableName, sizeof(fkTableName), nullptr);

  REQUIRE(std::string(pkTableName) == "TEST_FK_BOTH_PK");
  REQUIRE(std::string(fkTableName) == "TEST_FK_BOTH_FK");

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture,
                 "SQLForeignKeys: PK table referenced by multiple children returns all relationships",
                 "[odbc-api][foreignkeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret =
      SQLExecDirect(stmt_handle(), sqlchar("CREATE TABLE test_fk_multi_parent (id INT PRIMARY KEY)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  ret = SQLExecDirect(stmt_handle(),
                      sqlchar("CREATE TABLE test_fk_multi_child_a (id INT, parent_id INT, "
                              "FOREIGN KEY (parent_id) REFERENCES test_fk_multi_parent(id))"),
                      SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  ret = SQLExecDirect(stmt_handle(),
                      sqlchar("CREATE TABLE test_fk_multi_child_b (id INT, ref_id INT, "
                              "FOREIGN KEY (ref_id) REFERENCES test_fk_multi_parent(id))"),
                      SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());

  // Query by PK table: both children should appear
  ret = SQLForeignKeys(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()), SQL_NTS,
                       sqlchar("TEST_FK_MULTI_PARENT"), SQL_NTS, nullptr, 0, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  int rowCount = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS) {
    char fkTable[256] = {};
    SQLGetData(stmt_handle(), 7, SQL_C_CHAR, fkTable, sizeof(fkTable), nullptr);
    rowCount++;
  }
  REQUIRE(rowCount == 2);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLForeignKeys: Table without foreign keys returns empty result set",
                 "[odbc-api][foreignkeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret =
      SQLExecDirect(stmt_handle(), sqlchar("CREATE TABLE test_fk_nofk (id INT, name VARCHAR(50))"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());

  ret = SQLForeignKeys(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0, sqlchar(currentDb.c_str()), SQL_NTS,
                       sqlchar(schema.name().c_str()), SQL_NTS, sqlchar("TEST_FK_NOFK"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLForeignKeys: Non-existent table returns empty result set",
                 "[odbc-api][foreignkeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  const std::string currentDb = get_current_database(dbc_handle());

  SQLRETURN ret =
      SQLForeignKeys(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0, sqlchar(currentDb.c_str()), SQL_NTS,
                     sqlchar(schema.name().c_str()), SQL_NTS, sqlchar("NONEXISTENT_TABLE_XYZ_99999"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

// ============================================================================
// SQLForeignKeys - Statement Reuse
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLForeignKeys: Can call multiple times on same statement after close cursor",
                 "[odbc-api][foreignkeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("CREATE TABLE test_fk_reuse_pk (id INT PRIMARY KEY)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  ret = SQLExecDirect(
      stmt_handle(),
      sqlchar(
          "CREATE TABLE test_fk_reuse_fk (id INT, ref_id INT, FOREIGN KEY (ref_id) REFERENCES test_fk_reuse_pk(id))"),
      SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());

  ret = SQLForeignKeys(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0, sqlchar(currentDb.c_str()), SQL_NTS,
                       sqlchar(schema.name().c_str()), SQL_NTS, sqlchar("TEST_FK_REUSE_FK"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  int count1 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count1++;
  REQUIRE(count1 == 1);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLForeignKeys(stmt_handle(), nullptr, 0, nullptr, 0, nullptr, 0, sqlchar(currentDb.c_str()), SQL_NTS,
                       sqlchar(schema.name().c_str()), SQL_NTS, sqlchar("TEST_FK_REUSE_FK"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  int count2 = 0;
  while (SQLFetch(stmt_handle()) == SQL_SUCCESS)
    count2++;
  REQUIRE(count2 == 1);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLForeignKeys: SQLRowCount after catalog function call",
                 "[odbc-api][foreignkeys][catalog]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("CREATE TABLE test_fk_rc_pk (id INT PRIMARY KEY)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());

  ret = SQLForeignKeys(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()), SQL_NTS,
                       sqlchar("TEST_FK_RC_PK"), SQL_NTS, nullptr, 0, nullptr, 0, nullptr, 0);
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLForeignKeys: 24000 - Cursor already open",
                 "[odbc-api][foreignkeys][catalog][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("CREATE TABLE test_fk_cursor_pk (id INT PRIMARY KEY)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  const std::string currentDb = get_current_database(dbc_handle());

  ret = SQLForeignKeys(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()), SQL_NTS,
                       sqlchar("TEST_FK_CURSOR_PK"), SQL_NTS, nullptr, 0, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Second call without closing cursor
  ret = SQLForeignKeys(stmt_handle(), sqlchar(currentDb.c_str()), SQL_NTS, sqlchar(schema.name().c_str()), SQL_NTS,
                       sqlchar("TEST_FK_CURSOR_PK"), SQL_NTS, nullptr, 0, nullptr, 0, nullptr, 0);
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(DbcFixture, "SQLForeignKeys: Requires active connection", "[odbc-api][foreignkeys][catalog][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHSTMT stmt = SQL_NULL_HSTMT;
  const SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);

  // Note: Reference driver refuses to allocate statement on disconnected handle
  REQUIRE(ret == SQL_ERROR);
}
