#include <picojson.h>
#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cmath>
#include <cstring>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "WideString.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

static picojson::value parse_json(const std::string& text) {
  picojson::value v;
  REQUIRE(picojson::parse(v, text).empty());
  return v;
}

// ============================================================================
// SQL_C_WCHAR round-trip
// ============================================================================

TEST_CASE("VECTOR to SQL_C_WCHAR returns parseable JSON array", "[datatype][vector][conversion][c_wchar]") {
  SKIP_OLD_DRIVER("BD#119", "Reference driver has no VECTOR support");

  // Given Snowflake client is logged in
  Connection conn;

  // When An INT VECTOR is fetched as SQL_C_WCHAR
  auto stmt = conn.execute_fetch("SELECT [10, 20, 30]::VECTOR(INT, 3)");

  // Then The result is a valid JSON array with the expected int values
  auto wide_result = get_data<SQL_C_WCHAR>(stmt, 1);
  auto json = parse_json(sf::wide::utf32_to_utf8(wide_result));
  REQUIRE(json.is<picojson::array>());
  const auto& arr = json.get<picojson::array>();
  REQUIRE(arr.size() == 3);
  CHECK(arr[0].get<double>() == 10.0);
  CHECK(arr[1].get<double>() == 20.0);
  CHECK(arr[2].get<double>() == 30.0);
}

TEST_CASE("FLOAT VECTOR to SQL_C_WCHAR returns parseable JSON array", "[datatype][vector][conversion][c_wchar]") {
  SKIP_OLD_DRIVER("BD#119", "Reference driver has no VECTOR support");

  // Given Snowflake client is logged in
  Connection conn;

  // When A FLOAT VECTOR is fetched as SQL_C_WCHAR
  auto stmt = conn.execute_fetch("SELECT [1.5, -2.5]::VECTOR(FLOAT, 2)");

  // Then The result is a valid JSON array with the expected float values
  auto wide_result = get_data<SQL_C_WCHAR>(stmt, 1);
  auto json = parse_json(sf::wide::utf32_to_utf8(wide_result));
  REQUIRE(json.is<picojson::array>());
  const auto& arr = json.get<picojson::array>();
  REQUIRE(arr.size() == 2);
  CHECK(std::abs(arr[0].get<double>() - 1.5) < 1e-5);
  CHECK(std::abs(arr[1].get<double>() - (-2.5)) < 1e-5);
}

// ============================================================================
// SQL_C_BINARY round-trip
// ============================================================================

TEST_CASE("VECTOR to SQL_C_BINARY returns UTF-8 JSON bytes", "[datatype][vector][conversion][c_binary]") {
  SKIP_OLD_DRIVER("BD#119", "Reference driver has no VECTOR support");

  // Given Snowflake client is logged in
  Connection conn;

  // When An INT VECTOR is fetched as SQL_C_BINARY
  auto stmt = conn.execute_fetch("SELECT [1, 2, 3]::VECTOR(INT, 3)");

  // Then Raw UTF-8 bytes are returned and parseable as a JSON array
  SQLCHAR buffer[256];
  std::memset(buffer, 0xFF, sizeof(buffer));
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
  REQUIRE_ODBC(ret, stmt);
  CHECK(indicator > 0);
  auto json = parse_json(std::string(reinterpret_cast<char*>(buffer), static_cast<size_t>(indicator)));
  CHECK(json.is<picojson::array>());
  CHECK(json.get<picojson::array>().size() == 3);
}

// ============================================================================
// Truncation — SQLSTATE 01004
// ============================================================================

TEST_CASE("VECTOR SQL_C_CHAR truncation returns 01004", "[datatype][vector][conversion][truncation][01004]") {
  SKIP_OLD_DRIVER("BD#119", "Reference driver has no VECTOR support");

  // Given Snowflake client is logged in
  Connection conn;

  // When An INT VECTOR is fetched with a buffer too small to hold the full JSON string
  auto stmt = conn.execute_fetch("SELECT [100, 200, 300]::VECTOR(INT, 3)");

  // Then SQL_SUCCESS_WITH_INFO with SQLSTATE 01004 is returned
  SQLCHAR tiny[4];
  std::memset(tiny, 0xFF, sizeof(tiny));
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, tiny, sizeof(tiny), &indicator);
  CHECK(ret == SQL_SUCCESS_WITH_INFO);
  CHECK(get_sqlstate(stmt) == "01004");
  CHECK(indicator > 0);
}

// ============================================================================
// SQL_DESC_TYPE_NAME
// ============================================================================

TEST_CASE("VECTOR SQL_DESC_TYPE_NAME reports VECTOR", "[datatype][vector][conversion][descriptor]") {
  SKIP_OLD_DRIVER("BD#119", "Reference driver has no VECTOR support");

  // Given Snowflake client is logged in
  Connection conn;

  // When Query with a VECTOR column is prepared and described
  auto stmt = conn.execute_fetch("SELECT [1, 2]::VECTOR(INT, 2)");

  // Then SQL_DESC_TYPE_NAME returns "VECTOR"
  char type_name[64] = {};
  SQLLEN type_name_indicator = 0;
  SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_TYPE_NAME, type_name, sizeof(type_name), nullptr,
                                  &type_name_indicator);
  REQUIRE_ODBC(ret, stmt);
  CHECK(std::string(type_name) == "VECTOR");

  // And SQL_DESC_SCALE / SQL_DESC_PRECISION are both 0 for VECTOR
  SQLLEN scale = -1;
  ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_SCALE, nullptr, 0, nullptr, &scale);
  REQUIRE_ODBC(ret, stmt);
  CHECK(scale == 0);

  SQLLEN precision = -1;
  ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_PRECISION, nullptr, 0, nullptr, &precision);
  REQUIRE_ODBC(ret, stmt);
  CHECK(precision == 0);
}
