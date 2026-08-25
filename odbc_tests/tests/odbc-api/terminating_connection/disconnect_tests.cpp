#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "ODBCConfig.hpp"
#include "ODBCFixtures.hpp"
#include "compatibility.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"
#include "test_setup.hpp"

// ============================================================================
// SQLDisconnect - Basic Functionality
// ============================================================================

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDisconnect: Successfully disconnects from data source",
                 "[odbc-api][disconnect][terminating_connection]") {
  // Connect first
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // 08003: Connection not open
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE_EXPECTED_ERROR(ret, "08003", dbc_handle(), SQL_HANDLE_DBC);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDisconnect: Autocommit OFF with no open transaction succeeds",
                 "[odbc-api][disconnect][terminating_connection]") {
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, SQL_AUTOCOMMIT_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // No statement executed -> no transaction in process.
  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDisconnect: Autocommit OFF after commit succeeds",
                 "[odbc-api][disconnect][terminating_connection]") {
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, SQL_AUTOCOMMIT_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt, sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_COMMIT);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDisconnect: Can reconnect after disconnect",
                 "[odbc-api][disconnect][terminating_connection]") {
  // Connect, disconnect, reconnect
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);

  SQLFreeHandle(SQL_HANDLE_STMT, stmt);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDisconnect: Second disconnect returns 08003 error",
                 "[odbc-api][disconnect][terminating_connection]") {
  // Connect first
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // First disconnect
  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // Note: Reference driver returns error for disconnecting an already disconnected handle.
  // This is a deviation from the ODBC specification which allows for idempotent disconnects.
  // 08003: Connection not open
  ret = SQLDisconnect(dbc_handle());
  REQUIRE_EXPECTED_ERROR(ret, "08003", dbc_handle(), SQL_HANDLE_DBC);
}

// ============================================================================
// SQLDisconnect - With Active Statements
// ============================================================================

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDisconnect: Closes open statements automatically",
                 "[odbc-api][disconnect][terminating_connection]") {
  // Connect
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Allocate statement
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);

  OLD_IODBC_ONLY("BD#68") {
    // iODBC keeps the statement entry in its DM-side alloc table after the
    //   old driver's SQLDisconnect, so a follow-up SQLFreeHandle does NOT
    //   return SQL_INVALID_HANDLE - the handle is still addressable through
    //   iODBC's alloc table. The new driver coordinates with iODBC to invalidate
    //   child statements on disconnect, so the else branch checks for the
    //   spec-mandated SQL_INVALID_HANDLE.
    ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt);
    REQUIRE(ret != SQL_INVALID_HANDLE);
  }
  else {
    REQUIRE_INVALID_HANDLE(SQL_HANDLE_STMT, stmt);
  }
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDisconnect: Handles active transactions",
                 "[odbc-api][disconnect][terminating_connection]") {
  // Connect with manual commit mode
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Set manual commit mode
  ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, SQL_AUTOCOMMIT_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Execute a statement to start transaction
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt, sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLFreeHandle(SQL_HANDLE_STMT, stmt);

  // 25000: transaction still in process. Both drivers refuse and keep the
  // connection open.
  ret = SQLDisconnect(dbc_handle());
  REQUIRE_EXPECTED_ERROR(ret, "25000", dbc_handle(), SQL_HANDLE_DBC);

  // Connection is still usable: end the transaction, then disconnect succeeds.
  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_ROLLBACK);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDisconnect: Autocommit OFF after rollback succeeds",
                 "[odbc-api][disconnect][terminating_connection]") {
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, SQL_AUTOCOMMIT_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt, sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_ROLLBACK);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDisconnect: Switching autocommit back ON clears open transaction",
                 "[odbc-api][disconnect][terminating_connection]") {
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, SQL_AUTOCOMMIT_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);

  // Opens an ODBC-managed transaction while autocommit is OFF.
  ret = SQLExecDirect(stmt, sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  REQUIRE(ret == SQL_SUCCESS);

  // Switching autocommit back ON commits and clears the open transaction, so
  // SQLDisconnect must no longer be refused with 25000. Exercises the third
  // open_transaction clearing path (SQL_ATTR_AUTOCOMMIT -> ON) in set_connect_attr.
  ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, reinterpret_cast<SQLPOINTER>(SQL_AUTOCOMMIT_ON), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDisconnect: Explicit BEGIN TRANSACTION under autocommit ON succeeds",
                 "[odbc-api][disconnect][terminating_connection]") {
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt, sqlchar("BEGIN TRANSACTION"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLExecDirect(stmt, sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  REQUIRE(ret == SQL_SUCCESS);

  // Autocommit is ON, so no ODBC-managed transaction is tracked: disconnect is
  // allowed on both drivers even though a server-side transaction is open.
  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDisconnect: With active result sets",
                 "[odbc-api][disconnect][terminating_connection]") {
  // Connect
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Execute query with result set
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt, sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // Note: Reference driver allows disconnecting with active result sets
  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);

  OLD_IODBC_ONLY("BD#68") {
    // See "Closes open statements automatically": iODBC's statement-alloc
    //   table outlives the old driver's SQLDisconnect, so the handle stays
    //   addressable through iODBC and SQLFreeHandle does NOT return
    //   SQL_INVALID_HANDLE. The exact return value (SQL_SUCCESS / SQL_ERROR).
    ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt);
    REQUIRE(ret != SQL_INVALID_HANDLE);
  }
  else {
    REQUIRE_INVALID_HANDLE(SQL_HANDLE_STMT, stmt);
  }
}

// ============================================================================
// SQLDisconnect - Error Cases: Invalid Handle
// ============================================================================

TEST_CASE("SQLDisconnect: SQL_INVALID_HANDLE for null connection handle",
          "[odbc-api][disconnect][terminating_connection][error]") {
  const SQLRETURN ret = SQLDisconnect(SQL_NULL_HDBC);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(EnvFixture, "SQLDisconnect: SQL_INVALID_HANDLE for wrong handle type",
                 "[odbc-api][disconnect][terminating_connection][error]") {
  // Pass environment handle as connection handle
  const SQLRETURN ret = SQLDisconnect(env_handle());
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(DbcFixture, "SQLDisconnect: 08003 - Connection not open when not connected",
                 "[odbc-api][disconnect][terminating_connection][error]") {
  // Try to disconnect without connecting first
  // 08003: Connection not open
  const SQLRETURN ret = SQLDisconnect(dbc_handle());
  REQUIRE_EXPECTED_ERROR(ret, "08003", dbc_handle(), SQL_HANDLE_DBC);
}

// ============================================================================
// SQLDisconnect - Edge Cases
// ============================================================================

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDisconnect: After failed connection attempt",
                 "[odbc-api][disconnect][terminating_connection]") {
  // Attempt to connect with invalid credentials
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar("InvalidDSN"), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_ERROR);

  // Note: Reference driver returns error (08003: Connection not open)
  ret = SQLDisconnect(dbc_handle());
  REQUIRE_EXPECTED_ERROR(ret, "08003", dbc_handle(), SQL_HANDLE_DBC);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDisconnect: With multiple statement handles",
                 "[odbc-api][disconnect][terminating_connection]") {
  // Connect
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Allocate multiple statements
  SQLHSTMT stmt1 = SQL_NULL_HSTMT, stmt2 = SQL_NULL_HSTMT, stmt3 = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt1);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt2);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt3);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);

  OLD_IODBC_ONLY("BD#68") {
    // Same alloc-table leak as "Closes open statements automatically": after the
    //   old driver's SQLDisconnect, iODBC still has the child statement entries,
    //   so a follow-up SQLFreeHandle does not return SQL_INVALID_HANDLE.
    // Probe only the first child. Freeing every leftover statement (or freeing
    //   the last remaining child, then the DBC in fixture teardown) SIGSEGVs
    //   inside the old driver's per-statement cleanup (BD#59).
    ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt1);
    REQUIRE(ret != SQL_INVALID_HANDLE);
  }
  else {
    REQUIRE_INVALID_HANDLE(SQL_HANDLE_STMT, stmt1);
    REQUIRE_INVALID_HANDLE(SQL_HANDLE_STMT, stmt2);
    REQUIRE_INVALID_HANDLE(SQL_HANDLE_STMT, stmt3);
  }
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDisconnect: Preserves connection handle for reuse",
                 "[odbc-api][disconnect][terminating_connection]") {
  // Connect, disconnect, verify handle can be reused
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLUINTEGER timeout = 0;
  ret = SQLGetConnectAttr(dbc_handle(), SQL_ATTR_CONNECTION_TIMEOUT, &timeout, 0, nullptr);
  IODBC_ONLY {
    // iODBC serves connection-attribute reads from its DM-side cache after
    //   SQLDisconnect for both drivers (SQL_SUCCESS) instead of the
    //   spec-mandated SQL_ERROR asserted on unixODBC / Windows (BD#64).
    REQUIRE(ret == SQL_SUCCESS);
  }
  else {
    // Note: Reference driver Get fails but Set succeeds on disconnected handle
    REQUIRE(ret == SQL_ERROR);
  }

  ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_CONNECTION_TIMEOUT, reinterpret_cast<SQLPOINTER>(30), 0);
  REQUIRE(ret == SQL_SUCCESS);
}

// ============================================================================
// SQLDisconnect - Diagnostic Information
// ============================================================================

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDisconnect: Clears previous diagnostic records",
                 "[odbc-api][disconnect][terminating_connection]") {
  // Connect
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Cause an error to populate diagnostic records
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);

  // Execute invalid SQL
  ret = SQLExecDirect(stmt, sqlchar("INVALID SQL STATEMENT"), SQL_NTS);
  REQUIRE(ret == SQL_ERROR);

  SQLCHAR temp_sqlstate[6];
  SQLINTEGER temp_native_error;
  SQLCHAR temp_message[256];
  SQLSMALLINT temp_message_len;

  ret = SQLGetDiagRec(SQL_HANDLE_STMT, stmt, 1, temp_sqlstate, &temp_native_error, temp_message, sizeof(temp_message),
                      &temp_message_len);
  // SQL_SUCCESS_WITH_INFO is also valid here (message may be truncated in the
  // fixed-size buffer); we only need to confirm a diagnostic record exists.
  REQUIRE(SQL_SUCCEEDED(ret));

  SQLFreeHandle(SQL_HANDLE_STMT, stmt);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // Note: Reference driver returns SQL_NO_DATA indicating no diagnostic records
  SQLCHAR sqlstate[6];
  SQLINTEGER native_error;
  SQLCHAR message[256];
  SQLSMALLINT message_len;

  ret = SQLGetDiagRec(SQL_HANDLE_DBC, dbc_handle(), 1, sqlstate, &native_error, message, sizeof(message), &message_len);
  REQUIRE(ret == SQL_NO_DATA);
}
