// DECFLOAT datatype ODBC E2E tests
// Based on: tests/definitions/shared/types/decfloat.feature
//
// Snowflake DECFLOAT: 38-digit precision with extreme exponents (up to E+16384).
// No numeric C type can represent this, so values are read as SQL_C_CHAR strings.
//
// The new driver does not yet support DECFLOAT Arrow format; all tests
// are skipped via SKIP_NEW_DRIVER_NOT_IMPLEMENTED() until support lands.

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"

// ============================================================================
// TYPE CASTING
// ============================================================================

TEST_CASE("should cast decfloat values to appropriate type", "[decfloat]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT 0::DECFLOAT, 123.456::DECFLOAT, 1.23e37::DECFLOAT,
  // '12345678901234567890123456789012345678'::DECFLOAT" is executed
  auto stmt = conn.execute_fetch(
      "SELECT 0::DECFLOAT, 123.456::DECFLOAT, 1.23e37::DECFLOAT, "
      "'12345678901234567890123456789012345678'::DECFLOAT");

  // Then All values should be returned as appropriate type
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "0");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "123.456");

  std::string large_val = get_data<SQL_C_CHAR>(stmt, 3);
  CHECK(!large_val.empty());

  // And Values should maintain full 38-digit precision
  std::string full_precision = get_data<SQL_C_CHAR>(stmt, 4);
  CHECK(full_precision.find("12345678901234567890123456789012345678") != std::string::npos);
}

// ============================================================================
// SELECT WITH LITERALS (no tables)
// ============================================================================

TEST_CASE("should select decfloat literals", "[decfloat]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT 0::DECFLOAT, 1.5::DECFLOAT, -1.5::DECFLOAT, 123.456789::DECFLOAT, -987.654321::DECFLOAT" is
  // executed
  auto stmt = conn.execute_fetch(
      "SELECT 0::DECFLOAT, 1.5::DECFLOAT, -1.5::DECFLOAT, 123.456789::DECFLOAT, -987.654321::DECFLOAT");

  // Then Result should contain exact decimals [0, 1.5, -1.5, 123.456789, -987.654321]
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "0");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "1.5");
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "-1.5");
  CHECK(get_data<SQL_C_CHAR>(stmt, 4) == "123.456789");
  CHECK(get_data<SQL_C_CHAR>(stmt, 5) == "-987.654321");
}

TEST_CASE("should handle full 38-digit precision values from literals", "[decfloat]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '12345678901234567890123456789012345678'::DECFLOAT,
  // '1.2345678901234567890123456789012345678E+100'::DECFLOAT,
  // '1.2345678901234567890123456789012345678E-100'::DECFLOAT" is executed
  auto stmt = conn.execute_fetch(
      "SELECT '12345678901234567890123456789012345678'::DECFLOAT, "
      "'1.2345678901234567890123456789012345678E+100'::DECFLOAT, "
      "'1.2345678901234567890123456789012345678E-100'::DECFLOAT");

  // Then Result should preserve all 38 digits for each value
  std::string val1 = get_data<SQL_C_CHAR>(stmt, 1);
  CHECK(val1.find("12345678901234567890123456789012345678") != std::string::npos);

  std::string val2 = get_data<SQL_C_CHAR>(stmt, 2);
  CHECK(!val2.empty());
  CHECK(val2.find("123456789012345678901234567890123456") != std::string::npos);

  std::string val3 = get_data<SQL_C_CHAR>(stmt, 3);
  CHECK(!val3.empty());
  CHECK(val3.find("123456789012345678901234567890123456") != std::string::npos);
}

TEST_CASE("should handle extreme exponent values from literals", "[decfloat]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '1E+16384'::DECFLOAT, '1E-16383'::DECFLOAT" is executed
  auto stmt = conn.execute_fetch("SELECT '1E+16384'::DECFLOAT, '1E-16383'::DECFLOAT");

  // Then Result should contain [1E+16384, 1E-16383]
  std::string val1 = get_data<SQL_C_CHAR>(stmt, 1);
  CHECK(!val1.empty());

  std::string val2 = get_data<SQL_C_CHAR>(stmt, 2);
  CHECK(!val2.empty());

  // When Query "SELECT '-1.234E+8000'::DECFLOAT, '9.876E-8000'::DECFLOAT" is executed
  auto stmt2 = conn.execute_fetch("SELECT '-1.234E+8000'::DECFLOAT, '9.876E-8000'::DECFLOAT");

  // Then Result should contain [-1.234E+8000, 9.876E-8000]
  std::string val3 = get_data<SQL_C_CHAR>(stmt2, 1);
  CHECK(!val3.empty());
  CHECK(val3[0] == '-');

  std::string val4 = get_data<SQL_C_CHAR>(stmt2, 2);
  CHECK(!val4.empty());
}

TEST_CASE("should handle NULL values from literals", "[decfloat]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT NULL::DECFLOAT, 42.5::DECFLOAT, NULL::DECFLOAT" is executed
  auto stmt = conn.execute_fetch("SELECT NULL::DECFLOAT, 42.5::DECFLOAT, NULL::DECFLOAT");

  // Then Result should contain [NULL, 42.5, NULL]
  CHECK(get_data_optional<SQL_C_CHAR>(stmt, 1) == std::nullopt);
  CHECK(get_data_optional<SQL_C_CHAR>(stmt, 2) == std::optional<std::string>("42.5"));
  CHECK(get_data_optional<SQL_C_CHAR>(stmt, 3) == std::nullopt);
}

TEST_CASE("should download large result set with multiple chunks from GENERATOR", "[decfloat]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT seq8()::DECFLOAT as id FROM TABLE(GENERATOR(ROWCOUNT => 20000)) v" is executed
  auto stmt = conn.createStatement();
  const auto sql = "SELECT seq8()::DECFLOAT as id FROM TABLE(GENERATOR(ROWCOUNT => 20000)) v ORDER BY 1";
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)sql, SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then Result should contain consecutive numbers from 0 to 19999 returned as appropriate type
  int row_count = 0;

  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) break;
    CHECK_ODBC(ret, stmt);

    std::string val = get_data<SQL_C_CHAR>(stmt, 1);
    REQUIRE(!val.empty());

    int int_val = std::stoi(val);
    REQUIRE(int_val == row_count);
    row_count++;
  }

  REQUIRE(row_count == 20000);
}

// ============================================================================
// TABLE OPERATIONS
// ============================================================================

TEST_CASE("should select decfloats from table", "[decfloat]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with DECFLOAT column exists with values [0, 123.456, -789.012, 1.23e20, -9.87e-15]
  conn.execute("CREATE TABLE decfloat_basic (col DECFLOAT)");
  conn.execute(
      "INSERT INTO decfloat_basic SELECT column1::DECFLOAT FROM VALUES "
      "('0'), ('123.456'), ('-789.012'), ('1.23e20'), ('-9.87e-15')");

  // When Query "SELECT * FROM <table>" is executed
  auto stmt = conn.execute_fetch("SELECT * FROM decfloat_basic");

  // Then Result should contain exact decimals [0, 123.456, -789.012, 1.23e20, -9.87e-15]
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "0");

  SQLRETURN ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "123.456");

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "-789.012");

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  std::string sci_pos = get_data<SQL_C_CHAR>(stmt, 1);
  CHECK(!sci_pos.empty());

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  std::string sci_neg = get_data<SQL_C_CHAR>(stmt, 1);
  CHECK(!sci_neg.empty());
  CHECK(sci_neg[0] == '-');
}

TEST_CASE("should handle full 38-digit precision values from table", "[decfloat]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with DECFLOAT column exists with values [12345678901234567890123456789012345678,
  // 1.2345678901234567890123456789012345678E+100, 1.2345678901234567890123456789012345678E-100]
  conn.execute("CREATE TABLE decfloat_precision (col DECFLOAT)");
  conn.execute(
      "INSERT INTO decfloat_precision SELECT column1::DECFLOAT FROM VALUES "
      "('12345678901234567890123456789012345678'), "
      "('1.2345678901234567890123456789012345678E+100'), "
      "('1.2345678901234567890123456789012345678E-100')");

  // When Query "SELECT * FROM <table>" is executed
  auto stmt = conn.execute_fetch("SELECT * FROM decfloat_precision");

  // Then Result should preserve all 38 digits for each value
  std::string val1 = get_data<SQL_C_CHAR>(stmt, 1);
  CHECK(val1.find("12345678901234567890123456789012345678") != std::string::npos);

  SQLRETURN ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  std::string val2 = get_data<SQL_C_CHAR>(stmt, 1);
  CHECK(!val2.empty());
  CHECK(val2.find("123456789012345678901234567890123456") != std::string::npos);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  std::string val3 = get_data<SQL_C_CHAR>(stmt, 1);
  CHECK(!val3.empty());
  CHECK(val3.find("123456789012345678901234567890123456") != std::string::npos);
}

TEST_CASE("should handle extreme exponent values from table", "[decfloat]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with DECFLOAT column exists with values [1E+16384, 1E-16383, -1.234E+8000, 9.876E-8000]
  conn.execute("CREATE TABLE decfloat_extreme (col DECFLOAT)");
  conn.execute(
      "INSERT INTO decfloat_extreme SELECT column1::DECFLOAT FROM VALUES "
      "('1E+16384'), ('1E-16383'), ('-1.234E+8000'), ('9.876E-8000')");

  // When Query "SELECT * FROM <table>" is executed
  auto stmt = conn.execute_fetch("SELECT * FROM decfloat_extreme");

  // Then Result should contain [1E+16384, 1E-16383, -1.234E+8000, 9.876E-8000]
  std::string val1 = get_data<SQL_C_CHAR>(stmt, 1);
  CHECK(!val1.empty());

  SQLRETURN ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  std::string val2 = get_data<SQL_C_CHAR>(stmt, 1);
  CHECK(!val2.empty());

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  std::string val3 = get_data<SQL_C_CHAR>(stmt, 1);
  CHECK(!val3.empty());
  CHECK(val3[0] == '-');

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  std::string val4 = get_data<SQL_C_CHAR>(stmt, 1);
  CHECK(!val4.empty());
}

TEST_CASE("should handle NULL values from table", "[decfloat]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with DECFLOAT column exists with values [NULL, 123.456, NULL, -789.012]
  conn.execute("CREATE TABLE decfloat_null (col DECFLOAT)");
  conn.execute("INSERT INTO decfloat_null SELECT NULL::DECFLOAT");
  conn.execute("INSERT INTO decfloat_null SELECT 123.456::DECFLOAT");
  conn.execute("INSERT INTO decfloat_null SELECT NULL::DECFLOAT");
  conn.execute("INSERT INTO decfloat_null SELECT (-789.012)::DECFLOAT");

  // When Query "SELECT * FROM <table>" is executed
  auto stmt = conn.execute_fetch("SELECT * FROM decfloat_null");

  // Then Result should contain [NULL, 123.456, NULL, -789.012]
  CHECK(get_data_optional<SQL_C_CHAR>(stmt, 1) == std::nullopt);

  SQLRETURN ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "123.456");

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data_optional<SQL_C_CHAR>(stmt, 1) == std::nullopt);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "-789.012");
}

TEST_CASE("should download large result set with multiple chunks from table", "[decfloat]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with DECFLOAT column exists with values from 0 to 19999
  conn.execute("CREATE TABLE decfloat_large (col DECFLOAT)");
  conn.execute("INSERT INTO decfloat_large SELECT seq8()::DECFLOAT FROM TABLE(GENERATOR(ROWCOUNT => 20000))");

  // When Query "SELECT * FROM <table>" is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT * FROM decfloat_large ORDER BY col", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then Result should contain consecutive numbers from 0 to 19999 returned as appropriate type
  int row_count = 0;

  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) break;
    CHECK_ODBC(ret, stmt);

    std::string val = get_data<SQL_C_CHAR>(stmt, 1);
    REQUIRE(!val.empty());

    int int_val = std::stoi(val);
    REQUIRE(int_val == row_count);
    row_count++;
  }

  REQUIRE(row_count == 20000);
}
