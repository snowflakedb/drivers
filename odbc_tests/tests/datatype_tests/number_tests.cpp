
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
#include "SqlCTypes.hpp"
#include "macros.hpp"
#include "test_setup.hpp"

class CellResultTester {
 public:
  virtual void test(const StatementHandleWrapper& stmt, SQLUSMALLINT col) const = 0;
  virtual std::string describe() const = 0;
  virtual ~CellResultTester() = default;
};

class CharResultTester : public CellResultTester {
 public:
  CharResultTester(std::string expected) : expected(std::move(expected)) {}
  void test(const StatementHandleWrapper& stmt, SQLUSMALLINT col) const override {
    char buffer[1000];
    SQLLEN indicator;
    SQLRETURN ret =
        SQLGetData(stmt.getHandle(), col, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
    CHECK_ODBC(ret, stmt);
    REQUIRE(std::string(buffer, indicator) == expected);
  }
  std::string describe() const override { return "StringResultMatcher[\"" + expected + "\"]"; }

 private:
  std::string expected;
};

template <typename T, int SQL_C_TYPE>
class NumResultTester : public CellResultTester {
 public:
  NumResultTester(T expected) : expected(expected) {}
  void test(const StatementHandleWrapper& stmt, SQLUSMALLINT col) const override {
    SQLINTEGER value;
    SQLLEN indicator;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_TYPE, &value, sizeof(value), &indicator);
    CHECK_ODBC(ret, stmt);
    REQUIRE(value == expected);
  }
  std::string describe() const override {
    return "IntResultMatcher[" + std::to_string(expected) + "," + std::to_string(SQL_C_TYPE) + "]";
  }

 private:
  T expected;
};

class RowResultTester {
 public:
  explicit RowResultTester(std::vector<std::unique_ptr<CellResultTester>> ts)
      : testers{std::move(ts)} {}

  void test(const StatementHandleWrapper& stmt) const {
    SQLUSMALLINT col = 1;
    for (const auto& tester : testers) {
      INFO("Testing: " << tester->describe());
      tester->test(stmt, col++);
    }
  }

  SQLUSMALLINT size() const { return static_cast<SQLUSMALLINT>(testers.size()); }

  std::string describe() const {
    return "RowResultTester[\n" +
           std::accumulate(testers.begin(), testers.end(), std::string(),
                           [](const std::string& a, const std::unique_ptr<CellResultTester>& b) {
                             return a + "  " + b->describe() + "\n";
                           }) +
           "]";
  }

  void append(std::unique_ptr<CellResultTester> tester) { testers.push_back(std::move(tester)); }

 private:
  std::vector<std::unique_ptr<CellResultTester>> testers;
};

template <typename... Args>
std::unique_ptr<RowResultTester> row_builder(std::vector<std::unique_ptr<CellResultTester>> acc,
                                             std::unique_ptr<CellResultTester> tester,
                                             Args&&... args) {
  acc.push_back(std::move(tester));
  return row_builder(std::move(acc), std::forward<Args>(args)...);
}

std::unique_ptr<RowResultTester> row_builder(std::vector<std::unique_ptr<CellResultTester>> acc) {
  return std::make_unique<RowResultTester>(std::move(acc));
}

template <typename... Args>
std::unique_ptr<RowResultTester> row(Args&&... args) {
  return row_builder(std::vector<std::unique_ptr<CellResultTester>>{}, std::forward<Args>(args)...);
}

std::unique_ptr<CellResultTester> str(const std::string& expected) {
  return std::make_unique<CharResultTester>(expected);
}

template <typename T, int SQL_C_TYPE>
std::unique_ptr<CellResultTester> num(T expected) {
  return std::make_unique<NumResultTester<T, SQL_C_TYPE>>(expected);
}

void test_field_conversion(const StatementHandleWrapper& stmt, std::string col,
                           const std::unique_ptr<RowResultTester>& tester) {
  std::stringstream queryBuilder;
  queryBuilder << "SELECT ";
  for (int i = 0; i < tester->size(); ++i) {
    if (i > 0) {
      queryBuilder << ", ";
    }
    queryBuilder << col;
  }
  queryBuilder << " FROM test_number";
  auto query = queryBuilder.str();
  INFO("Testing" << query << " with " << tester->describe());
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)query.c_str(), SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  tester->test(stmt);
}

template <int SQL_C_TYPE>
typename MetaOfSqlCType<SQL_C_TYPE>::type get_data(const StatementHandleWrapper& stmt,
                                                   SQLUSMALLINT col) {
  typename MetaOfSqlCType<SQL_C_TYPE>::type value;
  SQLLEN indicator;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), col, SQL_C_TYPE, &value, sizeof(value), &indicator);
  CHECK_ODBC(ret, stmt);
  return value;
}

// Template specialization for SQL_C_CHAR to return std::string
template <>
std::string get_data<SQL_C_CHAR>(const StatementHandleWrapper& stmt, SQLUSMALLINT col) {
  char buffer[1000];
  SQLLEN indicator;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), col, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);
  CHECK_ODBC(ret, stmt);
  return std::string(buffer, indicator);
}

TEST_CASE("Test decimal conversion", "[datatype][number]") {
  Connection conn;
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
