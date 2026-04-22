#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

// ============================================================================
// SQL_C_CHAR
// ============================================================================

TEST_CASE("DECFLOAT to SQL_C_CHAR", "[decfloat][conversion][c_char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When DECFLOAT values are fetched as SQL_C_CHAR
  auto stmt = conn.execute_fetch("SELECT 0::DECFLOAT, 123.456::DECFLOAT, -789.012::DECFLOAT");

  // Then SQL_C_CHAR returns correct string representations
  CHECK(check_char_success(stmt, 1) == "0");
  CHECK(check_char_success(stmt, 2) == "123.456");
  CHECK(check_char_success(stmt, 3) == "-789.012");
}

TEST_CASE("DECFLOAT full precision to SQL_C_CHAR", "[decfloat][conversion][c_char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A 38-digit DECFLOAT value is fetched as SQL_C_CHAR
  auto stmt = conn.execute_fetch("SELECT '12345678901234567890123456789012345678'::DECFLOAT");

  // Then SQL_C_CHAR preserves full 38-digit precision
  CHECK(check_char_success(stmt, 1) == "12345678901234567890123456789012345678");
}

// ============================================================================
// SQL_C_WCHAR
// ============================================================================

TEST_CASE("DECFLOAT to SQL_C_WCHAR", "[decfloat][conversion][c_char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When DECFLOAT values are fetched as SQL_C_WCHAR
  auto stmt = conn.execute_fetch("SELECT 0::DECFLOAT, 123.456::DECFLOAT, -789.012::DECFLOAT");

  // Then SQL_C_WCHAR returns correct wide string representations
  CHECK(check_wchar_success(stmt, 1) == u"0");
  CHECK(check_wchar_success(stmt, 2) == u"123.456");
  CHECK(check_wchar_success(stmt, 3) == u"-789.012");
}

// ============================================================================
// TRUNCATION
// ============================================================================

TEST_CASE("DECFLOAT SQL_C_CHAR truncation with small buffer", "[decfloat][conversion][c_char][truncation]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A 38-digit DECFLOAT value is fetched into a buffer too small
  auto stmt = conn.execute_fetch("SELECT '12345678901234567890123456789012345678'::DECFLOAT");

  char buffer[20] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);

  // Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004
  CHECK(ret == SQL_SUCCESS_WITH_INFO);
  CHECK(indicator == 38);
  CHECK(get_sqlstate(stmt) == "01004");
  CHECK(std::string(buffer, 19) == "1234567890123456789");
  CHECK(buffer[19] == '\0');
}

TEST_CASE("DECFLOAT SQL_C_CHAR exact-fit buffer", "[decfloat][conversion][c_char][truncation]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A DECFLOAT value is fetched into a buffer that exactly fits
  auto stmt = conn.execute_fetch("SELECT 42::DECFLOAT");

  char buffer[3] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);

  // Then SQL_SUCCESS is returned with correct content
  CHECK(ret == SQL_SUCCESS);
  CHECK(indicator == 2);
  CHECK(std::string(buffer) == "42");
}

TEST_CASE("DECFLOAT SQL_C_WCHAR truncation with small buffer", "[decfloat][conversion][c_char][truncation]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A 38-digit DECFLOAT value is fetched as SQL_C_WCHAR into a buffer too small
  auto stmt = conn.execute_fetch("SELECT '12345678901234567890123456789012345678'::DECFLOAT");

  char16_t buffer[10] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_WCHAR, buffer, sizeof(buffer), &indicator);

  // Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004
  CHECK(ret == SQL_SUCCESS_WITH_INFO);
  CHECK(indicator == 38 * static_cast<SQLLEN>(sizeof(char16_t)));
  CHECK(get_sqlstate(stmt) == "01004");
  auto truncated = std::u16string(buffer);
  CHECK(truncated.length() >= 8);
  CHECK(truncated.substr(0, 8) == u"12345678");
}

// ============================================================================
// SQLBindCol
// ============================================================================

TEST_CASE("DECFLOAT using SQLBindCol for SQL_C_CHAR", "[decfloat][conversion][c_char][bindcol]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When DECFLOAT values are fetched using SQLBindCol for SQL_C_CHAR
  const auto stmt = conn.createStatement();
  char buf1[64] = {};
  char buf2[64] = {};
  SQLLEN ind1 = 0;
  SQLLEN ind2 = 0;

  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_CHAR, buf1, sizeof(buf1), &ind1);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindCol(stmt.getHandle(), 2, SQL_C_CHAR, buf2, sizeof(buf2), &ind2);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT 0::DECFLOAT, 123.456::DECFLOAT"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then Bound buffers contain correct DECFLOAT string values
  CHECK(ind1 == 1);
  CHECK(std::string(buf1, ind1) == "0");
  CHECK(buf1[ind1] == '\0');
  CHECK(ind2 == 7);
  CHECK(std::string(buf2, ind2) == "123.456");
  CHECK(buf2[ind2] == '\0');
}

// ============================================================================
// NULL handling
// ============================================================================

TEST_CASE("DECFLOAT NULL to SQL_C_CHAR types", "[decfloat][conversion][c_char][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL DECFLOAT value is queried
  (void)0;
  // Then Indicator returns SQL_NULL_DATA for SQL_C_CHAR and SQL_C_WCHAR
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::DECFLOAT"), 1, SQL_C_CHAR);
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::DECFLOAT"), 1, SQL_C_WCHAR);
}
