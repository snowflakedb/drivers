#include <catch2/catch_approx.hpp>
#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "get_data.hpp"

TEST_CASE("should cast number values to appropriate type for number and synonyms", "[number]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT 0::<type>(10,0), 123::<type>(10,0), 0.00::<type>(10,2), 123.45::<type>(10,2)" is executed
  auto stmt = conn.execute_fetch("SELECT 0::NUMBER(10,0), 123::NUMBER(10,0), 0.00::NUMBER(10,2), 123.45::NUMBER(10,2)");

  // Then All values should be returned as appropriate type
  // And Values should match [0, 123, 0.00, 123.45]
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 0);
  CHECK(get_data<SQL_C_LONG>(stmt, 2) == 123);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 3) == 0.0);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 4) == 123.45);
}

TEST_CASE("should select number literals for number and synonyms", "[number]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT 0::<type>(10,0), -456::<type>(10,0), 1.50::<type>(10,2), -123.45::<type>(10,2),
  // 123.456::<type>(15,3), -789.012::<type>(15,3)" is executed
  auto stmt = conn.execute_fetch(
      "SELECT 0::NUMBER(10,0), -456::NUMBER(10,0), 1.50::NUMBER(10,2), -123.45::NUMBER(10,2), 123.456::NUMBER(15,3), "
      "-789.012::NUMBER(15,3)");

  // Then Result should contain [0, -456, 1.50, -123.45, 123.456, -789.012]
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 0);
  CHECK(get_data<SQL_C_LONG>(stmt, 2) == -456);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 3) == 1.50);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 4) == -123.45);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 5) == 123.456);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 6) == -789.012);
}

TEST_CASE("should handle scale and precision boundaries from literals for number and synonyms", "[number]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT 999.99::<type>(5,2), -999.99::<type>(5,2), 99999999::<type>(8,0), -99999999::<type>(8,0)" is
  // executed
  auto stmt = conn.execute_fetch(
      "SELECT 999.99::NUMBER(5,2), -999.99::NUMBER(5,2), 99999999::NUMBER(8,0), -99999999::NUMBER(8,0)");

  // Then Result should contain [999.99, -999.99, 99999999, -99999999]
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 1) == 999.99);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 2) == -999.99);
  CHECK(get_data<SQL_C_LONG>(stmt, 3) == 99999999);
  CHECK(get_data<SQL_C_LONG>(stmt, 4) == -99999999);
}

TEST_CASE("should download large result set with multiple chunks from GENERATOR for number and synonyms", "[number]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT seq8()::<type>(38,0), (seq8() + 0.12345)::<type>(20,5) FROM TABLE(GENERATOR(ROWCOUNT =>
  // 30000)) v" is executed
  auto stmt = conn.createStatement();
  const auto sql =
      "SELECT seq8()::NUMBER(38,0), (seq8() + 0.12345)::NUMBER(20,5) FROM TABLE(GENERATOR(ROWCOUNT => 30000)) v "
      "ORDER BY 1";
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)sql, SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then Column 1 should contain sequential integers from 0 to 29999
  // And Column 2 should contain sequential decimals starting from 0.12345
  int row_count = 0;
  int64_t expected_int = 0;
  double expected_decimal = 0.12345;

  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) {
      break;
    }
    CHECK_ODBC(ret, stmt);

    SQLBIGINT col1 = 0;
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SBIGINT, &col1, sizeof(col1), NULL);
    CHECK_ODBC(ret, stmt);
    REQUIRE(col1 == expected_int);

    double col2 = 0;
    ret = SQLGetData(stmt.getHandle(), 2, SQL_C_DOUBLE, &col2, sizeof(col2), NULL);
    CHECK_ODBC(ret, stmt);
    REQUIRE(col2 == Catch::Approx(expected_decimal).epsilon(0.00001));

    expected_int++;
    expected_decimal += 1.0;
    row_count++;
  }

  REQUIRE(row_count == 30000);
}

TEST_CASE("should select numbers from table with multiple scales for number and synonyms", "[number]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with columns (<type>(10,0), <type>(10,2), <type>(15,3), <type>(20,5)) exists
  conn.execute("DROP TABLE IF EXISTS number_table");
  conn.execute(
      "CREATE TABLE number_table (col1 NUMBER(10,0), col2 NUMBER(10,2), col3 NUMBER(15,3), col4 NUMBER(20,5))");

  // And Row (123, 123.45, 123.456, 12345.67890) is inserted
  conn.execute("INSERT INTO number_table VALUES (123, 123.45, 123.456, 12345.67890)");
  // And Row (-456, -67.89, -789.012, -98765.43210) is inserted
  conn.execute("INSERT INTO number_table VALUES (-456, -67.89, -789.012, -98765.43210)");
  // And Row (0, 0.00, 0.000, 0.00000) is inserted
  conn.execute("INSERT INTO number_table VALUES (0, 0.00, 0.000, 0.00000)");
  // And Row (999999, 999.99, 1000.500, 123456.78901) is inserted
  conn.execute("INSERT INTO number_table VALUES (999999, 999.99, 1000.500, 123456.78901)");

  // When Query "SELECT * FROM <table>" is executed
  auto stmt = conn.execute_fetch("SELECT * FROM number_table");

  // Then Result should contain 4 rows with expected values
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 123);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 2) == 123.45);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 3) == 123.456);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 4) == 12345.67890);

  // Row 2
  SQLRETURN ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == -456);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 2) == -67.89);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 3) == -789.012);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 4) == -98765.43210);

  // Row 3
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 0);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 2) == 0.00);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 3) == 0.000);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 4) == 0.00000);

  // Row 4
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 999999);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 2) == 999.99);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 3) == 1000.500);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 4) == Catch::Approx(123456.78901).epsilon(0.00001));
}

TEST_CASE("should handle scale and precision boundaries from table for number and synonyms", "[number]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with columns (<type>(5,2), <type>(8,0)) exists
  conn.execute("DROP TABLE IF EXISTS number_boundary_table");
  conn.execute("CREATE TABLE number_boundary_table (col1 NUMBER(5,2), col2 NUMBER(8,0))");

  // And Row (999.99, 99999999) is inserted
  conn.execute("INSERT INTO number_boundary_table VALUES (999.99, 99999999)");
  // And Row (-999.99, -99999999) is inserted
  conn.execute("INSERT INTO number_boundary_table VALUES (-999.99, -99999999)");
  // And Row (123.45, 12345678) is inserted
  conn.execute("INSERT INTO number_boundary_table VALUES (123.45, 12345678)");
  // And Row (0.01, 0) is inserted
  conn.execute("INSERT INTO number_boundary_table VALUES (0.01, 0)");

  // When Query "SELECT * FROM <table>" is executed
  auto stmt = conn.execute_fetch("SELECT * FROM number_boundary_table");

  // Then Result should contain 4 rows with expected boundary values
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 1) == 999.99);
  CHECK(get_data<SQL_C_LONG>(stmt, 2) == 99999999);

  SQLRETURN ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 1) == -999.99);
  CHECK(get_data<SQL_C_LONG>(stmt, 2) == -99999999);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 1) == 123.45);
  CHECK(get_data<SQL_C_LONG>(stmt, 2) == 12345678);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 1) == 0.01);
  CHECK(get_data<SQL_C_LONG>(stmt, 2) == 0);
}

TEST_CASE("should download large result set from table for number and synonyms", "[number]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with columns (<type>(38,0), <type>(20,5)) exists with 30000 sequential rows, from 0 to 29999 in the
  // first column and from 0.12345 to 29999.12345 in the second column
  conn.execute("DROP TABLE IF EXISTS number_large_table");
  conn.execute("CREATE TABLE number_large_table (col1 NUMBER(38,0), col2 NUMBER(20,5))");
  conn.execute(
      "INSERT INTO number_large_table SELECT seq8(), seq8() + 0.12345 FROM TABLE(GENERATOR(ROWCOUNT => 30000))");

  // When Query "SELECT * FROM <table>" is executed
  auto stmt = conn.createStatement();
  const auto sql = "SELECT * FROM number_large_table ORDER BY col1";
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)sql, SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then Column 1 should contain sequential integers from 0 to 29999
  // And Column 2 should contain sequential decimals starting from 0.12345
  int row_count = 0;
  int64_t expected_int = 0;
  double expected_decimal = 0.12345;

  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) {
      break;
    }
    CHECK_ODBC(ret, stmt);

    SQLBIGINT col1 = 0;
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SBIGINT, &col1, sizeof(col1), NULL);
    CHECK_ODBC(ret, stmt);
    REQUIRE(col1 == expected_int);

    double col2 = 0;
    ret = SQLGetData(stmt.getHandle(), 2, SQL_C_DOUBLE, &col2, sizeof(col2), NULL);
    CHECK_ODBC(ret, stmt);
    REQUIRE(col2 == Catch::Approx(expected_decimal).epsilon(0.00001));

    expected_int++;
    expected_decimal += 1.0;
    row_count++;
  }

  REQUIRE(row_count == 30000);
}
