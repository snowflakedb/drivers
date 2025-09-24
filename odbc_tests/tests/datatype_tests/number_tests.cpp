
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
  std::vector<std::string> expected_string_values = {"123",     "123.4", "123.45",
                                                     "123.456", "123",   "123"};

  for (int i = 1; i <= 6; ++i) {
    INFO("Testing column " << i << " with SQL_C_CHAR");
    CHECK(get_data<SQL_C_CHAR>(stmt, i) == expected_string_values[i - 1]);
  }
}

template <int SQL_C_TYPE>
void test_at_limits(Connection& conn) {
  std::stringstream queryBuilder;
  queryBuilder << "SELECT ";
  // prefix + to ensure numeric limits are treated as numbers, not characters
  queryBuilder << +std::numeric_limits<typename MetaOfSqlCType<SQL_C_TYPE>::type>::max()
               << " AS max, ";
  queryBuilder << +std::numeric_limits<typename MetaOfSqlCType<SQL_C_TYPE>::type>::min()
               << " AS min";
  auto query = queryBuilder.str();
  std::cout << "Executing query: " << query << std::endl;
  INFO("Executing query: " << query);
  auto stmt = conn.execute_fetch(query);
  CHECK(get_data<SQL_C_TYPE>(stmt, 1) ==
        std::numeric_limits<typename MetaOfSqlCType<SQL_C_TYPE>::type>::max());
  CHECK(get_data<SQL_C_TYPE>(stmt, 2) ==
        std::numeric_limits<typename MetaOfSqlCType<SQL_C_TYPE>::type>::min());
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
