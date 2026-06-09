#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "ODBCFixtures.hpp"
#include "SchemaFixtures.hpp"
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

TEST_CASE_METHOD(StmtSessionSchemaFixture, "SQLExecDirect: Executes DDL statement and table is queryable",
                 "[odbc-api][execdirect][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  std::string sql = "CREATE TABLE ed_ddl_t(c1 INTEGER)";
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "SELECT c1 FROM ed_ddl_t";
  ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_NO_DATA);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "DROP TABLE ed_ddl_t";
  ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(StmtSessionSchemaFixture, "SQLExecDirect: INSERT returns correct SQLRowCount and inserts rows",
                 "[odbc-api][execdirect][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  std::string sql = "CREATE TEMPORARY TABLE ed_ins_t(c1 INTEGER)";
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "INSERT INTO ed_ins_t VALUES(1),(2),(3)";
  ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN rowCount = -1;
  ret = SQLRowCount(stmt_handle(), &rowCount);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(rowCount == 3);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "SELECT c1 FROM ed_ins_t ORDER BY c1";
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

TEST_CASE_METHOD(StmtSessionSchemaFixture, "SQLExecDirect: UPDATE returns correct SQLRowCount and updates rows",
                 "[odbc-api][execdirect][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  std::string sql = "CREATE TEMPORARY TABLE ed_upd_t(c1 INTEGER)";
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "INSERT INTO ed_upd_t VALUES(1),(2),(3)";
  ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "UPDATE ed_upd_t SET c1 = c1 + 10";
  ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN rowCount = -1;
  ret = SQLRowCount(stmt_handle(), &rowCount);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(rowCount == 3);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "SELECT c1 FROM ed_upd_t ORDER BY c1";
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

TEST_CASE_METHOD(StmtSessionSchemaFixture, "SQLExecDirect: DELETE returns correct SQLRowCount and removes rows",
                 "[odbc-api][execdirect][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  std::string sql = "CREATE TEMPORARY TABLE ed_del_t(c1 INTEGER)";
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "INSERT INTO ed_del_t VALUES(1),(2),(3)";
  ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "DELETE FROM ed_del_t WHERE c1 IN (2, 3)";
  ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN rowCount = -1;
  ret = SQLRowCount(stmt_handle(), &rowCount);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(rowCount == 2);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "SELECT c1 FROM ed_del_t ORDER BY c1";
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

TEST_CASE_METHOD(StmtSessionSchemaFixture, "SQLExecDirect: SQL_NO_DATA for DML affecting zero rows",
                 "[odbc-api][execdirect][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // TODO: Restore SECTIONs once ConfigInstallation supports re-entry within sections
  {
    std::string create_sql = "CREATE TEMPORARY TABLE ed_nod_t(c1 INTEGER)";
    SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar(create_sql.c_str()), SQL_NTS);
    REQUIRE(ret == SQL_SUCCESS);
    SQLFreeStmt(stmt_handle(), SQL_CLOSE);

    std::string dml_sql = "DELETE FROM ed_nod_t WHERE c1 = 999";
    ret = SQLExecDirect(stmt_handle(), sqlchar(dml_sql.c_str()), SQL_NTS);
    REQUIRE(ret == SQL_NO_DATA);

    SQLLEN rowCount = -1;
    ret = SQLRowCount(stmt_handle(), &rowCount);
    OLD_IODBC_ONLY("BD#61") {
      // Under iODBC the old driver does not advance the statement state into a
      //   form SQLRowCount can read after SQL_NO_DATA, so the call surfaces
      //   SQL_ERROR. Under unixODBC the same call returns SQL_SUCCESS with
      //   rowCount=0.
      REQUIRE(ret == SQL_ERROR);
    }
    else {
      REQUIRE(ret == SQL_SUCCESS);
      REQUIRE(rowCount == 0);
    }
  }

  {
    std::string create_sql = "CREATE TEMPORARY TABLE ed_nou_t(c1 INTEGER)";
    SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar(create_sql.c_str()), SQL_NTS);
    REQUIRE(ret == SQL_SUCCESS);
    SQLFreeStmt(stmt_handle(), SQL_CLOSE);

    std::string dml_sql = "UPDATE ed_nou_t SET c1 = 2 WHERE c1 = 999";
    ret = SQLExecDirect(stmt_handle(), sqlchar(dml_sql.c_str()), SQL_NTS);
    REQUIRE(ret == SQL_NO_DATA);

    SQLLEN rowCount = -1;
    ret = SQLRowCount(stmt_handle(), &rowCount);
    OLD_IODBC_ONLY("BD#61") {
      // See "DELETE" case above: SQLRowCount after SQL_NO_DATA on the old
      //   driver under iODBC returns SQL_ERROR rather than 0.
      REQUIRE(ret == SQL_ERROR);
    }
    else {
      REQUIRE(ret == SQL_SUCCESS);
      REQUIRE(rowCount == 0);
    }
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
  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  SQLRETURN ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 100, 0,
                                   reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_NEED_DATA);

  SQLCancel(stmt_handle());
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecDirect: SQL_NEED_DATA even when SQL has no parameter markers",
                 "[odbc-api][execdirect][submitting_request]") {
  // DAE detection for exec-direct is based solely on APD bindings, not on
  // parsing the SQL for '?' markers.  A spurious DAE binding on a query with
  // no markers still triggers SQL_NEED_DATA (matches reference driver).
  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  SQLRETURN ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 100, 0,
                                   reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecDirect: HY090 for TextLength zero",
                 "[odbc-api][execdirect][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), 0);
  OLD_IODBC_ONLY("BD#60") {
    // iODBC's DM mangles negative-length / empty-string parameters before
    //   forwarding them to the old driver, which then surfaces HY000
    //   instead of the spec-mandated HY090. unixODBC passes the arg through
    //   unchanged, so the driver's HY090 validation fires.
    REQUIRE_EXPECTED_ERROR(ret, "HY000", stmt_handle(), SQL_HANDLE_STMT);
  }
  else {
    REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
  }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecDirect: HY010 during SQL_NEED_DATA",
                 "[odbc-api][execdirect][submitting_request][error]") {
  // Given a prepared statement with a SQL_DATA_AT_EXEC parameter whose execution has
  // entered the SQL_NEED_DATA state (waiting for SQLPutData)
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 100, 0,
                         reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  // When SQLExecDirect is called on the same statement while it is in SQL_NEED_DATA
  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  // Then DM surfaces HY010
  REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);

  // And the statement is cancelled to release any pending state
  SQLCancel(stmt_handle());
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecDirect: 24000 for cursor already open",
                 "[odbc-api][execdirect][submitting_request][error]") {
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

TEST_CASE_METHOD(StmtSessionSchemaFixture, "SQLExecDirect: 42S02 for table not found",
                 "[odbc-api][execdirect][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  std::string sql = "SELECT * FROM nonexistent_table";
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "42S02", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLExecDirect: 22012 for division by zero",
                 "[odbc-api][execdirect][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1/0"), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "22012", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtSessionSchemaFixture, "SQLExecDirect: 21S01 for INSERT column count mismatch",
                 "[odbc-api][execdirect][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  std::string sql = "CREATE TEMPORARY TABLE ed_mis_t(c1 INTEGER)";
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  sql = "INSERT INTO ed_mis_t(c1) VALUES(1, 2)";
  ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "21S01", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtSessionSchemaFixture, "SQLExecDirect: 22000 for NOT NULL constraint violation",
                 "[odbc-api][execdirect][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  std::string sql = "CREATE TEMPORARY TABLE ed_nn_t(c1 INTEGER NOT NULL)";
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  // Note: The reference driver returns 22000 instead of 23000 in ODBC spec
  // for integrity constraint violations.
  sql = "INSERT INTO ed_nn_t VALUES(NULL)";
  ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_ERROR);
  OLD_IODBC_ONLY("BD#70") {
    // Old driver on iODBC has been observed to surface either 22000 or HY000
    // for the same server-side NOT NULL violation.
    const std::string state = get_sqlstate(SQL_HANDLE_STMT, stmt_handle());
    CHECK((state == "22000" || state == "HY000"));
  }
  else {
    CHECK(get_sqlstate(SQL_HANDLE_STMT, stmt_handle()) == "22000");
  }
}

TEST_CASE_METHOD(StmtSessionSchemaFixture, "SQLExecDirect: 42710 for table already exists",
                 "[odbc-api][execdirect][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  std::string sql = "CREATE TABLE ed_dup_t(c1 INTEGER)";
  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  SQLFreeStmt(stmt_handle(), SQL_CLOSE);

  // Note: The reference driver returns 42710 instead of 42S01 in ODBC spec
  // for base table or view already exists.
  ret = SQLExecDirect(stmt_handle(), sqlchar(sql.c_str()), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "42710", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtSessionSchemaFixture, "SQLExecDirect: 42601 for CREATE VIEW column list mismatch",
                 "[odbc-api][execdirect][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Note: The reference driver returns 42601 instead of 21S02 in the ODBC
  // spec for a CREATE VIEW where the column list has more names than the
  // SELECT produces.
  std::string sql = "CREATE VIEW ed_vm_v (a, b) AS SELECT 1";
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
