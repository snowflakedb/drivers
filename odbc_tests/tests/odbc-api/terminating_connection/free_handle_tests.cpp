#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "ODBCConfig.hpp"
#include "ODBCFixtures.hpp"
#include "compatibility.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"
#include "test_setup.hpp"

// ============================================================================
// SQLFreeHandle - Environment Handle
// ============================================================================

TEST_CASE("SQLFreeHandle: Successfully frees environment handle", "[odbc-api][freehandle][terminating_connection]") {
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
  const SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_ENV, SQL_NULL_HENV);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(EnvFixture, "SQLFreeHandle: HY010 - Cannot free environment with active connections",
                 "[odbc-api][freehandle][terminating_connection][error]") {
  // iODBC still SIGTRAPs here on both drivers (LEAVE_HENV MEM_FREEs the env
  // wrapper even when the call is rejected as HY010) — not a fixture leak.
  SKIP_IODBC("iODBC aborts on SQLFreeHandle(ENV) with active connections (both drivers)");
  // Allocate connection on environment
  SQLHDBC dbc = SQL_NULL_HDBC;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env_handle(), &dbc);
  REQUIRE(ret == SQL_SUCCESS);

  // Try to free environment while connection exists
  // HY010: Function sequence error
  ret = SQLFreeHandle(SQL_HANDLE_ENV, env_handle());
  if (get_platform() == PLATFORM::PLATFORM_MACOS) {
    // Brew's unixODBC 2.3.14 short-circuits this call inside
    // the Driver Manager and marks the env handle as freed BEFORE the
    // function-sequence-error diagnostic is recorded. Any subsequent
    // SQLGetDiagRec on the env therefore returns SQL_INVALID_HANDLE instead
    // of HY010 (verified: probe in PR #1151 review). The driver never sees
    // the call. We can still observe that the DM rejected the free; the
    // strict SQLSTATE check is verified on Linux and Windows in the same
    // test, which exercises identical driver code paths.
    REQUIRE(ret == SQL_ERROR);
  } else {
    REQUIRE_EXPECTED_ERROR(ret, "HY010", env_handle(), SQL_HANDLE_ENV);
  }

  // Clean up
  SQLFreeHandle(SQL_HANDLE_DBC, dbc);
}

TEST_CASE("SQLFreeHandle: Double free environment handle", "[odbc-api][freehandle][terminating_connection][error]") {
  // No SQLConnect here, so no driver is loaded — the whole env lifecycle lives
  //   in iODBC's DriverManager (GENV). The second SQLFreeHandle(ENV) is decided
  //   entirely by iODBC, which returns SQL_SUCCESS instead of SQL_INVALID_HANDLE
  //   for both drivers. This is an iODBC DM limitation, not a driver behavior
  //   difference. unixODBC / Windows return SQL_INVALID_HANDLE.
  SKIP_IODBC("iODBC DM mishandles double-free of env handle, driver not in the loop (both drivers)");

  // Allocate and free environment
  SQLHENV env = SQL_NULL_HENV;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeHandle(SQL_HANDLE_ENV, env);
  REQUIRE(ret == SQL_SUCCESS);

  REQUIRE_INVALID_HANDLE(SQL_HANDLE_ENV, env);
}

// ============================================================================
// SQLFreeHandle - Connection Handle
// ============================================================================

TEST_CASE_METHOD(EnvFixture, "SQLFreeHandle: Successfully frees connection handle",
                 "[odbc-api][freehandle][terminating_connection]") {
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
  const SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_DBC, SQL_NULL_HDBC);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLFreeHandle: HY010 - Cannot free connected connection handle",
                 "[odbc-api][freehandle][terminating_connection][error]") {
  // iODBC still SIGTRAPs here on both drivers (LEAVE_HDBC MEM_FREEs the
  // wrapper even when the call is rejected as HY010) — not a fixture leak.
  SKIP_IODBC("iODBC SIGTRAPs on SQLFreeHandle(DBC) while still connected (both drivers)");
  // Connect
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
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
  // Connect and disconnect
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // Free disconnected connection
  ret = SQLFreeHandle(SQL_HANDLE_DBC, dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // Mark handle as freed to prevent double-free in fixture cleanup
  release_dbc();
}

TEST_CASE_METHOD(DbcDefaultDSNFixture,
                 "SQLFreeHandle: Frees dependent statement handles when connection handle is freed",
                 "[odbc-api][freehandle][terminating_connection]") {
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  StatementHandleWrapper stmt = create_statement_handle();

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeHandle(SQL_HANDLE_DBC, dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
  release_dbc();

  REQUIRE_INVALID_HANDLE(SQL_HANDLE_STMT, stmt.getHandle());
  stmt.release();
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLFreeHandle: Double free connection handle",
                 "[odbc-api][freehandle][terminating_connection][error]") {
  // In-process, a second SQLFreeHandle(DBC) SIGTRAPs (new) or SIGABRTs (old)
  // inside iODBC. REQUIRE_INVALID_HANDLE forks and only treats SIGSEGV/SIGBUS as
  // a crash, so it misreports the abort as SQL_SUCCESS.
  SKIP_IODBC("iODBC aborts on a second SQLFreeHandle(DBC) (both drivers)");
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);

  const SQLHDBC dbc = dbc_handle();
  ret = SQLFreeHandle(SQL_HANDLE_DBC, dbc);
  REQUIRE(ret == SQL_SUCCESS);
  release_dbc();

  REQUIRE_INVALID_HANDLE(SQL_HANDLE_DBC, dbc);
}

// ============================================================================
// SQLFreeHandle - Statement Handle
// ============================================================================

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLFreeHandle: Successfully frees statement handle",
                 "[odbc-api][freehandle][terminating_connection]") {
  // Connect first
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
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
  const SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, SQL_NULL_HSTMT);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLFreeHandle: Can free statement with prepared statement",
                 "[odbc-api][freehandle][terminating_connection]") {
  // Connect
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Allocate and prepare statement
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLPrepare(stmt, sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  REQUIRE(ret == SQL_SUCCESS);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLFreeHandle: Can free statement with active result set",
                 "[odbc-api][freehandle][terminating_connection]") {
  // Connect
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Execute query
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt, sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  REQUIRE(ret == SQL_SUCCESS);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLFreeHandle: Can free statement with bound parameters",
                 "[odbc-api][freehandle][terminating_connection]") {
  // Connect
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
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
  // Connect
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
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
  // Connect
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Allocate and free statement
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  REQUIRE(ret == SQL_SUCCESS);

  // Probe in-process. REQUIRE_INVALID_HANDLE forks and maps any non-INVALID_HANDLE
  //   return (including SQL_ERROR) to SQL_SUCCESS, which hid the old-driver result.
  const SQLRETURN second = SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  OLD_IODBC_ONLY("BD#126") {
    // The old driver still accepts the second free through iODBC and posts
    //   SQL_ERROR + HY000 on the DBC; a third SQLFreeHandle then returns
    //   SQL_INVALID_HANDLE.
    REQUIRE_EXPECTED_ERROR(second, "HY000", dbc_handle(), SQL_HANDLE_DBC);
  }
  else {
    REQUIRE(second == SQL_INVALID_HANDLE);
  }

  SQLDisconnect(dbc_handle());
}

// ============================================================================
// SQLFreeHandle - Descriptor Handle
// ============================================================================

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLFreeHandle: Can free explicitly allocated descriptor",
                 "[odbc-api][freehandle][terminating_connection]") {
  // Connect
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
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
  const SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_DESC, SQL_NULL_HDESC);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLFreeHandle: HY017 - Cannot free implicit descriptor",
                 "[odbc-api][freehandle][terminating_connection][error]") {
  // Freeing an implicit ARD must return SQL_ERROR + HY017 with the handle left valid, per the
  //   ODBC spec. A bare SQL_INVALID_HANDLE (no HY017 diagnostic) can segfault the DM on a
  //   follow-up SQLGetDiagRec. Fixed under SNOW-3240578 (Harden SQLFreeHandle state validation).
  //
  // iODBC does not safely handle this rejected free and never dispatches it to the driver,
  // so there is no driver-side behavior to assert under iODBC. Skip for both drivers.
  SKIP_IODBC(
      "iODBC does not safely handle freeing an implicit descriptor; the call never "
      "reaches the driver, so there is no driver-side behavior to assert.");
  // Connect
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
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
  // iODBC rejects this cross-type call before dispatching to either driver, but its
  // _SQLFreeHandle_DBC exit path still releases the rejected DM wrapper: ENTER_HDBC
  // jumps to LEAVE_HDBC's done label, where MEM_FREE(handle) escapes the unbraced
  // TRACE macro and runs regardless of the SQL_INVALID_HANDLE result. Cleanup then
  // depends on released wrapper memory. The reference driver's registered Arrow
  // jemalloc zone preserves the wrapper's type bytes, while the system allocator
  // used with the universal driver clears them and exposes the invalid DM state.
  // Neither outcome tests driver behavior, so skip both drivers under iODBC;
  // unixODBC/Windows and the Rust ABI-level handle-kind tests retain coverage.
  SKIP_IODBC("iODBC releases a rejected wrong-type handle before driver dispatch (both drivers)");

  // Connect
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
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
  SQLHENV env = SQL_NULL_HENV;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Try to free with invalid handle type (999)
  ret = SQLFreeHandle(999, env);
  IODBC_ONLY {
    // Under iODBC both drivers surface SQL_ERROR (DM dispatch fails the
    //   per-type lookup) instead of the spec SQL_INVALID_HANDLE asserted on
    //   unixODBC / Windows (BD#70).
    REQUIRE(ret == SQL_ERROR);
  }
  else {
    REQUIRE(ret == SQL_INVALID_HANDLE);
  }

  // Clean up properly
  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

// ============================================================================
// SQLFreeHandle - Edge Cases and Multiple Handles
// ============================================================================

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLFreeHandle: Can free multiple statement handles independently",
                 "[odbc-api][freehandle][terminating_connection]") {
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
  // iODBC still SIGTRAPs here on both drivers (LEAVE_HENV MEM_FREEs the env
  // wrapper even when the call is rejected as HY010) — not a fixture leak.
  SKIP_IODBC("iODBC SIGTRAPs on ENV/DBC hierarchy free sequences (both drivers)");
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
  if (get_platform() == PLATFORM::PLATFORM_MACOS) {
    // Brew unixODBC 2.3.14 marks env-as-freed before recording the diag, so
    // HY010 is unreachable. Observe only the SQL_ERROR rejection.
    REQUIRE(ret == SQL_ERROR);
  } else {
    REQUIRE_EXPECTED_ERROR(ret, "HY010", env, SQL_HANDLE_ENV);
  }

  ret = SQLFreeHandle(SQL_HANDLE_DBC, dbc);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeHandle(SQL_HANDLE_ENV, env);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(EnvDefaultDSNFixture, "SQLFreeHandle: Freeing handle clears attributes",
                 "[odbc-api][freehandle][terminating_connection]") {
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
  ret = SQLConnect(dbc2, sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLGetConnectAttr(dbc2, SQL_ATTR_CONNECTION_TIMEOUT, &timeout, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(timeout == 0);  // Default value

  ret = SQLDisconnect(dbc2);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeHandle(SQL_HANDLE_DBC, dbc2);
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLFreeHandle: HY010 during SQL_NEED_DATA",
                 "[odbc-api][freehandle][terminating_connection][error]") {
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

  // When SQLFreeHandle is called on the statement handle while it is in SQL_NEED_DATA
  ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt_handle());

  IODBC_ONLY {
    // Then DM return SQL_ERROR with an empty diag chain
    REQUIRE(ret == SQL_ERROR);
  }
  else {
    // Then DM surfaces HY010
    REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);
  }

  SQLCancel(stmt_handle());
}
