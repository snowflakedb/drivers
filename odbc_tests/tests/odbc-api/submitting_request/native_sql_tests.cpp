#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "ODBCFixtures.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "test_macros.hpp"

// ============================================================================
// SQLNativeSql - Basic Functionality
// ============================================================================

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLNativeSql: Plain SQL passes through unchanged",
                 "[odbc-api][nativesql][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR out[256] = {};
  SQLINTEGER outLen = 0;
  ret = SQLNativeSql(dbc_handle(), sqlchar("SELECT 42 AS val"), SQL_NTS, out, sizeof(out), &outLen);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(outLen == 16);
  REQUIRE(std::string(reinterpret_cast<char*>(out)) == "SELECT 42 AS val");

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLNativeSql: Explicit TextLength1 instead of SQL_NTS",
                 "[odbc-api][nativesql][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR out[256] = {};
  SQLINTEGER outLen = 0;
  ret = SQLNativeSql(dbc_handle(), sqlchar("SELECT 1"), 8, out, sizeof(out), &outLen);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(outLen == 8);
  REQUIRE(std::string(reinterpret_cast<char*>(out)) == "SELECT 1");

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLNativeSql: Zero TextLength1 returns empty string",
                 "[odbc-api][nativesql][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR out[256] = {};
  SQLINTEGER outLen = 0;
  ret = SQLNativeSql(dbc_handle(), sqlchar("SELECT 1"), 0, out, sizeof(out), &outLen);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(outLen == 0);
  REQUIRE(std::string(reinterpret_cast<char*>(out)).empty());

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLNativeSql: NULL OutStatementText returns length without writing output",
                 "[odbc-api][nativesql][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER outLen = 0;
  ret = SQLNativeSql(dbc_handle(), sqlchar("SELECT 12345"), SQL_NTS, nullptr, 0, &outLen);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(outLen == 12);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLNativeSql: Truncation returns SQL_SUCCESS_WITH_INFO and 01004",
                 "[odbc-api][nativesql][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR out[10] = {};
  SQLINTEGER outLen = 0;
  ret = SQLNativeSql(dbc_handle(), sqlchar("SELECT 123456789"), SQL_NTS, out, 10, &outLen);
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  REQUIRE(outLen == 16);
  REQUIRE(std::string(reinterpret_cast<char*>(out)) == "SELECT 12");

  const auto recs = get_diag_rec(SQL_HANDLE_DBC, dbc_handle());
  REQUIRE(!recs.empty());
  REQUIRE(recs[0].sqlState == "01004");

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLNativeSql: NULL TextLength2Ptr succeeds without crash",
                 "[odbc-api][nativesql][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR out[256] = {};
  ret = SQLNativeSql(dbc_handle(), sqlchar("SELECT 1"), SQL_NTS, out, sizeof(out), nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(std::string(reinterpret_cast<char*>(out)) == "SELECT 1");

  SQLDisconnect(dbc_handle());
}

// Note: Reference driver delegates ODBC escape sequence handling to the SimbaEngineSDK's Parser library.
// This is not invoked by SQLNativeSql. Therefore, all escape sequences are wrongly passed through unchanged.
TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLNativeSql: Escape sequences pass through unchanged",
                 "[odbc-api][nativesql][submitting_request]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // One representative from each escape category to confirm the no-op behavior.
  const char* samples[] = {
      "SELECT {d '2024-01-15'}",
      "SELECT {fn ABS(-1)}",
      "SELECT {fn CONCAT('a', 'b')}",
      "SELECT {fn DATABASE()}",
      "SELECT {fn CURRENT_DATE()}",
      "SELECT {fn CONVERT(123, SQL_VARCHAR)}",
      "SELECT * FROM {oj t1 LEFT OUTER JOIN t2 ON t1.id = t2.id}",
      "{call my_proc(1, 2)}",
  };

  for (const auto* input : samples) {
    SQLCHAR out[512] = {};
    SQLINTEGER outLen = 0;
    ret = SQLNativeSql(dbc_handle(), sqlchar(input), SQL_NTS, out, sizeof(out), &outLen);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(std::string(reinterpret_cast<char*>(out)) == input);
    REQUIRE(outLen == static_cast<SQLINTEGER>(strlen(input)));
  }

  SQLDisconnect(dbc_handle());
}

// ============================================================================
// SQLNativeSql - Error Cases
// ============================================================================

TEST_CASE("SQLNativeSql: SQL_INVALID_HANDLE for null connection handle",
          "[odbc-api][nativesql][submitting_request][error]") {
  const SQLRETURN ret = SQLNativeSql(SQL_NULL_HDBC, sqlchar("SELECT 1"), SQL_NTS, nullptr, 0, nullptr);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLNativeSql: HY009 for null InStatementText",
                 "[odbc-api][nativesql][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR out[256] = {};
  SQLINTEGER outLen = 0;
  ret = SQLNativeSql(dbc_handle(), nullptr, SQL_NTS, out, sizeof(out), &outLen);
  REQUIRE_EXPECTED_ERROR(ret, "HY009", dbc_handle(), SQL_HANDLE_DBC);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLNativeSql: HY090 for negative TextLength1",
                 "[odbc-api][nativesql][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR out[256] = {};
  SQLINTEGER outLen = 0;
  ret = SQLNativeSql(dbc_handle(), sqlchar("SELECT 1"), -5, out, sizeof(out), &outLen);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", dbc_handle(), SQL_HANDLE_DBC);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLNativeSql: HY090 for negative BufferLength",
                 "[odbc-api][nativesql][submitting_request][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR out[256] = {};
  SQLINTEGER outLen = 0;
  ret = SQLNativeSql(dbc_handle(), sqlchar("SELECT 1"), SQL_NTS, out, -1, &outLen);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", dbc_handle(), SQL_HANDLE_DBC);

  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcFixture, "SQLNativeSql: 08003 when connection is not open",
                 "[odbc-api][nativesql][submitting_request][error]") {
  SQLCHAR out[256] = {};
  SQLINTEGER outLen = 0;
  const SQLRETURN ret = SQLNativeSql(dbc_handle(), sqlchar("SELECT 1"), SQL_NTS, out, sizeof(out), &outLen);
  REQUIRE_EXPECTED_ERROR(ret, "08003", dbc_handle(), SQL_HANDLE_DBC);
}
