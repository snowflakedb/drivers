// Semi-structured type ODBC-specific conversion and metadata tests.
// These tests cover ODBC-only behaviors: SQL_VARCHAR column metadata,
// SQL_C_WCHAR / SQL_C_BINARY retrieval, buffer truncation, SQL_DESC_TYPE_NAME,
// multi-row fetching, and structured types.
// Based on: tests/definitions/shared/types/semi_structured_conversion.feature
#include <picojson.h>
#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "conversion_checks.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

static constexpr SQLULEN kExpectedSemiStructuredColumnSize = 134217728;

static picojson::value parse_json_text(const std::string& json_text);
static picojson::value parse_json_text(const std::u16string& json_text);
static void check_json_equals(const std::string& actual_json_text, const std::string& expected_json_text);
static void check_json_equals(const std::u16string& actual_json_text, const std::string& expected_json_text);

// ============================================================================
// TYPE CASTING (ODBC-specific)
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should cast semi-structured values to SQL_VARCHAR", "[semi_structured]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT PARSE_JSON('{\"a\":1}'), ARRAY_CONSTRUCT(1,2,3), OBJECT_CONSTRUCT('key','val')" is executed
  auto stmt = conn.execute_fetch(
      "SELECT PARSE_JSON('{\"a\":1}'), ARRAY_CONSTRUCT(1,2,3), "
      "OBJECT_CONSTRUCT('key','val')");

  // Then All columns should report SQL_VARCHAR with column_size 134217728 and decimal_digits 0
  for (SQLUSMALLINT col = 1; col <= 3; ++col) {
    SQLSMALLINT data_type = 0;
    SQLULEN column_size = 0;
    SQLSMALLINT decimal_digits = 0;
    SQLRETURN ret =
        SQLDescribeCol(stmt.getHandle(), col, nullptr, 0, nullptr, &data_type, &column_size, &decimal_digits, nullptr);
    REQUIRE_ODBC(ret, stmt);
    CHECK(data_type == SQL_VARCHAR);
    CHECK(column_size == kExpectedSemiStructuredColumnSize);
    CHECK(decimal_digits == 0);
  }
}

// ============================================================================
// CONVERSION TO SQL_C_CHAR - TRUNCATION (ODBC-specific)
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should truncate variant data when buffer is too short",
                 "[semi_structured][conversion][char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query returning a VARIANT value is executed
  auto stmt = conn.execute_fetch("SELECT PARSE_JSON('{\"long_key\":\"long_value_string\"}')");

  // And Attempt to get data with a buffer smaller than the JSON string
  char buffer[10] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);

  // Then The function should return SQL_SUCCESS_WITH_INFO (truncation occurred)
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccessWithInfo() && OdbcMatchers::HasSqlState("01004"));

  // And The buffer should contain a truncated null-terminated string
  CHECK(strlen(buffer) == sizeof(buffer) - 1);
  CHECK(buffer[sizeof(buffer) - 1] == 0);

  // And The indicator should report SQL_NO_TOTAL or the full untruncated length
  const bool indicator_reports_compatible_length =
      (indicator == SQL_NO_TOTAL) || (indicator > static_cast<SQLLEN>(sizeof(buffer)));
  CHECK(indicator_reports_compatible_length);
}

// ============================================================================
// CONVERSION TO SQL_C_WCHAR (ODBC-specific)
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should retrieve variant data as SQL_C_WCHAR",
                 "[semi_structured][conversion][wchar]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query returning a VARIANT value is executed
  auto stmt = conn.execute_fetch("SELECT PARSE_JSON('{\"w\":1}')");

  // Then Data should be retrievable as wide character string (SQL_C_WCHAR)
  check_json_equals(check_wchar_success(stmt, 1), R"({"w":1})");
}

// ============================================================================
// CONVERSION TO SQL_C_BINARY (ODBC-specific)
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should retrieve variant data as SQL_C_BINARY",
                 "[semi_structured][conversion][binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query returning a VARIANT value is executed
  auto stmt = conn.execute_fetch("SELECT PARSE_JSON('{\"b\":2}')");

  // Then Data should be retrievable as raw bytes (SQL_C_BINARY)
  SQLCHAR buffer[256] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
  REQUIRE_ODBC(ret, stmt);
  CHECK(indicator > 0);

  check_json_equals(std::string(reinterpret_cast<char*>(buffer), static_cast<size_t>(indicator)), R"({"b":2})");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should return SQL_NULL_DATA for NULL variant as SQL_C_BINARY",
                 "[semi_structured][conversion][binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query returning a NULL VARIANT is executed
  auto stmt = conn.execute_fetch("SELECT NULL::VARIANT");

  // Then Indicator should be SQL_NULL_DATA
  SQLCHAR buffer[64] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
  REQUIRE_ODBC(ret, stmt);
  CHECK(indicator == SQL_NULL_DATA);
}

// ============================================================================
// JSON WITH UNICODE VIA SQL_C_WCHAR (ODBC-specific)
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should handle JSON with unicode via SQL_C_WCHAR",
                 "[semi_structured][conversion][wchar]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query returning JSON with unicode characters is executed
  auto stmt = conn.execute_fetch("SELECT PARSE_JSON('{\"emoji\":\"\\u2744\",\"cjk\":\"\\u96EA\\u82B1\"}')");

  // Then Data should be retrievable as wide character string with unicode preserved
  auto wstr = check_wchar_success(stmt, 1);
  auto json = parse_json_text(wstr);
  REQUIRE(json.is<picojson::object>());
  const auto& obj = json.get<picojson::object>();
  auto emoji_it = obj.find("emoji");
  REQUIRE(emoji_it != obj.end());
  CHECK(emoji_it->second.get<std::string>() == "\xe2\x9d\x84");
}

// ============================================================================
// CONVERSION TO SQL_C_WCHAR - TRUNCATION (ODBC-specific)
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should truncate variant data as SQL_C_WCHAR when buffer is too short",
                 "[semi_structured][conversion][wchar][01004]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query returning a VARIANT value is executed
  auto stmt = conn.execute_fetch("SELECT PARSE_JSON('{\"long_key\":\"long_value_string\"}')");

  // And Attempt to get data with a wide-char buffer smaller than the JSON string
  char16_t buffer[6] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_WCHAR, buffer, sizeof(buffer), &indicator);

  // Then The function should return SQL_SUCCESS_WITH_INFO with SQLSTATE 01004
  CHECK(ret == SQL_SUCCESS_WITH_INFO);
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "01004");

  // And The buffer should contain a null-terminated truncated wide string
  CHECK(buffer[sizeof(buffer) / sizeof(char16_t) - 1] == u'\0');

  // And The indicator should report SQL_NO_TOTAL or the full untruncated byte length
  const bool indicator_reports_compatible_length =
      (indicator == SQL_NO_TOTAL) || (indicator > static_cast<SQLLEN>(sizeof(buffer)));
  CHECK(indicator_reports_compatible_length);
}

// ============================================================================
// SQLColAttribute - SQL_DESC_TYPE_NAME (ODBC-specific)
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should report SQL_DESC_TYPE_NAME for semi-structured columns",
                 "[semi_structured][metadata]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query returning VARIANT, ARRAY, and OBJECT columns is executed
  auto stmt = conn.execute_fetch(
      "SELECT PARSE_JSON('{\"a\":1}'), ARRAY_CONSTRUCT(1,2,3), "
      "OBJECT_CONSTRUCT('key','val')");

  // Then SQL_DESC_TYPE_NAME should report VARIANT, ARRAY, and STRUCT respectively
  const char* expected_type_names[] = {"VARIANT", "ARRAY", "STRUCT"};
  for (SQLUSMALLINT col = 1; col <= 3; ++col) {
    INFO("Column " << col << " (" << expected_type_names[col - 1] << ")");
    SQLCHAR type_name[128] = {};
    SQLSMALLINT name_len = 0;
    SQLRETURN ret =
        SQLColAttribute(stmt.getHandle(), col, SQL_DESC_TYPE_NAME, type_name, sizeof(type_name), &name_len, nullptr);
    REQUIRE_ODBC(ret, stmt);
    CHECK(std::string(reinterpret_cast<char*>(type_name), name_len) == expected_type_names[col - 1]);
  }
}

// ============================================================================
// MULTI-ROW TABLE OPERATIONS (ODBC-specific)
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should select multi-row table with all semi-structured columns",
                 "[semi_structured]") {
  // Given Snowflake client is logged in

  // And Table with VARIANT, OBJECT, and ARRAY columns exists with multiple rows including NULLs
  conn.execute("CREATE OR REPLACE TEMPORARY TABLE semi_multi (id INT, v VARIANT, o OBJECT, a ARRAY)");
  conn.execute(
      "INSERT INTO semi_multi "
      "SELECT 1, PARSE_JSON('{\"x\":1}'), OBJECT_CONSTRUCT('k','v1'), ARRAY_CONSTRUCT(1,2)");
  conn.execute(
      "INSERT INTO semi_multi "
      "SELECT 2, PARSE_JSON('[10,20]'), OBJECT_CONSTRUCT('a',1,'b',2), ARRAY_CONSTRUCT('x','y','z')");
  conn.execute(
      "INSERT INTO semi_multi "
      "SELECT 3, NULL, NULL, NULL");

  // When Query "SELECT v, o, a FROM <table> ORDER BY id" is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT v, o, a FROM semi_multi ORDER BY id"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then Each row should contain the expected semi-structured values including NULLs
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  check_json_equals(get_data<SQL_C_CHAR>(stmt, 1), R"({"x":1})");
  check_json_equals(get_data<SQL_C_CHAR>(stmt, 2), R"({"k":"v1"})");
  check_json_equals(get_data<SQL_C_CHAR>(stmt, 3), R"([1,2])");

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  check_json_equals(get_data<SQL_C_CHAR>(stmt, 1), R"([10,20])");
  check_json_equals(get_data<SQL_C_CHAR>(stmt, 2), R"({"a":1,"b":2})");
  check_json_equals(get_data<SQL_C_CHAR>(stmt, 3), R"(["x","y","z"])");

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(!get_data_optional<SQL_C_CHAR>(stmt, 1).has_value());
  CHECK(!get_data_optional<SQL_C_CHAR>(stmt, 2).has_value());
  CHECK(!get_data_optional<SQL_C_CHAR>(stmt, 3).has_value());

  ret = SQLFetch(stmt.getHandle());
  CHECK(ret == SQL_NO_DATA);
}

// ============================================================================
// STRUCTURED TYPES (ODBC-specific)
// ============================================================================

TEST_CASE("should handle structured types", "[semi_structured][structured_types]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Structured type expressions (typed array, typed object, typed map) are fetched as SQL_C_CHAR
  auto arr_stmt = conn.execute_fetch("SELECT ARRAY_CONSTRUCT(1,2,3)::ARRAY(INT)");

  // Then Each structured type returns valid JSON data
  check_json_equals(get_data<SQL_C_CHAR>(arr_stmt, 1), R"([1,2,3])");

  auto obj_stmt =
      conn.execute_fetch("SELECT OBJECT_CONSTRUCT('a', 1, 'b', 'two', 'c', 3)::OBJECT(a INT, b VARCHAR, c INT)");
  auto obj_json = parse_json_text(get_data<SQL_C_CHAR>(obj_stmt, 1));
  REQUIRE(obj_json.is<picojson::object>());
  CHECK(obj_json.get<picojson::object>().size() == 3);

  auto map_stmt = conn.execute_fetch("SELECT OBJECT_CONSTRUCT('x', 'foo', 'y', 'bar')::MAP(VARCHAR, VARCHAR)");
  auto map_json = parse_json_text(get_data<SQL_C_CHAR>(map_stmt, 1));
  REQUIRE(map_json.is<picojson::object>());
  CHECK(map_json.get<picojson::object>().size() == 2);
}

// ============================================================================
// Helpers
// ============================================================================

static picojson::value parse_json_text(const std::string& json_text) {
  picojson::value json;
  const auto error = picojson::parse(json, json_text);
  REQUIRE(error.empty());
  return json;
}

static std::string utf16_to_utf8(const std::u16string& src) {
  std::string utf8;
  utf8.reserve(src.size() * 3);
  for (char16_t c : src) {
    if (c < 0x80) {
      utf8.push_back(static_cast<char>(c));
    } else if (c < 0x800) {
      utf8.push_back(static_cast<char>(0xC0 | (c >> 6)));
      utf8.push_back(static_cast<char>(0x80 | (c & 0x3F)));
    } else {
      utf8.push_back(static_cast<char>(0xE0 | (c >> 12)));
      utf8.push_back(static_cast<char>(0x80 | ((c >> 6) & 0x3F)));
      utf8.push_back(static_cast<char>(0x80 | (c & 0x3F)));
    }
  }
  return utf8;
}

static picojson::value parse_json_text(const std::u16string& json_text) {
  return parse_json_text(utf16_to_utf8(json_text));
}

static void check_json_equals(const std::string& actual_json_text, const std::string& expected_json_text) {
  const auto actual_json = parse_json_text(actual_json_text);
  const auto expected_json = parse_json_text(expected_json_text);
  REQUIRE(actual_json.serialize() == expected_json.serialize());
}

static void check_json_equals(const std::u16string& actual_json_text, const std::string& expected_json_text) {
  check_json_equals(utf16_to_utf8(actual_json_text), expected_json_text);
}
