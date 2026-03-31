#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>
#include <catch2/generators/catch_generators.hpp>

#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "test_macros.hpp"

// SQL_CP_DRIVER_AWARE is defined in ODBC 3.8 but may not be in all system headers
#ifndef SQL_CP_DRIVER_AWARE
#define SQL_CP_DRIVER_AWARE 3UL
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

  // When SQL_ATTR_ODBC_VERSION is set to a valid value
  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(version), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then Getting the attribute should return the same value
  SQLINTEGER got = 0;
  ret = SQLGetEnvAttr(env, SQL_ATTR_ODBC_VERSION, &got, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(got == version);

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
  SQLHENV env = SQL_NULL_HENV;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQL_ATTR_OUTPUT_NTS is set to SQL_TRUE
  ret = SQLSetEnvAttr(env, SQL_ATTR_OUTPUT_NTS, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);

  // Then It should return SQL_SUCCESS
  REQUIRE(ret == SQL_SUCCESS);

  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

TEST_CASE("should reject SQL_ATTR_OUTPUT_NTS set to SQL_FALSE with HYC00", "[odbc-api][env_attr][output_nts][error]") {
  // Given An environment handle with ODBC version set
  SQLHENV env = SQL_NULL_HENV;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQL_ATTR_OUTPUT_NTS is set to SQL_FALSE
  ret = SQLSetEnvAttr(env, SQL_ATTR_OUTPUT_NTS, reinterpret_cast<SQLPOINTER>(SQL_FALSE), 0);

  // Then It should return SQL_ERROR — exact SQLSTATE depends on whether the Driver Manager
  // intercepts (HYC00) or forwards to the driver (HY092)
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(SQL_HANDLE_ENV, env);
  REQUIRE(!records.empty());
  CHECK((records[0].sqlState == "HYC00" || records[0].sqlState == "HY092"));

  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

// ============================================================================
// Unknown attributes
// ============================================================================

TEST_CASE("should return error when setting an unknown environment attribute", "[odbc-api][env_attr][error]") {
  // Given An environment handle with ODBC version set
  SQLHENV env = SQL_NULL_HENV;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When An unknown attribute ID is used in SQLSetEnvAttr
  ret = SQLSetEnvAttr(env, 99999, reinterpret_cast<SQLPOINTER>(1), 0);

  // Then It should return SQL_ERROR
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(SQL_HANDLE_ENV, env);
  REQUIRE(!records.empty());
  CHECK((records[0].sqlState == "HYC00" || records[0].sqlState == "HY092"));

  SQLFreeHandle(SQL_HANDLE_ENV, env);
}

TEST_CASE("should return error when getting an unknown environment attribute", "[odbc-api][env_attr][error]") {
  // Given An environment handle with ODBC version set
  SQLHENV env = SQL_NULL_HENV;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When An unknown attribute ID is used in SQLGetEnvAttr
  SQLINTEGER value = 0;
  ret = SQLGetEnvAttr(env, 99999, &value, 0, nullptr);

  // Then It should return SQL_ERROR
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(SQL_HANDLE_ENV, env);
  REQUIRE(!records.empty());
  CHECK((records[0].sqlState == "HYC00" || records[0].sqlState == "HY092"));

  SQLFreeHandle(SQL_HANDLE_ENV, env);
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

#include <dlfcn.h>

#include "ODBCConfig.hpp"

// Function pointer types matching ODBC C API signatures
using SQLAllocHandleFn = SQLRETURN (*)(SQLSMALLINT, SQLHANDLE, SQLHANDLE*);
using SQLFreeHandleFn = SQLRETURN (*)(SQLSMALLINT, SQLHANDLE);
using SQLSetEnvAttrFn = SQLRETURN (*)(SQLHENV, SQLINTEGER, SQLPOINTER, SQLINTEGER);
using SQLGetEnvAttrFn = SQLRETURN (*)(SQLHENV, SQLINTEGER, SQLPOINTER, SQLINTEGER, SQLINTEGER*);
using SQLGetDiagRecFn = SQLRETURN (*)(SQLSMALLINT, SQLHANDLE, SQLSMALLINT, SQLCHAR*, SQLINTEGER*, SQLCHAR*, SQLSMALLINT,
                                      SQLSMALLINT*);

struct DirectDriver {
  void* handle = nullptr;
  SQLAllocHandleFn AllocHandle = nullptr;
  SQLFreeHandleFn FreeHandle = nullptr;
  SQLSetEnvAttrFn SetEnvAttr = nullptr;
  SQLGetEnvAttrFn GetEnvAttr = nullptr;
  SQLGetDiagRecFn GetDiagRec = nullptr;

  DirectDriver() {
    std::string path = DriverConfig::get_driver_path();
    handle = dlopen(path.c_str(), RTLD_NOW);
    REQUIRE(handle != nullptr);

    AllocHandle = reinterpret_cast<SQLAllocHandleFn>(dlsym(handle, "SQLAllocHandle"));
    FreeHandle = reinterpret_cast<SQLFreeHandleFn>(dlsym(handle, "SQLFreeHandle"));
    SetEnvAttr = reinterpret_cast<SQLSetEnvAttrFn>(dlsym(handle, "SQLSetEnvAttr"));
    GetEnvAttr = reinterpret_cast<SQLGetEnvAttrFn>(dlsym(handle, "SQLGetEnvAttr"));
    GetDiagRec = reinterpret_cast<SQLGetDiagRecFn>(dlsym(handle, "SQLGetDiagRec"));

    REQUIRE(AllocHandle != nullptr);
    REQUIRE(FreeHandle != nullptr);
    REQUIRE(SetEnvAttr != nullptr);
    REQUIRE(GetEnvAttr != nullptr);
    REQUIRE(GetDiagRec != nullptr);
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

  // Then New driver returns HYC00 (optional feature not implemented);
  // old driver returns HY000 (general error)
  REQUIRE(ret == SQL_ERROR);
  NEW_DRIVER_ONLY() { CHECK(drv.get_sqlstate(env) == "HYC00"); }
  OLD_DRIVER_ONLY() { CHECK(drv.get_sqlstate(env) == "HY000"); }

  // When An unknown attribute is queried
  SQLINTEGER value = 0;
  ret = drv.GetEnvAttr(env, 99999, &value, 0, nullptr);

  // Then New driver returns SQL_ERROR with HYC00; old driver returns SQL_NO_DATA
  NEW_DRIVER_ONLY() {
    REQUIRE(ret == SQL_ERROR);
    CHECK(drv.get_sqlstate(env) == "HYC00");
  }
  OLD_DRIVER_ONLY() { REQUIRE(ret == SQL_NO_DATA); }

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

#endif  // !defined(_WIN32)
