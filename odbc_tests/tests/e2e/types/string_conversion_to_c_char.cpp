// String to ODBC character/binary type conversions tests
// Tests converting Snowflake VARCHAR/STRING type to character/binary ODBC C types:
// SQL_C_BINARY, SQL_C_CHAR, SQL_C_WCHAR

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "odbc_matchers.hpp"
#include "test_setup.hpp"

static unsigned int to_unsigned_int(char c) { return static_cast<unsigned int>(static_cast<unsigned char>(c)); }

// ============================================================================
// STRING TRUNCATION
// ============================================================================

// Byte length of data is longer than the buffer length, so the data is truncated.
TEST_CASE("should truncate string data when byte length is longer than the buffer length",
          "[datatype][string][conversion][char]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

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

TEST_CASE("should truncate wide string data when byte length is longer than the buffer length",
          "[datatype][string][conversion][wchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

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
  NEW_DRIVER_ONLY("BD#25") { CHECK(indicator == 98); }
  OLD_DRIVER_ONLY("BD#25") { CHECK((indicator == 98 || indicator == SQL_NO_TOTAL)); }
}

// ============================================================================
// SUCCESSFUL CONVERSIONS - String to SQL_C_BINARY
// ============================================================================

TEST_CASE("should convert string literals to SQL_C_BINARY", "[datatype][string][conversion][binary]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting various string literals is executed
  auto stmt = conn.execute_fetch("SELECT 'hello' AS c1, '' AS c2, 'ABC123!@#' AS c3, NULL::STRING AS c4");

  // Then ASCII string 'hello' should convert to raw bytes
  {
    SQLCHAR buffer[100];
    SQLLEN indicator;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
    REQUIRE_ODBC(ret, stmt);
    CHECK(indicator == 5);
    CHECK(buffer[0] == 'h');
    CHECK(buffer[1] == 'e');
    CHECK(buffer[2] == 'l');
    CHECK(buffer[3] == 'l');
    CHECK(buffer[4] == 'o');
  }

  // And empty string should return 0 bytes
  {
    SQLCHAR buffer[100];
    SQLLEN indicator;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 2, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
    REQUIRE_ODBC(ret, stmt);
    CHECK(indicator == 0);
  }

  // And mixed ASCII with special characters should convert correctly
  {
    SQLCHAR buffer[100];
    SQLLEN indicator;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 3, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
    REQUIRE_ODBC(ret, stmt);
    CHECK(indicator == 9);
    CHECK(buffer[0] == 'A');
    CHECK(buffer[1] == 'B');
    CHECK(buffer[2] == 'C');
    CHECK(buffer[3] == '1');
    CHECK(buffer[4] == '2');
    CHECK(buffer[5] == '3');
    CHECK(buffer[6] == '!');
    CHECK(buffer[7] == '@');
    CHECK(buffer[8] == '#');
  }

  // And NULL should return SQL_NULL_DATA
  {
    SQLCHAR buffer[100];
    SQLLEN indicator;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 4, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
    REQUIRE_ODBC(ret, stmt);
    CHECK(indicator == SQL_NULL_DATA);
  }
}

// ============================================================================
// SUCCESSFUL CONVERSIONS - UTF-8 String to SQL_C_BINARY
// ============================================================================

TEST_CASE("should convert UTF-8 string literals to SQL_C_BINARY", "[datatype][string][conversion][binary][utf8]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting UTF-8 string literals is executed
  auto stmt = conn.executew_fetch(
      u"SELECT '日本語' AS japanese, 'Привет' AS russian, '你好' AS chinese, "
      u"'émoji: 😀' AS emoji, 'café' AS french, 'Ñoño' AS spanish, '𝄞' AS clef");

  // Then Japanese '日本語' should convert to raw bytes
  {
    SQLCHAR buffer[100];
    SQLLEN indicator;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
    REQUIRE_ODBC(ret, stmt);
    WINDOWS_ONLY {
      // Win-1252 double-encoding: 9 UTF-8 bytes → 19 bytes
      // [E6 97 A5 E6 9C AC E8 AA 9E] reinterpreted as Win-1252:
      // E6→æ(C3 A6), 97→—(E2 80 94), A5→¥(C2 A5), ...
      CHECK(indicator == 19);
      CHECK(buffer[0] == 0xC3);
      CHECK(buffer[1] == 0xA6);  // '日' first byte E6 → Win-1252 'æ'
      CHECK(buffer[2] == 0xE2);
      CHECK(buffer[3] == 0x80);
      CHECK(buffer[4] == 0x94);  // '日' second byte 97 → Win-1252 '—'
    }
    UNIX_ONLY {
      // Raw UTF-8: 3 CJK chars × 3 bytes each = 9 bytes [E6 97 A5 E6 9C AC E8 AA 9E]
      CHECK(indicator == 9);
      CHECK(buffer[0] == 0xE6);
      CHECK(buffer[1] == 0x97);
      CHECK(buffer[2] == 0xA5);  // '日'
      CHECK(buffer[3] == 0xE6);
      CHECK(buffer[4] == 0x9C);
      CHECK(buffer[5] == 0xAC);  // '本'
      CHECK(buffer[6] == 0xE8);
      CHECK(buffer[7] == 0xAA);
      CHECK(buffer[8] == 0x9E);  // '語'
    }
  }

  // And Russian 'Привет' should convert to raw bytes
  {
    SQLCHAR buffer[100];
    SQLLEN indicator;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 2, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
    REQUIRE_ODBC(ret, stmt);
    WINDOWS_ONLY {
      // Win-1252 double-encoding: 12 UTF-8 bytes → 26 bytes
      // Each 2-byte Cyrillic sequence reinterpreted as Win-1252 and re-encoded to UTF-8
      CHECK(indicator == 26);
      CHECK(buffer[0] == 0xC3);
      CHECK(buffer[1] == 0x90);  // 'П' first byte D0 → Win-1252 'Ð'
    }
    UNIX_ONLY {
      // Raw UTF-8: 6 Cyrillic chars × 2 bytes each = 12 bytes [D0 9F D1 80 ...]
      CHECK(indicator == 12);
      CHECK(buffer[0] == 0xD0);
      CHECK(buffer[1] == 0x9F);  // 'П'
    }
  }

  // And Chinese '你好' should convert to raw bytes
  {
    SQLCHAR buffer[100];
    SQLLEN indicator;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 3, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
    REQUIRE_ODBC(ret, stmt);
    WINDOWS_ONLY {
      // Win-1252 double-encoding: 6 UTF-8 bytes → 12 bytes
      // [E4 BD A0 E5 A5 BD] reinterpreted as Win-1252 and re-encoded
      CHECK(indicator == 12);
      CHECK(buffer[0] == 0xC3);
      CHECK(buffer[1] == 0xA4);  // E4 → Win-1252 'ä'
      CHECK(buffer[2] == 0xC2);
      CHECK(buffer[3] == 0xBD);  // BD → Win-1252 '½'
      CHECK(buffer[4] == 0xC2);
      CHECK(buffer[5] == 0xA0);  // A0 → Win-1252 NBSP
      CHECK(buffer[6] == 0xC3);
      CHECK(buffer[7] == 0xA5);  // E5 → Win-1252 'å'
      CHECK(buffer[8] == 0xC2);
      CHECK(buffer[9] == 0xA5);  // A5 → Win-1252 '¥'
      CHECK(buffer[10] == 0xC2);
      CHECK(buffer[11] == 0xBD);  // BD → Win-1252 '½'
    }
    UNIX_ONLY {
      // Raw UTF-8: 2 CJK chars × 3 bytes each = 6 bytes [E4 BD A0 E5 A5 BD]
      CHECK(indicator == 6);
      CHECK(buffer[0] == 0xE4);
      CHECK(buffer[1] == 0xBD);
      CHECK(buffer[2] == 0xA0);  // '你'
      CHECK(buffer[3] == 0xE5);
      CHECK(buffer[4] == 0xA5);
      CHECK(buffer[5] == 0xBD);  // '好'
    }
  }

  // And emoji string 'émoji: 😀' should include multi-byte emoji
  {
    SQLCHAR buffer[100];
    SQLLEN indicator;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 4, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
    REQUIRE_ODBC(ret, stmt);
    WINDOWS_ONLY {
      // Win-1252 double-encoding: 12 UTF-8 bytes → 19 bytes
      // é [C3 A9] → [C3 83 C2 A9], ASCII 'moji: ' kept as-is,
      // 😀 [F0 9F 98 80] → [C3 B0 C5 B8 CB 9C E2 82 AC]
      CHECK(indicator == 19);
      CHECK(buffer[0] == 0xC3);
      CHECK(buffer[1] == 0x83);  // C3 → Win-1252 'Ã'
      CHECK(buffer[2] == 0xC2);
      CHECK(buffer[3] == 0xA9);  // A9 → Win-1252 '©'
      CHECK(buffer[4] == 'm');
      CHECK(buffer[9] == 0x20);  // space before emoji
      CHECK(buffer[16] == 0xE2);
      CHECK(buffer[17] == 0x82);
      CHECK(buffer[18] == 0xAC);  // last byte 80 → Win-1252 '€' (E2 82 AC)
    }
    UNIX_ONLY {
      // Raw UTF-8: 'é' (2 bytes) + 'moji: ' (6 bytes) + '😀' (4 bytes) = 12 bytes
      CHECK(indicator == 12);
      CHECK(buffer[0] == 0xC3);
      CHECK(buffer[1] == 0xA9);  // 'é'
      CHECK(buffer[8] == 0xF0);
      CHECK(buffer[9] == 0x9F);
      CHECK(buffer[10] == 0x98);
      CHECK(buffer[11] == 0x80);  // '😀'
    }
  }

  // And French 'café' should convert correctly
  {
    SQLCHAR buffer[100];
    SQLLEN indicator;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 5, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
    REQUIRE_ODBC(ret, stmt);
    WINDOWS_ONLY {
      // Win-1252 double-encoding: 5 UTF-8 bytes → 7 bytes
      // ASCII 'caf' kept, é [C3 A9] → [C3 83 C2 A9]
      CHECK(indicator == 7);
      CHECK(buffer[0] == 'c');
      CHECK(buffer[1] == 'a');
      CHECK(buffer[2] == 'f');
      CHECK(buffer[3] == 0xC3);
      CHECK(buffer[4] == 0x83);  // C3 → Win-1252 'Ã'
      CHECK(buffer[5] == 0xC2);
      CHECK(buffer[6] == 0xA9);  // A9 → Win-1252 '©'
    }
    UNIX_ONLY {
      // Raw UTF-8: 'c' + 'a' + 'f' + 'é' (2 bytes) = 5 bytes [63 61 66 C3 A9]
      CHECK(indicator == 5);
      CHECK(buffer[0] == 'c');
      CHECK(buffer[1] == 'a');
      CHECK(buffer[2] == 'f');
      CHECK(buffer[3] == 0xC3);
      CHECK(buffer[4] == 0xA9);  // 'é'
    }
  }

  // And Spanish 'Ñoño' should convert correctly
  {
    SQLCHAR buffer[100];
    SQLLEN indicator;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 6, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
    REQUIRE_ODBC(ret, stmt);
    WINDOWS_ONLY {
      // Win-1252 double-encoding: 6 UTF-8 bytes → 11 bytes
      // Ñ [C3 91] → [C3 83 E2 80 98], ñ [C3 B1] → [C3 83 C2 B1]
      CHECK(indicator == 11);
      CHECK(buffer[0] == 0xC3);
      CHECK(buffer[1] == 0x83);  // C3 → Win-1252 'Ã'
      CHECK(buffer[2] == 0xE2);
      CHECK(buffer[3] == 0x80);
      CHECK(buffer[4] == 0x98);  // 91 → Win-1252 ''' (U+2018)
      CHECK(buffer[5] == 'o');
      CHECK(buffer[6] == 0xC3);
      CHECK(buffer[7] == 0x83);
      CHECK(buffer[8] == 0xC2);
      CHECK(buffer[9] == 0xB1);  // B1 → Win-1252 '±'
      CHECK(buffer[10] == 'o');
    }
    UNIX_ONLY {
      // Raw UTF-8: 'Ñ' (2 bytes) + 'o' + 'ñ' (2 bytes) + 'o' = 6 bytes [C3 91 6F C3 B1 6F]
      CHECK(indicator == 6);
      CHECK(buffer[0] == 0xC3);
      CHECK(buffer[1] == 0x91);  // 'Ñ'
      CHECK(buffer[2] == 'o');
      CHECK(buffer[3] == 0xC3);
      CHECK(buffer[4] == 0xB1);  // 'ñ'
      CHECK(buffer[5] == 'o');
    }
  }

  // And musical symbol '𝄞' should convert correctly
  {
    SQLCHAR buffer[100];
    SQLLEN indicator;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 7, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
    REQUIRE_ODBC(ret, stmt);
    WINDOWS_ONLY {
      // Win-1252 double-encoding: 4 UTF-8 bytes → 9 bytes
      // U+1D11E [F0 9D 84 9E] → [C3 B0 C2 9D E2 80 9E C5 BE]
      CHECK(indicator == 9);
      CHECK(to_unsigned_int(buffer[0]) == 0xC3);
      CHECK(to_unsigned_int(buffer[1]) == 0xB0);  // F0 → Win-1252 'ð'
      CHECK(to_unsigned_int(buffer[2]) == 0xC2);
      CHECK(to_unsigned_int(buffer[3]) == 0x9D);  // 9D → Win-1252 U+009D
      CHECK(to_unsigned_int(buffer[4]) == 0xE2);
      CHECK(to_unsigned_int(buffer[5]) == 0x80);
      CHECK(to_unsigned_int(buffer[6]) == 0x9E);  // 84 → Win-1252 '„' (U+201E)
      CHECK(to_unsigned_int(buffer[7]) == 0xC5);
      CHECK(to_unsigned_int(buffer[8]) == 0xBE);  // 9E → Win-1252 'ž' (U+017E)
    }
    UNIX_ONLY {
      // Raw UTF-8: U+1D11E = 4 bytes [F0 9D 84 9E]
      CHECK(indicator == 4);
      CHECK(to_unsigned_int(buffer[0]) == 0xF0);
      CHECK(to_unsigned_int(buffer[1]) == 0x9D);
      CHECK(to_unsigned_int(buffer[2]) == 0x84);
      CHECK(to_unsigned_int(buffer[3]) == 0x9E);
    }
  }
}

// ============================================================================
// UTF-16 TO ASCII CONVERSION
// ============================================================================

TEST_CASE("should convert UTF-16 to ASCII with 0x1a substitution when using SQL_C_CHAR",
          "[datatype][string][conversion]") {
  if (!is_ascii_locale()) {
    SKIP("This test is not applicable on non-ASCII locales");
  }
  // ODBC-specific: When reading UTF-16 data using SQL_C_CHAR target type,
  // on non-UTF-8 locales non-ASCII characters (> 0x7F) are replaced with 0x1a (SUB character),
  // on UTF-8 locales the characters are preserved as UTF-8.
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

TEST_CASE("Test string basic query", "[e2e][types][string]") {
  // Given A Snowflake connection
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When A string value is inserted and selected via SQL_C_CHAR
  conn.execute("DROP TABLE IF EXISTS test_string_basic");
  conn.execute("CREATE TABLE test_string_basic (str_col VARCHAR(1000))");
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

TEST_CASE("Test basic string binding", "[e2e][types][string]") {
  // Given A Snowflake connection
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When A string value is inserted via parameter binding and selected
  conn.execute("DROP TABLE IF EXISTS test_string_basic_binding");
  conn.execute("CREATE TABLE test_string_basic_binding (str_col VARCHAR(1000))");
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
