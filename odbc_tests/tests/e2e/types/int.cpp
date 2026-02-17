#include <optional>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>
#include <catch2/generators/catch_generators.hpp>

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
  auto [values, query_values, expected_values] = GENERATE(table<std::string, std::string, std::vector<int64_t>>({
      {"zero", "SELECT 0::INT", {0}},
      {"tinyint", "SELECT -128::INT, 127::INT, 255::INT", {-128, 127, 255}},
      {"smallint", "SELECT -32768::INT, 32767::INT, 65535::INT", {-32768, 32767, 65535}},
      {"int",
       "SELECT -2147483648::INT, 2147483647::INT, 4294967295::BIGINT",
       {-2147483648LL, 2147483647LL, 4294967295LL}},
      {"bigint",
       "SELECT -9223372036854775808::BIGINT, 9223372036854775807::BIGINT",
       {-9223372036854775807LL - 1, 9223372036854775807LL}},
  }));

  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT <query_values>" is executed
  auto stmt = conn.execute_fetch(query_values);

  // Then Result should contain integers <expected_values>
  for (size_t i = 0; i < expected_values.size(); i++) {
    CHECK(get_data<SQL_C_SBIGINT>(stmt, static_cast<SQLUSMALLINT>(i + 1)) == expected_values[i]);
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
  auto [values, insert_values, expected_values] =
      GENERATE(table<std::string, std::string, std::vector<std::optional<int64_t>>>({
          {"positive",
           "INSERT INTO int_table VALUES (0), (1), (127), (255), (32767), (65535), "
           "(2147483647), (4294967295), (9223372036854775807)",
           {{0}, {1}, {127}, {255}, {32767}, {65535}, {2147483647LL}, {4294967295LL}, {9223372036854775807LL}}},
          {"negative",
           "INSERT INTO int_table VALUES (-1), (-128), (-32768), (-2147483648), (-9223372036854775808)",
           {{-9223372036854775807LL - 1}, {-2147483648LL}, {-32768}, {-128}, {-1}}},
          {"null", "INSERT INTO int_table VALUES (0), (NULL), (42)", {{0}, {42}, std::nullopt}},
      }));

  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with <type> column exists with values <insert_values>
  conn.execute("CREATE TABLE int_table (col BIGINT)");
  conn.execute(insert_values);

  // When Query "SELECT * FROM <table> ORDER BY col" is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT * FROM int_table ORDER BY col", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then Result should contain integers <expected_values>
  for (size_t i = 0; i < expected_values.size(); i++) {
    ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);
    auto result = get_data_optional<SQL_C_SBIGINT>(stmt, 1);
    REQUIRE(result == expected_values[i]);
  }
}

TEST_CASE("should select large result set from table for int and synonyms", "[int]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with <type> column exists with 50000 sequential values
  conn.execute("DROP TABLE IF EXISTS int_large_table");
  conn.execute("CREATE TABLE int_large_table (col BIGINT)");
  conn.execute("INSERT INTO int_large_table SELECT seq8() FROM TABLE(GENERATOR(ROWCOUNT => 50000))");

  // When Query "SELECT * FROM <table> ORDER BY col" is executed
  auto stmt = conn.createStatement();
  const auto sql = "SELECT * FROM int_large_table ORDER BY col";
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
