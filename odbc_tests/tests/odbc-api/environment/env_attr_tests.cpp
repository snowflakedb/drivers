#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstdint>
#include <cstring>
#include <string>

#include <catch2/catch_test_macros.hpp>
#include <catch2/generators/catch_generators.hpp>

#include "HandleWrapper.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "test_macros.hpp"

#if !defined(_WIN32)
#include <dlfcn.h>

#include "ODBCConfig.hpp"
#endif

// SQL_CP_DRIVER_AWARE is defined in ODBC 3.8 but may not be in all system headers
#ifndef SQL_CP_DRIVER_AWARE
#define SQL_CP_DRIVER_AWARE 3UL
#endif

// iODBC extension attribute and code pages, defined only in iODBC's iodbcext.h
#ifndef SQL_ATTR_DRIVER_UNICODE_TYPE
#define SQL_ATTR_DRIVER_UNICODE_TYPE 1065
#endif
#ifndef SQL_DM_CP_UTF16
#define SQL_DM_CP_UTF16 1
#endif
#ifndef SQL_DM_CP_UCS4
#define SQL_DM_CP_UCS4 3
#endif

// ============================================================================
// SQL_ATTR_ODBC_VERSION
// ============================================================================

TEST_CASE("should set and get SQL_ATTR_ODBC_VERSION with valid values", "[odbc-api][env_attr][version]") {
  SQLINTEGER version = GENERATE(SQL_OV_ODBC2, SQL_OV_ODBC3, SQL_OV_ODBC3_80);

  // Given A freshly allocated environment handle
  SQLHENV env = SQL_NULL_HENV;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQL_ATTR_ODBC_VERSION is set on that env
  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, sqlptr_value(version), 0);

  if (version == SQL_OV_ODBC3_80) {
    NON_IODBC {
      // Then the value is accepted and round-trips
      REQUIRE(ret == SQL_SUCCESS);
      SQLINTEGER got = 0;
      ret = SQLGetEnvAttr(env, SQL_ATTR_ODBC_VERSION, &got, 0, nullptr);
      REQUIRE(ret == SQL_SUCCESS);
      CHECK(got == version);
    }
    IODBC_ONLY {
      // Then the call is rejected with SQLSTATE HY024 (invalid attribute value)
      REQUIRE(ret == SQL_ERROR);
      auto records = get_diag_rec(SQL_HANDLE_ENV, env);
      REQUIRE(!records.empty());
      CHECK(records[0].sqlState == "HY024");
    }
  } else {
    REQUIRE(ret == SQL_SUCCESS);
    SQLINTEGER got = 0;
    ret = SQLGetEnvAttr(env, SQL_ATTR_ODBC_VERSION, &got, 0, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(got == version);
  }

  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

TEST_CASE("should return HY024 for invalid SQL_ATTR_ODBC_VERSION value", "[odbc-api][env_attr][version][error]") {
  // Given A freshly allocated environment handle
  SQLHENV env = SQL_NULL_HENV;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQL_ATTR_ODBC_VERSION is set to an invalid value
  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(9999), 0);

  // Then It should return SQL_ERROR
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(SQL_HANDLE_ENV, env);
  REQUIRE(!records.empty());
  CHECK((records[0].sqlState == "HY024" || records[0].sqlState == "S1009"));

  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

// ============================================================================
// SQL_ATTR_CONNECTION_POOLING
// ============================================================================

TEST_CASE("should set and get SQL_ATTR_CONNECTION_POOLING with SQL_CP_OFF", "[odbc-api][env_attr][pooling]") {
  // Given An environment handle with ODBC version set
  SQLHENV env = SQL_NULL_HENV;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQL_ATTR_CONNECTION_POOLING is set to SQL_CP_OFF
  ret = SQLSetEnvAttr(env, SQL_ATTR_CONNECTION_POOLING, reinterpret_cast<SQLPOINTER>(SQL_CP_OFF), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then Getting the attribute should return SQL_CP_OFF
  SQLINTEGER pooling = -1;
  ret = SQLGetEnvAttr(env, SQL_ATTR_CONNECTION_POOLING, &pooling, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(pooling == SQL_CP_OFF);

  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

TEST_CASE("should set and get SQL_ATTR_CONNECTION_POOLING with SQL_CP_ONE_PER_DRIVER",
          "[odbc-api][env_attr][pooling]") {
  // Given An environment handle with ODBC version set
  SQLHENV env = SQL_NULL_HENV;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQL_ATTR_CONNECTION_POOLING is set to SQL_CP_ONE_PER_DRIVER
  ret = SQLSetEnvAttr(env, SQL_ATTR_CONNECTION_POOLING, reinterpret_cast<SQLPOINTER>(SQL_CP_ONE_PER_DRIVER), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then Getting the attribute should return SQL_CP_ONE_PER_DRIVER
  SQLINTEGER pooling = -1;
  ret = SQLGetEnvAttr(env, SQL_ATTR_CONNECTION_POOLING, &pooling, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(pooling == SQL_CP_ONE_PER_DRIVER);

  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

TEST_CASE("should set and get SQL_ATTR_CONNECTION_POOLING with SQL_CP_ONE_PER_HENV", "[odbc-api][env_attr][pooling]") {
  // Given An environment handle with ODBC version set
  SQLHENV env = SQL_NULL_HENV;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQL_ATTR_CONNECTION_POOLING is set to SQL_CP_ONE_PER_HENV
  ret = SQLSetEnvAttr(env, SQL_ATTR_CONNECTION_POOLING, reinterpret_cast<SQLPOINTER>(SQL_CP_ONE_PER_HENV), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then Getting the attribute should return SQL_CP_ONE_PER_HENV
  SQLINTEGER pooling = -1;
  ret = SQLGetEnvAttr(env, SQL_ATTR_CONNECTION_POOLING, &pooling, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(pooling == SQL_CP_ONE_PER_HENV);

  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

TEST_CASE("should set and get SQL_ATTR_CONNECTION_POOLING with SQL_CP_DRIVER_AWARE", "[odbc-api][env_attr][pooling]") {
  // Given An environment handle with ODBC version set
  SQLHENV env = SQL_NULL_HENV;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQL_ATTR_CONNECTION_POOLING is set to SQL_CP_DRIVER_AWARE (ODBC 3.8)
  ret = SQLSetEnvAttr(env, SQL_ATTR_CONNECTION_POOLING, reinterpret_cast<SQLPOINTER>(SQL_CP_DRIVER_AWARE), 0);
  if (ret == SQL_SUCCESS) {
    // Then Getting the attribute should return SQL_CP_DRIVER_AWARE
    SQLINTEGER pooling = -1;
    ret = SQLGetEnvAttr(env, SQL_ATTR_CONNECTION_POOLING, &pooling, 0, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(pooling == SQL_CP_DRIVER_AWARE);
  } else {
    WARN("Driver Manager rejected SQL_CP_DRIVER_AWARE");
  }

  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

// ============================================================================
// SQL_ATTR_CP_MATCH
// ============================================================================

TEST_CASE("should set and get SQL_ATTR_CP_MATCH with SQL_CP_STRICT_MATCH", "[odbc-api][env_attr][cp_match]") {
  // Given An environment handle with ODBC version set
  SQLHENV env = SQL_NULL_HENV;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQL_ATTR_CP_MATCH is set to SQL_CP_STRICT_MATCH
  ret = SQLSetEnvAttr(env, SQL_ATTR_CP_MATCH, reinterpret_cast<SQLPOINTER>(SQL_CP_STRICT_MATCH), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then Getting the attribute should return SQL_CP_STRICT_MATCH
  SQLINTEGER cp_match = -1;
  ret = SQLGetEnvAttr(env, SQL_ATTR_CP_MATCH, &cp_match, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(cp_match == SQL_CP_STRICT_MATCH);

  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

TEST_CASE("should set and get SQL_ATTR_CP_MATCH with SQL_CP_RELAXED_MATCH", "[odbc-api][env_attr][cp_match]") {
  // Given An environment handle with ODBC version set
  SQLHENV env = SQL_NULL_HENV;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQL_ATTR_CP_MATCH is set to SQL_CP_RELAXED_MATCH
  ret = SQLSetEnvAttr(env, SQL_ATTR_CP_MATCH, reinterpret_cast<SQLPOINTER>(SQL_CP_RELAXED_MATCH), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then Getting the attribute should return SQL_CP_RELAXED_MATCH
  SQLINTEGER cp_match = -1;
  ret = SQLGetEnvAttr(env, SQL_ATTR_CP_MATCH, &cp_match, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(cp_match == SQL_CP_RELAXED_MATCH);

  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

TEST_CASE("should return HY024 for invalid SQL_ATTR_CONNECTION_POOLING value", "[odbc-api][env_attr][pooling][error]") {
  // Given An environment handle with ODBC version set
  SQLHENV env = SQL_NULL_HENV;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQL_ATTR_CONNECTION_POOLING is set to an invalid value
  ret = SQLSetEnvAttr(env, SQL_ATTR_CONNECTION_POOLING, reinterpret_cast<SQLPOINTER>(9999), 0);

  // Then It should return SQL_ERROR with HY024
  REQUIRE_EXPECTED_ERROR(ret, "HY024", env, SQL_HANDLE_ENV);

  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

TEST_CASE("should return HY024 for invalid SQL_ATTR_CP_MATCH value", "[odbc-api][env_attr][cp_match][error]") {
  // Given An environment handle with ODBC version set
  SQLHENV env = SQL_NULL_HENV;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQL_ATTR_CP_MATCH is set to an invalid value
  ret = SQLSetEnvAttr(env, SQL_ATTR_CP_MATCH, reinterpret_cast<SQLPOINTER>(9999), 0);

  // Then It should return SQL_ERROR with HY024
  REQUIRE_EXPECTED_ERROR(ret, "HY024", env, SQL_HANDLE_ENV);

  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

// ============================================================================
// SQL_ATTR_OUTPUT_NTS
// ============================================================================

TEST_CASE("should accept SQL_ATTR_OUTPUT_NTS set to SQL_TRUE", "[odbc-api][env_attr][output_nts]") {
  // Given An environment handle with ODBC version set
  EnvironmentHandleWrapper env;
  SQLRETURN ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQL_ATTR_OUTPUT_NTS is set to SQL_TRUE
  ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_OUTPUT_NTS, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);

  // Then It should return SQL_SUCCESS
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE("should get SQL_ATTR_OUTPUT_NTS as SQL_TRUE", "[odbc-api][env_attr][output_nts]") {
  // Given An environment handle with ODBC version set
  EnvironmentHandleWrapper env;
  SQLRETURN ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQL_ATTR_OUTPUT_NTS is queried. The Driver Manager usually services
  // this attribute itself (it always requests null-terminated strings), so the
  // call may not reach the driver; either way the value must be SQL_TRUE. The
  // driver's own behavior is pinned by the direct-driver test below.
  SQLINTEGER nts = -1;
  ret = SQLGetEnvAttr(env.getHandle(), SQL_ATTR_OUTPUT_NTS, &nts, 0, nullptr);
  if (SQL_SUCCEEDED(ret)) {
    CHECK(nts == SQL_TRUE);
  }
}

TEST_CASE("should reject SQL_ATTR_OUTPUT_NTS set to SQL_FALSE with HYC00", "[odbc-api][env_attr][output_nts][error]") {
  // Given An environment handle with ODBC version set
  EnvironmentHandleWrapper env;
  SQLRETURN ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQL_ATTR_OUTPUT_NTS is set to SQL_FALSE
  ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_OUTPUT_NTS, reinterpret_cast<SQLPOINTER>(SQL_FALSE), 0);

  // Then It should return SQL_ERROR — exact SQLSTATE depends on whether the Driver Manager
  // intercepts (HYC00) or forwards to the driver (HY092)
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(SQL_HANDLE_ENV, env.getHandle());
  REQUIRE(!records.empty());
  CHECK((records[0].sqlState == "HYC00" || records[0].sqlState == "HY092"));
}

// ============================================================================
// Unknown attributes
// ============================================================================

TEST_CASE("should return error when setting an unknown environment attribute", "[odbc-api][env_attr][error]") {
  // Given An environment handle with ODBC version set
  EnvironmentHandleWrapper env;
  SQLRETURN ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When An unknown attribute ID is used in SQLSetEnvAttr
  ret = SQLSetEnvAttr(env.getHandle(), 99999, reinterpret_cast<SQLPOINTER>(1), 0);

  // Then It should return SQL_ERROR
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(SQL_HANDLE_ENV, env.getHandle());
  REQUIRE(!records.empty());
  CHECK((records[0].sqlState == "HYC00" || records[0].sqlState == "HY092"));
}

TEST_CASE("should return error when getting an unknown environment attribute", "[odbc-api][env_attr][error]") {
  // Given An environment handle with ODBC version set
  EnvironmentHandleWrapper env;
  SQLRETURN ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When An unknown attribute ID is used in SQLGetEnvAttr
  SQLINTEGER value = 0;
  ret = SQLGetEnvAttr(env.getHandle(), 99999, &value, 0, nullptr);

  // Then It should return SQL_ERROR
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(SQL_HANDLE_ENV, env.getHandle());
  REQUIRE(!records.empty());
  CHECK((records[0].sqlState == "HYC00" || records[0].sqlState == "HY092"));
}

// ============================================================================
// Diagnostics
// ============================================================================

TEST_CASE("should populate diagnostic records on environment attribute error", "[odbc-api][env_attr][diagnostics]") {
  // Given An environment handle with ODBC version set
  SQLHENV env = SQL_NULL_HENV;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQLSetEnvAttr fails with an unknown attribute
  ret = SQLSetEnvAttr(env, 99999, reinterpret_cast<SQLPOINTER>(1), 0);
  REQUIRE(ret == SQL_ERROR);

  // Then SQLGetDiagRec should return a diagnostic record with the error
  auto records = get_diag_rec(SQL_HANDLE_ENV, env);
  REQUIRE(!records.empty());
  CHECK((records[0].sqlState == "HYC00" || records[0].sqlState == "HY092"));
  CHECK(!records[0].messageText.empty());

  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

TEST_CASE("should clear diagnostics between environment attribute calls", "[odbc-api][env_attr][diagnostics]") {
  // Given An environment handle with a previous error
  SQLHENV env = SQL_NULL_HENV;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Trigger an error first
  ret = SQLSetEnvAttr(env, 99999, reinterpret_cast<SQLPOINTER>(1), 0);
  REQUIRE(ret == SQL_ERROR);

  // When A successful attribute call is made
  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then Diagnostics should be cleared (no records from previous error)
  auto records = get_diag_rec(SQL_HANDLE_ENV, env);
  CHECK(records.empty());

  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

// ============================================================================
// StringLengthPtr output for SQLGetEnvAttr
// ============================================================================

TEST_CASE("should accept null StringLengthPtr in SQLGetEnvAttr", "[odbc-api][env_attr][string_length]") {
  // Given An environment handle with ODBC version set
  SQLHENV env = SQL_NULL_HENV;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQLGetEnvAttr is called with a null StringLengthPtr
  SQLINTEGER version = 0;
  ret = SQLGetEnvAttr(env, SQL_ATTR_ODBC_VERSION, &version, 0, nullptr);

  // Then The call should succeed and return the value
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(version == SQL_OV_ODBC3);

  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

// ============================================================================
// Direct driver tests (bypass Driver Manager via dlopen)
//
// The Driver Manager intercepts some env attributes (OUTPUT_NTS, unknown IDs)
// and returns its own SQLSTATEs. These tests load the driver directly to
// verify the exact behavior our driver produces.
//
// dlopen/dlsym is POSIX-only; skip on Windows.
// ============================================================================

#if !defined(_WIN32)

// Function pointer types matching ODBC C API signatures
using SQLAllocHandleFn = SQLRETURN (*)(SQLSMALLINT, SQLHANDLE, SQLHANDLE*);
using SQLFreeHandleFn = SQLRETURN (*)(SQLSMALLINT, SQLHANDLE);
using SQLSetEnvAttrFn = SQLRETURN (*)(SQLHENV, SQLINTEGER, SQLPOINTER, SQLINTEGER);
using SQLGetEnvAttrFn = SQLRETURN (*)(SQLHENV, SQLINTEGER, SQLPOINTER, SQLINTEGER, SQLINTEGER*);
using SQLGetDiagRecFn = SQLRETURN (*)(SQLSMALLINT, SQLHANDLE, SQLSMALLINT, SQLCHAR*, SQLINTEGER*, SQLCHAR*, SQLSMALLINT,
                                      SQLSMALLINT*);
using SQLGetDiagRecWFn = SQLRETURN (*)(SQLSMALLINT, SQLHANDLE, SQLSMALLINT, SQLWCHAR*, SQLINTEGER*, SQLWCHAR*,
                                       SQLSMALLINT, SQLSMALLINT*);

struct DirectDriver {
  void* handle = nullptr;
  SQLAllocHandleFn AllocHandle = nullptr;
  SQLFreeHandleFn FreeHandle = nullptr;
  SQLSetEnvAttrFn SetEnvAttr = nullptr;
  SQLGetEnvAttrFn GetEnvAttr = nullptr;
  SQLGetDiagRecFn GetDiagRec = nullptr;
  SQLGetDiagRecWFn GetDiagRecW = nullptr;

  DirectDriver() {
    std::string path = DriverConfig::get_driver_path();
    handle = dlopen(path.c_str(), RTLD_NOW);
    REQUIRE(handle != nullptr);

    AllocHandle = reinterpret_cast<SQLAllocHandleFn>(dlsym(handle, "SQLAllocHandle"));
    FreeHandle = reinterpret_cast<SQLFreeHandleFn>(dlsym(handle, "SQLFreeHandle"));
    SetEnvAttr = reinterpret_cast<SQLSetEnvAttrFn>(dlsym(handle, "SQLSetEnvAttr"));
    GetEnvAttr = reinterpret_cast<SQLGetEnvAttrFn>(dlsym(handle, "SQLGetEnvAttr"));
    GetDiagRec = reinterpret_cast<SQLGetDiagRecFn>(dlsym(handle, "SQLGetDiagRec"));
    GetDiagRecW = reinterpret_cast<SQLGetDiagRecWFn>(dlsym(handle, "SQLGetDiagRecW"));

    REQUIRE(AllocHandle != nullptr);
    REQUIRE(FreeHandle != nullptr);
    REQUIRE(SetEnvAttr != nullptr);
    REQUIRE(GetEnvAttr != nullptr);
    REQUIRE(GetDiagRec != nullptr);
    REQUIRE(GetDiagRecW != nullptr);
  }

  ~DirectDriver() {
    if (handle) dlclose(handle);
  }

  std::string get_sqlstate(SQLHENV env) {
    SQLCHAR state[6] = {};
    SQLCHAR msg[256] = {};
    SQLINTEGER native = 0;
    SQLSMALLINT len = 0;
    SQLRETURN rc = GetDiagRec(SQL_HANDLE_ENV, env, 1, state, &native, msg, sizeof(msg), &len);
    if (rc == SQL_SUCCESS || rc == SQL_SUCCESS_WITH_INFO) {
      return std::string(reinterpret_cast<char*>(state));
    }
    return "";
  }
};

TEST_CASE("direct driver: should return HY092 when setting SQL_ATTR_OUTPUT_NTS",
          "[odbc-api][env_attr][output_nts][direct]") {
  // Given A driver loaded directly, bypassing the Driver Manager
  DirectDriver drv;
  SQLHENV env = SQL_NULL_HENV;
  SQLRETURN ret = drv.AllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  ret = drv.SetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQL_ATTR_OUTPUT_NTS is set to SQL_TRUE
  ret = drv.SetEnvAttr(env, SQL_ATTR_OUTPUT_NTS, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);

  // Then New driver treats it as read-only (HY092); old driver accepts SQL_TRUE silently
  NEW_DRIVER_ONLY() {
    REQUIRE(ret == SQL_ERROR);
    CHECK(drv.get_sqlstate(env) == "HY092");
  }
  OLD_DRIVER_ONLY() { REQUIRE(ret == SQL_SUCCESS); }

  // When SQL_ATTR_OUTPUT_NTS is set to SQL_FALSE
  ret = drv.SetEnvAttr(env, SQL_ATTR_OUTPUT_NTS, reinterpret_cast<SQLPOINTER>(SQL_FALSE), 0);

  // Then Both drivers reject SQL_FALSE (new: HY092 read-only, old: HYC00 optional feature)
  REQUIRE(ret == SQL_ERROR);
  NEW_DRIVER_ONLY() { CHECK(drv.get_sqlstate(env) == "HY092"); }
  OLD_DRIVER_ONLY() { CHECK(drv.get_sqlstate(env) == "HYC00"); }

  drv.FreeHandle(SQL_HANDLE_ENV, env);
}

TEST_CASE("direct driver: should return SQL_TRUE when getting SQL_ATTR_OUTPUT_NTS",
          "[odbc-api][env_attr][output_nts][direct]") {
  // Given A driver loaded directly
  DirectDriver drv;
  SQLHENV env = SQL_NULL_HENV;
  SQLRETURN ret = drv.AllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQL_ATTR_OUTPUT_NTS is queried
  SQLINTEGER nts = -1;
  ret = drv.GetEnvAttr(env, SQL_ATTR_OUTPUT_NTS, &nts, 0, nullptr);

  // Then New driver returns SQL_TRUE; old driver does not support getting OUTPUT_NTS directly
  NEW_DRIVER_ONLY() {
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(nts == SQL_TRUE);
  }
  OLD_DRIVER_ONLY() { REQUIRE(ret == SQL_ERROR); }

  drv.FreeHandle(SQL_HANDLE_ENV, env);
}

TEST_CASE("direct driver: should return error for unknown attribute", "[odbc-api][env_attr][error][direct]") {
  // Given A driver loaded directly
  DirectDriver drv;
  SQLHENV env = SQL_NULL_HENV;
  SQLRETURN ret = drv.AllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  ret = drv.SetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When An unknown attribute is set
  ret = drv.SetEnvAttr(env, 99999, reinterpret_cast<SQLPOINTER>(1), 0);

  // Then New driver returns HY092 (invalid attribute identifier); old driver returns HY000 (general error)
  REQUIRE(ret == SQL_ERROR);
  NEW_DRIVER_ONLY("BD#env-attr-unknown-id-hy092") { CHECK(drv.get_sqlstate(env) == "HY092"); }
  OLD_DRIVER_ONLY("BD#env-attr-unknown-id-hy092") { CHECK(drv.get_sqlstate(env) == "HY000"); }

  // When An unknown attribute is queried
  SQLINTEGER value = 0;
  ret = drv.GetEnvAttr(env, 99999, &value, 0, nullptr);

  // Then New driver returns SQL_ERROR with HY092; old driver returns SQL_NO_DATA
  NEW_DRIVER_ONLY("BD#env-attr-unknown-id-hy092") {
    REQUIRE(ret == SQL_ERROR);
    CHECK(drv.get_sqlstate(env) == "HY092");
  }
  OLD_DRIVER_ONLY("BD#env-attr-unknown-id-hy092") { REQUIRE(ret == SQL_NO_DATA); }

  drv.FreeHandle(SQL_HANDLE_ENV, env);
}

TEST_CASE("direct driver: should write StringLengthPtr for integer attributes",
          "[odbc-api][env_attr][string_length][direct]") {
  // Given A driver loaded directly with ODBC version set
  DirectDriver drv;
  SQLHENV env = SQL_NULL_HENV;
  SQLRETURN ret = drv.AllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  ret = drv.SetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQLGetEnvAttr is called with a non-null StringLengthPtr
  SQLINTEGER version = 0;
  SQLINTEGER string_length = 0;
  ret = drv.GetEnvAttr(env, SQL_ATTR_ODBC_VERSION, &version, 0, &string_length);

  // Then StringLengthPtr should contain sizeof(SQLINTEGER)
  // Note: ODBC spec §SQLGetEnvAttr leaves *StringLengthPtr undefined for non-string attributes.
  // Writing sizeof(SQLINTEGER) is our deliberate implementation choice for the direct-driver path.
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(version == SQL_OV_ODBC3);
  CHECK(string_length == static_cast<SQLINTEGER>(sizeof(SQLINTEGER)));

  drv.FreeHandle(SQL_HANDLE_ENV, env);
}

TEST_CASE("direct driver: should report SQL_ATTR_DRIVER_UNICODE_TYPE matching the width of its wide output",
          "[odbc-api][env_attr][driver_unicode_type][direct]") {
  // Given A driver loaded directly with DriverManagerEncoding pinned to the suite's driver manager
  PinDriverManagerEncoding pin_encoding;
  DirectDriver drv;
  SQLHENV env = SQL_NULL_HENV;
  SQLRETURN ret = drv.AllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  ret = drv.SetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQL_ATTR_DRIVER_UNICODE_TYPE is queried, as iODBC 3.52.14 and later do while loading the driver
  SQLINTEGER unicode_type = -1;
  ret = drv.GetEnvAttr(env, SQL_ATTR_DRIVER_UNICODE_TYPE, &unicode_type, 0, nullptr);

  // Then It returns SQL_SUCCESS with the code page pinned by DriverManagerEncoding
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(unicode_type == (is_iodbc_test_suite() ? SQL_DM_CP_UCS4 : SQL_DM_CP_UTF16));

  // And SQLGetDiagRecW writes the SQLSTATE in code units of that width
  ret = drv.SetEnvAttr(env, 99999, reinterpret_cast<SQLPOINTER>(1), 0);
  REQUIRE(ret == SQL_ERROR);
  const size_t unit_size = unicode_type == SQL_DM_CP_UTF16 ? 2 : 4;
  alignas(4) unsigned char state[6 * 4];
  std::memset(state, 0xFF, sizeof(state));
  alignas(4) unsigned char message[256 * 4];
  SQLINTEGER native = 0;
  SQLSMALLINT message_length = 0;
  ret = drv.GetDiagRecW(SQL_HANDLE_ENV, env, 1, reinterpret_cast<SQLWCHAR*>(state), &native,
                        reinterpret_cast<SQLWCHAR*>(message), 256, &message_length);
  REQUIRE(SQL_SUCCEEDED(ret));

  std::string sqlstate;
  for (size_t i = 0; i < 5; ++i) {
    uint32_t code_unit = 0;
    std::memcpy(&code_unit, state + i * unit_size, unit_size);
    sqlstate.push_back(code_unit < 0x80 ? static_cast<char>(code_unit) : '?');
  }
  uint32_t terminator = 0;
  std::memcpy(&terminator, state + 5 * unit_size, unit_size);
  CHECK(terminator == 0);
  NEW_DRIVER_ONLY("BD#env-attr-unknown-id-hy092") { CHECK(sqlstate == "HY092"); }
  OLD_DRIVER_ONLY("BD#env-attr-unknown-id-hy092") { CHECK(sqlstate == "HY000"); }

  drv.FreeHandle(SQL_HANDLE_ENV, env);
}

TEST_CASE("direct driver: should reject setting SQL_ATTR_DRIVER_UNICODE_TYPE and keep the reported encoding",
          "[odbc-api][env_attr][driver_unicode_type][error][direct]") {
  // Given A driver loaded directly with ODBC version set and its reported SQL_ATTR_DRIVER_UNICODE_TYPE
  DirectDriver drv;
  SQLHENV env = SQL_NULL_HENV;
  SQLRETURN ret = drv.AllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  ret = drv.SetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);
  SQLINTEGER before = -1;
  ret = drv.GetEnvAttr(env, SQL_ATTR_DRIVER_UNICODE_TYPE, &before, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE((before == SQL_DM_CP_UTF16 || before == SQL_DM_CP_UCS4));

  // When SQL_ATTR_DRIVER_UNICODE_TYPE is set to the other code page
  const SQLINTEGER other = before == SQL_DM_CP_UTF16 ? SQL_DM_CP_UCS4 : SQL_DM_CP_UTF16;
  ret = drv.SetEnvAttr(env, SQL_ATTR_DRIVER_UNICODE_TYPE, reinterpret_cast<SQLPOINTER>(static_cast<SQLLEN>(other)), 0);

  // Then New driver returns HY092 (not an ODBC environment attribute); old driver returns HY000 (general error)
  REQUIRE(ret == SQL_ERROR);
  NEW_DRIVER_ONLY("BD#env-attr-unknown-id-hy092") { CHECK(drv.get_sqlstate(env) == "HY092"); }
  OLD_DRIVER_ONLY("BD#env-attr-unknown-id-hy092") { CHECK(drv.get_sqlstate(env) == "HY000"); }

  // And The reported encoding is unchanged
  SQLINTEGER after = -1;
  ret = drv.GetEnvAttr(env, SQL_ATTR_DRIVER_UNICODE_TYPE, &after, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(after == before);

  drv.FreeHandle(SQL_HANDLE_ENV, env);
}

#endif  // !defined(_WIN32)
