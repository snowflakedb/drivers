#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "compatibility.hpp"
#include "Connection.hpp"
#include "ODBCConfig.hpp"
#include "ODBCFixtures.hpp"
#include "get_diag_rec.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

// ============================================================================
// SQLFreeHandle - Environment Handle
// ============================================================================

TEST_CASE("SQLFreeHandle: Successfully frees environment handle",
          "[odbc-api][freehandle][terminating_connection]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Allocate environment
  SQLHENV env = SQL_NULL_HENV;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(env != SQL_NULL_HENV);

  // Set ODBC version
  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Free environment
  ret = SQLFreeHandle(SQL_HANDLE_ENV, env);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE("SQLFreeHandle: SQL_INVALID_HANDLE for null environment handle",
          "[odbc-api][freehandle][terminating_connection][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_ENV, SQL_NULL_HENV);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(EnvFixture, "SQLFreeHandle: HY010 - Cannot free environment with active connections",
                 "[odbc-api][freehandle][terminating_connection][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Allocate connection on environment
  SQLHDBC dbc = SQL_NULL_HDBC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env_handle(), &dbc);
  REQUIRE(ret == SQL_SUCCESS);

  // Try to free environment while connection exists
  // HY010: Function sequence error
  ret = SQLFreeHandle(SQL_HANDLE_ENV, env_handle());
  REQUIRE_EXPECTED_ERROR(ret, "HY010", env_handle(), SQL_HANDLE_ENV);

  // Clean up
  SQLFreeHandle(SQL_HANDLE_DBC, dbc);
}

TEST_CASE("SQLFreeHandle: Double free environment handle",
          "[odbc-api][freehandle][terminating_connection][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Allocate and free environment
  SQLHENV env = SQL_NULL_HENV;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeHandle(SQL_HANDLE_ENV, env);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeHandle(SQL_HANDLE_ENV, env);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

// ============================================================================
// SQLFreeHandle - Connection Handle
// ============================================================================

TEST_CASE_METHOD(EnvFixture, "SQLFreeHandle: Successfully frees connection handle",
                 "[odbc-api][freehandle][terminating_connection]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Allocate connection
  SQLHDBC dbc = SQL_NULL_HDBC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env_handle(), &dbc);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(dbc != SQL_NULL_HDBC);

  // Free connection (not connected)
  ret = SQLFreeHandle(SQL_HANDLE_DBC, dbc);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE("SQLFreeHandle: SQL_INVALID_HANDLE for null connection handle",
          "[odbc-api][freehandle][terminating_connection][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_DBC, SQL_NULL_HDBC);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLFreeHandle: HY010 - Cannot free connected connection handle",
                 "[odbc-api][freehandle][terminating_connection][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Connect
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Try to free while still connected
  // HY010: Function sequence error
  ret = SQLFreeHandle(SQL_HANDLE_DBC, dbc_handle());
  REQUIRE_EXPECTED_ERROR(ret, "HY010", dbc_handle(), SQL_HANDLE_DBC);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLFreeHandle: Can free disconnected connection handle",
                 "[odbc-api][freehandle][terminating_connection]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Connect and disconnect
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // Free disconnected connection
  ret = SQLFreeHandle(SQL_HANDLE_DBC, dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // Mark handle as freed to prevent double-free in fixture cleanup
  dbc_wrapper.reset();
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLFreeHandle: Frees dependent statement handles when connection handle is freed",
                 "[odbc-api][freehandle][terminating_connection]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  
  // Connect first (required to allocate statement)
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Allocate statement
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);

  // Disconnect first
  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // Note: Reference driver automatically frees dependent statement handles
  ret = SQLFreeHandle(SQL_HANDLE_DBC, dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
  
  ret = SQLExecDirect(stmt, reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT 1")), SQL_NTS);
  REQUIRE(ret == SQL_INVALID_HANDLE);
  
  dbc_wrapper.reset();
}

// SQLFreeHandle: Double free connection handle
// Note: Reference driver crashes on double-free of connection handle
// This is undefined behavior that must be avoided
// Skipping test case to prevent crash

// ============================================================================
// SQLFreeHandle - Statement Handle
// ============================================================================

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLFreeHandle: Successfully frees statement handle",
                 "[odbc-api][freehandle][terminating_connection]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Connect first
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Allocate statement
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(stmt != SQL_NULL_HSTMT);

  // Free statement
  ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  REQUIRE(ret == SQL_SUCCESS);

  SQLDisconnect(dbc_handle());
}

TEST_CASE("SQLFreeHandle: SQL_INVALID_HANDLE for null statement handle",
          "[odbc-api][freehandle][terminating_connection][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, SQL_NULL_HSTMT);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLFreeHandle: Can free statement with prepared statement",
                 "[odbc-api][freehandle][terminating_connection]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Connect
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Allocate and prepare statement
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLPrepare(stmt, reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT 1")), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  
  ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  REQUIRE(ret == SQL_SUCCESS);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLFreeHandle: Can free statement with active result set",
                 "[odbc-api][freehandle][terminating_connection]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Connect
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Execute query
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt, reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT 1")), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  
  ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  REQUIRE(ret == SQL_SUCCESS);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLFreeHandle: Can free statement with bound parameters",
                 "[odbc-api][freehandle][terminating_connection]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Connect
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Allocate statement and bind parameter
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER param_value = 42;
  ret = SQLBindParameter(stmt, 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param_value, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  
  ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  REQUIRE(ret == SQL_SUCCESS);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLFreeHandle: Can free statement with bound columns",
                 "[odbc-api][freehandle][terminating_connection]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Connect
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Allocate statement and bind column
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER col_value = 0;
  ret = SQLBindCol(stmt, 1, SQL_C_SLONG, &col_value, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  
  ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  REQUIRE(ret == SQL_SUCCESS);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLFreeHandle: Double free statement handle",
                 "[odbc-api][freehandle][terminating_connection][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Connect
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Allocate and free statement
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  REQUIRE(ret == SQL_INVALID_HANDLE);

  SQLDisconnect(dbc_handle());
}

// ============================================================================
// SQLFreeHandle - Descriptor Handle
// ============================================================================

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLFreeHandle: Can free explicitly allocated descriptor",
                 "[odbc-api][freehandle][terminating_connection]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Connect
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Allocate explicit descriptor
  SQLHDESC desc = SQL_NULL_HDESC;
  ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc_handle(), &desc);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(desc != SQL_NULL_HDESC);

  // Free descriptor
  ret = SQLFreeHandle(SQL_HANDLE_DESC, desc);
  REQUIRE(ret == SQL_SUCCESS);

  SQLDisconnect(dbc_handle());
}

TEST_CASE("SQLFreeHandle: SQL_INVALID_HANDLE for null descriptor handle",
          "[odbc-api][freehandle][terminating_connection][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_DESC, SQL_NULL_HDESC);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLFreeHandle: HY017 - Cannot free implicit descriptor",
                 "[odbc-api][freehandle][terminating_connection][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Connect
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Allocate statement
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);

  // Get implicit descriptor (ARD)
  SQLHDESC ard = SQL_NULL_HDESC;
  ret = SQLGetStmtAttr(stmt, SQL_ATTR_APP_ROW_DESC, &ard, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(ard != SQL_NULL_HDESC);
  
  // Try to free implicit descriptor
  // HY017: Invalid use of an automatically allocated descriptor handle
  ret = SQLFreeHandle(SQL_HANDLE_DESC, ard);
  REQUIRE_EXPECTED_ERROR(ret, "HY017", ard, SQL_HANDLE_DESC);

  SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  SQLDisconnect(dbc_handle());
}

// ============================================================================
// SQLFreeHandle - Invalid Handle Type
// ============================================================================

TEST_CASE_METHOD(EnvFixture, "SQLFreeHandle: SQL_INVALID_HANDLE for invalid handle type",
                 "[odbc-api][freehandle][terminating_connection][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Allocate connection
  SQLHDBC dbc = SQL_NULL_HDBC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env_handle(), &dbc);
  REQUIRE(ret == SQL_SUCCESS);

  // Try to free with wrong handle type
  ret = SQLFreeHandle(SQL_HANDLE_STMT, dbc);
  REQUIRE(ret == SQL_INVALID_HANDLE);

  // Clean up properly
  SQLFreeHandle(SQL_HANDLE_DBC, dbc);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLFreeHandle: SQL_INVALID_HANDLE for wrong statement/connection handle type",
                 "[odbc-api][freehandle][terminating_connection][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Connect
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
                             SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Allocate statement
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeHandle(SQL_HANDLE_DBC, stmt);
  REQUIRE(ret == SQL_INVALID_HANDLE);

  SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  SQLDisconnect(dbc_handle());
}

TEST_CASE("SQLFreeHandle: SQL_INVALID_HANDLE for completely invalid handle type value",
          "[odbc-api][freehandle][terminating_connection][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLHENV env = SQL_NULL_HENV;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Try to free with invalid handle type (999)
  ret = SQLFreeHandle(999, env);
  REQUIRE(ret == SQL_INVALID_HANDLE);

  // Clean up properly
  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

// ============================================================================
// SQLFreeHandle - Edge Cases and Multiple Handles
// ============================================================================

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLFreeHandle: Can free multiple statement handles independently",
                 "[odbc-api][freehandle][terminating_connection]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Connect
  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
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

  // Free in different order
  ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt2);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt1);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt3);
  REQUIRE(ret == SQL_SUCCESS);

  SQLDisconnect(dbc_handle());
}

TEST_CASE("SQLFreeHandle: Complete handle hierarchy cleanup in correct order",
          "[odbc-api][freehandle][terminating_connection]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  
  // Create hierarchy: ENV -> DBC
  SQLHENV env = SQL_NULL_HENV;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDBC dbc = SQL_NULL_HDBC;
  ret = SQLAllocHandle(SQL_HANDLE_DBC, env, &dbc);
  REQUIRE(ret == SQL_SUCCESS);

  // HY010: Function sequence error
  ret = SQLFreeHandle(SQL_HANDLE_ENV, env);
  REQUIRE_EXPECTED_ERROR(ret, "HY010", env, SQL_HANDLE_ENV);

  ret = SQLFreeHandle(SQL_HANDLE_DBC, dbc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeHandle(SQL_HANDLE_ENV, env);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(EnvDefaultDSNFixture, "SQLFreeHandle: Freeing handle clears attributes",
                 "[odbc-api][freehandle][terminating_connection]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Allocate connection and set attribute
  SQLHDBC dbc = SQL_NULL_HDBC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env_handle(), &dbc);
  REQUIRE(ret == SQL_SUCCESS);

  // Set connection timeout
  ret = SQLSetConnectAttr(dbc, SQL_ATTR_CONNECTION_TIMEOUT, reinterpret_cast<SQLPOINTER>(30), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Free and reallocate
  ret = SQLFreeHandle(SQL_HANDLE_DBC, dbc);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDBC dbc2 = SQL_NULL_HDBC;
  ret = SQLAllocHandle(SQL_HANDLE_DBC, env_handle(), &dbc2);
  REQUIRE(ret == SQL_SUCCESS);

  // Note: Reference driver returns SQL_ERROR when getting attribute from unconnected handle
  SQLUINTEGER timeout = 999;
  ret = SQLGetConnectAttr(dbc2, SQL_ATTR_CONNECTION_TIMEOUT, &timeout, 0, nullptr);
  REQUIRE(ret == SQL_ERROR);

  // Connect and verify the attribute is the default after connecting
  ret = SQLConnect(dbc2, reinterpret_cast<SQLCHAR*>(const_cast<char*>(config.value().dsn_name().c_str())),
        SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLGetConnectAttr(dbc2, SQL_ATTR_CONNECTION_TIMEOUT, &timeout, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(timeout == 0); // Default value

  ret = SQLDisconnect(dbc2);
  REQUIRE(ret == SQL_SUCCESS);


  ret = SQLFreeHandle(SQL_HANDLE_DBC, dbc2);
  REQUIRE(ret == SQL_SUCCESS);
}
