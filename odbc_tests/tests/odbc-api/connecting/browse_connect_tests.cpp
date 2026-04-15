#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "ODBCFixtures.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

// ============================================================================
// SQLBrowseConnect - Basic API Tests
// ============================================================================

TEST_CASE("SQLBrowseConnect: SQL_INVALID_HANDLE with NULL connection",
          "[odbc-api][browse_connect][connecting][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const std::string connStr = "DSN=Test";
  SQLCHAR outConnStr[1024];
  SQLSMALLINT outLen = 0;

  const SQLRETURN ret =
      SQLBrowseConnect(SQL_NULL_HDBC, sqlchar(connStr.c_str()), SQL_NTS, outConnStr, sizeof(outConnStr), &outLen);

  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(DbcFixture, "SQLBrowseConnect: IM002 - Non-existent DSN",
                 "[odbc-api][browse_connect][connecting][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const std::string connStr = "DSN=NonExistentDSN_12345";
  SQLCHAR outConnStr[1024];
  SQLSMALLINT outLen = 0;

  const SQLRETURN ret =
      SQLBrowseConnect(dbc_handle(), sqlchar(connStr.c_str()), SQL_NTS, outConnStr, sizeof(outConnStr), &outLen);

  REQUIRE_EXPECTED_ERROR(ret, "IM002", dbc_handle(), SQL_HANDLE_DBC);
}

TEST_CASE_METHOD(DbcFixture, "SQLBrowseConnect: NULL OutConnectionString returns error",
                 "[odbc-api][browse_connect][connecting][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const std::string connStr = "DSN=Test";
  SQLSMALLINT outLen = 0;

  const SQLRETURN ret = SQLBrowseConnect(dbc_handle(), sqlchar(connStr.c_str()), SQL_NTS, nullptr, 1024, &outLen);

  // Note: Snowflake driver returns ERROR when OutConnectionString is NULL
  // DM may return IM002 (DSN not found) before checking output buffer, or HY009
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(SQL_HANDLE_DBC, dbc_handle());
  REQUIRE(!records.empty());
  REQUIRE((records[0].sqlState == "IM002" || records[0].sqlState == "HY009"));
}

TEST_CASE_METHOD(DbcFixture, "SQLBrowseConnect: NULL InConnectionString returns error",
                 "[odbc-api][browse_connect][connecting][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLCHAR outConnStr[1024];
  SQLSMALLINT outLen = 0;

  const SQLRETURN ret = SQLBrowseConnect(dbc_handle(), nullptr, SQL_NTS, outConnStr, sizeof(outConnStr), &outLen);

  // Note: unixODBC DM treats NULL as empty string → IM002 (no DSN found)
  // ODBC spec says HY009, but DM behavior varies
  REQUIRE_EXPECTED_ERROR(ret, "IM002", dbc_handle(), SQL_HANDLE_DBC);
}

TEST_CASE_METHOD(DbcFixture, "SQLBrowseConnect: Empty InConnectionString returns error",
                 "[odbc-api][browse_connect][connecting][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const std::string connStr = "";
  SQLCHAR outConnStr[1024];
  SQLSMALLINT outLen = 0;

  const SQLRETURN ret =
      SQLBrowseConnect(dbc_handle(), sqlchar(connStr.c_str()), SQL_NTS, outConnStr, sizeof(outConnStr), &outLen);

  // IM002: Data source not found (empty string has no DSN/DRIVER)
  REQUIRE_EXPECTED_ERROR(ret, "IM002", dbc_handle(), SQL_HANDLE_DBC);
}

TEST_CASE_METHOD(DbcFixture, "SQLBrowseConnect: HY090 - Negative StringLength",
                 "[odbc-api][browse_connect][connecting][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const std::string connStr = "DSN=Test";
  SQLCHAR outConnStr[1024];
  SQLSMALLINT outLen = 0;

  const SQLRETURN ret = SQLBrowseConnect(dbc_handle(), sqlchar(connStr.c_str()),
                                         -5,  // Invalid negative length
                                         outConnStr, sizeof(outConnStr), &outLen);

  // HY090: Invalid string or buffer length
  // Note: DM-dependent - unixODBC may return HY090 or pass to driver which returns IM002
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(SQL_HANDLE_DBC, dbc_handle());
  REQUIRE(!records.empty());
  REQUIRE((records[0].sqlState == "HY090" || records[0].sqlState == "IM002"));
}

TEST_CASE_METHOD(DbcFixture, "SQLBrowseConnect: HY090 - Negative BufferLength",
                 "[odbc-api][browse_connect][connecting][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const std::string connStr = "DSN=Test";
  SQLCHAR outConnStr[1024];
  SQLSMALLINT outLen = 0;

  const SQLRETURN ret = SQLBrowseConnect(dbc_handle(), sqlchar(connStr.c_str()), SQL_NTS, outConnStr,
                                         -1,  // Invalid negative buffer length
                                         &outLen);

  // HY090: Invalid string or buffer length
  // Note: DM-dependent - may return HY090 or IM002 (if DSN lookup happens first)
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(SQL_HANDLE_DBC, dbc_handle());
  REQUIRE(!records.empty());
  REQUIRE((records[0].sqlState == "HY090" || records[0].sqlState == "IM002"));
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLBrowseConnect: Zero BufferLength returns SQL_NEED_DATA",
                 "[odbc-api][browse_connect][connecting]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const std::string connStr = "DSN=" + dsn_name();
  SQLCHAR outConnStr[1024] = {};
  SQLSMALLINT outLen = 0;

  const SQLRETURN ret = SQLBrowseConnect(dbc_handle(), sqlchar(connStr.c_str()), SQL_NTS, outConnStr,
                                         0,  // Zero buffer length
                                         &outLen);

  // Note: Snowflake driver returns SQL_NEED_DATA when BufferLength is 0
  // This is because it cannot write the output connection string
  // Per ODBC spec, 01004 truncation should return SQL_NEED_DATA for SQLBrowseConnect
  REQUIRE(ret == SQL_NEED_DATA);

  // outLen should indicate actual string length needed
  REQUIRE(outLen > 0);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLBrowseConnect: 08002 - Connection already open",
                 "[odbc-api][browse_connect][connecting][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // First connect using SQLDriverConnect
  const std::string connStr1 = connection_string();
  SQLRETURN ret = SQLDriverConnect(dbc_handle(), nullptr, sqlchar(connStr1.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_NOPROMPT);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  // Try to connect again using SQLBrowseConnect
  const std::string connStr2 = "DSN=" + dsn_name();
  SQLCHAR outConnStr[1024];
  SQLSMALLINT outLen = 0;

  ret = SQLBrowseConnect(dbc_handle(), sqlchar(connStr2.c_str()), SQL_NTS, outConnStr, sizeof(outConnStr), &outLen);

  // 08002: Connection name in use
  REQUIRE_EXPECTED_ERROR(ret, "08002", dbc_handle(), SQL_HANDLE_DBC);

  SQLDisconnect(dbc_handle());
}

// ============================================================================
// SQLBrowseConnect - Iterative Connection Process
// ============================================================================

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLBrowseConnect: Initial call with DSN",
                 "[odbc-api][browse_connect][connecting][integration]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Initial browse with just DSN
  const std::string connStr = "DSN=" + dsn_name();
  SQLCHAR outConnStr[1024] = {};
  SQLSMALLINT outLen = 0;

  SQLRETURN ret =
      SQLBrowseConnect(dbc_handle(), sqlchar(connStr.c_str()), SQL_NTS, outConnStr, sizeof(outConnStr), &outLen);

  // Note: Snowflake driver supports SQLBrowseConnect and completes connection if DSN has all info
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  // Verify OutConnectionString is populated
  REQUIRE(outLen > 0);
  REQUIRE(outConnStr[0] != '\0');

  // Verify connection works
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt, sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLBrowseConnect: Reconnect after disconnect",
                 "[odbc-api][browse_connect][connecting][integration]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const std::string connStr = "DSN=" + dsn_name();
  SQLCHAR outConnStr[1024] = {};
  SQLSMALLINT outLen = 0;

  // First connection
  SQLRETURN ret =
      SQLBrowseConnect(dbc_handle(), sqlchar(connStr.c_str()), SQL_NTS, outConnStr, sizeof(outConnStr), &outLen);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  // Disconnect
  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);

  // Reconnect using same handle
  outLen = 0;
  std::memset(outConnStr, 0, sizeof(outConnStr));

  ret = SQLBrowseConnect(dbc_handle(), sqlchar(connStr.c_str()), SQL_NTS, outConnStr, sizeof(outConnStr), &outLen);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  // Verify second connection works
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt, sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  SQLDisconnect(dbc_handle());
}

// ============================================================================
// SQLBrowseConnect - Authentication Errors
// ============================================================================

TEST_CASE_METHOD(DbcNoAuthDSNFixture, "SQLBrowseConnect: 28000 - Invalid credentials",
                 "[odbc-api][browse_connect][connecting][integration][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Use DSN without auth but provide invalid credentials
  const std::string connStr = "DSN=" + dsn_name() + ";UID=invalid_user_xyz;PWD=invalid_cred_xyz";
  SQLCHAR outConnStr[1024] = {};
  SQLSMALLINT outLen = 0;

  const SQLRETURN ret =
      SQLBrowseConnect(dbc_handle(), sqlchar(connStr.c_str()), SQL_NTS, outConnStr, sizeof(outConnStr), &outLen);

  // Note: Snowflake driver returns 28000 for authentication failures.
  REQUIRE_EXPECTED_ERROR(ret, "28000", dbc_handle(), SQL_HANDLE_DBC);
}

// ============================================================================
// SQLBrowseConnect - Buffer Handling
// ============================================================================

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLBrowseConnect: small output buffer behavior",
                 "[odbc-api][browse_connect][connecting]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const std::string connStr = "DSN=" + dsn_name();
  SQLCHAR outConnStr[10];  // Very small buffer to force truncation
  SQLSMALLINT outLen = 0;

  const SQLRETURN ret =
      SQLBrowseConnect(dbc_handle(), sqlchar(connStr.c_str()), SQL_NTS, outConnStr, sizeof(outConnStr), &outLen);

  // The old driver may complete the connection immediately (SQL_SUCCESS), signal that more
  // input is required (SQL_NEED_DATA), or report truncation (SQL_SUCCESS_WITH_INFO / 01004)
  // depending on whether the DSN holds full credentials and how long the output string is.
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_NEED_DATA || ret == SQL_SUCCESS_WITH_INFO));

  // Then: when the driver signals truncation or needs more input, outLen should reflect the
  // full required length (larger than our deliberately small buffer).
  if (ret == SQL_NEED_DATA || ret == SQL_SUCCESS_WITH_INFO) {
    CHECK(outLen >= static_cast<SQLSMALLINT>(sizeof(outConnStr)));
  } else {
    CHECK(outLen > 0);
  }

  // Teardown: disconnect if a connection was established (no-op on SQL_NEED_DATA).
  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLBrowseConnect: NULL StringLength2Ptr is allowed",
                 "[odbc-api][browse_connect][connecting]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const std::string connStr = "DSN=" + dsn_name();
  SQLCHAR outConnStr[1024] = {};

  // NULL StringLength2Ptr is allowed per ODBC spec - caller doesn't need length
  SQLRETURN ret = SQLBrowseConnect(dbc_handle(), sqlchar(connStr.c_str()), SQL_NTS, outConnStr, sizeof(outConnStr),
                                   nullptr);  // NULL length pointer

  // Should succeed - NULL length pointer is valid
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  // OutConnectionString should still be populated
  REQUIRE(outConnStr[0] != '\0');

  // Connection should work
  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

// ============================================================================
// SQLBrowseConnect - Cancellation
// ============================================================================

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLBrowseConnect: Disconnecting after successful browse",
                 "[odbc-api][browse_connect][connecting][lifecycle]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Browse and connect
  const std::string connStr = "DSN=" + dsn_name();
  SQLCHAR outConnStr[1024];
  SQLSMALLINT outLen = 0;

  SQLRETURN ret =
      SQLBrowseConnect(dbc_handle(), sqlchar(connStr.c_str()), SQL_NTS, outConnStr, sizeof(outConnStr), &outLen);

  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  // Disconnect after successful connection
  ret = SQLDisconnect(dbc_handle());
  REQUIRE(ret == SQL_SUCCESS);
}

// ============================================================================
// SQLBrowseConnect - Driver Support Detection
// ============================================================================

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLBrowseConnect: Driver support with complete output verification",
                 "[odbc-api][browse_connect][connecting][integration]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const std::string connStr = "DSN=" + dsn_name();
  SQLCHAR outConnStr[2048] = {};
  SQLSMALLINT outLen = 0;

  SQLRETURN ret =
      SQLBrowseConnect(dbc_handle(), sqlchar(connStr.c_str()), SQL_NTS, outConnStr, sizeof(outConnStr), &outLen);

  // Note: Snowflake driver SUPPORTS SQLBrowseConnect
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  // Verify OutConnectionString contains the complete connection info
  std::string outStr = reinterpret_cast<char*>(outConnStr);
  REQUIRE(!outStr.empty());
  REQUIRE(outLen > 0);

  // OutConnectionString should be properly null-terminated
  REQUIRE(static_cast<size_t>(outLen) <= outStr.length());

  // Verify connection is usable
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt, sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));

  SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  SQLDisconnect(dbc_handle());
}

// ============================================================================
// SQLBrowseConnect - Iterative Browsing Tests
// ============================================================================

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLBrowseConnect: Iterative browse with incomplete connection string",
                 "[odbc-api][browse_connect][connecting][integration][iterative]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Provide ONLY the driver path (no server, no credentials)
  // True iterative browsing would return SQL_NEED_DATA with required attributes
  const std::string driverPath = DriverConfig::get_driver_path();
  const std::string connStr = "DRIVER=" + driverPath;
  SQLCHAR outConnStr[2048] = {};
  SQLSMALLINT outLen = 0;

  const SQLRETURN ret =
      SQLBrowseConnect(dbc_handle(), sqlchar(connStr.c_str()), SQL_NTS, outConnStr, sizeof(outConnStr), &outLen);

  // Note: Snowflake driver does NOT support iterative browsing.
  // It requires a complete connection string and returns SQL_ERROR when given incomplete info.
  REQUIRE(ret == SQL_ERROR);

  auto records = get_diag_rec(SQL_HANDLE_DBC, dbc_handle());
  REQUIRE(!records.empty());
}
