#include <picojson.h>
#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"

static picojson::value parse_json(const std::string& text) {
  picojson::value v;
  REQUIRE(picojson::parse(v, text).empty());
  return v;
}

TEST_CASE("VARIANT to SQL_C_BINARY", "[variant][conversion][c_binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A VARIANT object value is fetched as SQL_C_BINARY
  auto stmt = conn.execute_fetch("SELECT PARSE_JSON('{\"b\":2}')");
  SQLCHAR buffer[256] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);

  // Then Raw JSON bytes are returned and parseable as object
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(indicator > 0);
  auto json = parse_json(std::string(reinterpret_cast<char*>(buffer), static_cast<size_t>(indicator)));
  CHECK(json.is<picojson::object>());
}

TEST_CASE("VARIANT to SQL_C_BINARY array value", "[variant][conversion][c_binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A VARIANT holding an array is fetched as SQL_C_BINARY
  auto stmt = conn.execute_fetch("SELECT PARSE_JSON('[1,2,3]')");
  SQLCHAR buffer[256] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);

  // Then Raw JSON bytes are returned and parseable as array
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(indicator > 0);
  auto json = parse_json(std::string(reinterpret_cast<char*>(buffer), static_cast<size_t>(indicator)));
  CHECK(json.is<picojson::array>());
}

TEST_CASE("VARIANT to SQL_C_BINARY empty object", "[variant][conversion][c_binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A VARIANT holding an empty object is fetched as SQL_C_BINARY
  auto stmt = conn.execute_fetch("SELECT PARSE_JSON('{}')");
  SQLCHAR buffer[256] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);

  // Then Raw JSON bytes are returned and parseable as empty object
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(indicator > 0);
  auto json = parse_json(std::string(reinterpret_cast<char*>(buffer), static_cast<size_t>(indicator)));
  CHECK(json.is<picojson::object>());
  CHECK(json.get<picojson::object>().empty());
}

TEST_CASE("VARIANT to SQL_C_BINARY nested value", "[variant][conversion][c_binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A VARIANT holding nested JSON is fetched as SQL_C_BINARY
  auto stmt = conn.execute_fetch("SELECT PARSE_JSON('{\"a\":{\"b\":[1,2]}}')");
  SQLCHAR buffer[512] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);

  // Then Raw JSON bytes are returned and parseable with nested structure
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(indicator > 0);
  auto json = parse_json(std::string(reinterpret_cast<char*>(buffer), static_cast<size_t>(indicator)));
  CHECK(json.is<picojson::object>());
}

TEST_CASE("VARIANT NULL to SQL_C_BINARY", "[variant][conversion][c_binary][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL VARIANT value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::VARIANT");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_BINARY);
}
