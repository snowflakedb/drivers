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
  picojson::value v;
  REQUIRE(picojson::parse(v, text).empty());
  return v;
}

static void check_json_eq(const std::string& actual, const std::string& expected) {
  REQUIRE(parse_json(actual).serialize() == parse_json(expected).serialize());
}

// ============================================================================
// SQL_C_CHAR
// ============================================================================

TEST_CASE("VARIANT to SQL_C_CHAR", "[variant][conversion][c_char]") {
  Connection conn;

  {
    INFO("simple object");
    auto result = check_char_success(conn.execute_fetch("SELECT PARSE_JSON('{\"a\":1}')"), 1);
    check_json_eq(result, R"({"a":1})");
  }

  {
    INFO("key-value string");
    auto result = check_char_success(conn.execute_fetch("SELECT PARSE_JSON('{\"key\":\"value\"}')"), 1);
    check_json_eq(result, R"({"key":"value"})");
  }

  {
    INFO("empty object");
    auto result = check_char_success(conn.execute_fetch("SELECT PARSE_JSON('{}')"), 1);
    check_json_eq(result, "{}");
  }

  {
    INFO("empty array");
    auto result = check_char_success(conn.execute_fetch("SELECT PARSE_JSON('[]')"), 1);
    check_json_eq(result, "[]");
  }

  {
    INFO("numeric value");
    auto result = check_char_success(conn.execute_fetch("SELECT PARSE_JSON('42')"), 1);
    CHECK(result == "42");
  }

  {
    INFO("string value");
    auto result = check_char_success(conn.execute_fetch("SELECT PARSE_JSON('\"hello\"')"), 1);
    CHECK(result == "\"hello\"");
  }

  {
    INFO("boolean true");
    auto result = check_char_success(conn.execute_fetch("SELECT PARSE_JSON('true')"), 1);
    CHECK(result == "true");
  }

  {
    INFO("boolean false");
    auto result = check_char_success(conn.execute_fetch("SELECT PARSE_JSON('false')"), 1);
    CHECK(result == "false");
  }
}

TEST_CASE("VARIANT to SQL_C_CHAR nested", "[variant][conversion][c_char]") {
  Connection conn;

  {
    INFO("deeply nested");
    auto result = check_char_success(
        conn.execute_fetch("SELECT PARSE_JSON('{\"a\":{\"b\":[1,2,{\"c\":true}]}}')"), 1);
    check_json_eq(result, R"({"a":{"b":[1,2,{"c":true}]}})");
  }

  {
    INFO("array of objects");
    auto result = check_char_success(
        conn.execute_fetch("SELECT PARSE_JSON('[{\"x\":1},{\"y\":2}]')"), 1);
    check_json_eq(result, R"([{"x":1},{"y":2}])");
  }
}

TEST_CASE("VARIANT to SQL_C_CHAR with special characters", "[variant][conversion][c_char]") {
  Connection conn;

  {
    INFO("escaped quotes");
    auto result = check_char_success(
        conn.execute_fetch("SELECT PARSE_JSON('{\"msg\":\"say \\\\\"hi\\\\\"\"}')"), 1);
    auto json = parse_json(result);
    CHECK(json.is<picojson::object>());
  }

  {
    INFO("newlines and tabs");
    auto result = check_char_success(
        conn.execute_fetch("SELECT PARSE_JSON('{\"text\":\"line1\\\\nline2\"}')"), 1);
    auto json = parse_json(result);
    CHECK(json.is<picojson::object>());
  }
}

TEST_CASE("VARIANT to SQL_C_CHAR truncation", "[variant][conversion][c_char][01004]") {
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT PARSE_JSON('{\"long_key\":\"long_value_string\"}')");
  char buffer[8] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);

  CHECK(ret == SQL_SUCCESS_WITH_INFO);
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "01004");
}

TEST_CASE("VARIANT NULL to SQL_C_CHAR", "[variant][conversion][c_char][null]") {
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT NULL::VARIANT");

  check_null_via_get_data(stmt, 1, SQL_C_CHAR);
}

// ============================================================================
// SQL_C_WCHAR
// ============================================================================

TEST_CASE("VARIANT to SQL_C_WCHAR", "[variant][conversion][c_char]") {
  Connection conn;

  {
    INFO("simple object");
    auto result = check_wchar_success(conn.execute_fetch("SELECT PARSE_JSON('{\"w\":1}')"), 1);
    auto utf8 = std::string(result.begin(), result.end());
    check_json_eq(utf8, R"({"w":1})");
  }

  {
    INFO("empty containers");
    auto result = check_wchar_success(conn.execute_fetch("SELECT PARSE_JSON('{}')"), 1);
    auto utf8 = std::string(result.begin(), result.end());
    check_json_eq(utf8, "{}");
  }
}

TEST_CASE("VARIANT to SQL_C_WCHAR truncation", "[variant][conversion][c_char][01004]") {
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT PARSE_JSON('{\"long_key\":\"long_value_string\"}')");
  char16_t buffer[6] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_WCHAR, buffer, sizeof(buffer), &indicator);

  CHECK(ret == SQL_SUCCESS_WITH_INFO);
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "01004");
}

TEST_CASE("VARIANT NULL to SQL_C_WCHAR", "[variant][conversion][c_char][null]") {
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT NULL::VARIANT");

  check_null_via_get_data(stmt, 1, SQL_C_WCHAR);
}
