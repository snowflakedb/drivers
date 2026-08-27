#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstdlib>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "ODBCConfig.hpp"
#include "ODBCFixtures.hpp"
#include "compatibility.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"
#include "test_setup.hpp"

// ============================================================================
// SQLFreeEnv - Basic Functionality (ODBC 2.x; maps to SQLFreeHandle ENV)
// ============================================================================

TEST_CASE_METHOD(EnvFixture, "SQLFreeEnv: Successfully frees environment handle",
                 "[odbc-api][freeenv][terminating_connection]") {
  // Given An allocated environment with ODBC version set
  REQUIRE(env_handle() != SQL_NULL_HENV);

  // When SQLFreeEnv is called
  const SQLRETURN ret = SQLFreeEnv(env_handle());

  // Then The call succeeds (same as SQLFreeHandle(SQL_HANDLE_ENV, env))
  REQUIRE(ret == SQL_SUCCESS);
  release_env();
}

TEST_CASE("SQLFreeEnv: SQL_INVALID_HANDLE for null environment handle",
          "[odbc-api][freeenv][terminating_connection][error]") {
  // Given A null environment handle
  // When SQLFreeEnv is called
  const SQLRETURN ret = SQLFreeEnv(SQL_NULL_HENV);

  // Then The call returns SQL_INVALID_HANDLE
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(DbcFixture, "SQLFreeEnv: HY010 - Cannot free environment with active connections",
                 "[odbc-api][freeenv][terminating_connection][error]") {
  // iODBC still SIGTRAPs here on both drivers (LEAVE_HENV MEM_FREEs the env
  // wrapper even when the call is rejected as HY010) — not a fixture leak.
  SKIP_IODBC("iODBC aborts on SQLFreeEnv with active connections (both drivers)");

  // Given An environment with an allocated connection
  // When SQLFreeEnv is called while the connection still exists
  const SQLRETURN ret = SQLFreeEnv(env_handle());

  // Then HY010: Function sequence error (macOS brew unixODBC may only surface SQL_ERROR)
  if (get_platform() == PLATFORM::PLATFORM_MACOS) {
    REQUIRE(ret == SQL_ERROR);
  } else {
    REQUIRE_EXPECTED_ERROR(ret, "HY010", env_handle(), SQL_HANDLE_ENV);
  }
}

TEST_CASE_METHOD(EnvFixture, "SQLFreeEnv: Double free environment handle",
                 "[odbc-api][freeenv][terminating_connection][error]") {
  // No SQLConnect here, so no driver is loaded — the whole env lifecycle lives
  //   in iODBC's DriverManager (GENV). The second SQLFreeEnv is decided entirely
  //   by iODBC for both drivers. unixODBC / Windows return SQL_INVALID_HANDLE.
  SKIP_IODBC("iODBC DM mishandles double-free of env handle, driver not in the loop (both drivers)");

  // Given An environment that was freed once via SQLFreeEnv
  const SQLHENV env = env_handle();
  SQLRETURN ret = SQLFreeEnv(env);
  REQUIRE(ret == SQL_SUCCESS);
  release_env();

  // When SQLFreeEnv / free is probed again on the freed handle
  // Then SQL_INVALID_HANDLE
  REQUIRE_INVALID_HANDLE(SQL_HANDLE_ENV, env);
}

TEST_CASE_METHOD(DbcFixture, "SQLFreeEnv: Complete handle hierarchy cleanup in correct order",
                 "[odbc-api][freeenv][terminating_connection]") {
  // iODBC still SIGTRAPs here on both drivers (LEAVE_HENV MEM_FREEs the env
  // wrapper even when the call is rejected as HY010) — not a fixture leak.
  SKIP_IODBC("iODBC SIGTRAPs on ENV/DBC hierarchy free sequences (both drivers)");

  // Given ENV -> DBC hierarchy
  // When SQLFreeEnv is called with a live connection
  SQLRETURN ret = SQLFreeEnv(env_handle());
  if (get_platform() == PLATFORM::PLATFORM_MACOS) {
    REQUIRE(ret == SQL_ERROR);
  } else {
    REQUIRE_EXPECTED_ERROR(ret, "HY010", env_handle(), SQL_HANDLE_ENV);
  }

  // And the connection is freed first
  ret = SQLFreeHandle(SQL_HANDLE_DBC, dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
  release_dbc();

  // Then SQLFreeEnv succeeds
  ret = SQLFreeEnv(env_handle());
  REQUIRE(ret == SQL_SUCCESS);
  release_env();
}

// ============================================================================
// Direct driver export checks (bypass Driver Manager)
// ============================================================================

#if !defined(_WIN32)

#include <dlfcn.h>

using SQLAllocHandleFn = SQLRETURN (*)(SQLSMALLINT, SQLHANDLE, SQLHANDLE*);
using SQLFreeEnvFn = SQLRETURN (*)(SQLHENV);
using SQLFreeHandleFn = SQLRETURN (*)(SQLSMALLINT, SQLHANDLE);
using SQLSetEnvAttrFn = SQLRETURN (*)(SQLHENV, SQLINTEGER, SQLPOINTER, SQLINTEGER);

struct DirectDriverFreeEnv {
  void* handle = nullptr;
  SQLAllocHandleFn AllocHandle = nullptr;
  SQLFreeEnvFn FreeEnv = nullptr;
  SQLFreeHandleFn FreeHandle = nullptr;
  SQLSetEnvAttrFn SetEnvAttr = nullptr;

  DirectDriverFreeEnv() {
    std::string path = DriverConfig::get_driver_path();
    handle = dlopen(path.c_str(), RTLD_NOW);
    REQUIRE(handle != nullptr);

    AllocHandle = reinterpret_cast<SQLAllocHandleFn>(dlsym(handle, "SQLAllocHandle"));
    FreeEnv = reinterpret_cast<SQLFreeEnvFn>(dlsym(handle, "SQLFreeEnv"));
    FreeHandle = reinterpret_cast<SQLFreeHandleFn>(dlsym(handle, "SQLFreeHandle"));
    SetEnvAttr = reinterpret_cast<SQLSetEnvAttrFn>(dlsym(handle, "SQLSetEnvAttr"));

    REQUIRE(AllocHandle != nullptr);
    REQUIRE(FreeHandle != nullptr);
    REQUIRE(SetEnvAttr != nullptr);
  }

  ~DirectDriverFreeEnv() {
    if (handle) dlclose(handle);
  }
};

TEST_CASE("direct driver: SQLFreeEnv is exported and frees an environment handle",
          "[odbc-api][freeenv][terminating_connection][direct]") {
  // The reference driver does not export SQLFreeEnv as a dlsym-visible symbol;
  // unixODBC maps SQLFreeEnv → SQLFreeHandle when the driver omits the 2.x entry
  // point. This test verifies the new driver's explicit export.
  SKIP_OLD_DRIVER("BD#127", "reference driver does not export SQLFreeEnv; DM maps it");

  // Given A driver loaded directly (bypassing the Driver Manager)
  DirectDriverFreeEnv drv;
  REQUIRE(drv.FreeEnv != nullptr);

  SQLHENV env = SQL_NULL_HENV;
  SQLRETURN ret = drv.AllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  ret = drv.SetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQLFreeEnv is resolved from the driver and invoked
  ret = drv.FreeEnv(env);

  // Then The export exists and the free succeeds
  REQUIRE(ret == SQL_SUCCESS);
}

#else

#include <Windows.h>

TEST_CASE("direct driver: SQLFreeEnv is exported", "[odbc-api][freeenv][terminating_connection][direct]") {
  SKIP_OLD_DRIVER("BD#127", "reference driver does not export SQLFreeEnv; DM maps it");

  // Given The driver DLL from DRIVER_PATH
  const char* path = std::getenv("DRIVER_PATH");
  REQUIRE(path != nullptr);
  HMODULE lib = LoadLibraryA(path);
  REQUIRE(lib != nullptr);

  // When SQLFreeEnv is looked up
  FARPROC fn = GetProcAddress(lib, "SQLFreeEnv");

  // Then The symbol is present in the export table
  REQUIRE(fn != nullptr);

  FreeLibrary(lib);
}

#endif
