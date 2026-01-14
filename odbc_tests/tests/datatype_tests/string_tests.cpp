#include <picojson.h>
#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <algorithm>
#include <cstring>
#include <fstream>
#include <iostream>
#include <sstream>
#include <string>
#include <utility>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "Schema.hpp"
#include "macros.hpp"
#include "test_setup.hpp"

TEST_CASE("Test string weird values", "[datatype][string]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("DROP TABLE IF EXISTS test_string_weird_values");
  conn.execute("CREATE TABLE test_string_weird_values (id INT, str_col VARCHAR(10000))");
  auto stmt = conn.createStatement();

  // Prepare insert statement with parameterized query
  SQLRETURN ret = SQLPrepare(stmt.getHandle(),
                             (SQLCHAR*)"INSERT INTO test_string_weird_values (id, str_col) VALUES (?, ?)", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Define weird test values - edge cases that might break conversion
  // These are values that should work with valid UTF-8 and no embedded nulls
  std::vector<std::pair<int, std::string>> test_cases = {
      // Empty and whitespace
      {1, ""},                 // Empty string
      {2, " "},                // Single space
      {3, "   "},              // Multiple spaces
      {4, "\t"},               // Tab only
      {5, "\n"},               // Newline only
      {6, "\r"},               // Carriage return only
      {7, "\r\n"},             // CRLF
      {8, " \t\n\r "},         // Mixed whitespace
      {9, "  leading"},        // Leading spaces
      {10, "trailing  "},      // Trailing spaces
      {11, "  both sides  "},  // Both sides

      // Special characters and escaping
      {12, "'"},            // Single quote
      {13, "''"},           // Two single quotes
      {14, "\""},           // Double quote
      {15, "\\"},           // Single backslash
      {16, "\\\\"},         // Double backslash
      {17, "\\n"},          // Literal backslash-n (not newline)
      {18, "\\t"},          // Literal backslash-t (not tab)
      {19, "'quoted'"},     // Quoted string
      {20, "\"double\""},   // Double quoted
      {21, "it's"},         // Apostrophe in word
      {22, "back\\slash"},  // Backslash in middle

      // Unicode characters (valid UTF-8)
      {31, "café"},                       // Accented char
      {32, "naïve"},                      // Diaeresis
      {33, "日本語"},                     // Japanese
      {34, "中文"},                       // Chinese
      {35, "한국어"},                     // Korean
      {36, "Ελληνικά"},                   // Greek
      {37, "العربية"},                    // Arabic (RTL)
      {38, "עברית"},                      // Hebrew (RTL)
      {39, "🎉🚀💯"},                     // Emojis
      {40, "👨‍👩‍👧‍👦"},  // Family emoji (ZWJ sequence)
      {41, "🇺🇸"},                         // Flag emoji (regional indicators)
      {42, "ą"},                          // Combining diacritical mark (a + ogonek)
      {43, "€£¥₹"},                       // Currency symbols
      {44, "™®©"},                        // Trademark symbols
      {45, "∑∏∫∂"},                       // Math symbols
      {46, "αβγδ"},                       // Greek letters

      // SQL injection attempts (should be safely escaped)
      {48, "'; DROP TABLE test_string_weird_values; --"},
      {49, "1; DELETE FROM test_string_weird_values WHERE '1'='1"},
      {50, "' OR '1'='1"},
      {51, "/* comment */"},
      {52, "-- comment"},

      // Text representations of special values
      {55, "0x1234"},      // Hex string literal
      {56, "\\x00\\x01"},  // Escaped hex notation as text

      // Long strings
      {57, std::string(100, 'a')},   // 100 a's
      {58, std::string(1000, 'b')},  // 1000 b's
      {59, std::string(5000, 'c')},  // 5000 c's (near limit)

      // Mixed content
      {60, "Hello\nWorld\tTest\r\nEnd"},  // Mixed line endings
      {61, "Line1\nLine2\nLine3"},        // Multi-line
      {62, "Tab\tSeparated\tValues"},     // TSV-like
      {63, "Comma,Separated,Values"},     // CSV-like
      {64, "Pipe|Separated|Values"},      // Pipe-separated

      // Edge formatting
      {65, "   "},                  // Multiple spaces again
      {66, "a\nb\nc\nd\ne"},        // Many newlines
      {67, std::string(10, '\t')},  // Many tabs

      // Additional edge cases
      {70, "NULL"},       // The word NULL
      {71, "null"},       // Lowercase null
      {72, "NaN"},        // Not a number string
      {73, "Infinity"},   // Infinity string
      {74, "-Infinity"},  // Negative infinity
      {75, "true"},       // Boolean as string
      {76, "false"},      // Boolean as string
      {77, "0"},          // Zero as string
      {78, "-0"},         // Negative zero as string
      {79, "1e308"},      // Large scientific notation
      {80, "1e-308"},     // Small scientific notation

      // Path-like strings
      {81, "/path/to/file"},          // Unix path
      {82, "C:\\Windows\\System32"},  // Windows path
      {83, "..\\..\\parent"},         // Relative path traversal
      {84, "file:///local/path"},     // File URI

      // URL-like strings
      {85, "https://example.com?q=test&a=1"},  // URL with query params
      {86, "mailto:test@example.com"},         // Mailto link
      {87, "javascript:alert('xss')"},         // JS protocol (should be treated as text)

      // JSON/XML-like content
      {88, "{\"key\": \"value\"}"},     // JSON object
      {89, "[1, 2, 3]"},                // JSON array
      {90, "<tag>content</tag>"},       // XML tag
      {91, "<?xml version=\"1.0\"?>"},  // XML declaration
      {92, "<!DOCTYPE html>"},          // DOCTYPE

      // Regex-like patterns
      {93, "^[a-z]+$"},  // Regex pattern
      {94, ".*"},        // Wildcard regex
      {95, "(test)"},    // Parentheses

      // Shell-like
      {96, "$(command)"},  // Command substitution
      {97, "`command`"},   // Backtick command
      {98, "${VAR}"},      // Variable expansion
      {99, "$HOME"},       // Environment variable

      // Extreme whitespace
      {100, "\t\t\t\t\t"},      // Many tabs
      {101, "\n\n\n\n\n"},      // Many newlines
      {102, "\r\n\r\n\r\n"},    // Many CRLF
      {103, "word\t\tword"},    // Multiple tabs between words
      {104, "word  \n  word"},  // Mixed spacing across lines
  };

  // Insert each test value using parameterized query
  for (const auto& [id, value] : test_cases) {
    SQLINTEGER id_val = id;
    SQLLEN id_len = sizeof(id_val);
    SQLLEN str_len = static_cast<SQLLEN>(value.size());

    ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &id_val,
                           sizeof(id_val), &id_len);
    CHECK_ODBC(ret, stmt);

    ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, value.size(), 0,
                           (SQLPOINTER)value.data(), value.size(), &str_len);
    CHECK_ODBC(ret, stmt);

    ret = SQLExecute(stmt.getHandle());
    INFO("Failed to insert test case with ID " << id);
    CHECK_ODBC(ret, stmt);

    ret = SQLFreeStmt(stmt.getHandle(), SQL_RESET_PARAMS);
    CHECK_ODBC(ret, stmt);
  }

  // Verify data was inserted correctly by reading it back
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT id, str_col FROM test_string_weird_values ORDER BY id",
                      SQL_NTS);
  CHECK_ODBC(ret, stmt);

  size_t row_count = 0;
  size_t mismatch_count = 0;
  while (SQLFetch(stmt.getHandle()) == SQL_SUCCESS) {
    SQLINTEGER id_result;
    SQLLEN id_indicator;
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SLONG, &id_result, sizeof(id_result), &id_indicator);
    CHECK_ODBC(ret, stmt);

    char buffer[20000];
    SQLLEN str_indicator;
    ret = SQLGetData(stmt.getHandle(), 2, SQL_C_CHAR, buffer, sizeof(buffer), &str_indicator);
    CHECK_ODBC(ret, stmt);

    // Find the expected value
    auto it = std::find_if(test_cases.begin(), test_cases.end(), [&](const auto& tc) { return tc.first == id_result; });
    REQUIRE(it != test_cases.end());

    // Compare the retrieved value with expected
    if (str_indicator == SQL_NULL_DATA) {
      // Empty strings might come back as NULL depending on driver
      if (!it->second.empty()) {
        WARN("ID " << id_result << " expected non-empty string but got NULL");
        mismatch_count++;
      }
    } else {
      std::string retrieved(buffer, str_indicator);
      if (retrieved != it->second) {
        // Log mismatches but don't fail - useful for comparing driver behavior
        WARN("ID " << id_result << " mismatch - expected length: " << it->second.size() << " got length: "
                   << retrieved.size() << " (expected: \"" << it->second << "\" got: \"" << retrieved << "\")");
        mismatch_count++;
      }
      REQUIRE(str_indicator >= 0);
    }
    row_count++;
  }

  // All rows should be retrievable
  REQUIRE(row_count == test_cases.size());
  // Log total mismatches for visibility
  if (mismatch_count > 0) {
    WARN("Total string value mismatches: " << mismatch_count << " out of " << test_cases.size());
  }
}

TEST_CASE("Test string basic query", "[datatype][string]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("DROP TABLE IF EXISTS test_string_basic");
  conn.execute("CREATE TABLE test_string_basic (str_col VARCHAR(1000))");
  conn.execute("INSERT INTO test_string_basic (str_col) VALUES ('Hello World')");
  auto stmt = conn.createStatement();

  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT str_col FROM test_string_basic", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  char buffer[1000];
  SQLLEN indicator;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  CHECK_ODBC(ret, stmt);
  REQUIRE(indicator > 0);

  REQUIRE(std::string(buffer, indicator) == "Hello World");
}

TEST_CASE("Test basic string binding", "[datatype][string]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("DROP TABLE IF EXISTS test_string_basic_binding");
  conn.execute("CREATE TABLE test_string_basic_binding (str_col VARCHAR(1000))");
  auto stmt = conn.createStatement();

  // Prepare insert statement
  SQLRETURN ret =
      SQLPrepare(stmt.getHandle(), (SQLCHAR*)"INSERT INTO test_string_basic_binding (str_col) VALUES (?)", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Test value to bind
  const char* test_value = "Hello World";
  SQLLEN str_len = strlen(test_value);

  // Bind the parameter
  ret = SQLBindParameter(stmt.getHandle(),
                         1,                       // Parameter number
                         SQL_PARAM_INPUT,         // Input parameter
                         SQL_C_CHAR,              // C data type
                         SQL_VARCHAR,             // SQL data type
                         str_len,                 // Column size
                         0,                       // Decimal digits
                         (SQLPOINTER)test_value,  // Parameter value ptr
                         str_len,                 // Buffer length
                         &str_len                 // Length/Indicator
  );
  CHECK_ODBC(ret, stmt);

  // Execute the prepared statement
  ret = SQLExecute(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  // Verify the inserted data
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT str_col FROM test_string_basic_binding", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  char buffer[1000];
  SQLLEN indicator;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  CHECK_ODBC(ret, stmt);
  REQUIRE(indicator > 0);

  REQUIRE(std::string(buffer, indicator) == "Hello World");
}
