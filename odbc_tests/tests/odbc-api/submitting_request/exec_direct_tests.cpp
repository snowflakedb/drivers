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
#include "test_macros.hpp"
#include "test_setup.hpp"

// ============================================================================
// SQLExecDirect - Basic Functionality
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecDirect: Executes SELECT and returns result set",
                 "[odbc-api][execdirect][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42 AS val"), SQL_NTS);
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecDirect: Executes DDL statement and table is queryable",
                 "[odbc-api][execdirect][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  std::string sql = "CREATE TABLE " + schema.name() + ".ed_ddl_t(c1 INTEGER)";
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "SELECT c1 FROM " + schema.name() + ".ed_ddl_t";
  ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "DROP TABLE " + schema.name() + ".ed_ddl_t";
  ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecDirect: INSERT returns correct SQLRowCount and inserts rows",
                 "[odbc-api][execdirect][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  std::string sql = "CREATE TABLE " + schema.name() + ".ed_ins_t(c1 INTEGER)";
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "INSERT INTO " + schema.name() + ".ed_ins_t VALUES(1),(2),(3)";
  ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN rowCount = -1;
  ret = SQLRowCount(stmt_handle(), &rowCount);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(rowCount == 3);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "SELECT c1 FROM " + schema.name() + ".ed_ins_t ORDER BY c1";
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecDirect: UPDATE returns correct SQLRowCount and updates rows",
                 "[odbc-api][execdirect][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  std::string sql = "CREATE TABLE " + schema.name() + ".ed_upd_t(c1 INTEGER)";
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "INSERT INTO " + schema.name() + ".ed_upd_t VALUES(1),(2),(3)";
  ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "UPDATE " + schema.name() + ".ed_upd_t SET c1 = c1 + 10";
  ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN rowCount = -1;
  ret = SQLRowCount(stmt_handle(), &rowCount);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(rowCount == 3);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "SELECT c1 FROM " + schema.name() + ".ed_upd_t ORDER BY c1";
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecDirect: DELETE returns correct SQLRowCount and removes rows",
                 "[odbc-api][execdirect][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  std::string sql = "CREATE TABLE " + schema.name() + ".ed_del_t(c1 INTEGER)";
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "INSERT INTO " + schema.name() + ".ed_del_t VALUES(1),(2),(3)";
  ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "DELETE FROM " + schema.name() + ".ed_del_t WHERE c1 IN (2, 3)";
  ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN rowCount = -1;
  ret = SQLRowCount(stmt_handle(), &rowCount);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(rowCount == 2);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "SELECT c1 FROM " + schema.name() + ".ed_del_t ORDER BY c1";
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
// SQLExecDirect - SQL_NO_DATA
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecDirect: SQL_NO_DATA for DML affecting zero rows",
                 "[odbc-api][execdirect][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  // TODO: Restore SECTIONs once ConfigInstallation supports re-entry within sections
  {
    std::string create_sql = "CREATE TABLE " + schema.name() + ".ed_nod_t(c1 INTEGER)";
    SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar(create_sql.c_str()), SQL_NTS);
    REQUIRE(ret == SQL_SUCCESS);
    SQLFreeStmt(stmt_handle(), SQL_CLOSE);

    std::string dml_sql = "DELETE FROM " + schema.name() + ".ed_nod_t WHERE c1 = 999";
    ret = SQLExecDirect(stmt_handle(), sqlchar(dml_sql.c_str()), SQL_NTS);
    REQUIRE(ret == SQL_NO_DATA);

    SQLLEN rowCount = -1;
    ret = SQLRowCount(stmt_handle(), &rowCount);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(rowCount == 0);
  }

  {
    std::string create_sql = "CREATE TABLE " + schema.name() + ".ed_nou_t(c1 INTEGER)";
    SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar(create_sql.c_str()), SQL_NTS);
    REQUIRE(ret == SQL_SUCCESS);
    SQLFreeStmt(stmt_handle(), SQL_CLOSE);

    std::string dml_sql = "UPDATE " + schema.name() + ".ed_nou_t SET c1 = 2 WHERE c1 = 999";
    ret = SQLExecDirect(stmt_handle(), sqlchar(dml_sql.c_str()), SQL_NTS);
    REQUIRE(ret == SQL_NO_DATA);

    SQLLEN rowCount = -1;
    ret = SQLRowCount(stmt_handle(), &rowCount);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(rowCount == 0);
  }
}

// ============================================================================
// SQLExecDirect - TextLength and Statement Reuse
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecDirect: Explicit TextLength instead of SQL_NTS",
                 "[odbc-api][execdirect][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto sql = "SELECT 99 AS val";
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar(sql), static_cast<SQLINTEGER>(strlen(sql)));
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER result = 0;
  SQLLEN ind = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &result, 0, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(result == 99);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecDirect: Multiple executions on same statement after close cursor",
                 "[odbc-api][execdirect][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1 AS val"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 2 AS val"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER result = 0;
  SQLLEN ind = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &result, 0, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(result == 2);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecDirect: TextLength truncates SQL to shorter valid statement",
                 "[odbc-api][execdirect][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // TextLength=9 truncates "SELECT 42 AS val" to "SELECT 42", so the column
  // alias "val" is never sent. The column name comes back as "42", not "VAL".
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42 AS val"), 9);
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR col_name[64] = {};
  SQLSMALLINT name_len = 0;
  ret = SQLDescribeCol(stmt_handle(), 1, col_name, sizeof(col_name), &name_len, nullptr, nullptr, nullptr, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(reinterpret_cast<char*>(col_name)) == "42");

  SQLINTEGER result = 0;
  SQLLEN ind = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &result, 0, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(result == 42);
}

// ============================================================================
// SQLExecDirect - With Bound Parameters
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecDirect: Executes with bound parameter",
                 "[odbc-api][execdirect][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLINTEGER param_val = 77;
  SQLLEN ind = 0;
  SQLRETURN ret =
      SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param_val, 0, &ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER result = 0;
  SQLLEN rind = 0;
  ret = SQLBindCol(stmt_handle(), 1, SQL_C_SLONG, &result, 0, &rind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(result == 77);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecDirect: SQL_NEED_DATA with data-at-execution parameter",
                 "[odbc-api][execdirect][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  SQLRETURN ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 100, 0,
                                   reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_NEED_DATA);

  SQLCancel(stmt_handle());
}

// ============================================================================
// SQLExecDirect - Error Cases
// ============================================================================

TEST_CASE("SQLExecDirect: SQL_INVALID_HANDLE for null statement handle",
          "[odbc-api][execdirect][submitting_request][error]") {
  const SQLRETURN ret = SQLExecDirect(SQL_NULL_HSTMT, sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecDirect: HY009 for null StatementText",
                 "[odbc-api][execdirect][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), nullptr, SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "HY009", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecDirect: HY090 for negative TextLength",
                 "[odbc-api][execdirect][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), -99);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecDirect: HY090 for TextLength zero",
                 "[odbc-api][execdirect][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), 0);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecDirect: HY010 during SQL_NEED_DATA",
                 "[odbc-api][execdirect][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 100, 0,
                         reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);

  SQLCancel(stmt_handle());
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecDirect: 24000 for cursor already open",
                 "[odbc-api][execdirect][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 2"), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);

  SQLFreeStmt(stmt_handle(), SQL_CLOSE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecDirect: 42000 for syntax error",
                 "[odbc-api][execdirect][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SLECT 1"), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "42000", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecDirect: 42S02 for table not found",
                 "[odbc-api][execdirect][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  std::string sql = "SELECT * FROM " + schema.name() + ".nonexistent_table";
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "42S02", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecDirect: 22012 for division by zero",
                 "[odbc-api][execdirect][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1/0"), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "22012", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecDirect: 21S01 for INSERT column count mismatch",
                 "[odbc-api][execdirect][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  std::string sql = "CREATE TABLE " + schema.name() + ".ed_mis_t(c1 INTEGER)";
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "INSERT INTO " + schema.name() + ".ed_mis_t(c1) VALUES(1, 2)";
  ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "21S01", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecDirect: 22000 for NOT NULL constraint violation",
                 "[odbc-api][execdirect][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  std::string sql = "CREATE TABLE " + schema.name() + ".ed_nn_t(c1 INTEGER NOT NULL)";
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  // Note: The reference driver returns 22000 instead of 23000 in ODBC spec
  // for integrity constraint violations.
  sql = "INSERT INTO " + schema.name() + ".ed_nn_t VALUES(NULL)";
  ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "22000", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecDirect: 42710 for table already exists",
                 "[odbc-api][execdirect][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  std::string sql = "CREATE TABLE " + schema.name() + ".ed_dup_t(c1 INTEGER)";
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  // Note: The reference driver returns 42710 instead of 42S01 in ODBC spec
  // for base table or view already exists.
  ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "42710", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecDirect: 42601 for CREATE VIEW column list mismatch",
                 "[odbc-api][execdirect][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const auto schema = Schema::use_random_schema(dbc_handle());

  // Note: The reference driver returns 42601 instead of 21S02 in the ODBC
  // spec for a CREATE VIEW where the column list has more names than the
  // SELECT produces.
  std::string sql = "CREATE VIEW " + schema.name() + ".ed_vm_v (a, b) AS SELECT 1";
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "42601", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecDirect: 22023 for invalid LIKE escape character",
                 "[odbc-api][execdirect][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Note: The reference driver returns 22023 instead of 22019 in the ODBC
  // spec for a LIKE predicate with an ESCAPE clause where the escape
  // character is not exactly one character long.
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1 WHERE 'abc' LIKE 'a%' ESCAPE 'xy'"), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "22023", stmt_handle(), SQL_HANDLE_STMT);
}
