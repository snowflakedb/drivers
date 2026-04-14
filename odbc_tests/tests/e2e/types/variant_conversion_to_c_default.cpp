#include <picojson.h>
#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"

TEST_CASE("VARIANT to SQL_C_DEFAULT", "[variant][conversion][c_default]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A VARIANT object value is fetched as SQL_C_DEFAULT
  auto result = get_data_default_as_string(conn.execute_fetch("SELECT PARSE_JSON('{\"b\":2}')"), 1);

  // Then The result is a valid JSON object string
  picojson::value v;
  REQUIRE(picojson::parse(v, result).empty());
  CHECK(v.is<picojson::object>());
}

TEST_CASE("VARIANT to SQL_C_DEFAULT array value", "[variant][conversion][c_default]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A VARIANT holding an array is fetched as SQL_C_DEFAULT
  auto result = get_data_default_as_string(conn.execute_fetch("SELECT PARSE_JSON('[1,2,3]')"), 1);

  // Then The result is a valid JSON array string
  picojson::value v;
  REQUIRE(picojson::parse(v, result).empty());
  CHECK(v.is<picojson::array>());
}

TEST_CASE("VARIANT to SQL_C_DEFAULT scalar value", "[variant][conversion][c_default]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A VARIANT holding a scalar is fetched as SQL_C_DEFAULT
  auto result = get_data_default_as_string(conn.execute_fetch("SELECT 42::VARIANT"), 1);

  // Then The result is a string representation of the scalar
  CHECK(!result.empty());
}

TEST_CASE("VARIANT NULL to SQL_C_DEFAULT", "[variant][conversion][c_default][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL VARIANT value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::VARIANT");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_DEFAULT);
}
