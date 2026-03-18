// DECFLOAT datatype ODBC E2E tests
// Based on: tests/definitions/shared/types/decfloat.feature
//
// Snowflake DECFLOAT: 38-digit precision with extreme exponents (up to E+16384).
// No numeric C type can represent this, so values are read as SQL_C_CHAR strings.
//
// The new driver does not yet support DECFLOAT Arrow format; all tests
// are skipped via SKIP_NEW_DRIVER_NOT_IMPLEMENTED() until support lands.
#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "macros.hpp"

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
  for (SQLUSMALLINT col = 1; col <= 4; ++col) {
    SQLSMALLINT data_type = 0;
    SQLULEN column_size = 0;
    SQLSMALLINT decimal_digits = 0;
    SQLRETURN ret =
        SQLDescribeCol(stmt.getHandle(), col, nullptr, 0, nullptr, &data_type, &column_size, &decimal_digits, nullptr);
    CHECK_ODBC(ret, stmt);
    CHECK(data_type == SQL_NUMERIC);
    CHECK(column_size == 38);
    CHECK(decimal_digits == 0);
  }

  // And Values should maintain full 38-digit precision
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "0");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "123.456");
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "12300000000000000000000000000000000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 4) == "12345678901234567890123456789012345678");
}

// ============================================================================
// SELECT WITH LITERALS (no tables)
// ============================================================================

TEST_CASE("should select decfloat literals", "[decfloat]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT 0::DECFLOAT, 1.5::DECFLOAT, -1.5::DECFLOAT, 123.456789::DECFLOAT, -987.654321::DECFLOAT"
  // is executed
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
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "12345678901234567890123456789012345678");

  NEW_DRIVER_ONLY("BD#21") {
    CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "1.2345678901234567890123456789012345678e100");
    CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "1.2345678901234567890123456789012345678e-100");
  }

  OLD_DRIVER_ONLY("BD#21") {
    CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "12345678901234567890123456789012345678e63");
    CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "12345678901234567890123456789012345678e-137");
  }
}

TEST_CASE("should handle case exponent values from literals", "[decfloat]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Given Snowflake client is logged in
  Connection conn;

  {
    INFO("max positive and min positive");
    // When Query "SELECT <query_values>" is executed
    auto stmt = conn.execute_fetch("SELECT '1E+16384'::DECFLOAT, '1E-16383'::DECFLOAT");

    // Then Result should contain [<expected_values>]
    CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "1e16384");
    CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "1e-16383");
  }

  {
    INFO("large negative and small positive");
    // When Query "SELECT '-1.234E+8000'::DECFLOAT, '9.876E-8000'::DECFLOAT" is executed
    auto stmt = conn.execute_fetch("SELECT '-1.234E+8000'::DECFLOAT, '9.876E-8000'::DECFLOAT");

    // Then Result should contain [-1.234E+8000, 9.876E-8000]
    NEW_DRIVER_ONLY("BD#21") {
      CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "-1.234e8000");
      CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "9.876e-8000");
    }

    OLD_DRIVER_ONLY("BD#21") {
      CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "-1234e7997");
      CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "9876e-8003");
    }
  }
}

TEST_CASE("should handle NULL values from literals", "[decfloat]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT NULL::DECFLOAT, 42.5::DECFLOAT, NULL::DECFLOAT" is executed
  auto stmt = conn.execute_fetch("SELECT NULL::DECFLOAT, 42.5::DECFLOAT, NULL::DECFLOAT");

  // Then Result should contain [NULL, 42.5, NULL]
  CHECK(!get_data_optional<SQL_C_CHAR>(stmt, 1).has_value());
  CHECK(get_data_optional<SQL_C_CHAR>(stmt, 2) == "42.5");
  CHECK(!get_data_optional<SQL_C_CHAR>(stmt, 3).has_value());
}

TEST_CASE("should download large result set with multiple chunks from GENERATOR", "[decfloat]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT seq8()::DECFLOAT as id FROM TABLE(GENERATOR(ROWCOUNT => 20000)) v" is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(
      stmt.getHandle(), (SQLCHAR*)"SELECT seq8()::DECFLOAT as id FROM TABLE(GENERATOR(ROWCOUNT => 20000)) v ORDER BY 1",
      SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then Result should contain consecutive numbers from 0 to 19999 returned as appropriate type
  int row_count = 0;
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) {
      break;
    }
    CHECK_ODBC(ret, stmt);
    CHECK(get_data<SQL_C_CHAR>(stmt, 1) == std::to_string(row_count));

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
  conn.execute("CREATE OR REPLACE TABLE decfloat_table (col DECFLOAT)");
  conn.execute("INSERT INTO decfloat_table VALUES ('0'), ('123.456'), ('-789.012'), ('1.23E+20'), ('-9.87E-15')");

  // When Query "SELECT * FROM <table>" is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT * FROM decfloat_table", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then Result should contain exact decimals [0, 123.456, -789.012, 1.23e20, -9.87e-15]
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "0");

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "123.456");

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "-789.012");

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "123000000000000000000");

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "-0.00000000000000987");
}

TEST_CASE("should handle full 38-digit precision values from table", "[decfloat]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with DECFLOAT column exists with values [12345678901234567890123456789012345678,
  // 1.2345678901234567890123456789012345678E+100, 1.2345678901234567890123456789012345678E-100]
  conn.execute("CREATE OR REPLACE TABLE decfloat_precision_table (col DECFLOAT)");
  conn.execute(
      "INSERT INTO decfloat_precision_table VALUES "
      "('12345678901234567890123456789012345678'), "
      "('1.2345678901234567890123456789012345678E+100'), "
      "('1.2345678901234567890123456789012345678E-100')");

  // When Query "SELECT * FROM <table>" is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT * FROM decfloat_precision_table", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then Result should preserve all 38 digits for each value
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "12345678901234567890123456789012345678");

  NEW_DRIVER_ONLY("BD#21") {
    ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);
    CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "1.2345678901234567890123456789012345678e100");

    ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);
    CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "1.2345678901234567890123456789012345678e-100");
  }

  OLD_DRIVER_ONLY("BD#21") {
    ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);
    CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "12345678901234567890123456789012345678e63");

    ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);
    CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "12345678901234567890123456789012345678e-137");
  }
}

TEST_CASE("should handle extreme exponent values from table", "[decfloat]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with DECFLOAT column exists with values [1E+16384, 1E-16383, -1.234E+8000, 9.876E-8000]
  conn.execute("CREATE OR REPLACE TABLE decfloat_extreme_table (col DECFLOAT)");
  conn.execute(
      "INSERT INTO decfloat_extreme_table VALUES "
      "('1E+16384'), ('1E-16383'), ('-1.234E+8000'), ('9.876E-8000')");

  // When Query "SELECT * FROM <table>" is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT * FROM decfloat_extreme_table", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then Result should contain [1E+16384, 1E-16383, -1.234E+8000, 9.876E-8000]
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "1e16384");

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "1e-16383");

  NEW_DRIVER_ONLY("BD#21") {
    ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);
    CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "-1.234e8000");

    ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);
    CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "9.876e-8000");
  }

  OLD_DRIVER_ONLY("BD#21") {
    ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);
    CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "-1234e7997");

    ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);
    CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "9876e-8003");
  }
}

TEST_CASE("should handle NULL values from table", "[decfloat]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with DECFLOAT column exists with values [NULL, 123.456, NULL, -789.012]
  conn.execute("CREATE OR REPLACE TABLE decfloat_null_table (col DECFLOAT)");
  conn.execute("INSERT INTO decfloat_null_table VALUES (NULL), ('123.456'), (NULL), ('-789.012')");

  // When Query "SELECT * FROM <table>" is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT * FROM decfloat_null_table", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then Result should contain [NULL, 123.456, NULL, -789.012]
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(!get_data_optional<SQL_C_CHAR>(stmt, 1).has_value());

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data_optional<SQL_C_CHAR>(stmt, 1) == "123.456");

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(!get_data_optional<SQL_C_CHAR>(stmt, 1).has_value());

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data_optional<SQL_C_CHAR>(stmt, 1) == "-789.012");
}

TEST_CASE("should download large result set with multiple chunks from table", "[decfloat]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with DECFLOAT column exists with values from 0 to 19999
  conn.execute("CREATE OR REPLACE TABLE decfloat_large_table (col DECFLOAT)");
  conn.execute(
      "INSERT INTO decfloat_large_table "
      "SELECT seq8()::DECFLOAT FROM TABLE(GENERATOR(ROWCOUNT => 20000))");

  // When Query "SELECT * FROM <table>" is executed
  auto stmt = conn.createStatement();
  const auto sql = "SELECT * FROM decfloat_large_table ORDER BY col";
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)sql, SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then Result should contain consecutive numbers from 0 to 19999 returned as appropriate type
  int row_count = 0;
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) {
      break;
    }
    CHECK_ODBC(ret, stmt);

    CHECK(get_data<SQL_C_CHAR>(stmt, 1) == std::to_string(row_count));

    row_count++;
  }

  REQUIRE(row_count == 20000);
}

// ============================================================================
// PARAMETER BINDING
// ============================================================================

TEST_CASE("should select decfloat using parameter binding", "[decfloat]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT ?::DECFLOAT, ?::DECFLOAT, ?::DECFLOAT" is executed with bound DECFLOAT values
  // [123.456, -789.012, 42.0]
  {
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"SELECT ?::DECFLOAT, ?::DECFLOAT, ?::DECFLOAT", SQL_NTS);
    CHECK_ODBC(ret, stmt);

    const char* values[] = {"123.456", "-789.012", "42.0"};
    SQLLEN lens[3];
    for (int i = 0; i < 3; ++i) {
      lens[i] = static_cast<SQLLEN>(strlen(values[i]));
      ret = SQLBindParameter(stmt.getHandle(), static_cast<SQLUSMALLINT>(i + 1), SQL_PARAM_INPUT, SQL_C_CHAR,
                             SQL_VARCHAR, lens[i], 0, (SQLPOINTER)values[i], lens[i], &lens[i]);
      CHECK_ODBC(ret, stmt);
    }

    ret = SQLExecute(stmt.getHandle());
    CHECK_ODBC(ret, stmt);
    ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);

    // Then Result should contain [123.456, -789.012, 42.0]
    CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "123.456");
    CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "-789.012");
    CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "42");
  }

  // When Query "SELECT ?::DECFLOAT" is executed with bound NULL value
  {
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"SELECT ?::DECFLOAT", SQL_NTS);
    CHECK_ODBC(ret, stmt);

    SQLLEN null_indicator = SQL_NULL_DATA;
    ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 0, 0, nullptr, 0,
                           &null_indicator);
    CHECK_ODBC(ret, stmt);

    ret = SQLExecute(stmt.getHandle());
    CHECK_ODBC(ret, stmt);
    ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);

    // Then Result should contain [NULL]
    CHECK(!get_data_optional<SQL_C_CHAR>(stmt, 1).has_value());
  }
}

TEST_CASE("should select case decfloat using parameter binding", "[decfloat]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Given Snowflake client is logged in
  Connection conn;

  {
    INFO("max exponent");
    // When Query "SELECT ?::DECFLOAT" is executed with bound value <value>
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"SELECT ?::DECFLOAT", SQL_NTS);
    CHECK_ODBC(ret, stmt);

    const char* value = "1E+16384";
    SQLLEN len = static_cast<SQLLEN>(strlen(value));
    ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, len, 0, (SQLPOINTER)value,
                           len, &len);
    CHECK_ODBC(ret, stmt);

    ret = SQLExecute(stmt.getHandle());
    CHECK_ODBC(ret, stmt);
    ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);

    // Then Result should contain [<expected>]
    CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "1e16384");
  }

  {
    INFO("large negative exponent");
    // When Query "SELECT ?::DECFLOAT" is executed with bound value <value>
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"SELECT ?::DECFLOAT", SQL_NTS);
    CHECK_ODBC(ret, stmt);

    const char* value = "-1.234E+8000";
    SQLLEN len = static_cast<SQLLEN>(strlen(value));
    ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, len, 0, (SQLPOINTER)value,
                           len, &len);
    CHECK_ODBC(ret, stmt);

    ret = SQLExecute(stmt.getHandle());
    CHECK_ODBC(ret, stmt);
    ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);

    // Then Result should contain [<expected>]
    NEW_DRIVER_ONLY("BD#21") { CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "-1.234e8000"); }
    OLD_DRIVER_ONLY("BD#21") { CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "-1234e7997"); }
  }
}

TEST_CASE("should insert decfloat using parameter binding", "[decfloat]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with DECFLOAT column exists
  conn.execute("CREATE OR REPLACE TABLE decfloat_bind_insert (col DECFLOAT)");

  // When DECFLOAT values [0, 123.456, -789.012, NULL] are inserted using explicit binding
  const char* values[] = {"0", "123.456", "-789.012"};
  for (const auto* val : values) {
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"INSERT INTO decfloat_bind_insert VALUES (?)", SQL_NTS);
    CHECK_ODBC(ret, stmt);

    SQLLEN len = static_cast<SQLLEN>(strlen(val));
    ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, len, 0, (SQLPOINTER)val, len,
                           &len);
    CHECK_ODBC(ret, stmt);
    ret = SQLExecute(stmt.getHandle());
    CHECK_ODBC(ret, stmt);
  }

  {
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"INSERT INTO decfloat_bind_insert VALUES (?)", SQL_NTS);
    CHECK_ODBC(ret, stmt);

    SQLLEN null_indicator = SQL_NULL_DATA;
    ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 0, 0, nullptr, 0,
                           &null_indicator);
    CHECK_ODBC(ret, stmt);
    ret = SQLExecute(stmt.getHandle());
    CHECK_ODBC(ret, stmt);
  }

  // Then SELECT should return the same exact values
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT * FROM decfloat_bind_insert", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data_optional<SQL_C_CHAR>(stmt, 1) == "0");

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data_optional<SQL_C_CHAR>(stmt, 1) == "123.456");

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data_optional<SQL_C_CHAR>(stmt, 1) == "-789.012");

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(!get_data_optional<SQL_C_CHAR>(stmt, 1).has_value());
}

TEST_CASE("should insert extreme decfloat values using parameter binding", "[decfloat]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And Table with DECFLOAT column exists
  conn.execute("CREATE OR REPLACE TABLE decfloat_extreme_bind (col DECFLOAT)");

  // When DECFLOAT values [1E+16384, 1E-16383, -1.234E+8000] are inserted using explicit binding
  const char* values[] = {"1E+16384", "1E-16383", "-1.234E+8000"};
  for (const auto* val : values) {
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"INSERT INTO decfloat_extreme_bind VALUES (?)", SQL_NTS);
    CHECK_ODBC(ret, stmt);

    SQLLEN len = static_cast<SQLLEN>(strlen(val));
    ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, len, 0, (SQLPOINTER)val, len,
                           &len);
    CHECK_ODBC(ret, stmt);
    ret = SQLExecute(stmt.getHandle());
    CHECK_ODBC(ret, stmt);
  }

  // And Query "SELECT * FROM <table>" is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT * FROM decfloat_extreme_bind", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then SELECT should return the same exact values
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "1e16384");

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "1e-16383");

  NEW_DRIVER_ONLY("BD#21") {
    ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);
    CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "-1.234e8000");
  }

  OLD_DRIVER_ONLY("BD#21") {
    ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);
    CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "-1234e7997");
  }
}
