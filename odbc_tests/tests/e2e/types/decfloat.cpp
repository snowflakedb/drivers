#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <optional>
#include <set>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "macros.hpp"

// NOTE: Extreme exponent and scientific notation values may be returned by the
// old driver in a normalized form (e.g. "1e16384" instead of "1E+16384",
// "-1234e7997" instead of "-1.234E+8000"). The expected strings in these tests
// reflect the old driver's actual output. A Behavior Difference entry may be
// needed once the new driver is implemented.

// ============================================================================
// Type casting
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
    SQLCHAR col_name[128] = {0};
    SQLSMALLINT name_length = 0;
    SQLSMALLINT data_type = 0;
    SQLULEN col_size = 0;
    SQLSMALLINT decimal_digits = 0;
    SQLSMALLINT nullable = 0;
    SQLRETURN ret = SQLDescribeCol(stmt.getHandle(), col, col_name, sizeof(col_name), &name_length, &data_type,
                                   &col_size, &decimal_digits, &nullable);
    CHECK_ODBC(ret, stmt);
    INFO("Column " << col << ": data_type=" << data_type);
    CHECK(data_type == SQL_NUMERIC);
  }

  // And Values should maintain full 38-digit precision
  std::vector<std::string> expected = {"0", "123.456", "12300000000000000000000000000000000000",
                                       "12345678901234567890123456789012345678"};
  for (size_t i = 0; i < expected.size(); ++i) {
    auto col = static_cast<SQLUSMALLINT>(i + 1);
    auto value = get_data<SQL_C_CHAR>(stmt, col);
    INFO("Column " << col << ": got '" << value << "', expected '" << expected[i] << "'");
    CHECK(value == expected[i]);
  }
}

// ============================================================================
// SELECT with literals (no tables)
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
  std::vector<std::string> expected = {"0", "1.5", "-1.5", "123.456789", "-987.654321"};
  for (size_t i = 0; i < expected.size(); ++i) {
    auto col = static_cast<SQLUSMALLINT>(i + 1);
    auto value = get_data<SQL_C_CHAR>(stmt, col);
    INFO("Column " << col << ": got '" << value << "', expected '" << expected[i] << "'");
    CHECK(value == expected[i]);
  }
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
  auto col1 = get_data<SQL_C_CHAR>(stmt, 1);
  auto col2 = get_data<SQL_C_CHAR>(stmt, 2);
  auto col3 = get_data<SQL_C_CHAR>(stmt, 3);

  INFO("Column 1: " << col1);
  CHECK(col1.find("12345678901234567890123456789012345678") != std::string::npos);
  INFO("Column 2: " << col2);
  CHECK(col2.find("12345678901234567890123456789012345678") != std::string::npos);
  INFO("Column 3: " << col3);
  CHECK(col3.find("12345678901234567890123456789012345678") != std::string::npos);
}

TEST_CASE("should handle extreme exponent values from literals", "[decfloat]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '1E+16384'::DECFLOAT, '1E-16383'::DECFLOAT" is executed
  auto stmt1 = conn.execute_fetch("SELECT '1E+16384'::DECFLOAT, '1E-16383'::DECFLOAT");

  // Then Result should contain [1E+16384, 1E-16383]
  auto val1 = get_data<SQL_C_CHAR>(stmt1, 1);
  auto val2 = get_data<SQL_C_CHAR>(stmt1, 2);
  INFO("Column 1: " << val1);
  CHECK(val1 == "1e16384");
  INFO("Column 2: " << val2);
  CHECK(val2 == "1e-16383");

  // When Query "SELECT '-1.234E+8000'::DECFLOAT, '9.876E-8000'::DECFLOAT" is executed
  auto stmt2 = conn.execute_fetch("SELECT '-1.234E+8000'::DECFLOAT, '9.876E-8000'::DECFLOAT");

  // Then Result should contain [-1.234E+8000, 9.876E-8000]
  auto val3 = get_data<SQL_C_CHAR>(stmt2, 1);
  auto val4 = get_data<SQL_C_CHAR>(stmt2, 2);
  INFO("Column 1: " << val3);
  CHECK(val3 == "-1234e7997");
  INFO("Column 2: " << val4);
  CHECK(val4 == "9876e-8003");
}

TEST_CASE("should handle NULL values from literals", "[decfloat]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT NULL::DECFLOAT, 42.5::DECFLOAT, NULL::DECFLOAT" is executed
  auto stmt = conn.execute_fetch("SELECT NULL::DECFLOAT, 42.5::DECFLOAT, NULL::DECFLOAT");

  // Then Result should contain [NULL, 42.5, NULL]
  auto val1 = get_data_optional<SQL_C_CHAR>(stmt, 1);
  auto val2 = get_data_optional<SQL_C_CHAR>(stmt, 2);
  auto val3 = get_data_optional<SQL_C_CHAR>(stmt, 3);
  CHECK(!val1.has_value());
  REQUIRE(val2.has_value());
  CHECK(val2.value() == "42.5");
  CHECK(!val3.has_value());
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

  // Then Result should contain consecutive numbers from 0 to 19999
  // And All values should be returned as appropriate type
  int row_count = 0;
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) {
      break;
    }
    CHECK_ODBC(ret, stmt);

    auto value = get_data<SQL_C_CHAR>(stmt, 1);
    INFO("Row " << row_count << ": " << value);
    CHECK(value == std::to_string(row_count));

    row_count++;
  }
  REQUIRE(row_count == 20000);
}

// ============================================================================
// Table operations
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
  std::vector<std::string> expected = {"0", "123.456", "-789.012", "123000000000000000000", "-0.00000000000000987"};
  for (size_t i = 0; i < expected.size(); ++i) {
    ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);
    auto value = get_data<SQL_C_CHAR>(stmt, 1);
    INFO("Row " << i << ": got '" << value << "', expected '" << expected[i] << "'");
    CHECK(value == expected[i]);
  }
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
  for (int i = 0; i < 3; ++i) {
    ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);
    auto value = get_data<SQL_C_CHAR>(stmt, 1);
    INFO("Row " << i << ": " << value);
    CHECK(value.find("12345678901234567890123456789012345678") != std::string::npos);
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
  std::vector<std::string> expected = {"1e16384", "1e-16383", "-1234e7997", "9876e-8003"};
  for (size_t i = 0; i < expected.size(); ++i) {
    ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);
    auto value = get_data<SQL_C_CHAR>(stmt, 1);
    INFO("Row " << i << ": got '" << value << "', expected '" << expected[i] << "'");
    CHECK(value == expected[i]);
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
  std::vector<std::optional<std::string>> expected = {std::nullopt, "123.456", std::nullopt, "-789.012"};
  for (size_t i = 0; i < expected.size(); ++i) {
    ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);
    auto value = get_data_optional<SQL_C_CHAR>(stmt, 1);
    INFO("Row " << i);
    CHECK(value == expected[i]);
  }
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

  // Then Result should contain consecutive numbers from 0 to 19999
  // And All values should be returned as appropriate type
  int row_count = 0;
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) {
      break;
    }
    CHECK_ODBC(ret, stmt);

    auto value = get_data<SQL_C_CHAR>(stmt, 1);
    INFO("Row " << row_count << ": " << value);
    CHECK(value == std::to_string(row_count));

    row_count++;
  }
  REQUIRE(row_count == 20000);
}

// ============================================================================
// Parameter binding
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
    std::vector<std::string> expected = {"123.456", "-789.012", "42"};
    for (size_t i = 0; i < expected.size(); ++i) {
      auto col = static_cast<SQLUSMALLINT>(i + 1);
      auto value = get_data<SQL_C_CHAR>(stmt, col);
      INFO("Column " << col << ": got '" << value << "', expected '" << expected[i] << "'");
      CHECK(value == expected[i]);
    }
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
    auto value = get_data_optional<SQL_C_CHAR>(stmt, 1);
    CHECK(!value.has_value());
  }
}

TEST_CASE("should select extreme decfloat values using parameter binding", "[decfloat]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT ?::DECFLOAT" is executed with bound value 1E+16384
  {
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

    // Then Result should contain [1E+16384]
    auto result = get_data<SQL_C_CHAR>(stmt, 1);
    INFO("Result: " << result);
    CHECK(result == "1e16384");
  }

  // When Query "SELECT ?::DECFLOAT" is executed with bound value -1.234E+8000
  {
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

    // Then Result should contain [-1.234E+8000]
    auto result = get_data<SQL_C_CHAR>(stmt, 1);
    INFO("Result: " << result);
    CHECK(result == "-1234e7997");
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

  std::set<std::optional<std::string>> expected = {"0", "123.456", "-789.012", std::nullopt};
  std::set<std::optional<std::string>> actual;
  for (int i = 0; i < 4; ++i) {
    ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);
    actual.insert(get_data_optional<SQL_C_CHAR>(stmt, 1));
  }
  CHECK(actual == expected);
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
  std::set<std::string> expected = {"1e16384", "1e-16383", "-1234e7997"};
  std::set<std::string> actual;
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) {
      break;
    }
    CHECK_ODBC(ret, stmt);
    actual.insert(get_data<SQL_C_CHAR>(stmt, 1));
  }
  CHECK(actual == expected);
}
