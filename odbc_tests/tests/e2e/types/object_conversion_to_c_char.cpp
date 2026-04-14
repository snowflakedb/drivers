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

TEST_CASE("OBJECT to SQL_C_CHAR", "[object][conversion][c_char]") {
  Connection conn;

  {
    INFO("simple key-value");
    auto result = check_char_success(conn.execute_fetch("SELECT OBJECT_CONSTRUCT('key','val')"), 1);
    check_json_eq(result, R"({"key":"val"})");
  }

  {
    INFO("multiple keys");
    auto result = check_char_success(conn.execute_fetch("SELECT OBJECT_CONSTRUCT('a', 1, 'b', 2)"), 1);
    check_json_eq(result, R"({"a":1,"b":2})");
  }

  {
    INFO("empty object");
    auto result = check_char_success(conn.execute_fetch("SELECT OBJECT_CONSTRUCT()"), 1);
    check_json_eq(result, "{}");
  }

  {
    INFO("mixed value types");
    auto result = check_char_success(
        conn.execute_fetch("SELECT OBJECT_CONSTRUCT('str', 'hello', 'num', 42, 'bool', true)"), 1);
    auto json = parse_json(result);
    CHECK(json.is<picojson::object>());
    CHECK(json.get<picojson::object>().size() == 3);
  }
}

TEST_CASE("OBJECT to SQL_C_CHAR nested", "[object][conversion][c_char]") {
  Connection conn;

  {
    INFO("nested object");
    auto result = check_char_success(
        conn.execute_fetch("SELECT OBJECT_CONSTRUCT('outer', OBJECT_CONSTRUCT('inner', 42))"), 1);
    check_json_eq(result, R"({"outer":{"inner":42}})");
  }

  {
    INFO("object with array value");
    auto result = check_char_success(
        conn.execute_fetch("SELECT OBJECT_CONSTRUCT('items', ARRAY_CONSTRUCT(1,2,3))"), 1);
    check_json_eq(result, R"({"items":[1,2,3]})");
  }

  {
    INFO("deeply nested");
    auto result = check_char_success(
        conn.execute_fetch("SELECT OBJECT_CONSTRUCT('a', OBJECT_CONSTRUCT('b', OBJECT_CONSTRUCT('c', 1)))"), 1);
    check_json_eq(result, R"({"a":{"b":{"c":1}}})");
  }
}

TEST_CASE("OBJECT to SQL_C_CHAR truncation", "[object][conversion][c_char][01004]") {
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT OBJECT_CONSTRUCT('long_key','long_value_string')");
  char buffer[8] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);

  CHECK(ret == SQL_SUCCESS_WITH_INFO);
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "01004");
}

TEST_CASE("OBJECT NULL to SQL_C_CHAR", "[object][conversion][c_char][null]") {
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT NULL::OBJECT");

  check_null_via_get_data(stmt, 1, SQL_C_CHAR);
}

// ============================================================================
// SQL_C_WCHAR
// ============================================================================

TEST_CASE("OBJECT to SQL_C_WCHAR", "[object][conversion][c_char]") {
  Connection conn;

  {
    INFO("simple key-value");
    auto result = check_wchar_success(conn.execute_fetch("SELECT OBJECT_CONSTRUCT('key','val')"), 1);
    auto utf8 = std::string(result.begin(), result.end());
    check_json_eq(utf8, R"({"key":"val"})");
  }

  {
    INFO("empty object");
    auto result = check_wchar_success(conn.execute_fetch("SELECT OBJECT_CONSTRUCT()"), 1);
    auto utf8 = std::string(result.begin(), result.end());
    check_json_eq(utf8, "{}");
  }
}

TEST_CASE("OBJECT to SQL_C_WCHAR truncation", "[object][conversion][c_char][01004]") {
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT OBJECT_CONSTRUCT('long_key','long_value_string')");
  char16_t buffer[6] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_WCHAR, buffer, sizeof(buffer), &indicator);

  CHECK(ret == SQL_SUCCESS_WITH_INFO);
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "01004");
}

TEST_CASE("OBJECT NULL to SQL_C_WCHAR", "[object][conversion][c_char][null]") {
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT NULL::OBJECT");

  check_null_via_get_data(stmt, 1, SQL_C_WCHAR);
}
