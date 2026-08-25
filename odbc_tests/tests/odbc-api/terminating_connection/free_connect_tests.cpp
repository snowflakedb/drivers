#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstdlib>
#include <string>

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
// SQLFreeConnect - Basic Functionality (ODBC 2.x; maps to SQLFreeHandle DBC)
// ============================================================================

TEST_CASE_METHOD(DbcFixture, "SQLFreeConnect: Successfully frees connection handle",
                 "[odbc-api][freeconnect][terminating_connection]") {
  // Given An allocated (unconnected) connection handle
  REQUIRE(dbc_handle() != SQL_NULL_HDBC);

  // When SQLFreeConnect is called
  const SQLRETURN ret = SQLFreeConnect(dbc_handle());

  // Then The call succeeds (same as SQLFreeHandle(SQL_HANDLE_DBC, dbc))
  REQUIRE(ret == SQL_SUCCESS);
  release_dbc();
}

TEST_CASE("SQLFreeConnect: SQL_INVALID_HANDLE for null connection handle",
          "[odbc-api][freeconnect][terminating_connection][error]") {
  // Given A null connection handle
  // When SQLFreeConnect is called
  const SQLRETURN ret = SQLFreeConnect(SQL_NULL_HDBC);

  // Then The call returns SQL_INVALID_HANDLE
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLFreeConnect: HY010 - Cannot free connected connection handle",
                 "[odbc-api][freeconnect][terminating_connection][error]") {
  // iODBC still SIGTRAPs here on both drivers (LEAVE_HDBC MEM_FREEs the
  // wrapper even when the call is rejected as HY010) — not a fixture leak.
  SKIP_IODBC("iODBC SIGTRAPs on SQLFreeConnect while still connected (both drivers)");

  // Given A connected connection handle
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQLFreeConnect is called while still connected
  ret = SQLFreeConnect(dbc_handle());

  // Then HY010: Function sequence error
  REQUIRE_EXPECTED_ERROR(ret, "HY010", dbc_handle(), SQL_HANDLE_DBC);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLFreeConnect: Can free disconnected connection handle",
                 "[odbc-api][freeconnect][terminating_connection]") {
  // Given A connection that has been connected and disconnected
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // When SQLFreeConnect is called on the disconnected handle
  ret = SQLFreeConnect(dbc_handle());

  // Then The call succeeds
  REQUIRE(ret == SQL_SUCCESS);

  // Mark handle as freed to prevent double-free in fixture cleanup
  release_dbc();
}

TEST_CASE_METHOD(DbcDefaultDSNFixture,
                 "SQLFreeConnect: Frees dependent statement handles when connection handle is freed",
                 "[odbc-api][freeconnect][terminating_connection]") {
  // Given A disconnected connection that still had a child statement before disconnect
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  StatementHandleWrapper stmt = create_statement_handle();

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // When SQLFreeConnect frees the connection
  ret = SQLFreeConnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
  release_dbc();

  // Then The child statement handle is invalid
  REQUIRE_INVALID_HANDLE(SQL_HANDLE_STMT, stmt.getHandle());
  stmt.release();
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLFreeConnect: Double free connection handle",
                 "[odbc-api][freeconnect][terminating_connection][error]") {
  // In-process, a second SQLFreeConnect / SQLFreeHandle(DBC) SIGTRAPs (new) or
  // SIGABRTs (old) inside iODBC. REQUIRE_INVALID_HANDLE forks and only treats
  // SIGSEGV/SIGBUS as a crash, so it misreports the abort as SQL_SUCCESS.
  SKIP_IODBC("iODBC aborts on a second SQLFreeConnect / SQLFreeHandle(DBC) (both drivers)");

  // Given A connection that was freed once via SQLFreeConnect
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);

  const SQLHDBC dbc = dbc_handle();
  ret = SQLFreeConnect(dbc);
  REQUIRE(ret == SQL_SUCCESS);
  release_dbc();

  // When SQLFreeConnect is called again on the freed handle
  // Then SQL_INVALID_HANDLE
  REQUIRE_INVALID_HANDLE(SQL_HANDLE_DBC, dbc);
}

TEST_CASE_METHOD(EnvFixture, "SQLFreeConnect: SQL_INVALID_HANDLE for environment handle",
                 "[odbc-api][freeconnect][terminating_connection][error]") {
  // iODBC rejects the cross-type call before dispatching to either driver, but its exit
  // path still releases the rejected DM wrapper, so cleanup SIGTRAPs on released memory
  // without testing driver behavior — same DM defect skipped for SQLFreeHandle in
  // "SQLFreeHandle: SQL_INVALID_HANDLE for wrong statement/connection handle type".
  SKIP_IODBC("iODBC releases a rejected wrong-type handle before driver dispatch (both drivers)");

  // Given An environment handle passed where a connection handle is expected
  // When SQLFreeConnect is called with the env handle
  const SQLRETURN ret = SQLFreeConnect(env_handle());

  // Then SQL_INVALID_HANDLE
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLFreeConnect: Freeing handle clears attributes",
                 "[odbc-api][freeconnect][terminating_connection]") {
  // Given A connection with a non-default connection timeout, then freed via SQLFreeConnect
  SQLRETURN ret = SQLSetConnectAttr(dbc_handle(), SQL_ATTR_CONNECTION_TIMEOUT, reinterpret_cast<SQLPOINTER>(30), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLFreeConnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
  release_dbc();

  // When A new connection is allocated on the same environment
  ConnectionHandleWrapper dbc2 = create_connection_handle();

  // Then Attributes are not carried over; Get fails until connected, then timeout is default 0
  SQLUINTEGER timeout = 999;
  ret = SQLGetConnectAttr(dbc2.getHandle(), SQL_ATTR_CONNECTION_TIMEOUT, &timeout, 0, nullptr);
  REQUIRE(ret == SQL_ERROR);

  ret = SQLConnect(dbc2.getHandle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLGetConnectAttr(dbc2.getHandle(), SQL_ATTR_CONNECTION_TIMEOUT, &timeout, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(timeout == 0);

  ret = SQLDisconnect(dbc2.getHandle());
  REQUIRE(ret == SQL_SUCCESS);
}

// ============================================================================
// Direct driver export checks (bypass Driver Manager)
// ============================================================================

#if !defined(_WIN32)

#include <dlfcn.h>

using SQLAllocHandleFn = SQLRETURN (*)(SQLSMALLINT, SQLHANDLE, SQLHANDLE*);
using SQLFreeConnectFn = SQLRETURN (*)(SQLHDBC);
using SQLFreeHandleFn = SQLRETURN (*)(SQLSMALLINT, SQLHANDLE);
using SQLSetEnvAttrFn = SQLRETURN (*)(SQLHENV, SQLINTEGER, SQLPOINTER, SQLINTEGER);

struct DirectDriverFreeConnect {
  void* handle = nullptr;
  SQLAllocHandleFn AllocHandle = nullptr;
  SQLFreeConnectFn FreeConnect = nullptr;
  SQLFreeHandleFn FreeHandle = nullptr;
  SQLSetEnvAttrFn SetEnvAttr = nullptr;

  DirectDriverFreeConnect() {
    const std::string path = DriverConfig::get_driver_path();
    handle = dlopen(path.c_str(), RTLD_NOW);
    REQUIRE(handle != nullptr);

    AllocHandle = reinterpret_cast<SQLAllocHandleFn>(dlsym(handle, "SQLAllocHandle"));
    FreeConnect = reinterpret_cast<SQLFreeConnectFn>(dlsym(handle, "SQLFreeConnect"));
    FreeHandle = reinterpret_cast<SQLFreeHandleFn>(dlsym(handle, "SQLFreeHandle"));
    SetEnvAttr = reinterpret_cast<SQLSetEnvAttrFn>(dlsym(handle, "SQLSetEnvAttr"));

    REQUIRE(AllocHandle != nullptr);
    REQUIRE(FreeHandle != nullptr);
    REQUIRE(SetEnvAttr != nullptr);
  }

  ~DirectDriverFreeConnect() {
    if (handle) dlclose(handle);
  }
};

TEST_CASE("direct driver: SQLFreeConnect is exported and frees a connection handle",
          "[odbc-api][freeconnect][terminating_connection][direct]") {
  // The reference driver does not export SQLFreeConnect as a dlsym-visible
  // symbol; unixODBC maps SQLFreeConnect → SQLFreeHandle when the driver omits
  // the 2.x entry point. This test verifies the new driver's explicit export.
  SKIP_OLD_DRIVER("BD#127", "reference driver does not export SQLFreeConnect; DM maps it");

  // Given A driver loaded directly (bypassing the Driver Manager)
  DirectDriverFreeConnect drv;
  REQUIRE(drv.FreeConnect != nullptr);

  SQLHENV env = SQL_NULL_HENV;
  SQLRETURN ret = drv.AllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  ret = drv.SetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDBC dbc = SQL_NULL_HDBC;
  ret = drv.AllocHandle(SQL_HANDLE_DBC, env, &dbc);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQLFreeConnect is resolved from the driver and invoked
  ret = drv.FreeConnect(dbc);

  // Then The export exists and the free succeeds
  REQUIRE(ret == SQL_SUCCESS);

  ret = drv.FreeHandle(SQL_HANDLE_ENV, env);
  REQUIRE(ret == SQL_SUCCESS);
}

#else

#include <Windows.h>

TEST_CASE("direct driver: SQLFreeConnect is exported", "[odbc-api][freeconnect][terminating_connection][direct]") {
  SKIP_OLD_DRIVER("BD#127", "reference driver does not export SQLFreeConnect; DM maps it");

  // Given The driver DLL from DRIVER_PATH
  const char* path = std::getenv("DRIVER_PATH");
  REQUIRE(path != nullptr);
  HMODULE lib = LoadLibraryA(path);
  REQUIRE(lib != nullptr);

  // When SQLFreeConnect is looked up
  FARPROC fn = GetProcAddress(lib, "SQLFreeConnect");

  // Then The symbol is present in the export table
  REQUIRE(fn != nullptr);

  FreeLibrary(lib);
}

#endif
