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

TEST_CASE("VARIANT to SQL_C_CHAR", "[variant][conversion][c_char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When VARIANT values (object, string, empty, numeric, boolean) are fetched as SQL_C_CHAR
  auto obj = check_char_success(conn.execute_fetch("SELECT PARSE_JSON('{\"a\":1}')"), 1);
  auto kv = check_char_success(conn.execute_fetch("SELECT PARSE_JSON('{\"key\":\"value\"}')"), 1);
  auto empty_obj = check_char_success(conn.execute_fetch("SELECT PARSE_JSON('{}')"), 1);
  auto empty_arr = check_char_success(conn.execute_fetch("SELECT PARSE_JSON('[]')"), 1);
  auto num = check_char_success(conn.execute_fetch("SELECT PARSE_JSON('42')"), 1);
  auto str = check_char_success(conn.execute_fetch("SELECT PARSE_JSON('\"hello\"')"), 1);
  auto bool_t = check_char_success(conn.execute_fetch("SELECT PARSE_JSON('true')"), 1);
  auto bool_f = check_char_success(conn.execute_fetch("SELECT PARSE_JSON('false')"), 1);

  // Then JSON string representation is returned for each variant type
  check_json_eq(obj, R"({"a":1})");
  check_json_eq(kv, R"({"key":"value"})");
  check_json_eq(empty_obj, "{}");
  check_json_eq(empty_arr, "[]");
  CHECK(num == "42");
  CHECK(str == "\"hello\"");
  CHECK(bool_t == "true");
  CHECK(bool_f == "false");
}

TEST_CASE("VARIANT to SQL_C_CHAR nested", "[variant][conversion][c_char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Deeply nested VARIANT values and arrays of objects are fetched as SQL_C_CHAR
  auto deep = check_char_success(conn.execute_fetch("SELECT PARSE_JSON('{\"a\":{\"b\":[1,2,{\"c\":true}]}}')"), 1);
  auto arr_of_obj = check_char_success(conn.execute_fetch("SELECT PARSE_JSON('[{\"x\":1},{\"y\":2}]')"), 1);

  // Then Nested JSON string is returned
  check_json_eq(deep, R"({"a":{"b":[1,2,{"c":true}]}})");
  check_json_eq(arr_of_obj, R"([{"x":1},{"y":2}])");
}

TEST_CASE("VARIANT to SQL_C_CHAR with special characters", "[variant][conversion][c_char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When VARIANT values containing escaped quotes and control characters are fetched
  auto escaped = check_char_success(conn.execute_fetch("SELECT PARSE_JSON('{\"msg\":\"say \\\\\"hi\\\\\"\"}')"), 1);
  auto control = check_char_success(conn.execute_fetch("SELECT PARSE_JSON('{\"text\":\"line1\\\\nline2\"}')"), 1);

  // Then Valid JSON is returned preserving special characters
  auto json1 = parse_json(escaped);
  CHECK(json1.is<picojson::object>());
  auto json2 = parse_json(control);
  CHECK(json2.is<picojson::object>());
}

TEST_CASE("VARIANT to SQL_C_CHAR truncation", "[variant][conversion][c_char][01004]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A VARIANT value is fetched into a buffer smaller than the JSON string
  auto stmt = conn.execute_fetch("SELECT PARSE_JSON('{\"long_key\":\"long_value_string\"}')");
  char buffer[8] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);

  // Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004
  CHECK(ret == SQL_SUCCESS_WITH_INFO);
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "01004");
}

TEST_CASE("VARIANT NULL to SQL_C_CHAR", "[variant][conversion][c_char][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL VARIANT value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::VARIANT");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_CHAR);
}

TEST_CASE("VARIANT to SQL_C_CHAR scalar values", "[variant][conversion][c_char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Scalar values (integer, float, cast, boolean, string) are converted to VARIANT and fetched as SQL_C_CHAR
  auto int_val = check_char_success(conn.execute_fetch("SELECT to_variant(100)"), 1);
  auto float_val = check_char_success(conn.execute_fetch("SELECT to_variant(3.14)"), 1);
  auto cast_val = check_char_success(conn.execute_fetch("SELECT 42::VARIANT"), 1);
  auto bool_val = check_char_success(conn.execute_fetch("SELECT to_variant(true)"), 1);
  auto str_val = check_char_success(conn.execute_fetch("SELECT to_variant('hello')"), 1);

  // Then Each scalar VARIANT value is returned as its JSON representation
  CHECK(int_val == "100");
  CHECK(parse_json(float_val).is<double>());
  CHECK(cast_val == "42");
  CHECK(bool_val == "true");
  CHECK(str_val == "\"hello\"");
}

TEST_CASE("VARIANT to SQL_C_CHAR parse_json NULL", "[variant][conversion][c_char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When PARSE_JSON NULL and SQL NULL VARIANT values are fetched
  auto json_null = check_char_success(conn.execute_fetch("SELECT PARSE_JSON('NULL')"), 1);

  // Then PARSE_JSON NULL returns JSON null string, SQL NULL returns SQL_NULL_DATA
  CHECK(json_null == "null");
  auto sql_null_stmt = conn.execute_fetch("SELECT NULL::VARIANT");
  check_null_via_get_data(sql_null_stmt, 1, SQL_C_CHAR);
}

// ============================================================================
// SQL_C_WCHAR
// ============================================================================

TEST_CASE("VARIANT to SQL_C_WCHAR", "[variant][conversion][c_char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When VARIANT values are fetched as SQL_C_WCHAR
  auto obj = check_wchar_success(conn.execute_fetch("SELECT PARSE_JSON('{\"w\":1}')"), 1);
  auto empty = check_wchar_success(conn.execute_fetch("SELECT PARSE_JSON('{}')"), 1);

  // Then JSON wide string representation is returned
  check_json_eq(std::string(obj.begin(), obj.end()), R"({"w":1})");
  check_json_eq(std::string(empty.begin(), empty.end()), "{}");
}

TEST_CASE("VARIANT to SQL_C_WCHAR truncation", "[variant][conversion][c_char][01004]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A VARIANT value is fetched into a WCHAR buffer smaller than the JSON string
  auto stmt = conn.execute_fetch("SELECT PARSE_JSON('{\"long_key\":\"long_value_string\"}')");
  char16_t buffer[6] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_WCHAR, buffer, sizeof(buffer), &indicator);

  // Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004
  CHECK(ret == SQL_SUCCESS_WITH_INFO);
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "01004");
}

TEST_CASE("VARIANT NULL to SQL_C_WCHAR", "[variant][conversion][c_char][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL VARIANT value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::VARIANT");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_WCHAR);
}
