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
// SQLEndTran - Statement Handle
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLEndTran: Commit persists inserted data",
                 "[odbc-api][endtran][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, SQL_AUTOCOMMIT_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);

  auto schema = Schema::use_random_schema(dbc_handle());

  ret = SQLExecDirect(stmt_handle(), sqlchar("CREATE TABLE ENDTRAN_COMMIT_T (ID INTEGER)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_COMMIT);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("INSERT INTO ENDTRAN_COMMIT_T VALUES (1)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_COMMIT);
  REQUIRE(ret == SQL_SUCCESS);

  // Insert a second row and roll it back - the first committed row must survive
  ret = SQLExecDirect(stmt_handle(), sqlchar("INSERT INTO ENDTRAN_COMMIT_T VALUES (99)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_ROLLBACK);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT COUNT(*) FROM ENDTRAN_COMMIT_T"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER count = -1;
  SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &count, 0, nullptr);
  REQUIRE(count == 1);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_COMMIT);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, reinterpret_cast<SQLPOINTER>(SQL_AUTOCOMMIT_ON), 0);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLEndTran: Rollback discards inserted data",
                 "[odbc-api][endtran][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, SQL_AUTOCOMMIT_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);

  auto schema = Schema::use_random_schema(dbc_handle());

  ret = SQLExecDirect(stmt_handle(), sqlchar("CREATE TABLE ENDTRAN_ROLLBACK_T (ID INTEGER)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_COMMIT);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("INSERT INTO ENDTRAN_ROLLBACK_T VALUES (1)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_ROLLBACK);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT COUNT(*) FROM ENDTRAN_ROLLBACK_T"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER count = -1;
  SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &count, 0, nullptr);
  REQUIRE(count == 0);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_COMMIT);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, reinterpret_cast<SQLPOINTER>(SQL_AUTOCOMMIT_ON), 0);
  REQUIRE(ret == SQL_SUCCESS);
}

// ============================================================================
// SQLEndTran - Environment Handle
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLEndTran: Commit on environment handle",
                 "[odbc-api][endtran][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, SQL_AUTOCOMMIT_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);

  auto schema = Schema::use_random_schema(dbc_handle());

  ret = SQLExecDirect(stmt_handle(), sqlchar("CREATE TABLE ENDTRAN_ENV_COMMIT_T (ID INTEGER)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLEndTran(SQL_HANDLE_ENV, env_handle(), SQL_COMMIT);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("INSERT INTO ENDTRAN_ENV_COMMIT_T VALUES (5)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLEndTran(SQL_HANDLE_ENV, env_handle(), SQL_COMMIT);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("INSERT INTO ENDTRAN_ENV_COMMIT_T VALUES (99)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLEndTran(SQL_HANDLE_ENV, env_handle(), SQL_ROLLBACK);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT COUNT(*) FROM ENDTRAN_ENV_COMMIT_T"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER count = -1;
  SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &count, 0, nullptr);
  REQUIRE(count == 1);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLEndTran(SQL_HANDLE_ENV, env_handle(), SQL_COMMIT);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, reinterpret_cast<SQLPOINTER>(SQL_AUTOCOMMIT_ON), 0);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLEndTran: Rollback on environment handle",
                 "[odbc-api][endtran][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, SQL_AUTOCOMMIT_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);

  auto schema = Schema::use_random_schema(dbc_handle());

  ret = SQLExecDirect(stmt_handle(), sqlchar("CREATE TABLE ENDTRAN_ENV_ROLLBACK_T (ID INTEGER)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLEndTran(SQL_HANDLE_ENV, env_handle(), SQL_COMMIT);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("INSERT INTO ENDTRAN_ENV_ROLLBACK_T VALUES (1)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLEndTran(SQL_HANDLE_ENV, env_handle(), SQL_ROLLBACK);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT COUNT(*) FROM ENDTRAN_ENV_ROLLBACK_T"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER count = -1;
  SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &count, 0, nullptr);
  REQUIRE(count == 0);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLEndTran(SQL_HANDLE_ENV, env_handle(), SQL_COMMIT);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, reinterpret_cast<SQLPOINTER>(SQL_AUTOCOMMIT_ON), 0);
  REQUIRE(ret == SQL_SUCCESS);
}

// ============================================================================
// SQLEndTran - Cursor Behavior After Commit or Rollback
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLEndTran: Commit closes open cursors",
                 "[odbc-api][endtran][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, SQL_AUTOCOMMIT_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_COMMIT);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);

  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_COMMIT);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, reinterpret_cast<SQLPOINTER>(SQL_AUTOCOMMIT_ON), 0);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLEndTran: Rollback closes open cursors",
                 "[odbc-api][endtran][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, SQL_AUTOCOMMIT_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_ROLLBACK);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);

  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_COMMIT);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, reinterpret_cast<SQLPOINTER>(SQL_AUTOCOMMIT_ON), 0);
  REQUIRE(ret == SQL_SUCCESS);
}

// ============================================================================
// SQLEndTran - Autocommit Mode
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLEndTran: Commit in autocommit mode",
                 "[odbc-api][endtran][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // In autocommit mode the Driver Manager intercepts SQLEndTran and returns
  // SQL_SUCCESS without forwarding to the driver.
  const SQLRETURN ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_COMMIT);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLEndTran: Rollback in autocommit mode does not undo committed data",
                 "[odbc-api][endtran][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  auto schema = Schema::use_random_schema(dbc_handle());

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("CREATE TABLE ENDTRAN_AC_ROLLBACK_T (ID INTEGER)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("INSERT INTO ENDTRAN_AC_ROLLBACK_T VALUES (1)"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_ROLLBACK);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT COUNT(*) FROM ENDTRAN_AC_ROLLBACK_T"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER count = -1;
  SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &count, 0, nullptr);
  REQUIRE(count == 1);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);
}

// ============================================================================
// SQLEndTran - Statement Reuse After Transaction
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLEndTran: Statement reusable after commit",
                 "[odbc-api][endtran][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, SQL_AUTOCOMMIT_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_COMMIT);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(val == 42);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_COMMIT);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, reinterpret_cast<SQLPOINTER>(SQL_AUTOCOMMIT_ON), 0);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLEndTran: Prepared statement survives commit",
                 "[odbc-api][endtran][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, SQL_AUTOCOMMIT_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_COMMIT);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(val == 42);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_COMMIT);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, reinterpret_cast<SQLPOINTER>(SQL_AUTOCOMMIT_ON), 0);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLEndTran: Prepared statement survives rollback",
                 "[odbc-api][endtran][terminating_statement]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, SQL_AUTOCOMMIT_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 42"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_ROLLBACK);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER val = 0;
  SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &val, 0, nullptr);
  REQUIRE(val == 42);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_COMMIT);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, reinterpret_cast<SQLPOINTER>(SQL_AUTOCOMMIT_ON), 0);
  REQUIRE(ret == SQL_SUCCESS);
}

// ============================================================================
// SQLEndTran - Error Cases
// ============================================================================

TEST_CASE("SQLEndTran: SQL_INVALID_HANDLE for null connection handle",
          "[odbc-api][endtran][terminating_statement][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const SQLRETURN ret = SQLEndTran(SQL_HANDLE_DBC, SQL_NULL_HDBC, SQL_COMMIT);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE("SQLEndTran: SQL_INVALID_HANDLE for null environment handle",
          "[odbc-api][endtran][terminating_statement][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const SQLRETURN ret = SQLEndTran(SQL_HANDLE_ENV, SQL_NULL_HENV, SQL_COMMIT);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLEndTran: HY012 - Invalid completion type",
                 "[odbc-api][endtran][terminating_statement][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, SQL_AUTOCOMMIT_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), 999);
  REQUIRE_EXPECTED_ERROR(ret, "HY012", dbc_handle(), SQL_HANDLE_DBC);

  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_ROLLBACK);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, reinterpret_cast<SQLPOINTER>(SQL_AUTOCOMMIT_ON), 0);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcFixture, "SQLEndTran: 08003 - Connection not open",
                 "[odbc-api][endtran][terminating_statement][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const SQLRETURN ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_COMMIT);
  REQUIRE_EXPECTED_ERROR(ret, "08003", dbc_handle(), SQL_HANDLE_DBC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLEndTran: HY092 - Invalid handle type",
                 "[odbc-api][endtran][terminating_statement][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const SQLRETURN ret = SQLEndTran(SQL_HANDLE_STMT, stmt_handle(), SQL_COMMIT);
  REQUIRE_EXPECTED_ERROR(ret, "HY092", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLEndTran: HY010 - Called during SQL_NEED_DATA",
                 "[odbc-api][endtran][terminating_statement][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, SQL_AUTOCOMMIT_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN dae_indicator = SQL_DATA_AT_EXEC;
  SQLINTEGER param_id = 1;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0,
                         reinterpret_cast<SQLPOINTER>(static_cast<SQLLEN>(param_id)), 0, &dae_indicator);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_COMMIT);
  REQUIRE_EXPECTED_ERROR(ret, "HY010", dbc_handle(), SQL_HANDLE_DBC);

  SQLCancel(stmt_handle());
  SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_ROLLBACK);
  SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, reinterpret_cast<SQLPOINTER>(SQL_AUTOCOMMIT_ON), 0);
}
