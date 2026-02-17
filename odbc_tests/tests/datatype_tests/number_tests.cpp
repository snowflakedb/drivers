
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
#include "get_data.hpp"
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

  // 123.789 should truncate to 123 when fetched as integer types
  auto stmt = conn.execute_fetch("SELECT 123.789::DECIMAL(10,3)");

  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 123);
  CHECK(get_data<SQL_C_SLONG>(stmt, 1) == 123);
  CHECK(get_data<SQL_C_ULONG>(stmt, 1) == 123);
  CHECK(get_data<SQL_C_SHORT>(stmt, 1) == 123);
  CHECK(get_data<SQL_C_SSHORT>(stmt, 1) == 123);
  CHECK(get_data<SQL_C_USHORT>(stmt, 1) == 123);
  CHECK(get_data<SQL_C_TINYINT>(stmt, 1) == 123);
  CHECK(get_data<SQL_C_STINYINT>(stmt, 1) == 123);
  CHECK(get_data<SQL_C_UTINYINT>(stmt, 1) == 123);
  CHECK(get_data<SQL_C_SBIGINT>(stmt, 1) == 123);
  CHECK(get_data<SQL_C_UBIGINT>(stmt, 1) == 123);
}

TEST_CASE("SQL_DECIMAL explicit conversions - floating point preserves fractional part",
          "[datatype][number][decimal]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  auto stmt = conn.execute_fetch("SELECT 123.789::DECIMAL(10,3)");

  // Floating point types should preserve the fractional part
  float float_val = get_data<SQL_C_FLOAT>(stmt, 1);
  CHECK(float_val > 123.78f);
  CHECK(float_val < 123.80f);

  double double_val = get_data<SQL_C_DOUBLE>(stmt, 1);
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
