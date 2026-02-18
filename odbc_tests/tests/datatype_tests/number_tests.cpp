
#include <picojson.h>
#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <fstream>
#include <iostream>
#include <memory>
#include <numeric>
#include <sstream>
#include <string>
#include <utility>
#include <vector>

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "macros.hpp"
#include "test_setup.hpp"

/// Helper to call SQLGetData with SQL_C_DEFAULT and return the result as a string.
/// Per ODBC spec, SQL_C_DEFAULT for SQL_DECIMAL resolves to SQL_C_CHAR.
inline std::string get_data_default_as_string(const StatementHandleWrapper& stmt, SQLUSMALLINT col) {
  char buffer[1000];
  SQLLEN indicator;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), col, SQL_C_DEFAULT, buffer, sizeof(buffer), &indicator);
  CHECK_ODBC(ret, stmt);
  return std::string(buffer, indicator);
}

TEST_CASE("Test decimal conversion", "[datatype][number]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("DROP TABLE IF EXISTS test_number");
  conn.execute(
      "CREATE TABLE test_number (num0 NUMBER, num10 NUMBER(10,1), dec20 DECIMAL(20,2), numeric30 "
      "NUMERIC(30,3), int1 INT, int2 INTEGER)");
  conn.execute(
      "INSERT INTO test_number (num0, num10, dec20, numeric30, int1, int2) VALUES (123, 123.4, "
      "123.45, 123.456, 123, 123)");

  auto stmt = conn.execute_fetch("SELECT * FROM test_number");
  for (int i = 1; i <= 6; ++i) {
    INFO("Testing column " << i << " with SQL_C_LONG");
    CHECK(get_data<SQL_C_LONG>(stmt, i) == 123);
  }

  for (int i = 1; i <= 6; ++i) {
    INFO("Testing column " << i << " with SQL_C_SLONG");
    CHECK(get_data<SQL_C_SLONG>(stmt, i) == 123);
  }

  for (int i = 1; i <= 6; ++i) {
    INFO("Testing column " << i << " with SQL_C_ULONG");
    CHECK(get_data<SQL_C_ULONG>(stmt, i) == 123);
  }

  // Test 16-bit integer types - all should return 123 for the integer columns
  for (int i = 1; i <= 6; ++i) {
    INFO("Testing column " << i << " with SQL_C_SHORT");
    CHECK(get_data<SQL_C_SHORT>(stmt, i) == 123);
  }

  for (int i = 1; i <= 6; ++i) {
    INFO("Testing column " << i << " with SQL_C_SSHORT");
    CHECK(get_data<SQL_C_SSHORT>(stmt, i) == 123);
  }

  for (int i = 1; i <= 6; ++i) {
    INFO("Testing column " << i << " with SQL_C_USHORT");
    CHECK(get_data<SQL_C_USHORT>(stmt, i) == 123);
  }

  // Test 8-bit integer types - all should return 123 for the integer columns
  for (int i = 1; i <= 6; ++i) {
    INFO("Testing column " << i << " with SQL_C_TINYINT");
    CHECK(get_data<SQL_C_TINYINT>(stmt, i) == 123);
  }

  for (int i = 1; i <= 6; ++i) {
    INFO("Testing column " << i << " with SQL_C_STINYINT");
    CHECK(get_data<SQL_C_STINYINT>(stmt, i) == 123);
  }

  for (int i = 1; i <= 6; ++i) {
    INFO("Testing column " << i << " with SQL_C_UTINYINT");
    CHECK(get_data<SQL_C_UTINYINT>(stmt, i) == 123);
  }

  // Test 64-bit integer types - all should return 123 for the integer columns
  for (int i = 1; i <= 6; ++i) {
    INFO("Testing column " << i << " with SQL_C_SBIGINT");
    CHECK(get_data<SQL_C_SBIGINT>(stmt, i) == 123);
  }

  for (int i = 1; i <= 6; ++i) {
    INFO("Testing column " << i << " with SQL_C_UBIGINT");
    CHECK(get_data<SQL_C_UBIGINT>(stmt, i) == 123);
  }

  // Test floating point types - test all columns
  std::vector<float> expected_float_values = {123.0f, 123.4f, 123.45f, 123.456f, 123.0f, 123.0f};
  std::vector<double> expected_double_values = {123.0, 123.4, 123.45, 123.456, 123.0, 123.0};

  for (int i = 1; i <= 6; ++i) {
    INFO("Testing column " << i << " with SQL_C_FLOAT");
    CHECK(get_data<SQL_C_FLOAT>(stmt, i) == expected_float_values[i - 1]);
  }

  for (int i = 1; i <= 6; ++i) {
    INFO("Testing column " << i << " with SQL_C_DOUBLE");
    CHECK(get_data<SQL_C_DOUBLE>(stmt, i) == expected_double_values[i - 1]);
  }

  // Test character type conversions - each column should return its string representation
  std::vector<std::string> expected_string_values = {"123", "123.4", "123.45", "123.456", "123", "123"};

  for (int i = 1; i <= 6; ++i) {
    INFO("Testing column " << i << " with SQL_C_CHAR");
    CHECK(get_data<SQL_C_CHAR>(stmt, i) == expected_string_values[i - 1]);
  }

  // SQL_C_DEFAULT must produce the same results as SQL_C_CHAR for DECIMAL columns
  for (int i = 1; i <= 6; ++i) {
    INFO("Testing column " << i << " with SQL_C_DEFAULT (must match SQL_C_CHAR)");
    CHECK(get_data_default_as_string(stmt, i) == expected_string_values[i - 1]);
  }
}

template <int SQL_C_TYPE>
void test_at_limits(Connection& conn) {
  std::stringstream queryBuilder;
  queryBuilder << "SELECT ";
  // prefix + to ensure numeric limits are treated as numbers, not characters
  queryBuilder << +std::numeric_limits<typename MetaOfSqlCType<SQL_C_TYPE>::type>::max() << " AS max, ";
  queryBuilder << +std::numeric_limits<typename MetaOfSqlCType<SQL_C_TYPE>::type>::min() << " AS min";
  auto query = queryBuilder.str();
  std::cout << "Executing query: " << query << std::endl;
  INFO("Executing query: " << query);
  auto stmt = conn.execute_fetch(query);
  CHECK(get_data<SQL_C_TYPE>(stmt, 1) == std::numeric_limits<typename MetaOfSqlCType<SQL_C_TYPE>::type>::max());
  CHECK(get_data<SQL_C_TYPE>(stmt, 2) == std::numeric_limits<typename MetaOfSqlCType<SQL_C_TYPE>::type>::min());
}

void test_string_at_limits(Connection& conn) {
  std::stringstream queryBuilder;
  std::string max = std::string(37, '9');
  std::string min = "-" + std::string(37, '9');
  queryBuilder << "SELECT " << max << " AS max, " << min << " AS min";
  auto query = queryBuilder.str();
  std::cout << "Executing query: " << query << std::endl;
  INFO("Executing query: " << query);
  auto stmt = conn.execute_fetch(query);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == max);
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == min);
  // SQL_C_DEFAULT must produce the same results as SQL_C_CHAR
  CHECK(get_data_default_as_string(stmt, 1) == max);
  CHECK(get_data_default_as_string(stmt, 2) == min);
}

TEST_CASE("Test at limits", "[datatype][number]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  test_at_limits<SQL_C_LONG>(conn);
  test_at_limits<SQL_C_SLONG>(conn);
  test_at_limits<SQL_C_ULONG>(conn);
  test_at_limits<SQL_C_SHORT>(conn);
  test_at_limits<SQL_C_SSHORT>(conn);
  test_at_limits<SQL_C_USHORT>(conn);
  test_at_limits<SQL_C_TINYINT>(conn);
  test_at_limits<SQL_C_STINYINT>(conn);
  test_at_limits<SQL_C_UTINYINT>(conn);
  test_at_limits<SQL_C_SBIGINT>(conn);
  test_at_limits<SQL_C_UBIGINT>(conn);
  test_string_at_limits(conn);
}

// ============================================================================
// SQL_DECIMAL default conversion tests (SQL_C_DEFAULT)
// Per ODBC spec, SQL_DECIMAL's default C type is SQL_C_CHAR.
// These tests verify compatibility between old and new driver implementations.
// ============================================================================

TEST_CASE("SQL_DECIMAL default conversion - basic values", "[datatype][number][decimal][default]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // Use DECIMAL types with explicit scale to ensure the SQL type is SQL_DECIMAL.
  conn.execute("DROP TABLE IF EXISTS test_decimal_default");
  conn.execute(
      "CREATE TABLE test_decimal_default ("
      "  d1 DECIMAL(10,0), "
      "  d2 DECIMAL(10,1), "
      "  d3 DECIMAL(10,2), "
      "  d4 DECIMAL(10,3))");
  conn.execute("INSERT INTO test_decimal_default VALUES (123, 123.4, 123.45, 123.456)");

  auto stmt = conn.execute_fetch("SELECT * FROM test_decimal_default");

  // SQL_C_DEFAULT should resolve to SQL_C_CHAR for SQL_DECIMAL columns
  std::vector<std::string> expected = {"123", "123.4", "123.45", "123.456"};

  for (int i = 1; i <= 4; ++i) {
    INFO("Testing column " << i << " with SQL_C_DEFAULT (expects SQL_C_CHAR behavior)");
    CHECK(get_data_default_as_string(stmt, i) == expected[i - 1]);
  }
}

TEST_CASE("SQL_DECIMAL default conversion - negative values", "[datatype][number][decimal][default]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch(
      "SELECT -123::DECIMAL(10,0), -123.4::DECIMAL(10,1), "
      "-123.45::DECIMAL(10,2), -123.456::DECIMAL(10,3)");

  std::vector<std::string> expected = {"-123", "-123.4", "-123.45", "-123.456"};

  for (int i = 1; i <= 4; ++i) {
    INFO("Testing column " << i << " with SQL_C_DEFAULT (negative values)");
    CHECK(get_data_default_as_string(stmt, i) == expected[i - 1]);
  }
}

TEST_CASE("SQL_DECIMAL default conversion - zero", "[datatype][number][decimal][default]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch("SELECT 0::DECIMAL(10,0), 0::DECIMAL(10,2), 0::DECIMAL(10,5)");

  CHECK(get_data_default_as_string(stmt, 1) == "0");
  CHECK(get_data_default_as_string(stmt, 2) == "0.00");
  CHECK(get_data_default_as_string(stmt, 3) == "0.00000");
}

TEST_CASE("SQL_DECIMAL default conversion - small values with large scale", "[datatype][number][decimal][default]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch(
      "SELECT 0.05::DECIMAL(10,2), 0.001::DECIMAL(10,3), "
      "0.00001::DECIMAL(10,5)");

  CHECK(get_data_default_as_string(stmt, 1) == "0.05");
  CHECK(get_data_default_as_string(stmt, 2) == "0.001");
  CHECK(get_data_default_as_string(stmt, 3) == "0.00001");
}

TEST_CASE("SQL_DECIMAL default conversion - large precision values", "[datatype][number][decimal][default]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // Test with large precision numbers (up to 38 digits, Snowflake's max)
  conn.execute("DROP TABLE IF EXISTS test_decimal_large");
  conn.execute(
      "CREATE TABLE test_decimal_large ("
      "  a NUMBER(38,0), "
      "  b NUMBER(38,37))");
  conn.execute(
      "INSERT INTO test_decimal_large VALUES "
      "(10000000000000000000000000000000000000, "
      " 1.0000000000000000000000000000000000000)");

  auto stmt = conn.execute_fetch("SELECT * FROM test_decimal_large");

  CHECK(get_data_default_as_string(stmt, 1) == "10000000000000000000000000000000000000");
  CHECK(get_data_default_as_string(stmt, 2) == "1.0000000000000000000000000000000000000");
}

TEST_CASE("SQL_DECIMAL default conversion - max precision values", "[datatype][number][decimal][default]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  conn.execute("DROP TABLE IF EXISTS test_decimal_max");
  conn.execute(
      "CREATE TABLE test_decimal_max ("
      "  a NUMBER(38,0), "
      "  b NUMBER(38,37))");
  conn.execute(
      "INSERT INTO test_decimal_max VALUES "
      "(99999999999999999999999999999999999999, "
      " 9.9999999999999999999999999999999999999), "
      "(-99999999999999999999999999999999999999, "
      " -9.9999999999999999999999999999999999999)");

  auto stmt = conn.execute_fetch("SELECT * FROM test_decimal_max");

  // First row
  CHECK(get_data_default_as_string(stmt, 1) == "99999999999999999999999999999999999999");
  CHECK(get_data_default_as_string(stmt, 2) == "9.9999999999999999999999999999999999999");

  // Second row
  SQLRETURN ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data_default_as_string(stmt, 1) == "-99999999999999999999999999999999999999");
  CHECK(get_data_default_as_string(stmt, 2) == "-9.9999999999999999999999999999999999999");
}

TEST_CASE("SQL_DECIMAL default conversion matches explicit SQL_C_CHAR", "[datatype][number][decimal][default]") {
  // This is the key compatibility test: SQL_C_DEFAULT should produce
  // identical results to SQL_C_CHAR for SQL_DECIMAL columns.
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  conn.execute("DROP TABLE IF EXISTS test_decimal_default_vs_char");
  conn.execute(
      "CREATE TABLE test_decimal_default_vs_char ("
      "  d1 DECIMAL(10,1), "
      "  d2 DECIMAL(20,2), "
      "  d3 DECIMAL(30,3))");
  conn.execute("INSERT INTO test_decimal_default_vs_char VALUES (123.4, 123.45, 123.456)");

  // Get data with SQL_C_CHAR explicitly
  auto stmt_char = conn.execute_fetch("SELECT * FROM test_decimal_default_vs_char");
  std::vector<std::string> char_results;
  for (int i = 1; i <= 3; ++i) {
    char_results.push_back(get_data<SQL_C_CHAR>(stmt_char, i));
  }

  // Get data with SQL_C_DEFAULT
  auto stmt_default = conn.execute_fetch("SELECT * FROM test_decimal_default_vs_char");
  std::vector<std::string> default_results;
  for (int i = 1; i <= 3; ++i) {
    default_results.push_back(get_data_default_as_string(stmt_default, i));
  }

  // They must match
  for (int i = 0; i < 3; ++i) {
    INFO("Comparing column " << (i + 1) << ": SQL_C_CHAR='" << char_results[i] << "' vs SQL_C_DEFAULT='"
                             << default_results[i] << "'");
    CHECK(char_results[i] == default_results[i]);
  }
}

TEST_CASE("SQL_DECIMAL SQL_C_CHAR and SQL_C_DEFAULT - various SQL numeric type synonyms",
          "[datatype][number][decimal][default]") {
  // NUMBER, DECIMAL, NUMERIC are all "fixed" type in Snowflake.
  // They all should produce consistent results when fetched as SQL_C_CHAR or SQL_C_DEFAULT.
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch("SELECT 42::NUMBER(10,2), 42::DECIMAL(10,2), 42::NUMERIC(10,2)");

  std::string expected = "42.00";
  for (int i = 1; i <= 3; ++i) {
    INFO("Testing SQL numeric synonym column " << i << " with SQL_C_CHAR");
    CHECK(get_data<SQL_C_CHAR>(stmt, i) == expected);
  }
  for (int i = 1; i <= 3; ++i) {
    INFO("Testing SQL numeric synonym column " << i << " with SQL_C_DEFAULT");
    CHECK(get_data_default_as_string(stmt, i) == expected);
  }
}

TEST_CASE("SQL_DECIMAL explicit conversions - integers truncate fractional part", "[datatype][number][decimal]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  const std::string query = "SELECT 123.789::DECIMAL(10,3)";

  // Each type needs its own query execution because SQLGetData consumes the column.
  CHECK(get_data<SQL_C_LONG>(conn.execute_fetch(query), 1) == 123);
  CHECK(get_data<SQL_C_SLONG>(conn.execute_fetch(query), 1) == 123);
  CHECK(get_data<SQL_C_ULONG>(conn.execute_fetch(query), 1) == 123);
  CHECK(get_data<SQL_C_SHORT>(conn.execute_fetch(query), 1) == 123);
  CHECK(get_data<SQL_C_SSHORT>(conn.execute_fetch(query), 1) == 123);
  CHECK(get_data<SQL_C_USHORT>(conn.execute_fetch(query), 1) == 123);
  CHECK(get_data<SQL_C_TINYINT>(conn.execute_fetch(query), 1) == 123);
  CHECK(get_data<SQL_C_STINYINT>(conn.execute_fetch(query), 1) == 123);
  CHECK(get_data<SQL_C_UTINYINT>(conn.execute_fetch(query), 1) == 123);
  CHECK(get_data<SQL_C_SBIGINT>(conn.execute_fetch(query), 1) == 123);
  CHECK(get_data<SQL_C_UBIGINT>(conn.execute_fetch(query), 1) == 123);
}

TEST_CASE("SQL_DECIMAL explicit conversions - floating point preserves fractional part",
          "[datatype][number][decimal]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  const std::string query = "SELECT 123.789::DECIMAL(10,3)";

  // Each type needs its own query execution because SQLGetData consumes the column.
  float float_val = get_data<SQL_C_FLOAT>(conn.execute_fetch(query), 1);
  CHECK(float_val > 123.78f);
  CHECK(float_val < 123.80f);

  double double_val = get_data<SQL_C_DOUBLE>(conn.execute_fetch(query), 1);
  CHECK(double_val > 123.788);
  CHECK(double_val < 123.790);
}

TEST_CASE("SQL_DECIMAL SQL_C_CHAR and SQL_C_DEFAULT - negative small fractional values",
          "[datatype][number][decimal][default]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch("SELECT -0.05::DECIMAL(10,2), -0.001::DECIMAL(10,3), -0.5::DECIMAL(10,1)");

  // SQL_C_CHAR
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "-0.05");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "-0.001");
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "-0.5");

  // SQL_C_DEFAULT must produce the same results
  CHECK(get_data_default_as_string(stmt, 1) == "-0.05");
  CHECK(get_data_default_as_string(stmt, 2) == "-0.001");
  CHECK(get_data_default_as_string(stmt, 3) == "-0.5");
}

// ============================================================================
// NULL handling tests
// ============================================================================

TEST_CASE("NUMBER NULL values - indicator returns SQL_NULL_DATA", "[datatype][number][null]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch("SELECT NULL::NUMBER(10,0), NULL::DECIMAL(10,2), NULL::NUMERIC(20,5)");

  for (int i = 1; i <= 3; ++i) {
    SQLINTEGER value = 0;
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), i, SQL_C_LONG, &value, sizeof(value), &indicator);
    CHECK_ODBC(ret, stmt);
    INFO("Column " << i << " should be NULL");
    CHECK(indicator == SQL_NULL_DATA);
  }
}

TEST_CASE("NUMBER NULL mixed with non-NULL in multiple rows", "[datatype][number][null]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  conn.execute("DROP TABLE IF EXISTS test_number_null");
  conn.execute("CREATE TABLE test_number_null (val NUMBER(10,0))");
  conn.execute("INSERT INTO test_number_null VALUES (42), (NULL), (-7), (NULL), (0)");

  auto stmt = conn.execute_fetch("SELECT * FROM test_number_null");

  // Row 1: 42
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);
  CHECK(indicator != SQL_NULL_DATA);
  CHECK(value == 42);

  // Row 2: NULL
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);
  CHECK(indicator == SQL_NULL_DATA);

  // Row 3: -7
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);
  CHECK(indicator != SQL_NULL_DATA);
  CHECK(value == -7);

  // Row 4: NULL
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);
  CHECK(indicator == SQL_NULL_DATA);

  // Row 5: 0
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);
  CHECK(indicator != SQL_NULL_DATA);
  CHECK(value == 0);
}

TEST_CASE("NUMBER NULL with SQL_C_CHAR returns SQL_NULL_DATA", "[datatype][number][null]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch("SELECT NULL::DECIMAL(20,5)");

  char buffer[100];
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  CHECK_ODBC(ret, stmt);
  CHECK(indicator == SQL_NULL_DATA);
}

TEST_CASE("NUMBER NULL with SQL_C_DEFAULT returns SQL_NULL_DATA", "[datatype][number][null]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch("SELECT NULL::DECIMAL(10,2)");

  char buffer[100];
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DEFAULT, buffer, sizeof(buffer), &indicator);
  CHECK_ODBC(ret, stmt);
  CHECK(indicator == SQL_NULL_DATA);
}

// ============================================================================
// SQL_C_BIT conversion tests
// ============================================================================

TEST_CASE("SQL_DECIMAL to SQL_C_BIT - 0 and 1", "[datatype][number][bit]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch("SELECT 0::NUMBER(10,0), 1::NUMBER(10,0), 0.00::NUMBER(10,2)");

  CHECK(get_data<SQL_C_BIT>(stmt, 1) == 0);
  CHECK(get_data<SQL_C_BIT>(stmt, 2) == 1);
  CHECK(get_data<SQL_C_BIT>(stmt, 3) == 0);
}

TEST_CASE("SQL_DECIMAL to SQL_C_BIT - negative value is out of range", "[datatype][number][bit]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch("SELECT -1::NUMBER(10,0)");

  unsigned char bit_val = 0xFF;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BIT, &bit_val, sizeof(bit_val), &indicator);
  CHECK(ret == SQL_ERROR);
  auto diags = get_diag_rec(stmt);
  REQUIRE(!diags.empty());
  CHECK(diags[0].sqlState == "22003");
}

// ============================================================================
// Integer truncation toward zero tests
// ============================================================================

TEST_CASE("SQL_DECIMAL fractional truncation toward zero", "[datatype][number][truncation]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // Values less than 1 in magnitude should all truncate to 0
  auto stmt = conn.execute_fetch(
      "SELECT 0.9::DECIMAL(3,1), -0.9::DECIMAL(3,1), "
      "0.1::DECIMAL(3,1), -0.1::DECIMAL(3,1), "
      "0.5::DECIMAL(3,1), -0.5::DECIMAL(3,1)");

  // All should truncate to 0 (truncation toward zero, not rounding)
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 0);
  CHECK(get_data<SQL_C_LONG>(stmt, 2) == 0);
  CHECK(get_data<SQL_C_LONG>(stmt, 3) == 0);
  CHECK(get_data<SQL_C_LONG>(stmt, 4) == 0);
  CHECK(get_data<SQL_C_LONG>(stmt, 5) == 0);
  CHECK(get_data<SQL_C_LONG>(stmt, 6) == 0);
}

TEST_CASE("SQL_DECIMAL fractional truncation - values just below boundary", "[datatype][number][truncation]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch(
      "SELECT 1.99::DECIMAL(5,2), -1.99::DECIMAL(5,2), "
      "127.99::DECIMAL(5,2), -128.99::DECIMAL(6,2)");

  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 1);
  CHECK(get_data<SQL_C_LONG>(stmt, 2) == -1);
  CHECK(get_data<SQL_C_STINYINT>(stmt, 3) == 127);
  CHECK(get_data<SQL_C_STINYINT>(stmt, 4) == -128);
}

// ============================================================================
// Scale=0 pure integer tests
// ============================================================================

TEST_CASE("NUMBER scale=0 - INT and INTEGER types", "[datatype][number][integer]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  conn.execute("DROP TABLE IF EXISTS test_int_types");
  conn.execute("CREATE TABLE test_int_types (a INT, b INTEGER, c BIGINT, d SMALLINT, e TINYINT)");
  conn.execute("INSERT INTO test_int_types VALUES (100, -200, 9223372036854775807, -32000, 120)");

  auto stmt = conn.execute_fetch("SELECT * FROM test_int_types");

  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 100);
  CHECK(get_data<SQL_C_LONG>(stmt, 2) == -200);
  CHECK(get_data<SQL_C_SBIGINT>(stmt, 3) == 9223372036854775807LL);
  CHECK(get_data<SQL_C_SHORT>(stmt, 4) == -32000);
  CHECK(get_data<SQL_C_TINYINT>(stmt, 5) == 120);

  // Also verify SQL_C_CHAR representations for scale=0
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "100");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "-200");
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "9223372036854775807");
  CHECK(get_data<SQL_C_CHAR>(stmt, 4) == "-32000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 5) == "120");
}

// ============================================================================
// SQL_C_CHAR buffer truncation tests
// ============================================================================

TEST_CASE("SQL_DECIMAL SQL_C_CHAR with small buffer", "[datatype][number][char][buffer]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch("SELECT 123456::NUMBER(10,0)");

  char small_buffer[4];
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, small_buffer, sizeof(small_buffer), &indicator);

  OLD_DRIVER_ONLY("BD#13") { CHECK(ret == SQL_SUCCESS); }

  NEW_DRIVER_ONLY("BD#13") {
    CHECK(ret == SQL_SUCCESS_WITH_INFO);
    CHECK(indicator == SQL_NO_TOTAL);
    CHECK(std::string(small_buffer) == "123");
  }
}

TEST_CASE("SQL_DECIMAL SQL_C_CHAR with exact buffer", "[datatype][number][char][buffer]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch("SELECT 42::NUMBER(10,0)");

  // Buffer of size 3: "42" + null terminator
  char exact_buffer[3];
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, exact_buffer, sizeof(exact_buffer), &indicator);
  CHECK_ODBC(ret, stmt);
  CHECK(indicator == 2);
  CHECK(std::string(exact_buffer) == "42");
}

// ============================================================================
// Multiple rows with varying scales and mixed positive/negative
// ============================================================================

TEST_CASE("DECIMAL multiple rows with various values", "[datatype][number][multirow]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  conn.execute("DROP TABLE IF EXISTS test_number_multi");
  conn.execute("CREATE TABLE test_number_multi (val DECIMAL(10,2))");
  conn.execute(
      "INSERT INTO test_number_multi VALUES "
      "(0.00), (1.00), (-1.00), (999.99), (-999.99), (0.01), (-0.01)");

  auto stmt = conn.execute_fetch("SELECT * FROM test_number_multi");

  std::vector<std::string> expected = {"0.00", "1.00", "-1.00", "999.99", "-999.99", "0.01", "-0.01"};

  for (size_t row = 0; row < expected.size(); ++row) {
    if (row > 0) {
      SQLRETURN ret = SQLFetch(stmt.getHandle());
      CHECK_ODBC(ret, stmt);
    }
    INFO("Row " << row << " expected: " << expected[row]);
    CHECK(get_data<SQL_C_CHAR>(stmt, 1) == expected[row]);
  }
}

TEST_CASE("DECIMAL multiple rows fetched as SQL_C_DOUBLE", "[datatype][number][multirow]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  conn.execute("DROP TABLE IF EXISTS test_number_double_multi");
  conn.execute("CREATE TABLE test_number_double_multi (val DECIMAL(10,3))");
  conn.execute(
      "INSERT INTO test_number_double_multi VALUES "
      "(0.000), (1.500), (-2.750), (100.125)");

  auto stmt = conn.execute_fetch("SELECT * FROM test_number_double_multi");

  // These values are exactly representable in f64 (powers of 2 denominators)
  std::vector<double> expected = {0.0, 1.5, -2.75, 100.125};

  for (size_t row = 0; row < expected.size(); ++row) {
    if (row > 0) {
      SQLRETURN ret = SQLFetch(stmt.getHandle());
      CHECK_ODBC(ret, stmt);
    }
    INFO("Row " << row << " expected double: " << expected[row]);
    CHECK(get_data<SQL_C_DOUBLE>(stmt, 1) == expected[row]);
  }
}

// ============================================================================
// Exact scale conversion tests (scale divides evenly)
// ============================================================================

TEST_CASE("DECIMAL exact scale division - no fractional remainder", "[datatype][number][scale]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // 100.00 with scale=2 stored as 10000 in i128, divides by 100 evenly
  auto stmt = conn.execute_fetch(
      "SELECT 100.00::DECIMAL(10,2), 0.50::DECIMAL(10,2), "
      "-25.00::DECIMAL(10,2), 1.00::DECIMAL(10,2)");

  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 100);
  CHECK(get_data<SQL_C_LONG>(stmt, 2) == 0);  // 0.50 truncates to 0
  CHECK(get_data<SQL_C_LONG>(stmt, 3) == -25);
  CHECK(get_data<SQL_C_LONG>(stmt, 4) == 1);
}

// ============================================================================
// Numeric type to SQL_C_DOUBLE/SQL_C_FLOAT precision checks
// ============================================================================

TEST_CASE("DECIMAL to SQL_C_DOUBLE - precision for large values", "[datatype][number][precision]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // Value with 15 significant digits (within f64 precision)
  auto stmt = conn.execute_fetch("SELECT 123456789012345::NUMBER(15,0)");

  double val = get_data<SQL_C_DOUBLE>(stmt, 1);
  // f64 has ~15-16 significant digits, this should be exact
  CHECK(val == 123456789012345.0);
}

TEST_CASE("DECIMAL to SQL_C_FLOAT - limited precision", "[datatype][number][precision]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch("SELECT 123456::NUMBER(10,0)");

  float val = get_data<SQL_C_FLOAT>(stmt, 1);
  CHECK(val == 123456.0f);
}

// ============================================================================
// SQL_C_DEFAULT for various column definitions
// ============================================================================

TEST_CASE("SQL_C_DEFAULT for INT column resolves to SQL_C_CHAR", "[datatype][number][default]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // INT is NUMBER(38,0) under the hood - SQL type is SQL_DECIMAL
  auto stmt = conn.execute_fetch("SELECT 42::INT, -7::INTEGER, 0::BIGINT");

  CHECK(get_data_default_as_string(stmt, 1) == "42");
  CHECK(get_data_default_as_string(stmt, 2) == "-7");
  CHECK(get_data_default_as_string(stmt, 3) == "0");
}
