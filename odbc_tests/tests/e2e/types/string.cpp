// String datatype ODBC tests
// Based on: tests/definitions/shared/types/string.feature
//
// Snowflake String types: VARCHAR, CHAR, CHARACTER, NCHAR, STRING, TEXT, VARCHAR2, NVARCHAR,
// NVARCHAR2, CHAR VARYING, NCHAR VARYING All are synonymous with VARCHAR and store Unicode UTF-8
// characters. Maximum length: 134,217,728 characters (default 16,777,216 if unspecified) Maximum
// storage: 128 MB (134,217,728 bytes) Reference: https://docs.snowflake.com/en/sql-reference/data-types-text

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
#include "Schema.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "macros.hpp"
#include "test_setup.hpp"

// ============================================================================
// SIMPLE SELECTS - LITERALS (Happy path, Corner cases)
// ============================================================================

TEST_CASE("should select hardcoded string literals", "[datatype][string]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query "SELECT 'hello' AS str1, 'Hello World' AS str2, 'Snowflake Driver Test' AS str3"
  // is executed
  auto stmt = conn.execute_fetch("SELECT 'hello' AS str1, 'Hello World' AS str2, 'Snowflake Driver Test' AS str3");

  // Then the result should contain:
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "hello");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "Hello World");
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "Snowflake Driver Test");
}

TEST_CASE("should select hardcoded string literals using SQLBindCol", "[datatype][string]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query "SELECT 'hello' AS str1, 'Hello World' AS str2, 'Snowflake Driver Test' AS str3" is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(
      stmt.getHandle(), (SQLCHAR*)"SELECT 'hello' AS str1, 'Hello World' AS str2, 'Snowflake Driver Test' AS str3",
      SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And Columns are bound using SQLBindCol
  char buf1[100], buf2[100], buf3[100];
  SQLLEN ind1, ind2, ind3;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_CHAR, buf1, sizeof(buf1), &ind1);
  CHECK_ODBC(ret, stmt);
  ret = SQLBindCol(stmt.getHandle(), 2, SQL_C_CHAR, buf2, sizeof(buf2), &ind2);
  CHECK_ODBC(ret, stmt);
  ret = SQLBindCol(stmt.getHandle(), 3, SQL_C_CHAR, buf3, sizeof(buf3), &ind3);
  CHECK_ODBC(ret, stmt);

  // And SQLFetch is called
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  // Then the result should contain:
  CHECK(std::string(buf1, ind1) == "hello");
  CHECK(std::string(buf2, ind2) == "Hello World");
  CHECK(std::string(buf3, ind3) == "Snowflake Driver Test");
}

TEST_CASE("should select string literals with corner case values", "[datatype][string]") {
  SKIP_WINDOWS_STRING_ENCODING();
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting corner case string literals is executed
  auto stmt = conn.executew_fetch(
      u"SELECT "
      u"'' AS empty_str, "
      u"'X' AS single_char, "
      u"'   ' AS whitespace, "
      u"'\\t' AS tab_literal, "
      u"'\\n' AS newline_literal, "
      u"'⛄' AS unicode_snowman, "
      u"'日本語テスト' AS japanese, "
      u"'''' AS escaped_quote, "
      u"'\\\\' AS escaped_backslash, "
      u"NULL AS null_value, "
      u"'y̆es' AS combined, "
      u"'𝄞' AS surrogate_pair");

  // Then the result should contain expected corner case string values
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "X");
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "   ");
  CHECK(get_data<SQL_C_CHAR>(stmt, 4) == "\t");
  CHECK(get_data<SQL_C_CHAR>(stmt, 5) == "\n");
  CHECK(get_data<SQL_C_WCHAR>(stmt, 6) == u"⛄");
  CHECK(get_data<SQL_C_WCHAR>(stmt, 7) == u"日本語テスト");
  CHECK(get_data<SQL_C_CHAR>(stmt, 8) == "'");
  CHECK(get_data<SQL_C_CHAR>(stmt, 9) == "\\");
  CHECK(get_data_optional<SQL_C_CHAR>(stmt, 10) == std::nullopt);
  CHECK(get_data<SQL_C_WCHAR>(stmt, 11) == u"y̆es");
  CHECK(get_data<SQL_C_WCHAR>(stmt, 12) == u"𝄞");
}

// ============================================================================
// SIMPLE SELECTS - FROM TABLE (Happy path, Corner cases)
// ============================================================================

TEST_CASE("should select hardcoded string values from table", "[datatype][string]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And A temporary table with VARCHAR column is created
  conn.execute("DROP TABLE IF EXISTS test_string");
  conn.execute("CREATE TABLE test_string (id INT, val VARCHAR(1000))");

  // And The table is populated with string values
  conn.execute("INSERT INTO test_string VALUES (1, 'hello')");
  conn.execute("INSERT INTO test_string VALUES (2, 'Hello World')");
  conn.execute("INSERT INTO test_string VALUES (3, 'Snowflake Driver Test')");

  // When Query "SELECT * FROM {table}" is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT val FROM test_string ORDER BY id", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then the result should contain the inserted hardcoded string values
  std::vector<std::string> expected = {"hello", "Hello World", "Snowflake Driver Test"};
  int row = 0;
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) break;
    CHECK_ODBC(ret, stmt);
    std::string actual = get_data<SQL_C_CHAR>(stmt, 1);
    INFO("Row " << row << " expected: " << expected[row] << " actual: " << actual);
    CHECK(actual == expected[row]);
    row++;
  }
  CHECK(row == 3);
}

TEST_CASE("should select corner case string values from table", "[datatype][string]") {
  SKIP_WINDOWS_STRING_ENCODING();
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And A temporary table with VARCHAR column is created
  conn.execute("DROP TABLE IF EXISTS test_string");
  conn.execute("CREATE TABLE test_string (id INT, val VARCHAR(10000))");

  // And The table is populated with corner case string values
  conn.execute("INSERT INTO test_string VALUES (1, '')");                // empty string
  conn.execute("INSERT INTO test_string VALUES (2, 'X')");               // single char
  conn.execute("INSERT INTO test_string VALUES (3, '   ')");             // whitespace
  conn.execute("INSERT INTO test_string VALUES (4, '\\t')");             // tab character
  conn.execute("INSERT INTO test_string VALUES (5, '\\n')");             // newline character
  conn.executew(u"INSERT INTO test_string VALUES (6, '⛄')");            // unicode snowman
  conn.executew(u"INSERT INTO test_string VALUES (7, '日本語テスト')");  // Japanese
  conn.execute("INSERT INTO test_string VALUES (8, '''')");              // escaped quote
  conn.execute("INSERT INTO test_string VALUES (9, '\\\\')");            // escaped backslash
  conn.execute("INSERT INTO test_string VALUES (10, NULL)");             // NULL
  conn.executew(u"INSERT INTO test_string VALUES (11, 'y̆es')");  // combined character (y + combining breve + es)
  conn.executew(u"INSERT INTO test_string VALUES (12, '𝄞')");    // surrogate pair (musical G clef)

  // When Query "SELECT * FROM {table}" is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT val FROM test_string ORDER BY id", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then the result should contain the inserted corner case string values
  std::vector<std::optional<std::u16string>> expected = {
      u"",              // empty string
      u"X",             // single char
      u"   ",           // whitespace
      u"\t",            // tab character
      u"\n",            // newline character
      u"⛄",            // unicode snowman
      u"日本語テスト",  // Japanese
      u"'",             // escaped quote
      u"\\",            // escaped backslash
      std::nullopt,     // NULL
      u"y̆es",           // combined character
      u"𝄞"              // surrogate pair
  };

  int row = 0;
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) break;
    CHECK_ODBC(ret, stmt);

    auto value = get_data_optional<SQL_C_WCHAR>(stmt, 1);
    INFO("Row " << row);
    CHECK(value == expected[row]);
    row++;
  }
  CHECK(row == 12);
}

// ============================================================================
// SIMPLE INSERT WITH BINDING
// ============================================================================

TEST_CASE("should insert and select back hardcoded string values using parameter binding", "[datatype][string]") {
  SKIP_WINDOWS_STRING_ENCODING();
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And A temporary table with VARCHAR column is created
  conn.execute("DROP TABLE IF EXISTS test_string");
  conn.execute("CREATE TABLE test_string (id INT, val VARCHAR(10000))");

  // When String value 'Test binding value 日本語' is inserted using parameter binding
  {
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"INSERT INTO test_string (id, val) VALUES (1, ?)", SQL_NTS);
    CHECK_ODBC(ret, stmt);

    std::string value = "Test binding value 日本語";
    SQLLEN value_len = value.size();
    ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, value.size(), 0,
                           (SQLCHAR*)value.c_str(), value.size(), &value_len);
    CHECK_ODBC(ret, stmt);

    ret = SQLExecute(stmt.getHandle());
    CHECK_ODBC(ret, stmt);
  }

  // And Query "SELECT * FROM {table}" is executed
  auto stmt = conn.execute_fetch("SELECT val FROM test_string");

  // Then the result should contain the bound string value 'Test binding value 日本語'
  CHECK(get_data<SQL_C_WCHAR>(stmt, 1) == u"Test binding value 日本語");
}

// ============================================================================
// SELECT BINDING TESTS
// ============================================================================

TEST_CASE("should select string literals using parameter binding", "[datatype][string]") {
  SKIP_WINDOWS_STRING_ENCODING();
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"SELECT ?::VARCHAR, ?::VARCHAR, ?::VARCHAR", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  std::string value1 = "hello";
  std::string value2 = "Hello World";
  std::string value3 = "日本語テスト";  // Japanese text (UTF-8)

  SQLLEN len1 = value1.size();
  SQLLEN len2 = value2.size();
  SQLLEN len3 = value3.size();

  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, value1.size(), 0,
                         (SQLCHAR*)value1.c_str(), value1.size(), &len1);
  CHECK_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, value2.size(), 0,
                         (SQLCHAR*)value2.c_str(), value2.size(), &len2);
  CHECK_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 3, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, value3.size(), 0,
                         (SQLCHAR*)value3.c_str(), value3.size(), &len3);
  CHECK_ODBC(ret, stmt);
  // When Query "SELECT ?::VARCHAR, ?::VARCHAR, ?::VARCHAR" is executed with bound string values ['hello', 'Hello
  // World', '日本語テスト']
  ret = SQLExecute(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  // Then the result should contain:
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "hello");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "Hello World");
  CHECK(get_data<SQL_C_WCHAR>(stmt, 3) == u"日本語テスト");
}

// Skipped since null handling is not yet implemented for bindings
TEST_CASE("should select corner case string values using parameter binding", "[.][datatype][string]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query "SELECT ?::VARCHAR" is executed with each corner case string value bound
  auto test_bound_value = [&](const std::string& value, const std::string& expected) {
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"SELECT ?::VARCHAR", SQL_NTS);
    CHECK_ODBC(ret, stmt);

    SQLLEN len = value.size();
    ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR,
                           value.size() > 0 ? value.size() : 1, 0, (SQLCHAR*)value.c_str(), value.size(), &len);
    CHECK_ODBC(ret, stmt);

    ret = SQLExecute(stmt.getHandle());
    CHECK_ODBC(ret, stmt);

    ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);

    INFO("Testing value: '" << value << "'");
    CHECK(get_data<SQL_C_CHAR>(stmt, 1) == expected);
  };

  // Helper lambda to test a single bound wide value
  auto test_bound_wvalue = [&](const std::u16string& value, const std::u16string& expected) {
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"SELECT ?::VARCHAR", SQL_NTS);
    CHECK_ODBC(ret, stmt);

    SQLLEN len = value.size() * sizeof(char16_t);
    ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_WCHAR, SQL_WVARCHAR,
                           value.size() > 0 ? value.size() : 1, 0, (SQLWCHAR*)value.c_str(), len, &len);
    CHECK_ODBC(ret, stmt);

    ret = SQLExecute(stmt.getHandle());
    CHECK_ODBC(ret, stmt);

    ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);

    CHECK(get_data<SQL_C_WCHAR>(stmt, 1) == expected);
  };

  // Then the result should match the bound corner case value
  test_bound_value("", "");

  // Test single character
  test_bound_value("X", "X");

  // Test whitespace only
  test_bound_value("   ", "   ");

  // Test tab character
  test_bound_value("\t", "\t");

  // Test newline character
  test_bound_value("\n", "\n");

  // Test escaped single quote (just a quote character)
  test_bound_value("'", "'");

  // Test escaped backslash
  test_bound_value("\\", "\\");

  // Test unicode snowman (using wide binding)
  test_bound_wvalue(u"⛄", u"⛄");

  // Test Japanese characters (using wide binding)
  test_bound_wvalue(u"日本語テスト", u"日本語テスト");

  // Test combined character (using wide binding)
  test_bound_wvalue(u"y̆es", u"y̆es");

  // Test surrogate pair (using wide binding)
  test_bound_wvalue(u"𝄞", u"𝄞");

  // Test NULL value
  {
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"SELECT ?::VARCHAR", SQL_NTS);
    CHECK_ODBC(ret, stmt);

    SQLLEN null_indicator = SQL_NULL_DATA;
    ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 1, 0, nullptr, 0,
                           &null_indicator);
    CHECK_ODBC(ret, stmt);

    ret = SQLExecute(stmt.getHandle());
    CHECK_ODBC(ret, stmt);

    ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);

    CHECK(get_data_optional<SQL_C_CHAR>(stmt, 1) == std::nullopt);
  }
}

// ============================================================================
// MULTIPLE CHUNKS DOWNLOADING
// ============================================================================

TEST_CASE("should download string data in multiple chunks", "[datatype][string][large_result_set]") {
  // This test ensures proper handling of large result sets that span multiple chunks
  // ~10^6 values ensures data is downloaded in at least two chunks
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  const int num_values = 10000;

  // When Query "SELECT seq8() AS id, TO_VARCHAR(seq8()) AS str_val FROM TABLE(GENERATOR(ROWCOUNT => 10000)) v ORDER BY
  // id" is executed
  auto stmt = conn.createStatement();
  const char* sql =
      "SELECT seq8() AS id, TO_VARCHAR(seq8()) AS str_val FROM TABLE(GENERATOR(ROWCOUNT => 10000)) v ORDER BY id";
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)sql, SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then there are 10000 rows returned and all string values should match the generated values in order
  int row_count = 0;
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) break;
    CHECK_ODBC(ret, stmt);

    // Verify we can read the string value
    auto value = get_data<SQL_C_CHAR>(stmt, 1);
    CHECK(value == std::to_string(row_count));
    row_count++;
  }

  CHECK(row_count == num_values);
}

// ============================================================================
// UTF-16 TO ASCII CONVERSION
// ============================================================================

TEST_CASE("should convert UTF-16 to ASCII with 0x1a substitution when using SQL_C_CHAR",
          "[.][datatype][string][conversion]") {
  // ODBC-specific: When reading UTF-16 data using SQL_C_CHAR target type,
  // non-ASCII characters (> 0x7F) should be replaced with 0x1a (SUB character)
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

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

  // Then Japanese characters should be replaced with 0x1a (SUB) when reading as SQL_C_CHAR
  auto japanese = get_data<SQL_C_CHAR>(stmt, 1);
  CHECK(japanese == "\x1a\x1a\x1a");

  // And Mixed string should have ASCII preserved and non-ASCII replaced with 0x1a
  auto mixed = get_data<SQL_C_CHAR>(stmt, 2);
  CHECK(mixed.substr(0, 5) == "Hello");
  CHECK(mixed[5] == '\x1a');
  CHECK(mixed.substr(mixed.size() - 5) == "World");

  // And Emojis should all be replaced with 0x1a
  auto emojis = get_data<SQL_C_CHAR>(stmt, 3);
  for (char c : emojis) {
    CHECK(c == '\x1a');
  }

  // And Greek letters should be replaced with 0x1a
  auto greek = get_data<SQL_C_CHAR>(stmt, 4);
  for (char c : greek) {
    CHECK(c == '\x1a');
  }

  // And Pure ASCII string should remain unchanged
  auto ascii_only = get_data<SQL_C_CHAR>(stmt, 5);
  CHECK(ascii_only == "Hello");

  // And Combined string should have ASCII preserved and non-ASCII replaced with 0x1a
  auto combined = get_data<SQL_C_CHAR>(stmt, 6);
  CHECK(combined.substr(0, 1) == "y");
  CHECK(combined[1] == '\x1a');
  CHECK(combined[2] == 'e');
  CHECK(combined[3] == 's');

  auto surrogate_pair = get_data<SQL_C_CHAR>(stmt, 7);
  CHECK(surrogate_pair == "\x1a");
}

// ============================================================================
// MULTIPLE CHUNKS DOWNLOADING WITH SQLBindCol
// ============================================================================

TEST_CASE("should download string data in multiple chunks using SQLBindCol", "[datatype][string][large_result_set]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Expected row count is defined
  const int expected_row_count = 10000;

  // When Query "SELECT seq8() AS id, TO_VARCHAR(seq8()) AS str_val FROM TABLE(GENERATOR(ROWCOUNT => 10000)) v ORDER BY
  // 1" is executed
  auto stmt = conn.createStatement();
  const char* sql =
      "SELECT seq8() AS id, TO_VARCHAR(seq8()) AS str_val FROM TABLE(GENERATOR(ROWCOUNT => 10000)) v ORDER BY id";
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)sql, SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And Columns are bound using SQLBindCol
  SQLBIGINT id;
  SQLLEN id_indicator;
  char str_buffer[64];
  SQLLEN str_indicator;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_SBIGINT, &id, sizeof(id), &id_indicator);
  CHECK_ODBC(ret, stmt);
  ret = SQLBindCol(stmt.getHandle(), 2, SQL_C_CHAR, str_buffer, sizeof(str_buffer), &str_indicator);
  CHECK_ODBC(ret, stmt);

  // Then there are 10000 rows returned and all string values should match the generated values in order
  int row_count = 0;
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) break;
    CHECK_ODBC(ret, stmt);

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
