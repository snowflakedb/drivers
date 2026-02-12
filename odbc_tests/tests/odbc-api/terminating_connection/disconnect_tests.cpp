#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "ODBCConfig.hpp"
#include "ODBCFixtures.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

// ============================================================================
// SQLDisconnect - Basic Functionality
// ============================================================================

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDisconnect: Successfully disconnects from data source",
                 "[odbc-api][disconnect][terminating_connection]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Connect first
  SQLRETURN ret =
      SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                 SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // 08003: Connection not open
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE_EXPECTED_ERROR(ret, "08003", dbc_handle(), SQL_HANDLE_DBC);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDisconnect: Can reconnect after disconnect",
                 "[odbc-api][disconnect][terminating_connection]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Connect, disconnect, reconnect
  SQLRETURN ret =
      SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                 SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                   SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);

  SQLFreeHandle(SQL_HANDLE_STMT, stmt);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDisconnect: Idempotency of multiple disconnects",
                 "[odbc-api][disconnect][terminating_connection]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Connect first
  SQLRETURN ret =
      SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                 SQL_NTS, nullptr, 0, nullptr, 0);
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
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Connect
  SQLRETURN ret =
      SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                 SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Allocate statement
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // Note: Reference driver does not set statement handle variable to null after disconnect
  // However, the handle becomes invalid and operations on it return SQL_INVALID_HANDLE
  ret = SQLExecDirect(stmt, reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT 1")), SQL_NTS);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDisconnect: Handles active transactions",
                 "[odbc-api][disconnect][terminating_connection]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Connect with manual commit mode
  SQLRETURN ret =
      SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                 SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Set manual commit mode
  ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_AUTOCOMMIT, SQL_AUTOCOMMIT_OFF, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Execute a statement to start transaction
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt, reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT 1")), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLFreeHandle(SQL_HANDLE_STMT, stmt);

  // Note: Reference driver requires explicit transaction cleanup before disconnecting
  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_ERROR);

  ret = SQLEndTran(SQL_HANDLE_DBC, dbc_handle(), SQL_ROLLBACK);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDisconnect: With active result sets",
                 "[odbc-api][disconnect][terminating_connection]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Connect
  SQLRETURN ret =
      SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                 SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Execute query with result set
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt, reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT 1")), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // Note: Reference driver allows disconnecting with active result sets
  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

// ============================================================================
// SQLDisconnect - Error Cases: Invalid Handle
// ============================================================================

TEST_CASE("SQLDisconnect: SQL_INVALID_HANDLE for null connection handle",
          "[odbc-api][disconnect][terminating_connection][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const SQLRETURN ret = SQLDisconnect(SQL_NULL_HDBC);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(EnvFixture, "SQLDisconnect: SQL_INVALID_HANDLE for wrong handle type",
                 "[odbc-api][disconnect][terminating_connection][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Pass environment handle as connection handle
  const SQLRETURN ret = SQLDisconnect(env_handle());
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(DbcFixture, "SQLDisconnect: 08003 - Connection not open when not connected",
                 "[odbc-api][disconnect][terminating_connection][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

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
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Attempt to connect with invalid credentials
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("InvalidDSN")), SQL_NTS,
                             nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_ERROR);

  // Note: Reference driver returns error (08003: Connection not open)
  ret = SQLDisconnect(dbc_handle());
  REQUIRE_EXPECTED_ERROR(ret, "08003", dbc_handle(), SQL_HANDLE_DBC);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDisconnect: With multiple statement handles",
                 "[odbc-api][disconnect][terminating_connection]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Connect
  SQLRETURN ret =
      SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                 SQL_NTS, nullptr, 0, nullptr, 0);
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

  ret = SQLExecDirect(stmt1, reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT 1")), SQL_NTS);
  REQUIRE(ret == SQL_INVALID_HANDLE);

  ret = SQLExecDirect(stmt2, reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT 1")), SQL_NTS);
  REQUIRE(ret == SQL_INVALID_HANDLE);

  ret = SQLExecDirect(stmt3, reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT 1")), SQL_NTS);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDisconnect: Preserves connection handle for reuse",
                 "[odbc-api][disconnect][terminating_connection]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Connect, disconnect, verify handle can be reused
  SQLRETURN ret =
      SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                 SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // Note: Reference driver Get fails but Set succeeds on disconnected handle
  SQLUINTEGER timeout = 0;
  ret = SQLGetConnectAttr(dbc_handle(), SQL_ATTR_CONNECTION_TIMEOUT, &timeout, 0, nullptr);
  REQUIRE(ret == SQL_ERROR);

  ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_CONNECTION_TIMEOUT, reinterpret_cast<SQLPOINTER>(30), 0);
  REQUIRE(ret == SQL_SUCCESS);
}

// ============================================================================
// SQLDisconnect - Diagnostic Information
// ============================================================================

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLDisconnect: Clears previous diagnostic records",
                 "[odbc-api][disconnect][terminating_connection]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Connect
  SQLRETURN ret =
      SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                 SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Cause an error to populate diagnostic records
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);

  // Execute invalid SQL
  ret = SQLExecDirect(stmt, reinterpret_cast<SQLCHAR*>(const_cast<char*>("INVALID SQL STATEMENT")), SQL_NTS);
  REQUIRE(ret == SQL_ERROR);

  SQLCHAR temp_sqlstate[6];
  SQLINTEGER temp_native_error;
  SQLCHAR temp_message[256];
  SQLSMALLINT temp_message_len;

  ret = SQLGetDiagRec(SQL_HANDLE_STMT, stmt, 1, temp_sqlstate, &temp_native_error, temp_message, sizeof(temp_message),
                      &temp_message_len);
  REQUIRE(ret == SQL_SUCCESS);

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
