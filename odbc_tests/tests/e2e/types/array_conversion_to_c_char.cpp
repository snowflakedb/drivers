#include <picojson.h>
#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"
#include "get_diag_rec.hpp"

// Snowflake may serialize null values as "undefined" in semi-structured types,
// which is not valid JSON. Replace with "null" before parsing.
static std::string sanitize_json(const std::string& text) {
  std::string result = text;
  std::string target = "undefined";
  size_t pos = 0;
  while ((pos = result.find(target, pos)) != std::string::npos) {
    bool at_word_boundary =
        (pos == 0 || !std::isalnum(result[pos - 1])) &&
        (pos + target.size() >= result.size() || !std::isalnum(result[pos + target.size()]));
    if (at_word_boundary) {
      result.replace(pos, target.size(), "null");
      pos += 4;
    } else {
      pos += target.size();
    }
  }
  return result;
}

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
  Connection conn;

  {
    INFO("integer array");
    auto result = check_char_success(conn.execute_fetch("SELECT ARRAY_CONSTRUCT(1,2,3)"), 1);
    check_json_eq(result, "[1,2,3]");
  }

  {
    INFO("string array");
    auto result = check_char_success(conn.execute_fetch("SELECT ARRAY_CONSTRUCT('a','b','c')"), 1);
    check_json_eq(result, R"(["a","b","c"])");
  }

  {
    INFO("empty array");
    auto result = check_char_success(conn.execute_fetch("SELECT ARRAY_CONSTRUCT()"), 1);
    check_json_eq(result, "[]");
  }

  {
    INFO("single element");
    auto result = check_char_success(conn.execute_fetch("SELECT ARRAY_CONSTRUCT(42)"), 1);
    check_json_eq(result, "[42]");
  }

  {
    INFO("mixed types");
    auto result = check_char_success(conn.execute_fetch("SELECT ARRAY_CONSTRUCT(1, 'two', 3.0, true, null)"), 1);
    auto json = parse_json(result);
    CHECK(json.is<picojson::array>());
    CHECK(json.get<picojson::array>().size() == 5);
  }
}

TEST_CASE("ARRAY to SQL_C_CHAR nested", "[array][conversion][c_char]") {
  Connection conn;

  {
    INFO("nested arrays");
    auto result = check_char_success(
        conn.execute_fetch("SELECT ARRAY_CONSTRUCT(ARRAY_CONSTRUCT(1,2), ARRAY_CONSTRUCT(3,4))"), 1);
    check_json_eq(result, "[[1,2],[3,4]]");
  }

  {
    INFO("deeply nested");
    auto result = check_char_success(
        conn.execute_fetch("SELECT ARRAY_CONSTRUCT(ARRAY_CONSTRUCT(ARRAY_CONSTRUCT(1)))"), 1);
    check_json_eq(result, "[[[1]]]");
  }
}

TEST_CASE("ARRAY to SQL_C_CHAR large array", "[array][conversion][c_char]") {
  Connection conn;

  auto result = check_char_success(conn.execute_fetch(
      "SELECT ARRAY_CONSTRUCT(1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20)"), 1);
  auto json = parse_json(result);
  CHECK(json.is<picojson::array>());
  CHECK(json.get<picojson::array>().size() == 20);
}

TEST_CASE("ARRAY to SQL_C_CHAR truncation", "[array][conversion][c_char][01004]") {
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT ARRAY_CONSTRUCT(1,2,3,4,5,6,7,8,9,10)");
  char buffer[6] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);

  CHECK(ret == SQL_SUCCESS_WITH_INFO);
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "01004");
}

TEST_CASE("ARRAY NULL to SQL_C_CHAR", "[array][conversion][c_char][null]") {
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT NULL::ARRAY");

  check_null_via_get_data(stmt, 1, SQL_C_CHAR);
}

// ============================================================================
// SQL_C_WCHAR
// ============================================================================

TEST_CASE("ARRAY to SQL_C_WCHAR", "[array][conversion][c_char]") {
  Connection conn;

  {
    INFO("integer array");
    auto result = check_wchar_success(conn.execute_fetch("SELECT ARRAY_CONSTRUCT(1,2,3)"), 1);
    auto utf8 = std::string(result.begin(), result.end());
    check_json_eq(utf8, "[1,2,3]");
  }

  {
    INFO("empty array");
    auto result = check_wchar_success(conn.execute_fetch("SELECT ARRAY_CONSTRUCT()"), 1);
    auto utf8 = std::string(result.begin(), result.end());
    check_json_eq(utf8, "[]");
  }
}

TEST_CASE("ARRAY to SQL_C_WCHAR truncation", "[array][conversion][c_char][01004]") {
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT ARRAY_CONSTRUCT(1,2,3,4,5,6,7,8,9,10)");
  char16_t buffer[6] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_WCHAR, buffer, sizeof(buffer), &indicator);

  CHECK(ret == SQL_SUCCESS_WITH_INFO);
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "01004");
}

TEST_CASE("ARRAY NULL to SQL_C_WCHAR", "[array][conversion][c_char][null]") {
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT NULL::ARRAY");

  check_null_via_get_data(stmt, 1, SQL_C_WCHAR);
}
