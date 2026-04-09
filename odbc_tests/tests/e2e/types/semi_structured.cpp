// Semi-structured type (VARIANT/OBJECT/ARRAY) ODBC E2E tests
// Based on: tests/definitions/shared/types/semi_structured.feature
//           tests/definitions/odbc/types/semi_structured.feature
//
// Snowflake semi-structured types are transmitted as JSON-serialized UTF-8
// strings. ODBC reports them as SQL_VARCHAR, and the reported column_size
// follows the session parameter VARCHAR_AND_BINARY_MAX_SIZE_IN_RESULT.
// In the controlled ODBC E2E environment for this suite, that value is
// expected to be 134217728, so these metadata assertions pin to that size.
#include <picojson.h>
#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
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
// TYPE CASTING (shared)
// ============================================================================

TEST_CASE("should cast semi-structured values to appropriate type", "[semi_structured]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT PARSE_JSON('{\"a\":1}'), ARRAY_CONSTRUCT(1,2,3), OBJECT_CONSTRUCT('key','val')" is executed
  auto stmt = conn.execute_fetch(
      "SELECT PARSE_JSON('{\"a\":1}'), ARRAY_CONSTRUCT(1,2,3), "
      "OBJECT_CONSTRUCT('key','val')");

  // Then All values should be returned as appropriate type
  auto col1 = get_data<SQL_C_CHAR>(stmt, 1);
  auto col2 = get_data<SQL_C_CHAR>(stmt, 2);
  auto col3 = get_data<SQL_C_CHAR>(stmt, 3);

  auto json1 = parse_json_text(col1);
  auto json2 = parse_json_text(col2);
  auto json3 = parse_json_text(col3);

  CHECK(json1.is<picojson::object>());
  CHECK(json2.is<picojson::array>());
  CHECK(json3.is<picojson::object>());
}

// ============================================================================
// TYPE CASTING (ODBC-specific)
// ============================================================================

TEST_CASE("should cast semi-structured values to SQL_VARCHAR", "[semi_structured]") {
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
// SELECT WITH LITERALS (shared)
// ============================================================================

TEST_CASE("should select semi-structured literals", "[semi_structured]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT PARSE_JSON('{\"key\":\"value\"}'), ARRAY_CONSTRUCT(10, 20, 30), OBJECT_CONSTRUCT('a', 1, 'b',
  // 2)" is executed
  auto stmt = conn.execute_fetch(
      "SELECT PARSE_JSON('{\"key\":\"value\"}'), "
      "ARRAY_CONSTRUCT(10, 20, 30), "
      "OBJECT_CONSTRUCT('a', 1, 'b', 2)");

  // Then Result should contain the expected values for VARIANT, ARRAY, and OBJECT columns
  check_json_equals(get_data<SQL_C_CHAR>(stmt, 1), R"({"key":"value"})");
  check_json_equals(get_data<SQL_C_CHAR>(stmt, 2), R"([10,20,30])");
  check_json_equals(get_data<SQL_C_CHAR>(stmt, 3), R"({"a":1,"b":2})");
}

TEST_CASE("should select deeply nested semi-structured literals", "[semi_structured]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT PARSE_JSON('{\"a\":{\"b\":[1,2,{\"c\":true}]}}')" is executed
  auto stmt = conn.execute_fetch("SELECT PARSE_JSON('{\"a\":{\"b\":[1,2,{\"c\":true}]}}')");

  // Then Result should contain the expected nested value
  check_json_equals(get_data<SQL_C_CHAR>(stmt, 1), R"({"a":{"b":[1,2,{"c":true}]}})");
}

// ============================================================================
// NULL HANDLING (shared)
// ============================================================================

TEST_CASE("should handle NULL semi-structured values from literals", "[semi_structured]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT NULL::VARIANT, NULL::OBJECT, NULL::ARRAY" is executed
  auto stmt = conn.execute_fetch("SELECT NULL::VARIANT, NULL::OBJECT, NULL::ARRAY");

  // Then All columns should return null indicators
  CHECK(!get_data_optional<SQL_C_CHAR>(stmt, 1).has_value());
  CHECK(!get_data_optional<SQL_C_CHAR>(stmt, 2).has_value());
  CHECK(!get_data_optional<SQL_C_CHAR>(stmt, 3).has_value());
}

// ============================================================================
// TABLE OPERATIONS (shared)
// ============================================================================

TEST_CASE("should select semi-structured values from table", "[semi_structured]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with VARIANT, OBJECT, and ARRAY columns exists with JSON values
  conn.execute(
      "CREATE OR REPLACE TABLE semi_struct_table "
      "(v VARIANT, o OBJECT, a ARRAY)");
  conn.execute(
      "INSERT INTO semi_struct_table "
      "SELECT PARSE_JSON('{\"x\":42}'), "
      "OBJECT_CONSTRUCT('k','v'), "
      "ARRAY_CONSTRUCT('a','b')");

  // When Query "SELECT * FROM <table>" is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM semi_struct_table"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  for (SQLUSMALLINT col = 1; col <= 3; ++col) {
    SQLSMALLINT data_type = 0;
    SQLULEN column_size = 0;
    SQLSMALLINT decimal_digits = 0;
    ret =
        SQLDescribeCol(stmt.getHandle(), col, nullptr, 0, nullptr, &data_type, &column_size, &decimal_digits, nullptr);
    REQUIRE_ODBC(ret, stmt);
    CHECK(data_type == SQL_VARCHAR);
    CHECK(column_size == kExpectedSemiStructuredColumnSize);
    CHECK(decimal_digits == 0);
  }

  // Then Data should contain the expected semi-structured values
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  check_json_equals(get_data<SQL_C_CHAR>(stmt, 1), R"({"x":42})");
  check_json_equals(get_data<SQL_C_CHAR>(stmt, 2), R"({"k":"v"})");
  check_json_equals(get_data<SQL_C_CHAR>(stmt, 3), R"(["a","b"])");
}

TEST_CASE("should handle NULL semi-structured values from table", "[semi_structured]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with VARIANT column exists containing NULLs and values
  conn.execute("CREATE OR REPLACE TABLE semi_struct_null_table (v VARIANT, id INT)");
  conn.execute(
      "INSERT INTO semi_struct_null_table "
      "SELECT PARSE_JSON(column2), column1 FROM VALUES (1, NULL), (2, '{\"a\":1}'), (3, NULL)");

  // When Query "SELECT * FROM <table>" is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM semi_struct_null_table ORDER BY id"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain [NULL, {"a":1}, NULL]
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(!get_data_optional<SQL_C_CHAR>(stmt, 1).has_value());

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  auto val = get_data_optional<SQL_C_CHAR>(stmt, 1);
  REQUIRE(val.has_value());
  check_json_equals(*val, R"({"a":1})");

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(!get_data_optional<SQL_C_CHAR>(stmt, 1).has_value());
}

// ============================================================================
// MULTIPLE CHUNKS DOWNLOADING (shared)
// ============================================================================

TEST_CASE("should download semi-structured data in multiple chunks", "[semi_structured][large_result_set]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT OBJECT_CONSTRUCT('id', seq8()) AS obj FROM TABLE(GENERATOR(ROWCOUNT => 20000)) v ORDER BY 1" is
  // executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(),
                                sqlchar("SELECT OBJECT_CONSTRUCT('id', seq8()) AS obj "
                                        "FROM TABLE(GENERATOR(ROWCOUNT => 20000)) v ORDER BY 1"),
                                SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then All 20000 rows should be fetched and each should contain a value with "id" key
  int row_count = 0;
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) break;
    REQUIRE_ODBC(ret, stmt);
    auto json = parse_json_text(get_data<SQL_C_CHAR>(stmt, 1));
    REQUIRE(json.is<picojson::object>());
    const auto& object_values = json.get<picojson::object>();
    const auto id_it = object_values.find("id");
    REQUIRE(id_it != object_values.end());
    REQUIRE(id_it->second.is<double>());
    row_count++;
  }
  REQUIRE(row_count == 20000);
}

// ============================================================================
// PARAMETER BINDING (shared)
// ============================================================================

TEST_CASE("should select variant using parameter binding", "[semi_structured]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT PARSE_JSON(?)" is executed with bound JSON string '{"bound":true}'
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("SELECT PARSE_JSON(?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  const char* json = "{\"bound\":true}";
  SQLLEN len = static_cast<SQLLEN>(strlen(json));
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, len, 0, const_cast<char*>(json),
                         len, &len);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then Result should contain a value with "bound" key
  check_json_equals(get_data<SQL_C_CHAR>(stmt, 1), R"({"bound":true})");
}

TEST_CASE("should select NULL variant using parameter binding", "[semi_structured]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT PARSE_JSON(?)" is executed with bound NULL value
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("SELECT PARSE_JSON(?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQLLEN null_indicator = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 0, 0, nullptr, 0,
                         &null_indicator);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then Result should be NULL
  CHECK(!get_data_optional<SQL_C_CHAR>(stmt, 1).has_value());
}

TEST_CASE("should insert variant using parameter binding", "[semi_structured]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with VARIANT column exists
  conn.execute("CREATE OR REPLACE TABLE semi_struct_bind (v VARIANT, id INT)");

  // When JSON values are inserted using parameter binding via PARSE_JSON(?)
  const char* values[] = {"{\"x\":1}", "[1,2,3]", "{\"nested\":{\"a\":true}}"};
  for (int i = 0; i < 3; ++i) {
    const auto* val = values[i];
    auto stmt = conn.createStatement();
    const std::string insert_sql = "INSERT INTO semi_struct_bind SELECT PARSE_JSON(?), " + std::to_string(i + 1);
    SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar(insert_sql.c_str()), SQL_NTS);
    REQUIRE_ODBC(ret, stmt);

    SQLLEN len = static_cast<SQLLEN>(strlen(val));
    ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, len, 0,
                           const_cast<char*>(val), len, &len);
    REQUIRE_ODBC(ret, stmt);
    ret = SQLExecute(stmt.getHandle());
    REQUIRE_ODBC(ret, stmt);
  }

  // Then SELECT should return the inserted JSON values
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM semi_struct_bind ORDER BY id"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  check_json_equals(get_data<SQL_C_CHAR>(stmt, 1), R"({"x":1})");

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  check_json_equals(get_data<SQL_C_CHAR>(stmt, 1), R"([1,2,3])");

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  check_json_equals(get_data<SQL_C_CHAR>(stmt, 1), R"({"nested":{"a":true}})");
}

// ============================================================================
// CONVERSION TO SQL_C_CHAR - TRUNCATION (ODBC-specific)
// ============================================================================

TEST_CASE("should truncate variant data when buffer is too short", "[semi_structured][conversion][char]") {
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

TEST_CASE("should retrieve variant data as SQL_C_WCHAR", "[semi_structured][conversion][wchar]") {
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

TEST_CASE("should retrieve variant data as SQL_C_BINARY", "[semi_structured][conversion][binary]") {
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

TEST_CASE("should return SQL_NULL_DATA for NULL variant as SQL_C_BINARY", "[semi_structured][conversion][binary]") {
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
// EMPTY JSON CONTAINERS (shared)
// ============================================================================

TEST_CASE("should handle empty JSON containers", "[semi_structured]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT PARSE_JSON('{}'), ARRAY_CONSTRUCT(), OBJECT_CONSTRUCT()" is executed
  auto stmt = conn.execute_fetch("SELECT PARSE_JSON('{}'), ARRAY_CONSTRUCT(), OBJECT_CONSTRUCT()");

  // Then Each column should return a valid empty container
  check_json_equals(get_data<SQL_C_CHAR>(stmt, 1), R"({})");
  check_json_equals(get_data<SQL_C_CHAR>(stmt, 2), R"([])");
  check_json_equals(get_data<SQL_C_CHAR>(stmt, 3), R"({})");
}

TEST_CASE("should handle empty JSON array literal", "[semi_structured]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT PARSE_JSON('[]')" is executed
  auto stmt = conn.execute_fetch("SELECT PARSE_JSON('[]')");

  // Then Result should be an empty JSON array
  check_json_equals(get_data<SQL_C_CHAR>(stmt, 1), R"([])");
}

TEST_CASE("should round-trip empty JSON containers through a table", "[semi_structured]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with VARIANT, OBJECT, and ARRAY columns exists with empty containers
  conn.execute(
      "CREATE OR REPLACE TABLE semi_struct_empty "
      "(v VARIANT, o OBJECT, a ARRAY)");
  conn.execute(
      "INSERT INTO semi_struct_empty "
      "SELECT PARSE_JSON('{}'), OBJECT_CONSTRUCT(), ARRAY_CONSTRUCT()");

  // When Query "SELECT * FROM <table>" is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM semi_struct_empty"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then All columns should return valid empty containers
  check_json_equals(get_data<SQL_C_CHAR>(stmt, 1), R"({})");
  check_json_equals(get_data<SQL_C_CHAR>(stmt, 2), R"({})");
  check_json_equals(get_data<SQL_C_CHAR>(stmt, 3), R"([])");
}

// ============================================================================
// JSON WITH UNICODE CONTENT (shared)
// ============================================================================

TEST_CASE("should handle JSON with unicode content", "[semi_structured]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query returning JSON with unicode characters is executed
  auto stmt = conn.execute_fetch("SELECT PARSE_JSON('{\"emoji\":\"\\u2744\",\"cjk\":\"\\u96EA\\u82B1\"}')");

  // Then Result should preserve the unicode characters
  auto json = parse_json_text(check_wchar_success(stmt, 1));
  REQUIRE(json.is<picojson::object>());
  const auto& obj = json.get<picojson::object>();

  auto emoji_it = obj.find("emoji");
  REQUIRE(emoji_it != obj.end());
  CHECK(emoji_it->second.get<std::string>() == "\xe2\x9d\x84");

  auto cjk_it = obj.find("cjk");
  REQUIRE(cjk_it != obj.end());
  CHECK(cjk_it->second.get<std::string>() == "\xe9\x9b\xaa\xe8\x8a\xb1");
}

TEST_CASE("should handle JSON with unicode in keys", "[semi_structured]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query returning JSON with unicode characters in keys is executed
  auto stmt = conn.execute_fetch("SELECT PARSE_JSON('{\"\\u96EA\":\"snow\",\"\\u82B1\":\"flower\"}')");

  // Then Result should preserve unicode keys and their associated values
  auto json = parse_json_text(check_wchar_success(stmt, 1));
  REQUIRE(json.is<picojson::object>());
  const auto& obj = json.get<picojson::object>();

  auto snow_it = obj.find("\xe9\x9b\xaa");
  REQUIRE(snow_it != obj.end());
  CHECK(snow_it->second.get<std::string>() == "snow");

  auto flower_it = obj.find("\xe8\x8a\xb1");
  REQUIRE(flower_it != obj.end());
  CHECK(flower_it->second.get<std::string>() == "flower");
}

TEST_CASE("should handle JSON with unicode via SQL_C_WCHAR", "[semi_structured][conversion][wchar]") {
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

TEST_CASE("should truncate variant data as SQL_C_WCHAR when buffer is too short",
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

TEST_CASE("should report SQL_DESC_TYPE_NAME for semi-structured columns", "[semi_structured][metadata]") {
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

static picojson::value parse_json_text(const std::string& json_text) {
  picojson::value json;
  const auto error = picojson::parse(json, json_text);
  REQUIRE(error.empty());
  return json;
}

// picojson only accepts std::string (UTF-8). ODBC SQL_C_WCHAR returns UTF-16
// (char16_t on Linux/macOS), so we need manual conversion before parsing.
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
  const auto actual_canonical = actual_json.serialize();
  const auto expected_canonical = expected_json.serialize();
  REQUIRE(actual_canonical == expected_canonical);
}

static void check_json_equals(const std::u16string& actual_json_text, const std::string& expected_json_text) {
  check_json_equals(utf16_to_utf8(actual_json_text), expected_json_text);
}
