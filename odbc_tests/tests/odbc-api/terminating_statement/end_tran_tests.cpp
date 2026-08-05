#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "HandleWrapper.hpp"
#include "ODBCConfig.hpp"
#include "ODBCFixtures.hpp"
#include "Schema.hpp"
#include "SchemaFixtures.hpp"
#include "compatibility.hpp"
#include "get_descriptor.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "terminating_statement_helpers.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

// ============================================================================
// SQLEndTran - Statement Handle
// ============================================================================

TEST_CASE_METHOD(StmtSessionSchemaFixture, "SQLEndTran: Commit persists inserted data",
                 "[odbc-api][endtran][terminating_statement]") {
  SQLRETURN ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, SQL_AUTOCOMMIT_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("CREATE TEMPORARY TABLE ENDTRAN_COMMIT_T (ID INTEGER)"), SQL_NTS);
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

TEST_CASE_METHOD(StmtSessionSchemaFixture, "SQLEndTran: Rollback discards inserted data",
                 "[odbc-api][endtran][terminating_statement]") {
  SQLRETURN ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, SQL_AUTOCOMMIT_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("CREATE TEMPORARY TABLE ENDTRAN_ROLLBACK_T (ID INTEGER)"), SQL_NTS);
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

TEST_CASE_METHOD(StmtSessionSchemaFixture, "SQLEndTran: Commit on environment handle",
                 "[odbc-api][endtran][terminating_statement]") {
  SQLRETURN ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, SQL_AUTOCOMMIT_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("CREATE TEMPORARY TABLE ENDTRAN_ENV_COMMIT_T (ID INTEGER)"), SQL_NTS);
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

TEST_CASE_METHOD(StmtSessionSchemaFixture, "SQLEndTran: Rollback on environment handle",
                 "[odbc-api][endtran][terminating_statement]") {
  SQLRETURN ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, SQL_AUTOCOMMIT_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("CREATE TEMPORARY TABLE ENDTRAN_ENV_ROLLBACK_T (ID INTEGER)"), SQL_NTS);
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

TEST_CASE_METHOD(StmtSessionSchemaFixture, "SQLEndTran: Commit on environment handle commits all connections",
                 "[odbc-api][endtran][terminating_statement]") {
  ExtraConnectedDbc extra(env_handle(), dsn_name());
  Schema::use_temp_session_schema(extra.dbc());
  const std::string table = generate_unique_table_name("ENDTRAN_ENV_ALL_COMMIT");

  SQLRETURN ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, SQL_AUTOCOMMIT_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetConnectAttr(extra.dbc(), SQL_ATTR_AUTOCOMMIT, SQL_AUTOCOMMIT_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar(("CREATE OR REPLACE TABLE " + table + " (ID INTEGER)").c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_COMMIT);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar(("INSERT INTO " + table + " VALUES (1)").c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(extra.stmt(), sqlchar(("INSERT INTO " + table + " VALUES (2)").c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFreeStmt(extra.stmt(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLEndTran(SQL_HANDLE_ENV, env_handle(), SQL_COMMIT);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar(("SELECT COUNT(*) FROM " + table).c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER count = -1;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 2);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, reinterpret_cast<SQLPOINTER>(SQL_AUTOCOMMIT_ON), 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetConnectAttr(extra.dbc(), SQL_ATTR_AUTOCOMMIT, reinterpret_cast<SQLPOINTER>(SQL_AUTOCOMMIT_ON), 0);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(StmtSessionSchemaFixture, "SQLEndTran: Rollback on environment handle rolls back all connections",
                 "[odbc-api][endtran][terminating_statement]") {
  ExtraConnectedDbc extra(env_handle(), dsn_name());
  Schema::use_temp_session_schema(extra.dbc());
  const std::string table = generate_unique_table_name("ENDTRAN_ENV_ALL_ROLLBACK");

  SQLRETURN ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, SQL_AUTOCOMMIT_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetConnectAttr(extra.dbc(), SQL_ATTR_AUTOCOMMIT, SQL_AUTOCOMMIT_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar(("CREATE OR REPLACE TABLE " + table + " (ID INTEGER)").c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_COMMIT);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar(("INSERT INTO " + table + " VALUES (1)").c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(extra.stmt(), sqlchar(("INSERT INTO " + table + " VALUES (2)").c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFreeStmt(extra.stmt(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLEndTran(SQL_HANDLE_ENV, env_handle(), SQL_ROLLBACK);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar(("SELECT COUNT(*) FROM " + table).c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER count = -1;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 0);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, reinterpret_cast<SQLPOINTER>(SQL_AUTOCOMMIT_ON), 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetConnectAttr(extra.dbc(), SQL_ATTR_AUTOCOMMIT, reinterpret_cast<SQLPOINTER>(SQL_AUTOCOMMIT_ON), 0);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(StmtSessionSchemaFixture,
                 "SQLEndTran: Environment-handle rollback does not undo already-committed rows on other connections",
                 "[odbc-api][endtran][terminating_statement]") {
  ExtraConnectedDbc extra(env_handle(), dsn_name());
  Schema::use_temp_session_schema(extra.dbc());
  const std::string table = generate_unique_table_name("ENDTRAN_ENV_AGG");

  SQLRETURN ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, SQL_AUTOCOMMIT_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetConnectAttr(extra.dbc(), SQL_ATTR_AUTOCOMMIT, SQL_AUTOCOMMIT_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar(("CREATE OR REPLACE TABLE " + table + " (ID INTEGER)").c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_COMMIT);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar(("INSERT INTO " + table + " VALUES (1)").c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_COMMIT);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(extra.stmt(), sqlchar(("INSERT INTO " + table + " VALUES (99)").c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFreeStmt(extra.stmt(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLEndTran(SQL_HANDLE_ENV, env_handle(), SQL_ROLLBACK);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar(("SELECT COUNT(*) FROM " + table).c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER count = -1;
  ret = SQLGetData(stmt_handle(), 1, SQL_C_SLONG, &count, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(count == 1);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, reinterpret_cast<SQLPOINTER>(SQL_AUTOCOMMIT_ON), 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetConnectAttr(extra.dbc(), SQL_ATTR_AUTOCOMMIT, reinterpret_cast<SQLPOINTER>(SQL_AUTOCOMMIT_ON), 0);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(StmtSessionSchemaFixture, "SQLEndTran: Environment handle skips disconnected connections",
                 "[odbc-api][endtran][terminating_statement][flaky]") {
  ExtraConnectedDbc extra(env_handle(), dsn_name());
  Schema::use_temp_session_schema(extra.dbc());
  const std::string table = generate_unique_table_name("ENDTRAN_ENV_SKIP");

  SQLRETURN ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, SQL_AUTOCOMMIT_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetConnectAttr(extra.dbc(), SQL_ATTR_AUTOCOMMIT, SQL_AUTOCOMMIT_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar(("CREATE OR REPLACE TABLE " + table + " (ID INTEGER)").c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_COMMIT);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar(("INSERT INTO " + table + " VALUES (1)").c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(extra.stmt(), sqlchar(("INSERT INTO " + table + " VALUES (99)").c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFreeStmt(extra.stmt(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  // Cleanly end the extra connection's transaction before disconnecting. Since
  // SNOW-3831236, SQLDisconnect refuses with 25000 while a manual-commit
  // transaction is open, so rolling back first lets the disconnect succeed on
  // both drivers and makes `extra` genuinely closed. This keeps the test a true
  // "environment-scope SQLEndTran skips a disconnected connection" scenario; the
  // rolled-back INSERT (99) never commits, so only the main connection's row
  // survives regardless of driver (removing the old row-count difference).
  ret = SQLEndTran(SQL_HANDLE_DBC, extra.dbc(), SQL_ROLLBACK);
  REQUIRE(ret == SQL_SUCCESS);

  extra.disconnect();

  ret = SQLEndTran(SQL_HANDLE_ENV, env_handle(), SQL_COMMIT);

  // Result: the return code depends on the driver manager, identically for both drivers. Under
  //   iODBC the DM fans the env-scope call out per-connection and reaches the disconnected `extra`;
  //   both drivers return the spec-mandated 08003 for it, so the aggregated call is SQL_ERROR
  //   (iODBC does not surface the SQLSTATE on the env/connection handle). Under unixODBC the call
  //   routes to the driver's end_tran_env handler, which skips disconnected connections and returns
  //   SQL_SUCCESS. This is a driver-manager artifact, not an old-vs-new difference.
  IODBC_ONLY { REQUIRE(ret == SQL_ERROR); }
  else {
    REQUIRE(ret == SQL_SUCCESS);
  }

  // Effect (asserted for every permutation, independent of the return code above): the still-
  //   connected main connection's INSERT (1) is committed and the disconnected connection's
  //   INSERT (99) was rolled back before disconnecting, so exactly one row survives. Read from an
  //   independent autocommit-ON connection so this reflects committed data only (no read-your-
  //   writes from the main connection's own transaction).
  ExtraConnectedDbc reader(env_handle(), dsn_name());
  Schema::use_temp_session_schema(reader.dbc());
  SQLRETURN reader_ret = SQLExecDirect(reader.stmt(), sqlchar(("SELECT COUNT(*) FROM " + table).c_str()), SQL_NTS);
  REQUIRE(reader_ret == SQL_SUCCESS);
  reader_ret = SQLFetch(reader.stmt());
  REQUIRE(reader_ret == SQL_SUCCESS);
  SQLINTEGER durable_count = -1;
  reader_ret = SQLGetData(reader.stmt(), 1, SQL_C_SLONG, &durable_count, 0, nullptr);
  REQUIRE(reader_ret == SQL_SUCCESS);
  REQUIRE(durable_count == 1);

  ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, reinterpret_cast<SQLPOINTER>(SQL_AUTOCOMMIT_ON), 0);
  REQUIRE(ret == SQL_SUCCESS);
}

// ============================================================================
// SQLEndTran - Cursor Behavior After Commit or Rollback
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLEndTran: Commit closes open cursors",
                 "[odbc-api][endtran][terminating_statement]") {
  SQLRETURN ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, SQL_AUTOCOMMIT_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_COMMIT);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  OLD_IODBC_ONLY("BD#70") {
    // iODBC's DM catches the post-commit SQLFetch as a function-sequence
    //   error and surfaces it as the ODBC 2.x "S1010" before the call reaches
    //   the driver; the old driver doesn't synthesise ODBC 3.x "HY010" ahead
    //   of the DM check the way the new driver does.
    REQUIRE_EXPECTED_ERROR(ret, "S1010", stmt_handle(), SQL_HANDLE_STMT);
  }
  else {
    REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);
  }

  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_COMMIT);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, reinterpret_cast<SQLPOINTER>(SQL_AUTOCOMMIT_ON), 0);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLEndTran: Rollback closes open cursors",
                 "[odbc-api][endtran][terminating_statement]") {
  SQLRETURN ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, SQL_AUTOCOMMIT_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_ROLLBACK);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFetch(stmt_handle());
  OLD_IODBC_ONLY("BD#70") {
    // iODBC DM catches the post-rollback SQLFetch as a function-sequence
    //   error (see "Commit closes open cursors" above for details).
    REQUIRE_EXPECTED_ERROR(ret, "S1010", stmt_handle(), SQL_HANDLE_STMT);
  }
  else {
    REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);
  }

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
  // In autocommit mode the Driver Manager intercepts SQLEndTran and returns
  // SQL_SUCCESS without forwarding to the driver.
  const SQLRETURN ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_COMMIT);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(StmtSessionSchemaFixture, "SQLEndTran: Rollback in autocommit mode does not undo committed data",
                 "[odbc-api][endtran][terminating_statement]") {
  SQLRETURN ret =
      SQLExecDirect(stmt_handle(), sqlchar("CREATE TEMPORARY TABLE ENDTRAN_AC_ROLLBACK_T (ID INTEGER)"), SQL_NTS);
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
  const SQLRETURN ret = SQLEndTran(SQL_HANDLE_DBC, SQL_NULL_HDBC, SQL_COMMIT);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE("SQLEndTran: SQL_INVALID_HANDLE for null environment handle",
          "[odbc-api][endtran][terminating_statement][error]") {
  const SQLRETURN ret = SQLEndTran(SQL_HANDLE_ENV, SQL_NULL_HENV, SQL_COMMIT);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLEndTran: HY012 - Invalid completion type",
                 "[odbc-api][endtran][terminating_statement][error]") {
  SQLRETURN ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, SQL_AUTOCOMMIT_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeStmt(stmt_handle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), 999);
  IODBC_ONLY {
    // iODBC's DM validates the completion type before dispatching and rejects
    //   the invalid code with SQL_ERROR without posting a diagnostic record
    //   (SQLGetDiagRec returns SQL_NO_DATA). The DM short-circuits the call, so
    //   the driver's own HY012 mapping never runs — this holds for both the old
    //   and new drivers. unixODBC forwards to the driver, which posts HY012.
    REQUIRE(ret == SQL_ERROR);
    const auto records = get_diag_rec(SQL_HANDLE_DBC, dbc_handle());
    REQUIRE(records.empty());
  }
  else {
    REQUIRE_EXPECTED_ERROR(ret, "HY012", dbc_handle(), SQL_HANDLE_DBC);
  }

  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_ROLLBACK);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, reinterpret_cast<SQLPOINTER>(SQL_AUTOCOMMIT_ON), 0);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcFixture, "SQLEndTran: 08003 - Connection not open",
                 "[odbc-api][endtran][terminating_statement][error]") {
  const SQLRETURN ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_COMMIT);
  REQUIRE_EXPECTED_ERROR(ret, "08003", dbc_handle(), SQL_HANDLE_DBC);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLEndTran: HY092 - Invalid handle type",
                 "[odbc-api][endtran][terminating_statement][error]") {
  const SQLRETURN ret = SQLEndTran(SQL_HANDLE_STMT, stmt_handle(), SQL_COMMIT);
  IODBC_ONLY {
    // iODBC's DM validates the HandleType enum before dispatching and rejects
    //   SQL_HANDLE_STMT for SQLEndTran with SQL_INVALID_HANDLE (no diagnostic
    //   records) before the driver is ever called, for both the old and new
    //   drivers. unixODBC forwards to the driver, which maps it to HY092.
    REQUIRE(ret == SQL_INVALID_HANDLE);
  }
  else {
    REQUIRE_EXPECTED_ERROR(ret, "HY092", stmt_handle(), SQL_HANDLE_STMT);
  }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLEndTran: HY092 - Invalid handle type (descriptor)",
                 "[odbc-api][endtran][terminating_statement][error]") {
  const SQLHDESC ard = get_descriptor(stmt_handle(), SQL_ATTR_APP_ROW_DESC);
  const SQLRETURN ret = SQLEndTran(SQL_HANDLE_DESC, ard, SQL_COMMIT);
  IODBC_ONLY {
    // iODBC's DM validates the HandleType enum before dispatching and rejects
    //   SQL_HANDLE_DESC for SQLEndTran with SQL_INVALID_HANDLE (no diagnostic
    //   records) before the driver is ever called. unixODBC forwards to the
    //   driver, which maps it to HY092.
    REQUIRE(ret == SQL_INVALID_HANDLE);
  }
  else {
    REQUIRE_EXPECTED_ERROR(ret, "HY092", ard, SQL_HANDLE_DESC);
  }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLEndTran: HY010 - Called during SQL_NEED_DATA",
                 "[odbc-api][endtran][terminating_statement][error]") {
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
  OLD_IODBC_ONLY("BD#70") {
    // iODBC's DM tracks per-statement SQL_NEED_DATA across the connection and
    //   surfaces the SQLEndTran-during-DAE as ODBC 2.x "S1010" function
    //   sequence error before the old driver sees it; the new driver maps the
    //   same condition to "HY010" itself.
    REQUIRE_EXPECTED_ERROR(ret, "S1010", dbc_handle(), SQL_HANDLE_DBC);
  }
  else {
    REQUIRE_EXPECTED_ERROR(ret, "HY010", dbc_handle(), SQL_HANDLE_DBC);
  }

  SQLCancel(stmt_handle());
  SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_ROLLBACK);
  SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, reinterpret_cast<SQLPOINTER>(SQL_AUTOCOMMIT_ON), 0);
}
