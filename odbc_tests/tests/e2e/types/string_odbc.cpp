// String datatype ODBC-specific tests
// Based on: tests/definitions/shared/types/string_odbc.feature

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <algorithm>
#include <cstring>
#include <optional>
#include <sstream>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "SchemaFixtures.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "odbc_matchers.hpp"
#include "test_setup.hpp"

// ============================================================================
// SIMPLE SELECTS - LITERALS using SQLBindCol
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should select hardcoded string literals using SQLBindCol",
                 "[datatype][string_odbc]") {
  // Given Snowflake client is logged in

  // When Query "SELECT 'hello' AS str1, 'Hello World' AS str2, 'Snowflake Driver Test' AS str3" is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(
      stmt.getHandle(), (SQLCHAR*)"SELECT 'hello' AS str1, 'Hello World' AS str2, 'Snowflake Driver Test' AS str3",
      SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And Columns are bound using SQLBindCol
  char buf1[100], buf2[100], buf3[100];
  SQLLEN ind1, ind2, ind3;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_CHAR, buf1, sizeof(buf1), &ind1);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindCol(stmt.getHandle(), 2, SQL_C_CHAR, buf2, sizeof(buf2), &ind2);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindCol(stmt.getHandle(), 3, SQL_C_CHAR, buf3, sizeof(buf3), &ind3);
  REQUIRE_ODBC(ret, stmt);

  // And SQLFetch is called
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the result should contain:
  CHECK(std::string(buf1, ind1) == "hello");
  CHECK(std::string(buf2, ind2) == "Hello World");
  CHECK(std::string(buf3, ind3) == "Snowflake Driver Test");
}

// ============================================================================
// UTF-16 TO ASCII CONVERSION
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should convert UTF-16 to ASCII with 0x1a substitution when using SQL_C_CHAR",
                 "[datatype][string_odbc][conversion]") {
  if (!is_ascii_locale()) {
    SKIP("0x1a substitution only applies on non-UTF-8 locales");
  }

  // Given Snowflake client is logged in

  // When Query selecting strings with non-ASCII Unicode characters is executed
  auto stmt = conn.executew_fetch(
      u"SELECT "
      u"'\u65e5\u672c\u8a9e' AS japanese, "
      u"'Hello\u65e5World' AS mixed, "
      u"'\u26c4\U0001F680\U0001F389' AS emojis, "
      u"'\u03b1\u03b2\u03b3\u03b4' AS greek, "
      u"'Hello' AS ascii_only, "
      u"'y\u0306es' AS combined, "
      u"'\U0001D11E' AS surrogate_pair");
  // Then Japanese characters should be replaced with 0x1a (SUB) when reading as SQL_C_CHAR
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "\x1a\x1a\x1a");

  // And pure ASCII string should remain unchanged
  CHECK(get_data<SQL_C_CHAR>(stmt, 5) == "Hello");

  // And mixed string should have ASCII preserved and non-ASCII replaced with 0x1a
  auto mixed = get_data<SQL_C_CHAR>(stmt, 2);
  CHECK(mixed == "Hello\x1aWorld");

  // And emojis should all be replaced with 0x1a
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "\x1a\x1a\x1a");

  // And Greek letters should be replaced with 0x1a
  CHECK(get_data<SQL_C_CHAR>(stmt, 4) == "\x1a\x1a\x1a\x1a");

  // And combined string should have ASCII preserved and non-ASCII replaced with 0x1a
  auto combined = get_data<SQL_C_CHAR>(stmt, 6);
  CHECK(combined ==
        "y\x1a"
        "es");

  // And surrogate pair should be replaced with 0x1a
  CHECK(get_data<SQL_C_CHAR>(stmt, 7) == "\x1a");
}

// ============================================================================
// MULTIPLE CHUNKS DOWNLOADING WITH SQLBindCol
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should download string data in multiple chunks using SQLBindCol",
                 "[datatype][string_odbc][large_result_set]") {
  // Given Snowflake client is logged in

  // And Expected row count is defined
  const int expected_row_count = 10000;

  // When Query "SELECT seq8() AS id, TO_VARCHAR(seq8()) AS str_val FROM TABLE(GENERATOR(ROWCOUNT => 10000)) v ORDER BY
  // 1" is executed
  auto stmt = conn.createStatement();
  const char* sql =
      "SELECT seq8() AS id, TO_VARCHAR(seq8()) AS str_val FROM TABLE(GENERATOR(ROWCOUNT => 10000)) v ORDER BY id";
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)sql, SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And Columns are bound using SQLBindCol
  SQLBIGINT id;
  SQLLEN id_indicator;
  char str_buffer[64];
  SQLLEN str_indicator;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_SBIGINT, &id, sizeof(id), &id_indicator);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindCol(stmt.getHandle(), 2, SQL_C_CHAR, str_buffer, sizeof(str_buffer), &str_indicator);
  REQUIRE_ODBC(ret, stmt);

  // Then there are 10000 rows returned and all string values should match the generated values in order
  int row_count = 0;
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) break;
    REQUIRE_ODBC(ret, stmt);

    // Verify id is not null
    REQUIRE(id_indicator != SQL_NULL_DATA);

    // Verify string value matches expected (id converted to string)
    REQUIRE(str_indicator != SQL_NULL_DATA);
    std::string str_value(str_buffer, str_indicator);
    CHECK(str_value == std::to_string(id));

    row_count++;
  }

  CHECK(row_count == expected_row_count);
}
