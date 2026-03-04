#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "macros.hpp"
#include "odbc_cast.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

// ============================================================================
// SQL_HANDLE_ENV - Environment Handle Allocation
// ============================================================================

TEST_CASE("SQLAllocHandle ENV: Basic allocation succeeds", "[odbc-api][alloc_handle][env][connecting]") {
  SQLHENV env = SQL_NULL_HENV;

  const SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(env != SQL_NULL_HENV);

  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

TEST_CASE("SQLAllocHandle ENV: Multiple allocations succeed", "[odbc-api][alloc_handle][env][connecting]") {
  constexpr int NUM_HANDLES = 5;
  SQLHENV handles[NUM_HANDLES];

  for (auto& handle : handles) {
    handle = SQL_NULL_HENV;
    const SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &handle);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(handle != SQL_NULL_HENV);
  }

  // Note: All handles are unique in Snowflake driver (implementation-specific behavior)
  for (int i = 0; i < NUM_HANDLES; i++) {
    for (int j = i + 1; j < NUM_HANDLES; j++) {
      REQUIRE(handles[i] != handles[j]);
    }
  }

  // Clean up
  for (const auto& handle : handles) {
    SQLFreeHandle(SQL_HANDLE_ENV, handle);
  }
}

TEST_CASE("SQLAllocHandle ENV: HY009 - NULL OutputHandlePtr", "[odbc-api][alloc_handle][env][connecting][error]") {
  // SQLSTATE HY009: Invalid use of null pointer
  const SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, nullptr);

  REQUIRE(ret == SQL_ERROR);
}

TEST_CASE("SQLAllocHandle ENV: Non-NULL InputHandle returns error",
          "[odbc-api][alloc_handle][env][connecting][error]") {
  SQLHENV env1 = SQL_NULL_HENV;
  SQLHENV env2 = SQL_NULL_HENV;

  // Allocate first environment
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env1);
  REQUIRE(ret == SQL_SUCCESS);

  // Allocate second environment with first environment as InputHandle
  // Per ODBC spec, InputHandle should be SQL_NULL_HANDLE for ENV
  ret = SQLAllocHandle(SQL_HANDLE_ENV, env1, &env2);

  // Driver Manager returns SQL_INVALID_HANDLE when InputHandle is not SQL_NULL_HANDLE
  REQUIRE(ret == SQL_INVALID_HANDLE);

  SQLFreeHandle(SQL_HANDLE_ENV, env1);
}

// ============================================================================
// SQL_HANDLE_DBC - Connection Handle Allocation
// ============================================================================

TEST_CASE("SQLAllocHandle DBC: Basic allocation succeeds", "[odbc-api][alloc_handle][dbc][connecting]") {
  SQLHENV env = SQL_NULL_HENV;
  SQLHDBC dbc = SQL_NULL_HDBC;

  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLAllocHandle(SQL_HANDLE_DBC, env, &dbc);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(dbc != SQL_NULL_HDBC);

  SQLFreeHandle(SQL_HANDLE_DBC, dbc);
  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

TEST_CASE("SQLAllocHandle DBC: Multiple allocations from same environment",
          "[odbc-api][alloc_handle][dbc][connecting]") {
  SQLHENV env = SQL_NULL_HENV;
  constexpr int NUM_CONNECTIONS = 5;
  SQLHDBC connections[NUM_CONNECTIONS];

  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  for (auto& connection : connections) {
    connection = SQL_NULL_HDBC;
    ret = SQLAllocHandle(SQL_HANDLE_DBC, env, &connection);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(connection != SQL_NULL_HDBC);
  }

  // All handles should be unique
  for (int i = 0; i < NUM_CONNECTIONS; i++) {
    for (int j = i + 1; j < NUM_CONNECTIONS; j++) {
      REQUIRE(connections[i] != connections[j]);
    }
  }

  for (auto& connection : connections) {
    SQLFreeHandle(SQL_HANDLE_DBC, connection);
  }
  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

TEST_CASE("SQLAllocHandle DBC: HY009 - NULL OutputHandlePtr", "[odbc-api][alloc_handle][dbc][connecting][error]") {
  SQLHENV env = SQL_NULL_HENV;

  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // SQLSTATE HY009: Invalid use of null pointer
  ret = SQLAllocHandle(SQL_HANDLE_DBC, env, nullptr);
  REQUIRE(ret == SQL_ERROR);

  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

TEST_CASE("SQLAllocHandle DBC: SQL_INVALID_HANDLE - NULL InputHandle",
          "[odbc-api][alloc_handle][dbc][connecting][error]") {
  SQLHDBC dbc = SQL_NULL_HDBC;

  // SQL_HANDLE_DBC requires valid environment handle
  const SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, SQL_NULL_HANDLE, &dbc);

  REQUIRE(ret == SQL_INVALID_HANDLE);
  REQUIRE(dbc == SQL_NULL_HDBC);
}

TEST_CASE("SQLAllocHandle DBC: HY010 - Function sequence error (ODBC version not set)",
          "[odbc-api][alloc_handle][dbc][connecting][error]") {
  SQLHENV env = SQL_NULL_HENV;
  SQLHDBC dbc = SQL_NULL_HDBC;

  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);

  // Do NOT set SQL_ATTR_ODBC_VERSION - should cause function sequence error
  ret = SQLAllocHandle(SQL_HANDLE_DBC, env, &dbc);

  // Per ODBC spec, this should return HY010 (Function sequence error)
  // The Driver Manager enforces this - allocation fails without ODBC version set
  REQUIRE_EXPECTED_ERROR(ret, "HY010", env, SQL_HANDLE_ENV);

  REQUIRE(dbc == SQL_NULL_HDBC);
  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

TEST_CASE("SQLAllocHandle DBC: SQL_INVALID_HANDLE - Wrong InputHandle type (STMT handle)",
          "[odbc-api][alloc_handle][dbc][connecting][error]") {
  // Using a connected statement handle as InputHandle for DBC allocation
  Connection conn;
  const auto stmt = conn.createStatement();

  SQLHDBC dbc = SQL_NULL_HDBC;

  // Try to allocate DBC with statement handle as parent (should fail)
  const SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, stmt.getHandle(), &dbc);

  REQUIRE(ret == SQL_INVALID_HANDLE);
}

// ============================================================================
// SQL_HANDLE_STMT - Statement Handle Allocation
// ============================================================================

TEST_CASE("SQLAllocHandle STMT: Basic allocation succeeds", "[odbc-api][alloc_handle][stmt][connecting]") {
  const std::string conn_str = get_connection_string();
  SQLHENV env = SQL_NULL_HENV;
  SQLHDBC dbc = SQL_NULL_HDBC;
  SQLHSTMT stmt = SQL_NULL_HSTMT;

  // Setup
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLAllocHandle(SQL_HANDLE_DBC, env, &dbc);
  REQUIRE(ret == SQL_SUCCESS);

  // Connect
  ret = SQLDriverConnect(dbc, nullptr, sqlchar(conn_str.c_str()), SQL_NTS, nullptr, 0, nullptr, SQL_DRIVER_NOPROMPT);
  CHECK_ODBC_ERROR(ret, dbc, SQL_HANDLE_DBC);

  // Allocate statement
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(stmt != SQL_NULL_HSTMT);

  // Verify statement is usable
  ret = SQLExecDirect(stmt, sqlchar("SELECT 1"), SQL_NTS);
  CHECK_ODBC_ERROR(ret, stmt, SQL_HANDLE_STMT);

  SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  SQLDisconnect(dbc);
  SQLFreeHandle(SQL_HANDLE_DBC, dbc);
  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

TEST_CASE("SQLAllocHandle STMT: Multiple allocations from same connection",
          "[odbc-api][alloc_handle][stmt][connecting]") {
  Connection conn;
  constexpr int NUM_STATEMENTS = 10;
  std::vector<StatementHandleWrapper> statements;

  for (int i = 0; i < NUM_STATEMENTS; i++) {
    statements.push_back(conn.createStatement());
    REQUIRE(statements[i].getHandle() != SQL_NULL_HSTMT);
  }

  // All handles should be unique
  for (int i = 0; i < NUM_STATEMENTS; i++) {
    for (int j = i + 1; j < NUM_STATEMENTS; j++) {
      REQUIRE(statements[i].getHandle() != statements[j].getHandle());
    }
  }

  // Verify all statements are usable
  for (int i = 0; i < NUM_STATEMENTS; i++) {
    const std::string query = "SELECT " + std::to_string(i);
    SQLRETURN ret = SQLExecDirect(statements[i].getHandle(), sqlchar(query.c_str()), SQL_NTS);
    CHECK_ODBC(ret, statements[i]);
  }
}

TEST_CASE("SQLAllocHandle STMT: HY009 - NULL OutputHandlePtr", "[odbc-api][alloc_handle][stmt][connecting][error]") {
  SQLHENV env = SQL_NULL_HENV;
  SQLHDBC dbc = SQL_NULL_HDBC;

  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLAllocHandle(SQL_HANDLE_DBC, env, &dbc);
  REQUIRE(ret == SQL_SUCCESS);

  // SQLSTATE HY009: Invalid use of null pointer
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc, nullptr);
  REQUIRE(ret == SQL_ERROR);

  SQLDisconnect(dbc);
  SQLFreeHandle(SQL_HANDLE_DBC, dbc);
  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

TEST_CASE("SQLAllocHandle STMT: SQL_INVALID_HANDLE - NULL InputHandle",
          "[odbc-api][alloc_handle][stmt][connecting][error]") {
  SQLHSTMT stmt = SQL_NULL_HSTMT;

  const SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, SQL_NULL_HANDLE, &stmt);

  REQUIRE(ret == SQL_INVALID_HANDLE);
  REQUIRE(stmt == SQL_NULL_HSTMT);
}

TEST_CASE("SQLAllocHandle STMT: SQL_INVALID_HANDLE - Wrong InputHandle type (ENV handle)",
          "[odbc-api][alloc_handle][stmt][connecting][error]") {
  SQLHENV env = SQL_NULL_HENV;
  SQLHSTMT stmt = SQL_NULL_HSTMT;

  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Try to allocate STMT with environment handle (should fail - needs DBC)
  ret = SQLAllocHandle(SQL_HANDLE_STMT, env, &stmt);

  REQUIRE(ret == SQL_INVALID_HANDLE);

  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

TEST_CASE("SQLAllocHandle STMT: 08003 - Connection not open", "[odbc-api][alloc_handle][stmt][connecting][error]") {
  SQLHENV env = SQL_NULL_HENV;
  SQLHDBC dbc = SQL_NULL_HDBC;
  SQLHSTMT stmt = SQL_NULL_HSTMT;

  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLAllocHandle(SQL_HANDLE_DBC, env, &dbc);
  REQUIRE(ret == SQL_SUCCESS);

  // Do NOT connect - try to allocate statement on unconnected DBC
  // SQLSTATE 08003: Connection not open
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt);

  // Per ODBC spec, this should return 08003
  REQUIRE_EXPECTED_ERROR(ret, "08003", dbc, SQL_HANDLE_DBC);

  SQLFreeHandle(SQL_HANDLE_DBC, dbc);
  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

// ============================================================================
// SQL_HANDLE_DESC - Descriptor Handle Allocation
// ============================================================================

TEST_CASE("SQLAllocHandle DESC: Basic allocation succeeds", "[odbc-api][alloc_handle][desc][connecting]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const std::string conn_str = get_connection_string();
  SQLHENV env = SQL_NULL_HENV;
  SQLHDBC dbc = SQL_NULL_HDBC;
  SQLHANDLE desc = SQL_NULL_HANDLE;

  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLAllocHandle(SQL_HANDLE_DBC, env, &dbc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLDriverConnect(dbc, nullptr, sqlchar(conn_str.c_str()), SQL_NTS, nullptr, 0, nullptr, SQL_DRIVER_NOPROMPT);
  CHECK_ODBC_ERROR(ret, dbc, SQL_HANDLE_DBC);

  // Allocate descriptor handle - should succeed on ODBC 3.x compliant drivers
  ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc, &desc);

  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(desc != SQL_NULL_HANDLE);

  SQLFreeHandle(SQL_HANDLE_DESC, desc);
  SQLDisconnect(dbc);
  SQLFreeHandle(SQL_HANDLE_DBC, dbc);
  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

TEST_CASE("SQLAllocHandle DESC: Multiple allocations from same connection",
          "[odbc-api][alloc_handle][desc][connecting]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const std::string conn_str = get_connection_string();
  SQLHENV env = SQL_NULL_HENV;
  SQLHDBC dbc = SQL_NULL_HDBC;
  constexpr int NUM_DESCS = 3;
  SQLHANDLE descs[NUM_DESCS];

  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLAllocHandle(SQL_HANDLE_DBC, env, &dbc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLDriverConnect(dbc, nullptr, sqlchar(conn_str.c_str()), SQL_NTS, nullptr, 0, nullptr, SQL_DRIVER_NOPROMPT);
  CHECK_ODBC_ERROR(ret, dbc, SQL_HANDLE_DBC);

  for (auto& desc : descs) {
    desc = SQL_NULL_HANDLE;
    ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc, &desc);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(desc != SQL_NULL_HANDLE);
  }

  // All handles should be unique
  for (int i = 0; i < NUM_DESCS; i++) {
    for (int j = i + 1; j < NUM_DESCS; j++) {
      REQUIRE(descs[i] != descs[j]);
    }
  }

  for (auto& desc : descs) {
    SQLFreeHandle(SQL_HANDLE_DESC, desc);
  }

  SQLDisconnect(dbc);
  SQLFreeHandle(SQL_HANDLE_DBC, dbc);
  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

TEST_CASE("SQLAllocHandle DESC: HY009 - NULL OutputHandlePtr", "[odbc-api][alloc_handle][desc][connecting][error]") {
  const std::string conn_str = get_connection_string();
  SQLHENV env = SQL_NULL_HENV;
  SQLHDBC dbc = SQL_NULL_HDBC;

  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLAllocHandle(SQL_HANDLE_DBC, env, &dbc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLDriverConnect(dbc, nullptr, sqlchar(conn_str.c_str()), SQL_NTS, nullptr, 0, nullptr, SQL_DRIVER_NOPROMPT);
  CHECK_ODBC_ERROR(ret, dbc, SQL_HANDLE_DBC);

  ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc, nullptr);
  REQUIRE(ret == SQL_ERROR);

  SQLDisconnect(dbc);
  SQLFreeHandle(SQL_HANDLE_DBC, dbc);
  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

TEST_CASE("SQLAllocHandle DESC: SQL_INVALID_HANDLE - NULL InputHandle",
          "[odbc-api][alloc_handle][desc][connecting][error]") {
  SQLHANDLE desc = SQL_NULL_HANDLE;

  const SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DESC, SQL_NULL_HANDLE, &desc);

  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE("SQLAllocHandle DESC: 08003 - Connection not open", "[odbc-api][alloc_handle][desc][connecting][error]") {
  SQLHENV env = SQL_NULL_HENV;
  SQLHDBC dbc = SQL_NULL_HDBC;
  SQLHANDLE desc = SQL_NULL_HANDLE;

  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLAllocHandle(SQL_HANDLE_DBC, env, &dbc);
  REQUIRE(ret == SQL_SUCCESS);

  // Do NOT connect - try to allocate descriptor on unconnected DBC
  ret = SQLAllocHandle(SQL_HANDLE_DESC, dbc, &desc);

  // Per ODBC spec, this should return 08003: Connection not open
  REQUIRE_EXPECTED_ERROR(ret, "08003", dbc, SQL_HANDLE_DBC);

  SQLFreeHandle(SQL_HANDLE_DBC, dbc);
  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

// ============================================================================
// Invalid Handle Types - HY092
// ============================================================================

TEST_CASE("SQLAllocHandle: HY092 - Invalid HandleType (negative)", "[odbc-api][alloc_handle][connecting][error]") {
  SQLHANDLE handle = SQL_NULL_HANDLE;

  const SQLRETURN ret = SQLAllocHandle(-1, SQL_NULL_HANDLE, &handle);

  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE("SQLAllocHandle: HY092 - Invalid HandleType (zero)", "[odbc-api][alloc_handle][connecting][error]") {
  SQLHANDLE handle = SQL_NULL_HANDLE;

  const SQLRETURN ret = SQLAllocHandle(0, SQL_NULL_HANDLE, &handle);

  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE("SQLAllocHandle: HY092 - Invalid HandleType (large number)", "[odbc-api][alloc_handle][connecting][error]") {
  SQLHANDLE handle = SQL_NULL_HANDLE;

  const SQLRETURN ret = SQLAllocHandle(9999, SQL_NULL_HANDLE, &handle);

  REQUIRE(ret == SQL_INVALID_HANDLE);
}

// ============================================================================
// ODBC Version Compatibility
// ============================================================================

TEST_CASE("SQLAllocHandle: Works with SQL_OV_ODBC3", "[odbc-api][alloc_handle][connecting][version]") {
  SQLHENV env = SQL_NULL_HENV;
  SQLHDBC dbc = SQL_NULL_HDBC;

  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLAllocHandle(SQL_HANDLE_DBC, env, &dbc);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(dbc != SQL_NULL_HDBC);

  SQLFreeHandle(SQL_HANDLE_DBC, dbc);
  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

TEST_CASE("SQLAllocHandle: Works with SQL_OV_ODBC3_80", "[odbc-api][alloc_handle][connecting][version]") {
  SQLHENV env = SQL_NULL_HENV;
  SQLHDBC dbc = SQL_NULL_HDBC;

  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3_80), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLAllocHandle(SQL_HANDLE_DBC, env, &dbc);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(dbc != SQL_NULL_HDBC);

  SQLFreeHandle(SQL_HANDLE_DBC, dbc);
  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

TEST_CASE("SQLAllocHandle: Works with SQL_OV_ODBC2", "[odbc-api][alloc_handle][connecting][version]") {
  SQLHENV env = SQL_NULL_HENV;
  SQLHDBC dbc = SQL_NULL_HDBC;

  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC2), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLAllocHandle(SQL_HANDLE_DBC, env, &dbc);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(dbc != SQL_NULL_HDBC);

  SQLFreeHandle(SQL_HANDLE_DBC, dbc);
  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

// ============================================================================
// Handle Hierarchy and Lifecycle
// ============================================================================

TEST_CASE("SQLAllocHandle: Complete ENV -> DBC -> STMT hierarchy", "[odbc-api][alloc_handle][connecting][hierarchy]") {
  const std::string conn_str = get_connection_string();
  SQLHENV env = SQL_NULL_HENV;
  SQLHDBC dbc = SQL_NULL_HDBC;
  SQLHSTMT stmt = SQL_NULL_HSTMT;

  // Allocate environment
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(env != SQL_NULL_HENV);

  // Set ODBC version
  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Allocate connection
  ret = SQLAllocHandle(SQL_HANDLE_DBC, env, &dbc);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(dbc != SQL_NULL_HDBC);

  // Connect
  ret = SQLDriverConnect(dbc, nullptr, sqlchar(conn_str.c_str()), SQL_NTS, nullptr, 0, nullptr, SQL_DRIVER_NOPROMPT);
  CHECK_ODBC_ERROR(ret, dbc, SQL_HANDLE_DBC);

  // Allocate statement
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(stmt != SQL_NULL_HSTMT);

  // Execute a query to verify the hierarchy works
  ret = SQLExecDirect(stmt, sqlchar("SELECT 42"), SQL_NTS);
  CHECK_ODBC_ERROR(ret, stmt, SQL_HANDLE_STMT);

  ret = SQLFetch(stmt);
  CHECK_ODBC_ERROR(ret, stmt, SQL_HANDLE_STMT);

  SQLINTEGER value;
  ret = SQLGetData(stmt, 1, SQL_C_LONG, &value, sizeof(value), nullptr);
  CHECK_ODBC_ERROR(ret, stmt, SQL_HANDLE_STMT);
  REQUIRE(value == 42);

  // Clean up in reverse order
  ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLDisconnect(dbc);
  CHECK_ODBC_ERROR(ret, dbc, SQL_HANDLE_DBC);

  ret = SQLFreeHandle(SQL_HANDLE_DBC, dbc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeHandle(SQL_HANDLE_ENV, env);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE("SQLAllocHandle: Handle reuse after free", "[odbc-api][alloc_handle][connecting][lifecycle]") {
  SQLHENV env = SQL_NULL_HENV;

  // First allocation
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(env != SQL_NULL_HENV);

  // Free
  ret = SQLFreeHandle(SQL_HANDLE_ENV, env);
  REQUIRE(ret == SQL_SUCCESS);

  // Second allocation - variable can be reused after free
  env = SQL_NULL_HENV;
  ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(env != SQL_NULL_HENV);

  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

TEST_CASE("SQLAllocHandle: Overwriting existing handle variable", "[odbc-api][alloc_handle][connecting][lifecycle]") {
  SQLHENV env = SQL_NULL_HENV;
  SQLHENV first_handle = SQL_NULL_HENV;

  // First allocation
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  first_handle = env;

  // Allocate again WITHOUT freeing first - this overwrites the variable
  // This is incorrect usage per ODBC spec but should not crash
  ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);

  // env now contains new handle, first_handle still holds the original
  REQUIRE(env != first_handle);

  // Clean up both handles
  SQLFreeHandle(SQL_HANDLE_ENV, env);
  SQLFreeHandle(SQL_HANDLE_ENV, first_handle);
}

// ============================================================================
// Edge Cases
// ============================================================================

TEST_CASE("SQLAllocHandle: Rapid allocation and deallocation", "[odbc-api][alloc_handle][connecting][stress]") {
  constexpr int ITERATIONS = 100;

  for (int i = 0; i < ITERATIONS; i++) {
    SQLHENV env = SQL_NULL_HENV;
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(env != SQL_NULL_HENV);

    ret = SQLFreeHandle(SQL_HANDLE_ENV, env);
    REQUIRE(ret == SQL_SUCCESS);
  }
}

TEST_CASE("SQLAllocHandle: Mixed handle type allocations", "[odbc-api][alloc_handle][connecting]") {
  SQLHENV env1 = SQL_NULL_HENV;
  SQLHENV env2 = SQL_NULL_HENV;
  SQLHDBC dbc1 = SQL_NULL_HDBC;
  SQLHDBC dbc2 = SQL_NULL_HDBC;

  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env1);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetEnvAttr(env1, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env2);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetEnvAttr(env2, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Allocate connection from each environment
  ret = SQLAllocHandle(SQL_HANDLE_DBC, env1, &dbc1);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLAllocHandle(SQL_HANDLE_DBC, env2, &dbc2);
  REQUIRE(ret == SQL_SUCCESS);

  // Note: All handles are unique in Snowflake driver (implementation-specific behavior)
  REQUIRE(env1 != env2);
  REQUIRE(dbc1 != dbc2);
  REQUIRE(reinterpret_cast<SQLHANDLE>(env1) != reinterpret_cast<SQLHANDLE>(dbc1));
  REQUIRE(reinterpret_cast<SQLHANDLE>(env2) != reinterpret_cast<SQLHANDLE>(dbc2));

  // Clean up
  SQLFreeHandle(SQL_HANDLE_DBC, dbc1);
  SQLFreeHandle(SQL_HANDLE_DBC, dbc2);
  SQLFreeHandle(SQL_HANDLE_ENV, env1);
  SQLFreeHandle(SQL_HANDLE_ENV, env2);
}

// ============================================================================
// Return Value Verification
// ============================================================================

TEST_CASE("SQLAllocHandle: Returns SQL_SUCCESS on successful allocation",
          "[odbc-api][alloc_handle][connecting][return]") {
  SQLHENV env = SQL_NULL_HENV;

  const SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);

  // Must be exactly SQL_SUCCESS (not SQL_SUCCESS_WITH_INFO for basic allocation)
  REQUIRE(ret == SQL_SUCCESS);

  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

TEST_CASE("SQLAllocHandle: Fails with invalid parent handle", "[odbc-api][alloc_handle][connecting][return]") {
  SQLHDBC dbc = SQL_NULL_HDBC;

  // This should fail (DBC without valid ENV parent)
  const SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, SQL_NULL_HANDLE, &dbc);

  REQUIRE(ret == SQL_INVALID_HANDLE);
}

// ============================================================================
// Implicit Descriptor Allocation (via Statement)
// ============================================================================

TEST_CASE("SQLAllocHandle STMT: Implicit descriptor handles are allocated",
          "[odbc-api][alloc_handle][stmt][connecting][desc]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  Connection conn;
  auto stmt = conn.createStatement();

  // When a statement is allocated, four descriptor handles are implicitly allocated:
  // SQL_ATTR_APP_ROW_DESC, SQL_ATTR_APP_PARAM_DESC, SQL_ATTR_IMP_ROW_DESC, SQL_ATTR_IMP_PARAM_DESC

  SQLHANDLE ard = SQL_NULL_HANDLE;
  SQLHANDLE apd = SQL_NULL_HANDLE;
  SQLHANDLE ird = SQL_NULL_HANDLE;
  SQLHANDLE ipd = SQL_NULL_HANDLE;

  // Get the implicit descriptor handles - these should always succeed
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(ard != SQL_NULL_HANDLE);

  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_PARAM_DESC, &apd, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(apd != SQL_NULL_HANDLE);

  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_IMP_ROW_DESC, &ird, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(ird != SQL_NULL_HANDLE);

  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_IMP_PARAM_DESC, &ipd, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(ipd != SQL_NULL_HANDLE);

  // All four descriptors should be unique
  REQUIRE(ard != apd);
  REQUIRE(ard != ird);
  REQUIRE(ard != ipd);
  REQUIRE(apd != ird);
  REQUIRE(apd != ipd);
  REQUIRE(ird != ipd);
}
