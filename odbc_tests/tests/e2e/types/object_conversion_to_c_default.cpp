#include <picojson.h>
#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"

TEST_CASE("OBJECT to SQL_C_DEFAULT", "[object][conversion][c_default]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When An OBJECT value is fetched as SQL_C_DEFAULT
  auto result = get_data_default_as_string(conn.execute_fetch("SELECT OBJECT_CONSTRUCT('key','val')"), 1);

  // Then The result is a valid JSON object string
  auto sanitized = sanitize_json(result);
  picojson::value v;
  REQUIRE(picojson::parse(v, sanitized).empty());
  CHECK(v.is<picojson::object>());
}

TEST_CASE("OBJECT to SQL_C_DEFAULT empty", "[object][conversion][c_default]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When An empty OBJECT is fetched as SQL_C_DEFAULT
  auto result = get_data_default_as_string(conn.execute_fetch("SELECT OBJECT_CONSTRUCT()"), 1);

  // Then The result is a valid JSON empty object string
  auto sanitized = sanitize_json(result);
  picojson::value v;
  REQUIRE(picojson::parse(v, sanitized).empty());
  CHECK(v.is<picojson::object>());
  CHECK(v.get<picojson::object>().empty());
}

TEST_CASE("OBJECT to SQL_C_DEFAULT nested", "[object][conversion][c_default]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A nested OBJECT is fetched as SQL_C_DEFAULT
  auto result = get_data_default_as_string(
      conn.execute_fetch("SELECT OBJECT_CONSTRUCT('outer', OBJECT_CONSTRUCT('inner', 42))"), 1);

  // Then The result is a valid JSON nested object string
  auto sanitized = sanitize_json(result);
  picojson::value v;
  REQUIRE(picojson::parse(v, sanitized).empty());
  CHECK(v.is<picojson::object>());
  auto outer = v.get<picojson::object>().find("outer");
  REQUIRE(outer != v.get<picojson::object>().end());
  CHECK(outer->second.is<picojson::object>());
}

TEST_CASE("OBJECT NULL to SQL_C_DEFAULT", "[object][conversion][c_default][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL OBJECT value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::OBJECT");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_DEFAULT);
}
