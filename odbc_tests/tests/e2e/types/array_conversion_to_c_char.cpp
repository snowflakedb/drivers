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

TEST_CASE("ARRAY to SQL_C_CHAR", "[array][conversion][c_char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When ARRAY values (integer, string, empty, single element, mixed types) are fetched as SQL_C_CHAR
  auto int_arr = check_char_success(conn.execute_fetch("SELECT ARRAY_CONSTRUCT(1,2,3)"), 1);
  auto str_arr = check_char_success(conn.execute_fetch("SELECT ARRAY_CONSTRUCT('a','b','c')"), 1);
  auto empty_arr = check_char_success(conn.execute_fetch("SELECT ARRAY_CONSTRUCT()"), 1);
  auto single = check_char_success(conn.execute_fetch("SELECT ARRAY_CONSTRUCT(42)"), 1);
  auto mixed = check_char_success(conn.execute_fetch("SELECT ARRAY_CONSTRUCT(1, 'two', 3.0, true, null)"), 1);

  // Then JSON array string representation is returned
  check_json_eq(int_arr, "[1,2,3]");
  check_json_eq(str_arr, R"(["a","b","c"])");
  check_json_eq(empty_arr, "[]");
  check_json_eq(single, "[42]");
  auto json = parse_json(mixed);
  CHECK(json.is<picojson::array>());
  CHECK(json.get<picojson::array>().size() == 5);
}

TEST_CASE("ARRAY to SQL_C_CHAR nested", "[array][conversion][c_char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Nested and deeply nested ARRAY values are fetched as SQL_C_CHAR
  auto nested =
      check_char_success(conn.execute_fetch("SELECT ARRAY_CONSTRUCT(ARRAY_CONSTRUCT(1,2), ARRAY_CONSTRUCT(3,4))"), 1);
  auto deep = check_char_success(conn.execute_fetch("SELECT ARRAY_CONSTRUCT(ARRAY_CONSTRUCT(ARRAY_CONSTRUCT(1)))"), 1);

  // Then Nested JSON array string is returned
  check_json_eq(nested, "[[1,2],[3,4]]");
  check_json_eq(deep, "[[[1]]]");
}

TEST_CASE("ARRAY to SQL_C_CHAR large array", "[array][conversion][c_char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A large ARRAY with 20 elements is fetched as SQL_C_CHAR
  auto result = check_char_success(
      conn.execute_fetch("SELECT ARRAY_CONSTRUCT(1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20)"), 1);

  // Then All elements are present in the JSON array string
  auto json = parse_json(result);
  CHECK(json.is<picojson::array>());
  CHECK(json.get<picojson::array>().size() == 20);
}

TEST_CASE("ARRAY to SQL_C_CHAR truncation", "[array][conversion][c_char][01004]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When An ARRAY value is fetched into a buffer smaller than the JSON string
  auto stmt = conn.execute_fetch("SELECT ARRAY_CONSTRUCT(1,2,3,4,5,6,7,8,9,10)");
  char buffer[6] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);

  // Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004
  CHECK(ret == SQL_SUCCESS_WITH_INFO);
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "01004");
}

TEST_CASE("ARRAY NULL to SQL_C_CHAR", "[array][conversion][c_char][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL ARRAY value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::ARRAY");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_CHAR);
}

TEST_CASE("ARRAY to SQL_C_CHAR with null elements", "[array][conversion][c_char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When An ARRAY with interleaved null elements is fetched as SQL_C_CHAR
  auto result = check_char_success(conn.execute_fetch("SELECT ARRAY_CONSTRUCT(1, null, 3, null, 5)"), 1);

  // Then Null elements are represented as JSON null in the array
  auto json = parse_json(result);
  REQUIRE(json.is<picojson::array>());
  const auto& arr = json.get<picojson::array>();
  CHECK(arr.size() == 5);
  CHECK(arr[0].get<double>() == 1);
  CHECK(arr[1].is<picojson::null>());
  CHECK(arr[2].get<double>() == 3);
  CHECK(arr[3].is<picojson::null>());
  CHECK(arr[4].get<double>() == 5);
}

// ============================================================================
// SQL_C_WCHAR
// ============================================================================

TEST_CASE("ARRAY to SQL_C_WCHAR", "[array][conversion][c_char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When ARRAY values are fetched as SQL_C_WCHAR
  auto int_arr = check_wchar_success(conn.execute_fetch("SELECT ARRAY_CONSTRUCT(1,2,3)"), 1);
  auto empty = check_wchar_success(conn.execute_fetch("SELECT ARRAY_CONSTRUCT()"), 1);

  // Then JSON array wide string representation is returned
  check_json_eq(std::string(int_arr.begin(), int_arr.end()), "[1,2,3]");
  check_json_eq(std::string(empty.begin(), empty.end()), "[]");
}

TEST_CASE("ARRAY to SQL_C_WCHAR truncation", "[array][conversion][c_char][01004]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When An ARRAY value is fetched into a WCHAR buffer smaller than the JSON string
  auto stmt = conn.execute_fetch("SELECT ARRAY_CONSTRUCT(1,2,3,4,5,6,7,8,9,10)");
  char16_t buffer[6] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_WCHAR, buffer, sizeof(buffer), &indicator);

  // Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004
  CHECK(ret == SQL_SUCCESS_WITH_INFO);
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "01004");
}

TEST_CASE("ARRAY NULL to SQL_C_WCHAR", "[array][conversion][c_char][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL ARRAY value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::ARRAY");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_WCHAR);
}
