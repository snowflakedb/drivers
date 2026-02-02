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

TEST_CASE("should select integer literals for int and synonyms", "[int]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT 0::<type>, 1::<type>, -1::<type>, 42::<type>" is executed
  auto stmt = conn.execute_fetch("SELECT 0::INT, 1::INT, -1::INT, 42::INT");

  // Then Result should contain integers [0, 1, -1, 42]
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 0);
  CHECK(get_data<SQL_C_LONG>(stmt, 2) == 1);
  CHECK(get_data<SQL_C_LONG>(stmt, 3) == -1);
  CHECK(get_data<SQL_C_LONG>(stmt, 4) == 42);
}

TEST_CASE("should handle integer boundary values for int and synonyms", "[int]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT -128::<type>, 127::<type>, 255::<type>" is executed
  auto stmt = conn.execute_fetch("SELECT -128::INT, 127::INT, 255::INT");
  // Then Result should contain integers [-128, 127, 255]
  CHECK(get_data<SQL_C_SHORT>(stmt, 1) == -128);
  CHECK(get_data<SQL_C_SHORT>(stmt, 2) == 127);
  CHECK(get_data<SQL_C_SHORT>(stmt, 3) == 255);

  // When Query "SELECT -32768::<type>, 32767::<type>, 65535::<type>" is executed
  stmt = conn.execute_fetch("SELECT -32768::INT, 32767::INT, 65535::INT");
  // Then Result should contain integers [-32768, 32767, 65535]
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == -32768);
  CHECK(get_data<SQL_C_LONG>(stmt, 2) == 32767);
  CHECK(get_data<SQL_C_LONG>(stmt, 3) == 65535);

  // When Query "SELECT -2147483648::<type>, 2147483647::<type>, 4294967295::<type>" is executed
  stmt = conn.execute_fetch("SELECT -2147483648::INT, 2147483647::INT, 4294967295::BIGINT");
  // Then Result should contain integers [-2147483648, 2147483647, 4294967295]
  CHECK(get_data<SQL_C_SBIGINT>(stmt, 1) == -2147483648LL);
  CHECK(get_data<SQL_C_SBIGINT>(stmt, 2) == 2147483647LL);
  CHECK(get_data<SQL_C_SBIGINT>(stmt, 3) == 4294967295LL);

  // When Query "SELECT -9223372036854775808::<type>, 9223372036854775807::<type>" is executed
  stmt = conn.execute_fetch("SELECT -9223372036854775808::BIGINT, 9223372036854775807::BIGINT");
  // Then Result should contain integers [-9223372036854775808, 9223372036854775807]
  CHECK(get_data<SQL_C_SBIGINT>(stmt, 1) == (-9223372036854775807LL - 1));
  CHECK(get_data<SQL_C_SBIGINT>(stmt, 2) == 9223372036854775807LL);
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

TEST_CASE("should select integers from table for int and synonyms", "[int]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with <type> column exists with values [0, 1, -1, 100]
  conn.execute("DROP TABLE IF EXISTS int_table");
  conn.execute("CREATE TABLE int_table (col INT)");
  conn.execute("INSERT INTO int_table VALUES (0), (1), (-1), (100)");

  // When Query "SELECT * FROM int_table ORDER BY col" is executed
  auto stmt = conn.execute_fetch("SELECT * FROM int_table ORDER BY col");

  // Then Result should contain integers [-1, 0, 1, 100]
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == -1);
  SQLRETURN ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 0);
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 1);
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 100);
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
