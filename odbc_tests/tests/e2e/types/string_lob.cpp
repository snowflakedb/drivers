// String LOB (Large Object) datatype ODBC tests
// Based on: tests/definitions/shared/types/string_lob.feature
//
// Snowflake LOB feature supports large VARCHAR values:
//   - Historical limit: 16 MB (16,777,216 bytes) per value
//   - Increased LOB Size feature: up to 128 MB (134,217,728 bytes) per value

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <chrono>
#include <iostream>
#include <random>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "SchemaFixtures.hpp"
#include "odbc_matchers.hpp"
#include "test_setup.hpp"

// Helper to generate random ASCII string for LOB tests
static std::string generate_random_ascii_string(std::mt19937& gen, const size_t length) {
  static constexpr char charset[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
  std::uniform_int_distribution<> dist(0, sizeof(charset) - 2);
  std::string result;
  result.reserve(length);
  for (size_t i = 0; i < length; ++i) {
    result += charset[dist(gen)];
  }
  return result;
}

// ============================================================================
// LOB (LARGE OBJECT) STRING HANDLING
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should handle LOB string at maximum 128 MB limit with increased LOB size",
                 "[datatype][string][lob]") {
  // Corner case: string at maximum LOB limit (128 MB) - requires Increased LOB Size feature
  // Given Snowflake client is logged in

  // And A random seed is initialized and logged
  auto seed = static_cast<unsigned int>(std::chrono::steady_clock::now().time_since_epoch().count());
  INFO("Random seed: " << seed);
  std::mt19937 gen(seed);

  // And A temporary table with VARCHAR column is created
  conn.execute("CREATE OR REPLACE TABLE lob_128mb (val VARCHAR(134217728))");

  // When A string of 134217728 ASCII characters is generated and inserted
  constexpr size_t string_length = 134217728;  // 128 MB
  std::string lob_string = generate_random_ascii_string(gen, string_length);

  {
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO lob_128mb VALUES (?)"), SQL_NTS);
    REQUIRE_ODBC(ret, stmt);

    SQLLEN value_len = static_cast<SQLLEN>(lob_string.size());
    ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, lob_string.size(), 0,
                           sqlchar(lob_string.c_str()), lob_string.size(), &value_len);
    REQUIRE_ODBC(ret, stmt);
    ret = SQLExecute(stmt.getHandle());
    REQUIRE_ODBC(ret, stmt);
  }

  // And Query "SELECT val, LENGTH(val) as len FROM {table}" is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT val, LENGTH(val) as len FROM lob_128mb"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  // Use SQLBindCol to fetch the large string
  std::vector<char> buffer(string_length + 1);
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_CHAR, buffer.data(), buffer.size(), &indicator);
  REQUIRE_ODBC(ret, stmt);
  SQLBIGINT len = 0;
  SQLLEN len_indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 2, SQL_C_SBIGINT, &len, sizeof(len), &len_indicator);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should show length 134217728
  CHECK(len == static_cast<SQLBIGINT>(string_length));
  REQUIRE(indicator == buffer.size() - 1);
  // And the returned string should exactly match the generated string
  std::string retrieved_value(buffer.data(), indicator);
  CHECK(retrieved_value == lob_string);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should handle LOB string at historical 16 MB limit", "[datatype][string][lob]") {
  // Corner case: string at the historical LOB limit (16 MB = 16,777,216 bytes)
  // Given Snowflake client is logged in

  // And A random seed is initialized and logged
  auto seed = static_cast<unsigned int>(std::chrono::steady_clock::now().time_since_epoch().count());
  std::cout << "Random seed: " << seed << std::endl;
  INFO("Random seed: " << seed);
  std::mt19937 gen(seed);

  // And A temporary table with VARCHAR column is created
  conn.execute("CREATE OR REPLACE TABLE lob_16mb (val VARCHAR)");

  // When A string of 16777216 ASCII characters is generated and inserted
  constexpr size_t string_length = 16777216;  // 16 MB
  std::string lob_string = generate_random_ascii_string(gen, string_length);

  {
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO lob_16mb VALUES (?)"), SQL_NTS);
    REQUIRE_ODBC(ret, stmt);

    SQLLEN value_len = static_cast<SQLLEN>(lob_string.size());
    ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, lob_string.size(), 0,
                           sqlchar(lob_string.c_str()), lob_string.size(), &value_len);
    REQUIRE_ODBC(ret, stmt);

    ret = SQLExecute(stmt.getHandle());
    REQUIRE_ODBC(ret, stmt);
  }

  // And Query "SELECT val, LENGTH(val) as len FROM {table}" is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT val, LENGTH(val) as len FROM lob_16mb"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Use SQLBindCol to fetch the large string
  std::vector<char> buffer(string_length + 1);
  SQLLEN indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_CHAR, buffer.data(), buffer.size(), &indicator);
  REQUIRE_ODBC(ret, stmt);

  SQLBIGINT len = 0;
  SQLLEN len_indicator = 0;
  ret = SQLBindCol(stmt.getHandle(), 2, SQL_C_SBIGINT, &len, sizeof(len), &len_indicator);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the result should show length 16777216
  CHECK(len == static_cast<SQLBIGINT>(string_length));
  REQUIRE(indicator == buffer.size() - 1);

  // And the returned string should exactly match the generated string
  std::string retrieved_value(buffer.data(), indicator);
  CHECK(retrieved_value == lob_string);
}
