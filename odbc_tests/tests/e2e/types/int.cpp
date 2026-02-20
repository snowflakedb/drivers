#include <optional>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "get_data.hpp"

TEST_CASE("should cast integer values to appropriate type for int and synonyms", "[int]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT 0::<type>, 1000000::<type>, 9223372036854775807::<type>" is executed
  auto stmt = conn.execute_fetch("SELECT 0::INT, 1000000::INT, 9223372036854775807::BIGINT");

  // Then All values should be returned as appropriate type
  // And No precision loss should occur
  CHECK(get_data<SQL_C_SBIGINT>(stmt, 1) == 0);
  CHECK(get_data<SQL_C_SBIGINT>(stmt, 2) == 1000000);
  CHECK(get_data<SQL_C_SBIGINT>(stmt, 3) == 9223372036854775807LL);
}

TEST_CASE("should select integer values for int and synonyms", "[int]") {
  struct TestCase {
    std::string name;
    std::string query;
    std::vector<int64_t> expected;
  };

  std::vector<TestCase> test_cases = {
      {"zero", "SELECT 0::INT", {0}},
      {"tinyint", "SELECT -128::INT, 127::INT, 255::INT", {-128, 127, 255}},
      {"smallint", "SELECT -32768::INT, 32767::INT, 65535::INT", {-32768, 32767, 65535}},
      {"int",
       "SELECT -2147483648::INT, 2147483647::INT, 4294967295::BIGINT",
       {-2147483648LL, 2147483647LL, 4294967295LL}},
      {"bigint",
       "SELECT -9223372036854775808::BIGINT, 9223372036854775807::BIGINT",
       {-9223372036854775807LL - 1, 9223372036854775807LL}},
  };

  // Given Snowflake client is logged in
  Connection conn;

  for (const auto& [name, query, expected] : test_cases) {
    // When Query "SELECT <query_values>" is executed
    auto stmt = conn.execute_fetch(query);

    // Then Result should contain integers <expected_values>
    for (size_t i = 0; i < expected.size(); i++) {
      CHECK(get_data<SQL_C_SBIGINT>(stmt, static_cast<SQLUSMALLINT>(i + 1)) == expected[i]);
    }
  }
}

TEST_CASE("should download large result set with multiple chunks for int and synonyms", "[int]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT seq8()::<type> as id FROM TABLE(GENERATOR(ROWCOUNT => 50000)) v ORDER BY id" is executed
  auto stmt = conn.createStatement();
  const auto sql = "SELECT seq8()::BIGINT as id FROM TABLE(GENERATOR(ROWCOUNT => 50000)) v ORDER BY id";
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)sql, SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then Result should contain 50000 sequentially numbered rows from 0 to 49999
  int row_count = 0;
  int64_t expected_value = 0;

  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) {
      break;
    }
    CHECK_ODBC(ret, stmt);

    SQLBIGINT result = 0;
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SBIGINT, &result, sizeof(result), NULL);
    CHECK_ODBC(ret, stmt);

    REQUIRE(result == expected_value);
    expected_value++;
    row_count++;
  }

  REQUIRE(row_count == 50000);
}

TEST_CASE("should select values from table for int and synonyms", "[int]") {
  struct TestCase {
    std::string name;
    std::string insert_values;
    std::vector<std::optional<int64_t>> expected;
  };

  std::vector<TestCase> test_cases = {
      {"positive",
       "(0), (1), (127), (255), (32767), (65535), (2147483647), (4294967295), (9223372036854775807)",
       {{0}, {1}, {127}, {255}, {32767}, {65535}, {2147483647LL}, {4294967295LL}, {9223372036854775807LL}}},
      {"negative",
       "(-1), (-128), (-32768), (-2147483648), (-9223372036854775808)",
       {{-9223372036854775807LL - 1}, {-2147483648LL}, {-32768}, {-128}, {-1}}},
      {"null", "(0), (NULL), (42)", {{0}, {42}, std::nullopt}},
  };

  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  for (const auto& [name, insert_values, expected] : test_cases) {
    // And Table with <type> column exists with values <insert_values>
    auto table_name = "int_table_" + name;
    conn.execute("CREATE TABLE " + table_name + " (col BIGINT)");
    conn.execute("INSERT INTO " + table_name + " VALUES " + insert_values);

    // When Query "SELECT * FROM <table> ORDER BY col" is executed
    auto stmt = conn.createStatement();
    auto select_sql = "SELECT * FROM " + table_name + " ORDER BY col";
    SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)select_sql.c_str(), SQL_NTS);
    CHECK_ODBC(ret, stmt);

    // Then Result should contain integers <expected_values>
    for (size_t i = 0; i < expected.size(); i++) {
      ret = SQLFetch(stmt.getHandle());
      CHECK_ODBC(ret, stmt);
      auto result = get_data_optional<SQL_C_SBIGINT>(stmt, 1);
      REQUIRE(result == expected[i]);
    }
  }
}

TEST_CASE("should handle server-side Arrow memory optimization for int columns on multiple chunks", "[int]") {
  constexpr int total_rows = 50000;
  constexpr int64_t expected_col1 = 100;
  constexpr int64_t expected_col2 = 30000;
  constexpr int64_t expected_col3 = 2000000000;
  constexpr int64_t expected_col4 = 9000000000000000000LL;

  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with four INT columns exists
  conn.execute("CREATE TABLE int_different_column_sizes (col_int8 INT, col_int16 INT, col_int32 INT, col_int64 INT)");

  // And Each column contains values of different magnitudes (50000 rows to span multiple Arrow chunks)
  conn.execute(
      "INSERT INTO int_different_column_sizes "
      "SELECT 100, 30000, 2000000000, 9000000000000000000 "
      "FROM TABLE(GENERATOR(ROWCOUNT => " +
      std::to_string(total_rows) + "))");

  // When Query "SELECT * FROM <table>" is executed
  auto stmt = conn.createStatement();
  const auto sql = "SELECT * FROM int_different_column_sizes";
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)sql, SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then Result should contain 50000 rows
  int row_count = 0;

  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) {
      break;
    }
    CHECK_ODBC(ret, stmt);

    SQLBIGINT col1, col2, col3, col4;
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SBIGINT, &col1, sizeof(col1), NULL);
    CHECK_ODBC(ret, stmt);
    ret = SQLGetData(stmt.getHandle(), 2, SQL_C_SBIGINT, &col2, sizeof(col2), NULL);
    CHECK_ODBC(ret, stmt);
    ret = SQLGetData(stmt.getHandle(), 3, SQL_C_SBIGINT, &col3, sizeof(col3), NULL);
    CHECK_ODBC(ret, stmt);
    ret = SQLGetData(stmt.getHandle(), 4, SQL_C_SBIGINT, &col4, sizeof(col4), NULL);
    CHECK_ODBC(ret, stmt);

    // And All values should be equal to expected data
    REQUIRE(col1 == expected_col1);
    REQUIRE(col2 == expected_col2);
    REQUIRE(col3 == expected_col3);
    REQUIRE(col4 == expected_col4);

    row_count++;
  }

  REQUIRE(row_count == total_rows);
}
