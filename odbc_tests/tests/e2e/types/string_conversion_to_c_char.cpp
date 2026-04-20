// String to ODBC character type conversions tests
// Tests converting Snowflake VARCHAR/STRING type to character ODBC C types:
// SQL_C_CHAR, SQL_C_WCHAR

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
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
// STRING TRUNCATION
// ============================================================================

// Byte length of data is longer than the buffer length, so the data is truncated.
TEST_CASE_METHOD(ConnSchemaFixture, "should truncate string data when byte length is longer than the buffer length",
                 "[datatype][string][conversion][char]") {
  // Given Snowflake client is logged in

  // When Query selecting a long string is executed
  auto stmt = conn.execute_fetch("SELECT 'This is a very long string that will be truncated' AS long_str");

  // And Attempt to get data with a buffer that is too short
  char buffer[20];  // Buffer smaller than the string
  SQLLEN indicator;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);

  // Then the function should return SQL_SUCCESS_WITH_INFO (truncation occurred)
  CHECK(ret == SQL_SUCCESS_WITH_INFO);

  // And the buffer should contain the truncated string with null terminator
  CHECK(strlen(buffer) == sizeof(buffer) - 1);  // 19 characters + null terminator
  CHECK(std::string(buffer) == "This is a very long");
  CHECK(buffer[sizeof(buffer) - 1] == 0);

  // And the indicator should show the actual length of the original string
  if (is_ascii_locale() || (get_platform() == PLATFORM::PLATFORM_WINDOWS)) {
    // TODO: We are not guaranteed to get length of string, due to charset conversion
    CHECK((indicator == SQL_NO_TOTAL || indicator == 49));
  } else {
    CHECK(indicator == 49);
  }
}

TEST_CASE_METHOD(ConnSchemaFixture,
                 "should truncate wide string data when byte length is longer than the buffer length",
                 "[datatype][string][conversion][wchar]") {
  // Given Snowflake client is logged in

  // When Query selecting a long string is executed
  auto stmt = conn.execute_fetch("SELECT 'This is a very long string that will be truncated' AS long_str");

  // And Attempt to get data with a buffer that is too short
  SQLWCHAR buffer[20];  // Buffer smaller than the string (20 wide chars = 40 bytes)
  SQLLEN indicator;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_WCHAR, buffer, sizeof(buffer), &indicator);

  // Then the function should return SQL_SUCCESS_WITH_INFO (truncation occurred)
  CHECK(ret == SQL_SUCCESS_WITH_INFO);

  std::u16string expected_truncated = u"This is a very long";
  CHECK(std::u16string((char16_t*)buffer, sizeof(buffer) / sizeof(char16_t) - 1) == expected_truncated);
  CHECK(buffer[sizeof(buffer) / sizeof(char16_t) - 1] == 0);

  // And the indicator should show the actual byte length of the original string in wide char format
  NEW_DRIVER_ONLY("BD#23") { CHECK(indicator == 98); }
  OLD_DRIVER_ONLY("BD#23") { CHECK((indicator == 98 || indicator == SQL_NO_TOTAL)); }
}

// ============================================================================
// UTF-16 TO ASCII CONVERSION
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should convert UTF-16 to ASCII with 0x1a substitution when using SQL_C_CHAR",
                 "[datatype][string][conversion]") {
  if (!is_ascii_locale()) {
    SKIP("This test is not applicable on non-ASCII locales");
  }
  // ODBC-specific: When reading UTF-16 data using SQL_C_CHAR target type,
  // on non-UTF-8 locales non-ASCII characters (> 0x7F) are replaced with 0x1a (SUB character),
  // on UTF-8 locales the characters are preserved as UTF-8.
  // Given Snowflake client is logged in

  // When Query selecting strings with non-ASCII Unicode characters is executed
  auto stmt = conn.executew_fetch(
      u"SELECT "
      u"'日本語' AS japanese, "
      u"'Hello日World' AS mixed, "
      u"'⛄🚀🎉' AS emojis, "
      u"'αβγδ' AS greek, "
      u"'Hello' AS ascii_only, "
      u"'y̆es' AS combined, "
      u"'𝄞' AS surrogate_pair");

  // And Pure ASCII string should remain unchanged
  auto ascii_only = get_data<SQL_C_CHAR>(stmt, 5);
  CHECK(ascii_only == "Hello");

  // Then Japanese characters should be replaced with 0x1a (SUB) when reading as SQL_C_CHAR
  auto japanese = get_data<SQL_C_CHAR>(stmt, 1);
  CHECK(japanese == "\x1a\x1a\x1a");

  // And Mixed string should have ASCII preserved and non-ASCII replaced with 0x1a
  auto mixed = get_data<SQL_C_CHAR>(stmt, 2);
  CHECK(mixed == "Hello\x1aWorld");
  // And Emojis should all be replaced with 0x1a
  auto emojis = get_data<SQL_C_CHAR>(stmt, 3);
  CHECK(emojis == "\x1a\x1a\x1a");

  // And Greek letters should be replaced with 0x1a
  auto greek = get_data<SQL_C_CHAR>(stmt, 4);
  CHECK(greek == "\x1a\x1a\x1a\x1a");

  // And Combined string should have ASCII preserved and non-ASCII replaced with 0x1a
  auto combined = get_data<SQL_C_CHAR>(stmt, 6);
  CHECK(combined ==
        "y\x1a"
        "es");

  auto surrogate_pair = get_data<SQL_C_CHAR>(stmt, 7);
  CHECK(surrogate_pair == "\x1a");
  // UTF-8 locale: non-ASCII characters preserved as UTF-8
}

// ============================================================================
// BASIC STRING QUERY AND PARAMETER BINDING
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "Test string basic query", "[e2e][types][string]") {
  // Given A Snowflake connection

  // When A string value is inserted and selected via SQL_C_CHAR
  conn.execute("CREATE TEMPORARY TABLE test_string_basic (str_col VARCHAR(1000))");
  conn.execute("INSERT INTO test_string_basic (str_col) VALUES ('Hello World')");
  auto stmt = conn.createStatement();

  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT str_col FROM test_string_basic", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  char buffer[1000];
  SQLLEN indicator;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  REQUIRE_ODBC(ret, stmt);

  // Then The retrieved string matches the inserted value
  REQUIRE(indicator > 0);
  REQUIRE(std::string(buffer, indicator) == "Hello World");
}

TEST_CASE_METHOD(ConnSchemaFixture, "Test basic string binding", "[e2e][types][string]") {
  // Given A Snowflake connection

  // When A string value is inserted via parameter binding and selected
  conn.execute("CREATE TEMPORARY TABLE test_string_basic_binding (str_col VARCHAR(1000))");
  auto stmt = conn.createStatement();

  SQLRETURN ret =
      SQLPrepare(stmt.getHandle(), (SQLCHAR*)"INSERT INTO test_string_basic_binding (str_col) VALUES (?)", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  const char* test_value = "Hello World";
  SQLLEN str_len = strlen(test_value);

  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, str_len, 0,
                         (SQLPOINTER)test_value, str_len, &str_len);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT str_col FROM test_string_basic_binding", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  char buffer[1000];
  SQLLEN indicator;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  REQUIRE_ODBC(ret, stmt);

  // Then The retrieved string matches the bound parameter value
  REQUIRE(indicator > 0);
  REQUIRE(std::string(buffer, indicator) == "Hello World");
}
