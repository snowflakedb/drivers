#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

// ============================================================================
// SQL_C_CHAR — uppercase hex encoding
// ============================================================================

TEST_CASE("should convert binary to SQL_C_CHAR returning uppercase hex", "[datatype][binary][conversion][char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT X'48656C6C6F'::BINARY, X'ABCDEF'::BINARY" is executed
  const auto stmt = conn.execute_fetch("SELECT X'48656C6C6F'::BINARY, X'ABCDEF'::BINARY");

  // Then SQL_C_CHAR should return "48656C6C6F" and "ABCDEF" in uppercase
  REQUIRE(check_char_success(stmt, 1) == "48656C6C6F");
  REQUIRE(check_char_success(stmt, 2) == "ABCDEF");
}

// ============================================================================
// SQL_C_WCHAR — uppercase hex encoding
// ============================================================================

TEST_CASE("should convert binary to SQL_C_WCHAR returning uppercase hex", "[datatype][binary][conversion][char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT X'ABCDEF'::BINARY" is executed
  const auto stmt = conn.execute_fetch("SELECT X'ABCDEF'::BINARY");

  // Then SQL_C_WCHAR should return "ABCDEF" as wide string
  REQUIRE(check_wchar_success(stmt, 1) == u"ABCDEF");
}

// ============================================================================
// SQLBindCol with SQL_C_CHAR
// ============================================================================

TEST_CASE("should retrieve binary via SQLBindCol with SQL_C_CHAR", "[datatype][binary][conversion][char]") {
  // Given Snowflake client is logged in
  Connection conn;
  const auto stmt = conn.createStatement();

  // When Query "SELECT X'ABCDEF'::BINARY" is executed with SQLBindCol using SQL_C_CHAR
  char buffer[64] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT X'ABCDEF'::BINARY"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then Bound buffer should contain uppercase hex string "ABCDEF"
  REQUIRE(indicator == 6);
  REQUIRE(std::string(buffer, 6) == "ABCDEF");
  REQUIRE(buffer[6] == '\0');
}

// ============================================================================
// SQLBindCol with SQL_C_WCHAR
// ============================================================================

TEST_CASE("should retrieve binary via SQLBindCol with SQL_C_WCHAR", "[datatype][binary][conversion][char]") {
  // Given Snowflake client is logged in
  Connection conn;
  const auto stmt = conn.createStatement();

  // When Query "SELECT X'ABCDEF'::BINARY" is executed with SQLBindCol using SQL_C_WCHAR
  SQLWCHAR wbuffer[64] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_WCHAR, wbuffer, sizeof(wbuffer), &indicator);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT X'ABCDEF'::BINARY"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then Bound wide buffer should contain uppercase hex string "ABCDEF"
  REQUIRE(indicator == 12);
  REQUIRE(wbuffer[0] == static_cast<SQLWCHAR>('A'));
  REQUIRE(wbuffer[1] == static_cast<SQLWCHAR>('B'));
  REQUIRE(wbuffer[2] == static_cast<SQLWCHAR>('C'));
  REQUIRE(wbuffer[3] == static_cast<SQLWCHAR>('D'));
  REQUIRE(wbuffer[4] == static_cast<SQLWCHAR>('E'));
  REQUIRE(wbuffer[5] == static_cast<SQLWCHAR>('F'));
  REQUIRE(wbuffer[6] == 0);
}

// ============================================================================
// Empty binary conversion — SQL_C_CHAR
// ============================================================================

TEST_CASE("should convert empty binary to SQL_C_CHAR returning empty string", "[datatype][binary][conversion][char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT X''::BINARY" is executed
  const auto stmt = conn.execute_fetch("SELECT X''::BINARY");

  // Then SQL_C_CHAR should return empty string with null terminator and indicator 0
  char buffer[16] = {1};
  SQLLEN indicator = -1;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(indicator == 0);
  REQUIRE(buffer[0] == '\0');
}

// ============================================================================
// Empty binary conversion — SQL_C_WCHAR
// ============================================================================

TEST_CASE("should convert empty binary to SQL_C_WCHAR returning empty wide string",
          "[datatype][binary][conversion][char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT X''::BINARY" is executed
  const auto stmt = conn.execute_fetch("SELECT X''::BINARY");

  // Then SQL_C_WCHAR should return empty wide string with null terminator and indicator 0
  SQLWCHAR wbuffer[16] = {1};
  SQLLEN indicator = -1;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_WCHAR, wbuffer, sizeof(wbuffer), &indicator);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(indicator == 0);
  REQUIRE(wbuffer[0] == 0);
}

// ============================================================================
// NULL handling
// ============================================================================

TEST_CASE("should handle NULL binary with character C types", "[datatype][binary][conversion][char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT NULL::BINARY" is executed
  auto check_null = [&](const SQLSMALLINT c_type) {
    const auto stmt = conn.execute_fetch("SELECT NULL::BINARY");
    char buffer[100] = {};
    SQLLEN indicator = 0;
    const SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, c_type, buffer, sizeof(buffer), &indicator);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(indicator == SQL_NULL_DATA);
  };

  // Then SQL_C_CHAR should return SQL_NULL_DATA indicator
  check_null(SQL_C_CHAR);

  // And SQL_C_WCHAR should return SQL_NULL_DATA indicator
  check_null(SQL_C_WCHAR);
}

// ============================================================================
// Chunked SQLGetData for hex output — SQL_C_CHAR
// ============================================================================

TEST_CASE("should retrieve large binary as hex in chunks via SQLGetData with SQL_C_CHAR",
          "[datatype][binary][conversion][char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query selecting a binary value whose hex representation exceeds buffer size is executed
  const auto stmt = conn.execute_fetch("SELECT X'0102030405'::BINARY");

  char buffer[7] = {};
  SQLLEN indicator = 0;

  // Then First SQLGetData call with SQL_C_CHAR should return SQL_SUCCESS_WITH_INFO with partial hex
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  REQUIRE(get_sqlstate(stmt) == "01004");
  REQUIRE(indicator == 10);
  REQUIRE(std::string(buffer, 6) == "010203");

  // And Second SQLGetData call should return SQL_SUCCESS with remaining hex
  memset(buffer, 0, sizeof(buffer));
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(indicator == 4);
  REQUIRE(std::string(buffer, 4) == "0405");
}

// ============================================================================
// Chunked SQLGetData for hex output — SQL_C_WCHAR
// ============================================================================

TEST_CASE("should retrieve large binary as hex in chunks via SQLGetData with SQL_C_WCHAR",
          "[datatype][binary][conversion][char]") {
  SKIP_OLD_DRIVER("BD#24",
                  "Simba SDK uses sizeof(wchar_t)=4 for WCHAR buffer capacity on Linux, "
                  "fitting fewer characters per call than the ODBC spec expects with 2-byte SQLWCHAR");
  // Given Snowflake client is logged in
  Connection conn;

  // When Query selecting a binary value whose hex representation exceeds wide buffer size is executed
  const auto stmt = conn.execute_fetch("SELECT X'ABCDEF'::BINARY");

  SQLWCHAR wbuffer[4] = {};
  SQLLEN indicator = 0;

  // Then First SQLGetData call with SQL_C_WCHAR should return SQL_SUCCESS_WITH_INFO with truncated data
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_WCHAR, wbuffer, sizeof(wbuffer), &indicator);
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  REQUIRE(get_sqlstate(stmt) == "01004");
  REQUIRE(indicator == 12);
  REQUIRE(wbuffer[0] == static_cast<SQLWCHAR>('A'));
  REQUIRE(wbuffer[1] == static_cast<SQLWCHAR>('B'));
  REQUIRE(wbuffer[2] == static_cast<SQLWCHAR>('C'));
  REQUIRE(wbuffer[3] == 0);

  // And Second SQLGetData call with SQL_C_WCHAR should return SQL_SUCCESS with remaining wide hex
  memset(wbuffer, 0, sizeof(wbuffer));
  indicator = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_WCHAR, wbuffer, sizeof(wbuffer), &indicator);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(indicator == 6);
  REQUIRE(wbuffer[0] == static_cast<SQLWCHAR>('D'));
  REQUIRE(wbuffer[1] == static_cast<SQLWCHAR>('E'));
  REQUIRE(wbuffer[2] == static_cast<SQLWCHAR>('F'));
  REQUIRE(wbuffer[3] == 0);
}
