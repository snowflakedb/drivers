#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "ODBCFixtures.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

// ============================================================================
// SQLExecute - Basic Functionality
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecute: Executes prepared SELECT and returns result set",
                 "[odbc-api][execute][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 42 AS val"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER result = 0;
  SQLLEN ind = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &result, 0, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(result == 42);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecute: Executes prepared DDL statement and table is queryable",
                 "[odbc-api][execute][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  std::string sql = "CREATE TABLE " + schema.name() + ".ex_ddl_t(c1 INTEGER)";
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "SELECT c1 FROM " + schema.name() + ".ex_ddl_t";
  ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "DROP TABLE " + schema.name() + ".ex_ddl_t";
  ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecute: INSERT returns correct SQLRowCount and inserts rows",
                 "[odbc-api][execute][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  std::string sql = "CREATE TABLE " + schema.name() + ".ex_ins_t(c1 INTEGER)";
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "INSERT INTO " + schema.name() + ".ex_ins_t VALUES(1),(2),(3)";
  ret = SQLPrepare(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN rowCount = -1;
  ret = SQLRowCount(stmt_handle(), &rowCount);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(rowCount == 3);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "SELECT c1 FROM " + schema.name() + ".ex_ins_t ORDER BY c1";
  ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  SQLLEN ind = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &val, 0, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 1);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 2);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 3);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecute: UPDATE returns correct SQLRowCount and updates rows",
                 "[odbc-api][execute][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  std::string sql = "CREATE TABLE " + schema.name() + ".ex_upd_t(c1 INTEGER)";
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "INSERT INTO " + schema.name() + ".ex_upd_t VALUES(1),(2),(3)";
  ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "UPDATE " + schema.name() + ".ex_upd_t SET c1 = c1 + 10";
  ret = SQLPrepare(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN rowCount = -1;
  ret = SQLRowCount(stmt_handle(), &rowCount);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(rowCount == 3);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "SELECT c1 FROM " + schema.name() + ".ex_upd_t ORDER BY c1";
  ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  SQLLEN ind = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &val, 0, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 11);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 12);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 13);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecute: DELETE returns correct SQLRowCount and removes rows",
                 "[odbc-api][execute][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  std::string sql = "CREATE TABLE " + schema.name() + ".ex_del_t(c1 INTEGER)";
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "INSERT INTO " + schema.name() + ".ex_del_t VALUES(1),(2),(3)";
  ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "DELETE FROM " + schema.name() + ".ex_del_t WHERE c1 IN (2, 3)";
  ret = SQLPrepare(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN rowCount = -1;
  ret = SQLRowCount(stmt_handle(), &rowCount);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(rowCount == 2);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "SELECT c1 FROM " + schema.name() + ".ex_del_t ORDER BY c1";
  ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  SQLLEN ind = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &val, 0, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 1);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

// ============================================================================
// SQLExecute - SQL_NO_DATA
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecute: SQL_NO_DATA for DML affecting zero rows",
                 "[odbc-api][execute][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());
  std::string dml_sql;

  SECTION("DELETE affecting zero rows") {
    std::string create_sql = "CREATE TABLE " + schema.name() + ".ex_nod_t(c1 INTEGER)";
    SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar(create_sql.c_str()), SQL_NTS);
    REQUIRE(ret == SQL_SUCCESS);
    SQLFreeStmt(stmt_handle(), SQL_CLOSE);

    dml_sql = "DELETE FROM " + schema.name() + ".ex_nod_t WHERE c1 = 999";
  }

  SECTION("UPDATE affecting zero rows") {
    std::string create_sql = "CREATE TABLE " + schema.name() + ".ex_nou_t(c1 INTEGER)";
    SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar(create_sql.c_str()), SQL_NTS);
    REQUIRE(ret == SQL_SUCCESS);
    SQLFreeStmt(stmt_handle(), SQL_CLOSE);

    dml_sql = "UPDATE " + schema.name() + ".ex_nou_t SET c1 = 2 WHERE c1 = 999";
  }

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar(dml_sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);

  SQLLEN rowCount = -1;
  ret = SQLRowCount(stmt_handle(), &rowCount);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(rowCount == 0);
}

// ============================================================================
// SQLExecute - Re-execution (key advantage over SQLExecDirect)
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecute: Re-executes SELECT with updated parameter value",
                 "[odbc-api][execute][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLINTEGER param = 10;
  SQLLEN ind = 0;

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param, 0, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER result = 0;
  SQLLEN rind = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &result, 0, &rind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(result == 10);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  param = 99;
  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(result == 99);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecute: Re-executes INSERT with different parameter each time",
                 "[odbc-api][execute][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  std::string sql = "CREATE TABLE " + schema.name() + ".ex_reins_t(c1 INTEGER)";
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "INSERT INTO " + schema.name() + ".ex_reins_t VALUES(?)";
  ret = SQLPrepare(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER param = 0;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param, 0, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  for (SQLINTEGER i = 1; i <= 3; i++) {
    param = i;
    ret = SQLExecute(stmt_handle());
    REQUIRE(ret == SQL_SUCCESS);
    SQLLEN rowCount = -1;
    ret = SQLRowCount(stmt_handle(), &rowCount);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(rowCount == 1);
  }

  sql = "SELECT c1 FROM " + schema.name() + ".ex_reins_t ORDER BY c1";
  ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  SQLLEN vind = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &val, 0, &vind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 1);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 2);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(val == 3);
  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecute: Re-executes SELECT after SQLCloseCursor",
                 "[odbc-api][execute][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 1 AS val"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  SQLCloseCursor(stmt_handle());

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER result = 0;
  SQLLEN ind = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &result, 0, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(result == 1);
}

// ============================================================================
// SQLExecute - With Bound Parameters
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecute: Executes with bound parameter",
                 "[odbc-api][execute][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLINTEGER param_val = 77;
  SQLLEN ind = 0;
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param_val, 0, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER result = 0;
  SQLLEN rind = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &result, 0, &rind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(result == 77);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecute: SQL_NEED_DATA with data-at-execution parameter",
                 "[odbc-api][execute][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 100, 0,
                         reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  SQLCancel(stmt_handle());
}

// ============================================================================
// SQLExecute - Error Cases
// ============================================================================

TEST_CASE("SQLExecute: SQL_INVALID_HANDLE for null statement handle",
          "[odbc-api][execute][submitting_request][error]") {
  const SQLRETURN ret = SQLExecute(SQL_NULL_HSTMT);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecute: HY010 when statement not prepared",
                 "[odbc-api][execute][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecute(stmt_handle());
  REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecute: HY010 during SQL_NEED_DATA",
                 "[odbc-api][execute][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 100, 0,
                         reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  ret = SQLExecute(stmt_handle());
  REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);

  SQLCancel(stmt_handle());
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecute: 24000 for cursor already open",
                 "[odbc-api][execute][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);

  SQLFreeStmt(stmt_handle(), SQL_CLOSE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecute: 22012 for division by zero",
                 "[odbc-api][execute][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 1/0"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE_EXPECTED_ERROR(ret, "22012", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecute: 42S02 for table not found",
                 "[odbc-api][execute][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  std::string sql = "SELECT * FROM " + schema.name() + ".nonexistent_table";

  // Note: Snowflake validates table existence at prepare time, so the 42S02
  // error may be raised by SQLPrepare rather than SQLExecute.
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  if (ret == SQL_SUCCESS) {
    ret = SQLExecute(stmt_handle());
  }
  REQUIRE_EXPECTED_ERROR(ret, "42S02", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecute: 07002 for parameter count mismatch",
                 "[odbc-api][execute][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?, ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 1;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &val, 0, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE_EXPECTED_ERROR(ret, "07002", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecute: 22000 for NOT NULL constraint violation",
                 "[odbc-api][execute][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  std::string sql = "CREATE TABLE " + schema.name() + ".ex_nn_t(c1 INTEGER NOT NULL)";
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  // Note: The reference driver returns 22000 instead of 23000 in ODBC spec
  // for integrity constraint violations.
  sql = "INSERT INTO " + schema.name() + ".ex_nn_t VALUES(NULL)";
  ret = SQLPrepare(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE_EXPECTED_ERROR(ret, "22000", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecute: 42710 for table already exists",
                 "[odbc-api][execute][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  std::string sql = "CREATE TABLE " + schema.name() + ".ex_dup_t(c1 INTEGER)";
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  // Note: Snowflake validates DDL at prepare time, so the 42710 error may be
  // raised by SQLPrepare rather than SQLExecute. Also, the reference driver
  // returns 42710 instead of 42S01 in ODBC spec for base table already exists.
  SQLRETURN ret2 = SQLPrepare(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  if (ret2 == SQL_SUCCESS) {
    ret2 = SQLExecute(stmt_handle());
  }
  REQUIRE_EXPECTED_ERROR(ret2, "42710", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecute: 21S01 for INSERT column count mismatch",
                 "[odbc-api][execute][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  std::string sql = "CREATE TABLE " + schema.name() + ".ex_mis_t(c1 INTEGER)";
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "INSERT INTO " + schema.name() + ".ex_mis_t(c1) VALUES(1, 2)";

  // Note: Snowflake validates column counts at prepare time, so the 21S01
  // error may be raised by SQLPrepare rather than SQLExecute.
  SQLRETURN ret2 = SQLPrepare(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  if (ret2 == SQL_SUCCESS) {
    ret2 = SQLExecute(stmt_handle());
  }
  REQUIRE_EXPECTED_ERROR(ret2, "21S01", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecute: 42601 for CREATE VIEW column list mismatch",
                 "[odbc-api][execute][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  std::string sql = "CREATE VIEW " + schema.name() + ".ex_vm_v (a, b) AS SELECT 1";

  // Note: Snowflake validates at prepare time. The reference driver returns
  // 42601 instead of 21S02 in the ODBC spec for a CREATE VIEW where the
  // column list has more names than the SELECT produces.
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  if (ret == SQL_SUCCESS) {
    ret = SQLExecute(stmt_handle());
  }
  REQUIRE_EXPECTED_ERROR(ret, "42601", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecute: 22023 for invalid LIKE escape character",
                 "[odbc-api][execute][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Note: Snowflake validates at prepare time. The reference driver returns
  // 22023 instead of 22019 in the ODBC spec for a LIKE predicate with an
  // ESCAPE clause where the escape character is not exactly one character long.
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 1 WHERE 'abc' LIKE 'a%' ESCAPE 'xy'"), SQL_NTS);
  if (ret == SQL_SUCCESS) {
    ret = SQLExecute(stmt_handle());
  }
  REQUIRE_EXPECTED_ERROR(ret, "22023", stmt_handle(), SQL_HANDLE_STMT);
}
