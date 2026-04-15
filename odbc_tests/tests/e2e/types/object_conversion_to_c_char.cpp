#include <picojson.h>
#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"
#include "get_diag_rec.hpp"

static picojson::value parse_json(const std::string& text) {
  auto sanitized = sanitize_json(text);
  picojson::value v;
  REQUIRE(picojson::parse(v, sanitized).empty());
  return v;
}

static void check_json_eq(const std::string& actual, const std::string& expected) {
  REQUIRE(parse_json(actual).serialize() == parse_json(expected).serialize());
}

// ============================================================================
// SQL_C_CHAR
// ============================================================================

TEST_CASE("OBJECT to SQL_C_CHAR", "[object][conversion][c_char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When OBJECT values (simple, multiple keys, empty, mixed types) are fetched as SQL_C_CHAR
  auto simple = check_char_success(conn.execute_fetch("SELECT OBJECT_CONSTRUCT('key','val')"), 1);
  auto multi = check_char_success(conn.execute_fetch("SELECT OBJECT_CONSTRUCT('a', 1, 'b', 2)"), 1);
  auto empty = check_char_success(conn.execute_fetch("SELECT OBJECT_CONSTRUCT()"), 1);
  auto mixed =
      check_char_success(conn.execute_fetch("SELECT OBJECT_CONSTRUCT('str', 'hello', 'num', 42, 'bool', true)"), 1);

  // Then JSON object string representation is returned
  check_json_eq(simple, R"({"key":"val"})");
  check_json_eq(multi, R"({"a":1,"b":2})");
  check_json_eq(empty, "{}");
  auto json = parse_json(mixed);
  CHECK(json.is<picojson::object>());
  CHECK(json.get<picojson::object>().size() == 3);
}

TEST_CASE("OBJECT to SQL_C_CHAR nested", "[object][conversion][c_char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Nested OBJECT values (nested object, object with array, deeply nested) are fetched as SQL_C_CHAR
  auto nested =
      check_char_success(conn.execute_fetch("SELECT OBJECT_CONSTRUCT('outer', OBJECT_CONSTRUCT('inner', 42))"), 1);
  auto with_arr = check_char_success(conn.execute_fetch("SELECT OBJECT_CONSTRUCT('items', ARRAY_CONSTRUCT(1,2,3))"), 1);
  auto deep = check_char_success(
      conn.execute_fetch("SELECT OBJECT_CONSTRUCT('a', OBJECT_CONSTRUCT('b', OBJECT_CONSTRUCT('c', 1)))"), 1);

  // Then Nested JSON object string is returned
  check_json_eq(nested, R"({"outer":{"inner":42}})");
  check_json_eq(with_arr, R"({"items":[1,2,3]})");
  check_json_eq(deep, R"({"a":{"b":{"c":1}}})");
}

TEST_CASE("OBJECT to SQL_C_CHAR truncation", "[object][conversion][c_char][01004]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When An OBJECT value is fetched into a buffer smaller than the JSON string
  auto stmt = conn.execute_fetch("SELECT OBJECT_CONSTRUCT('long_key','long_value_string')");
  char buffer[8] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);

  // Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004
  CHECK(ret == SQL_SUCCESS_WITH_INFO);
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "01004");
}

TEST_CASE("OBJECT NULL to SQL_C_CHAR", "[object][conversion][c_char][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL OBJECT value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::OBJECT");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_CHAR);
}

TEST_CASE("OBJECT to SQL_C_CHAR null key omission", "[object][conversion][c_char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When An OBJECT with a null-valued key is constructed using OBJECT_CONSTRUCT
  auto result = check_char_success(conn.execute_fetch("SELECT OBJECT_CONSTRUCT('a',1,'b','BBBB','c',null)"), 1);

  // Then The null-valued key is omitted from the JSON result
  auto json = parse_json(result);
  REQUIRE(json.is<picojson::object>());
  const auto& obj = json.get<picojson::object>();
  CHECK(obj.find("a") != obj.end());
  CHECK(obj.find("b") != obj.end());
  CHECK(obj.find("c") == obj.end());
}

TEST_CASE("OBJECT to SQL_C_CHAR keep null", "[object][conversion][c_char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When An OBJECT with a null-valued key is constructed using OBJECT_CONSTRUCT_KEEP_NULL
  auto result =
      check_char_success(conn.execute_fetch("SELECT OBJECT_CONSTRUCT_KEEP_NULL('a',1,'b','BBBB','c',null)"), 1);

  // Then The null-valued key is preserved with JSON null value
  auto json = parse_json(result);
  REQUIRE(json.is<picojson::object>());
  const auto& obj = json.get<picojson::object>();
  CHECK(obj.find("a") != obj.end());
  CHECK(obj.find("b") != obj.end());
  auto c_it = obj.find("c");
  REQUIRE(c_it != obj.end());
  CHECK(c_it->second.is<picojson::null>());
}

TEST_CASE("OBJECT to SQL_C_CHAR parse_json NULL semantics", "[object][conversion][c_char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When An OBJECT is constructed with PARSE_JSON NULL, SQL NULL, and string null values
  auto result = check_char_success(
      conn.execute_fetch("SELECT OBJECT_CONSTRUCT('a', PARSE_JSON('NULL'), 'b', NULL, 'c', 'null')"), 1);

  // Then JSON null key is preserved, SQL NULL key is omitted, and string null is kept as string
  auto json = parse_json(result);
  REQUIRE(json.is<picojson::object>());
  const auto& obj = json.get<picojson::object>();

  auto a_it = obj.find("a");
  REQUIRE(a_it != obj.end());
  CHECK(a_it->second.is<picojson::null>());

  CHECK(obj.find("b") == obj.end());

  auto c_it = obj.find("c");
  REQUIRE(c_it != obj.end());
  CHECK(c_it->second.is<std::string>());
  CHECK(c_it->second.get<std::string>() == "null");
}

// ============================================================================
// SQL_C_WCHAR
// ============================================================================

TEST_CASE("OBJECT to SQL_C_WCHAR", "[object][conversion][c_char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When OBJECT values are fetched as SQL_C_WCHAR
  auto simple = check_wchar_success(conn.execute_fetch("SELECT OBJECT_CONSTRUCT('key','val')"), 1);
  auto empty = check_wchar_success(conn.execute_fetch("SELECT OBJECT_CONSTRUCT()"), 1);

  // Then JSON object wide string representation is returned
  check_json_eq(std::string(simple.begin(), simple.end()), R"({"key":"val"})");
  check_json_eq(std::string(empty.begin(), empty.end()), "{}");
}

TEST_CASE("OBJECT to SQL_C_WCHAR truncation", "[object][conversion][c_char][01004]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When An OBJECT value is fetched into a WCHAR buffer smaller than the JSON string
  auto stmt = conn.execute_fetch("SELECT OBJECT_CONSTRUCT('long_key','long_value_string')");
  char16_t buffer[6] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_WCHAR, buffer, sizeof(buffer), &indicator);

  // Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004
  CHECK(ret == SQL_SUCCESS_WITH_INFO);
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "01004");
}

TEST_CASE("OBJECT NULL to SQL_C_WCHAR", "[object][conversion][c_char][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL OBJECT value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::OBJECT");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_WCHAR);
}
