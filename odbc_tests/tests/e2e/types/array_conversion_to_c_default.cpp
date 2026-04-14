#include <picojson.h>
#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"

static std::string sanitize_json(const std::string& text) {
  std::string result = text;
  size_t pos = 0;
  while ((pos = result.find("undefined", pos)) != std::string::npos) {
    result.replace(pos, 9, "null");
    pos += 4;
  }
  return result;
}

TEST_CASE("ARRAY to SQL_C_DEFAULT", "[array][conversion][c_default]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When An ARRAY value is fetched as SQL_C_DEFAULT
  auto result = get_data_default_as_string(conn.execute_fetch("SELECT ARRAY_CONSTRUCT(1,2,3)"), 1);

  // Then The result is a valid JSON array string with correct elements
  auto sanitized = sanitize_json(result);
  picojson::value v;
  REQUIRE(picojson::parse(v, sanitized).empty());
  CHECK(v.is<picojson::array>());
  CHECK(v.get<picojson::array>().size() == 3);
}

TEST_CASE("ARRAY to SQL_C_DEFAULT empty", "[array][conversion][c_default]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When An empty ARRAY is fetched as SQL_C_DEFAULT
  auto result = get_data_default_as_string(conn.execute_fetch("SELECT ARRAY_CONSTRUCT()"), 1);

  // Then The result is a valid JSON empty array string
  auto sanitized = sanitize_json(result);
  picojson::value v;
  REQUIRE(picojson::parse(v, sanitized).empty());
  CHECK(v.is<picojson::array>());
  CHECK(v.get<picojson::array>().empty());
}

TEST_CASE("ARRAY to SQL_C_DEFAULT nested", "[array][conversion][c_default]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A nested ARRAY is fetched as SQL_C_DEFAULT
  auto result = get_data_default_as_string(
      conn.execute_fetch("SELECT ARRAY_CONSTRUCT(ARRAY_CONSTRUCT(1,2), ARRAY_CONSTRUCT(3,4))"), 1);

  // Then The result is a valid JSON nested array string
  auto sanitized = sanitize_json(result);
  picojson::value v;
  REQUIRE(picojson::parse(v, sanitized).empty());
  CHECK(v.is<picojson::array>());
  CHECK(v.get<picojson::array>().size() == 2);
}

TEST_CASE("ARRAY NULL to SQL_C_DEFAULT", "[array][conversion][c_default][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL ARRAY value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::ARRAY");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_DEFAULT);
}
