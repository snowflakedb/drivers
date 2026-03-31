// INTERVAL datatype ODBC E2E tests
// Based on: tests/definitions/shared/types/interval.feature
//
// Snowflake INTERVAL has two families:
//   - YEAR/MONTH: stored as signed month count (e.g., "14" for 1 year 2 months).
//   - DAY/TIME: stored as scaled nanosecond duration (e.g., "1000000.000" for 1 second).
// The reference ODBC driver surfaces all INTERVAL types as SQL_VARCHAR with
// numeric string values (column_size=134217728, decimal_digits=0).
//
// The new driver does not yet support INTERVAL Arrow format; most tests
// are skipped via SKIP_NEW_DRIVER_NOT_IMPLEMENTED() until support lands.
#include <sql.h>
#include <sqlext.h>

#include <cstring>
#include <optional>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"

// ============================================================================
// TYPE CASTING
// ============================================================================

TEST_CASE("should cast INTERVAL values to appropriate type for YEAR TO MONTH and DAY TO SECOND", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '1-2'::INTERVAL YEAR TO MONTH, '999999999-11'::INTERVAL YEAR TO MONTH,
  //   '0 0:0:1.2'::INTERVAL DAY TO SECOND, '99999 23:59:59.999999'::INTERVAL DAY TO SECOND" is executed
  auto stmt = conn.execute_fetch(
      "SELECT '1-2'::INTERVAL YEAR TO MONTH, "
      "'999999999-11'::INTERVAL YEAR TO MONTH, "
      "'0 0:0:1.2'::INTERVAL DAY TO SECOND, "
      "'99999 23:59:59.999999'::INTERVAL DAY TO SECOND");

  // Then all INTERVAL values should be returned as appropriate type for the driver
  for (SQLUSMALLINT col = 1; col <= 4; ++col) {
    SQLSMALLINT data_type = 0;
    SQLULEN column_size = 0;
    SQLSMALLINT decimal_digits = 0;
    SQLRETURN ret =
        SQLDescribeCol(stmt.getHandle(), col, nullptr, 0, nullptr, &data_type, &column_size, &decimal_digits, nullptr);
    REQUIRE_ODBC(ret, stmt);
    CHECK(data_type == SQL_VARCHAR);
  }

  // And values should match the canonical numeric representation
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "14");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "11999999999");
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "1200000.000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 4) == "8639999999999999.000");
}

// ============================================================================
// SELECT LITERALS
// ============================================================================

TEST_CASE("should select INTERVAL YEAR TO MONTH literals", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query selecting INTERVAL YEAR TO MONTH literals is executed
  auto stmt = conn.execute_fetch(
      "SELECT '0-0'::INTERVAL YEAR TO MONTH, "
      "'1-2'::INTERVAL YEAR TO MONTH, "
      "'-1-3'::INTERVAL YEAR TO MONTH, "
      "'999999999-11'::INTERVAL YEAR TO MONTH, "
      "'-999999999-11'::INTERVAL YEAR TO MONTH");

  // Then the result should contain expected INTERVAL YEAR TO MONTH literal values in order
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "0");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "14");
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "-15");
  CHECK(get_data<SQL_C_CHAR>(stmt, 4) == "11999999999");
  CHECK(get_data<SQL_C_CHAR>(stmt, 5) == "-11999999999");
}

TEST_CASE("should select INTERVAL DAY TO SECOND literals", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query selecting INTERVAL DAY TO SECOND literals is executed
  auto stmt = conn.execute_fetch(
      "SELECT '0 0:0:0.0'::INTERVAL DAY TO SECOND, "
      "'12 3:4:5.678'::INTERVAL DAY TO SECOND, "
      "'-1 2:3:4.567'::INTERVAL DAY TO SECOND, "
      "'99999 23:59:59.999999'::INTERVAL DAY TO SECOND, "
      "'-99999 23:59:59.999999'::INTERVAL DAY TO SECOND");

  // Then the result should contain expected INTERVAL DAY TO SECOND literal values in order
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "0.000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "1047845678000.000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "-93784567000.000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 4) == "8639999999999999.000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 5) == "-8639999999999999.000");
}

TEST_CASE("should select INTERVAL YEAR literals", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '0'::INTERVAL YEAR, '1'::INTERVAL YEAR, '-1'::INTERVAL YEAR,
  //   '999999999'::INTERVAL YEAR, '-999999999'::INTERVAL YEAR" is executed
  auto stmt = conn.execute_fetch(
      "SELECT '0'::INTERVAL YEAR, '1'::INTERVAL YEAR, '-1'::INTERVAL YEAR, "
      "'999999999'::INTERVAL YEAR, '-999999999'::INTERVAL YEAR");

  // Then the result should contain expected INTERVAL YEAR literal values in order
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "0.0");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "1.2");
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "-1.2");
  CHECK(get_data<SQL_C_CHAR>(stmt, 4) == "1199999998.8");
  CHECK(get_data<SQL_C_CHAR>(stmt, 5) == "-1199999998.8");
}

TEST_CASE("should select INTERVAL MONTH literals", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '0'::INTERVAL MONTH, '1'::INTERVAL MONTH, '-1'::INTERVAL MONTH,
  //   '999999999'::INTERVAL MONTH, '-999999999'::INTERVAL MONTH" is executed
  auto stmt = conn.execute_fetch(
      "SELECT '0'::INTERVAL MONTH, '1'::INTERVAL MONTH, '-1'::INTERVAL MONTH, "
      "'999999999'::INTERVAL MONTH, '-999999999'::INTERVAL MONTH");

  // Then the result should contain expected INTERVAL MONTH literal values in order
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "0.00");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "0.01");
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "-0.01");
  CHECK(get_data<SQL_C_CHAR>(stmt, 4) == "9999999.99");
  CHECK(get_data<SQL_C_CHAR>(stmt, 5) == "-9999999.99");
}

TEST_CASE("should select INTERVAL DAY literals", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '0'::INTERVAL DAY, '1'::INTERVAL DAY, '-1'::INTERVAL DAY,
  //   '999999999'::INTERVAL DAY, '-999999999'::INTERVAL DAY" is executed
  auto stmt = conn.execute_fetch(
      "SELECT '0'::INTERVAL DAY, '1'::INTERVAL DAY, '-1'::INTERVAL DAY, "
      "'999999999'::INTERVAL DAY, '-999999999'::INTERVAL DAY");

  // Then the result should contain expected INTERVAL DAY literal values in order
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "0.000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "86400000.000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "-86400000.000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 4) == "86399999913600000.000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 5) == "-86399999913600000.000000");
}

TEST_CASE("should select INTERVAL HOUR literals", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '0'::INTERVAL HOUR, '1'::INTERVAL HOUR, '-1'::INTERVAL HOUR,
  //   '999999999'::INTERVAL HOUR, '-999999999'::INTERVAL HOUR" is executed
  auto stmt = conn.execute_fetch(
      "SELECT '0'::INTERVAL HOUR, '1'::INTERVAL HOUR, '-1'::INTERVAL HOUR, "
      "'999999999'::INTERVAL HOUR, '-999999999'::INTERVAL HOUR");

  // Then the result should contain expected INTERVAL HOUR literal values in order
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "0.000000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "3600.000000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "-3600.000000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 4) == "3599999996400.000000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 5) == "-3599999996400.000000000");
}

TEST_CASE("should select INTERVAL MINUTE literals", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '0'::INTERVAL MINUTE, '1'::INTERVAL MINUTE, '-1'::INTERVAL MINUTE,
  //   '999999999'::INTERVAL MINUTE, '-999999999'::INTERVAL MINUTE" is executed
  auto stmt = conn.execute_fetch(
      "SELECT '0'::INTERVAL MINUTE, '1'::INTERVAL MINUTE, '-1'::INTERVAL MINUTE, "
      "'999999999'::INTERVAL MINUTE, '-999999999'::INTERVAL MINUTE");

  // Then the result should contain expected INTERVAL MINUTE literal values in order
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "0.00000000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "0.60000000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "-0.60000000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 4) == "599999999.40000000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 5) == "-599999999.40000000000");
}

TEST_CASE("should select INTERVAL SECOND literals", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '0'::INTERVAL SECOND, '1.0'::INTERVAL SECOND, '-1.0'::INTERVAL SECOND,
  //   '999999999.999999'::INTERVAL SECOND, '-999999999.999999'::INTERVAL SECOND" is executed
  auto stmt = conn.execute_fetch(
      "SELECT '0'::INTERVAL SECOND, '1.0'::INTERVAL SECOND, '-1.0'::INTERVAL SECOND, "
      "'999999999.999999'::INTERVAL SECOND, '-999999999.999999'::INTERVAL SECOND");

  // Then the result should contain expected INTERVAL SECOND literal values in order
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "0.000000000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "0.001000000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "-0.001000000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 4) == "999999.999999999000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 5) == "-999999.999999999000");
}

TEST_CASE("should select INTERVAL DAY TO HOUR literals", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '0 0'::INTERVAL DAY TO HOUR, '1 2'::INTERVAL DAY TO HOUR,
  //   '-1 2'::INTERVAL DAY TO HOUR, '999999999 23'::INTERVAL DAY TO HOUR,
  //   '-999999999 23'::INTERVAL DAY TO HOUR" is executed
  auto stmt = conn.execute_fetch(
      "SELECT '0 0'::INTERVAL DAY TO HOUR, '1 2'::INTERVAL DAY TO HOUR, "
      "'-1 2'::INTERVAL DAY TO HOUR, '999999999 23'::INTERVAL DAY TO HOUR, "
      "'-999999999 23'::INTERVAL DAY TO HOUR");

  // Then the result should contain expected INTERVAL DAY TO HOUR literal values in order
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "0.00000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "936000000.00000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "-936000000.00000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 4) == "863999999964000000.00000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 5) == "-863999999964000000.00000");
}

TEST_CASE("should select INTERVAL DAY TO MINUTE literals", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '0 0:0'::INTERVAL DAY TO MINUTE, '1 2:30'::INTERVAL DAY TO MINUTE,
  //   '-1 2:30'::INTERVAL DAY TO MINUTE, '999999999 23:59'::INTERVAL DAY TO MINUTE,
  //   '-999999999 23:59'::INTERVAL DAY TO MINUTE" is executed
  auto stmt = conn.execute_fetch(
      "SELECT '0 0:0'::INTERVAL DAY TO MINUTE, '1 2:30'::INTERVAL DAY TO MINUTE, "
      "'-1 2:30'::INTERVAL DAY TO MINUTE, '999999999 23:59'::INTERVAL DAY TO MINUTE, "
      "'-999999999 23:59'::INTERVAL DAY TO MINUTE");

  // Then the result should contain expected INTERVAL DAY TO MINUTE literal values in order
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "0.0000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "9540000000.0000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "-9540000000.0000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 4) == "8639999999994000000.0000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 5) == "-8639999999994000000.0000");
}

TEST_CASE("should select INTERVAL HOUR TO MINUTE literals", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '0:0'::INTERVAL HOUR TO MINUTE, '1:30'::INTERVAL HOUR TO MINUTE,
  //   '-1:30'::INTERVAL HOUR TO MINUTE, '999999999:59'::INTERVAL HOUR TO MINUTE,
  //   '-999999999:59'::INTERVAL HOUR TO MINUTE" is executed
  auto stmt = conn.execute_fetch(
      "SELECT '0:0'::INTERVAL HOUR TO MINUTE, '1:30'::INTERVAL HOUR TO MINUTE, "
      "'-1:30'::INTERVAL HOUR TO MINUTE, '999999999:59'::INTERVAL HOUR TO MINUTE, "
      "'-999999999:59'::INTERVAL HOUR TO MINUTE");

  // Then the result should contain expected INTERVAL HOUR TO MINUTE literal values in order
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "0.00000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "54000.00000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "-54000.00000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 4) == "35999999999400.00000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 5) == "-35999999999400.00000000");
}

TEST_CASE("should select INTERVAL HOUR TO SECOND literals", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '0:0:0.0'::INTERVAL HOUR TO SECOND, '1:30:45.123'::INTERVAL HOUR TO SECOND,
  //   '-1:30:45.123'::INTERVAL HOUR TO SECOND, '999999999:59:59.999999'::INTERVAL HOUR TO SECOND,
  //   '-999999999:59:59.999999'::INTERVAL HOUR TO SECOND" is executed
  auto stmt = conn.execute_fetch(
      "SELECT '0:0:0.0'::INTERVAL HOUR TO SECOND, '1:30:45.123'::INTERVAL HOUR TO SECOND, "
      "'-1:30:45.123'::INTERVAL HOUR TO SECOND, '999999999:59:59.999999'::INTERVAL HOUR TO SECOND, "
      "'-999999999:59:59.999999'::INTERVAL HOUR TO SECOND");

  // Then the result should contain expected INTERVAL HOUR TO SECOND literal values in order
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "0.0000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "544512.3000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "-544512.3000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 4) == "359999999999999.9999000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 5) == "-359999999999999.9999000");
}

TEST_CASE("should select INTERVAL MINUTE TO SECOND literals", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '0:0.0'::INTERVAL MINUTE TO SECOND, '30:45.123'::INTERVAL MINUTE TO SECOND,
  //   '-30:45.123'::INTERVAL MINUTE TO SECOND, '999999999:59.999999'::INTERVAL MINUTE TO SECOND,
  //   '-999999999:59.999999'::INTERVAL MINUTE TO SECOND" is executed
  auto stmt = conn.execute_fetch(
      "SELECT '0:0.0'::INTERVAL MINUTE TO SECOND, '30:45.123'::INTERVAL MINUTE TO SECOND, "
      "'-30:45.123'::INTERVAL MINUTE TO SECOND, '999999999:59.999999'::INTERVAL MINUTE TO SECOND, "
      "'-999999999:59.999999'::INTERVAL MINUTE TO SECOND");

  // Then the result should contain expected INTERVAL MINUTE TO SECOND literal values in order
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "0.0000000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "184.5123000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "-184.5123000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 4) == "5999999999.9999999000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 5) == "-5999999999.9999999000");
}

TEST_CASE("should select NULL INTERVAL literals", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT NULL::INTERVAL YEAR TO MONTH, NULL::INTERVAL DAY TO SECOND,
  //   NULL::INTERVAL YEAR, NULL::INTERVAL SECOND" is executed
  auto stmt = conn.execute_fetch(
      "SELECT NULL::INTERVAL YEAR TO MONTH, NULL::INTERVAL DAY TO SECOND, "
      "NULL::INTERVAL YEAR, NULL::INTERVAL SECOND");

  // Then the result should contain:
  CHECK(get_data_optional<SQL_C_CHAR>(stmt, 1) == std::nullopt);
  CHECK(get_data_optional<SQL_C_CHAR>(stmt, 2) == std::nullopt);
  CHECK(get_data_optional<SQL_C_CHAR>(stmt, 3) == std::nullopt);
  CHECK(get_data_optional<SQL_C_CHAR>(stmt, 4) == std::nullopt);
}

TEST_CASE("should treat INTERVAL without explicit part as seconds", "[interval]") {
  SKIP_NEW_DRIVER("INTERVAL", "07006: TIMESTAMP + INTERVAL result cannot be read as SQL_C_CHAR");
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '2024-04-15 12:00:00'::TIMESTAMP + INTERVAL '2' AS d1,
  //   '2024-04-15 12:00:00'::TIMESTAMP + INTERVAL '2 seconds' AS d2" is executed
  const auto stmt = conn.execute_fetch(
      "SELECT '2024-04-15 12:00:00'::TIMESTAMP + INTERVAL '2' AS d1, "
      "'2024-04-15 12:00:00'::TIMESTAMP + INTERVAL '2 seconds' AS d2");

  // Then the result should contain:
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "2024-04-15 12:00:02");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "2024-04-15 12:00:02");
}

// ============================================================================
// SELECT FROM TABLE
// ============================================================================

TEST_CASE("should select INTERVAL YEAR TO MONTH values from table", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And A temporary table with INTERVAL YEAR TO MONTH column is created
  conn.execute("CREATE TABLE interval_ym_table (C1 INTERVAL YEAR TO MONTH)");

  // And The table is populated with YEAR TO MONTH values including corner cases
  conn.execute(
      "INSERT INTO interval_ym_table VALUES "
      "('-999999999-11'), ('-1-3'), ('0-0'), ('1-2'), ('999999999-11'), (NULL)");

  // When Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM interval_ym_table ORDER BY C1 NULLS LAST"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then the result should contain the inserted INTERVAL YEAR TO MONTH values in order
  std::vector<std::optional<std::string>> results;
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) break;
    REQUIRE_ODBC(ret, stmt);
    results.push_back(get_data_optional<SQL_C_CHAR>(stmt, 1));
  }

  REQUIRE(results.size() == 6);
  CHECK(results[0] == "-11999999999");
  CHECK(results[1] == "-15");
  CHECK(results[2] == "0");
  CHECK(results[3] == "14");
  CHECK(results[4] == "11999999999");
  CHECK(results[5] == std::nullopt);
}

TEST_CASE("should select INTERVAL DAY TO SECOND values from table", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And A temporary table with INTERVAL DAY TO SECOND column is created
  conn.execute("CREATE TABLE interval_dt_table (C1 INTERVAL DAY TO SECOND)");

  // And The table is populated with DAY TO SECOND values including corner cases
  conn.execute(
      "INSERT INTO interval_dt_table VALUES "
      "('0 0:0:0.0'), ('12 3:4:5.678'), ('-1 2:3:4.567'), "
      "('99999 23:59:59.999999'), ('-99999 23:59:59.999999'), (NULL)");

  // When Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM interval_dt_table ORDER BY C1 NULLS LAST"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then the result should contain the inserted INTERVAL DAY TO SECOND values in order
  std::vector<std::optional<std::string>> results;
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) break;
    REQUIRE_ODBC(ret, stmt);
    results.push_back(get_data_optional<SQL_C_CHAR>(stmt, 1));
  }

  REQUIRE(results.size() == 6);
  CHECK(results[0] == "-8639999999999999.000");
  CHECK(results[1] == "-93784567000.000");
  CHECK(results[2] == "0.000");
  CHECK(results[3] == "1047845678000.000");
  CHECK(results[4] == "8639999999999999.000");
  CHECK(results[5] == std::nullopt);
}

TEST_CASE("should select INTERVAL YEAR(2) TO MONTH values from table", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And A temporary table with INTERVAL YEAR(2) TO MONTH column is created
  conn.execute("CREATE TABLE interval_ym2_table (C1 INTERVAL YEAR(2) TO MONTH)");

  // And The table is populated with values ['0-0', '1-2', '-1-3', '99-11', '-99-11', NULL]
  conn.execute(
      "INSERT INTO interval_ym2_table VALUES "
      "('0-0'), ('1-2'), ('-1-3'), ('99-11'), ('-99-11'), (NULL)");

  // When Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM interval_ym2_table ORDER BY C1 NULLS LAST"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then the result should contain the inserted INTERVAL YEAR(2) TO MONTH values in order
  std::vector<std::optional<std::string>> results;
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) break;
    REQUIRE_ODBC(ret, stmt);
    results.push_back(get_data_optional<SQL_C_CHAR>(stmt, 1));
  }

  REQUIRE(results.size() == 6);
  CHECK(results[0] == "-1199");
  CHECK(results[1] == "-15");
  CHECK(results[2] == "0");
  CHECK(results[3] == "14");
  CHECK(results[4] == "1199");
  CHECK(results[5] == std::nullopt);
}

TEST_CASE("should select INTERVAL YEAR(7) TO MONTH values from table", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And A temporary table with INTERVAL YEAR(7) TO MONTH column is created
  conn.execute("CREATE TABLE interval_ym7_table (C1 INTERVAL YEAR(7) TO MONTH)");

  // And The table is populated with values ['0-0', '1-2', '-1-3', '9999999-11', '-9999999-11', NULL]
  conn.execute(
      "INSERT INTO interval_ym7_table VALUES "
      "('0-0'), ('1-2'), ('-1-3'), ('9999999-11'), ('-9999999-11'), (NULL)");

  // When Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM interval_ym7_table ORDER BY C1 NULLS LAST"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then the result should contain the inserted INTERVAL YEAR(7) TO MONTH values in order
  std::vector<std::optional<std::string>> results;
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) break;
    REQUIRE_ODBC(ret, stmt);
    results.push_back(get_data_optional<SQL_C_CHAR>(stmt, 1));
  }

  REQUIRE(results.size() == 6);
  CHECK(results[0] == "-119999999");
  CHECK(results[1] == "-15");
  CHECK(results[2] == "0");
  CHECK(results[3] == "14");
  CHECK(results[4] == "119999999");
  CHECK(results[5] == std::nullopt);
}

TEST_CASE("should select INTERVAL DAY(3) TO SECOND values from table", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And A temporary table with INTERVAL DAY(3) TO SECOND column is created
  conn.execute("CREATE TABLE interval_dt3_table (C1 INTERVAL DAY(3) TO SECOND)");

  // And The table is populated with values ['0 0:0:0.0', '1 2:3:4.567', '-1 2:3:4.567',
  //   '999 23:59:59.999999', '-999 23:59:59.999999', NULL]
  conn.execute(
      "INSERT INTO interval_dt3_table VALUES "
      "('0 0:0:0.0'), ('1 2:3:4.567'), ('-1 2:3:4.567'), "
      "('999 23:59:59.999999'), ('-999 23:59:59.999999'), (NULL)");

  // When Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM interval_dt3_table ORDER BY C1 NULLS LAST"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then the result should contain the inserted INTERVAL DAY(3) TO SECOND values in order
  std::vector<std::optional<std::string>> results;
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) break;
    REQUIRE_ODBC(ret, stmt);
    results.push_back(get_data_optional<SQL_C_CHAR>(stmt, 1));
  }

  REQUIRE(results.size() == 6);
  CHECK(results[0] == "-86399999999999.000");
  CHECK(results[1] == "-93784567000.000");
  CHECK(results[2] == "0.000");
  CHECK(results[3] == "93784567000.000");
  CHECK(results[4] == "86399999999999.000");
  CHECK(results[5] == std::nullopt);
}

// ============================================================================
// BINDING
// ============================================================================

TEST_CASE("should insert and select back INTERVAL YEAR TO MONTH values using parameter binding", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And A temporary table with INTERVAL YEAR TO MONTH column is created
  conn.execute("CREATE TABLE interval_bind_ym (C1 INTERVAL YEAR TO MONTH)");

  // When INTERVAL YEAR TO MONTH values ['0-0', '1-2', '-1-3', '999999999-11', '-999999999-11', NULL]
  //   are inserted using parameter binding
  std::vector<std::optional<std::string>> values = {"0-0",          "1-2",           "-1-3",
                                                    "999999999-11", "-999999999-11", std::nullopt};

  for (const auto& val : values) {
    auto insert_stmt = conn.createStatement();
    char buf[64] = {};
    SQLLEN ind = SQL_NULL_DATA;

    if (val.has_value()) {
      std::strncpy(buf, val->c_str(), sizeof(buf) - 1);
      ind = static_cast<SQLLEN>(val->size());
    }

    SQLRETURN ret = SQLBindParameter(insert_stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, sizeof(buf),
                                     0, buf, sizeof(buf), &ind);
    REQUIRE_ODBC(ret, insert_stmt);

    ret = SQLExecDirect(insert_stmt.getHandle(), sqlchar("INSERT INTO interval_bind_ym VALUES (?)"), SQL_NTS);
    REQUIRE_ODBC(ret, insert_stmt);
  }

  // And Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM interval_bind_ym ORDER BY C1 NULLS LAST"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then the result should contain the bound INTERVAL YEAR TO MONTH values
  //   ['-999999999-11', '-1-3', '0-0', '1-2', '999999999-11', NULL]
  std::vector<std::optional<std::string>> results;
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) break;
    REQUIRE_ODBC(ret, stmt);
    results.push_back(get_data_optional<SQL_C_CHAR>(stmt, 1));
  }

  REQUIRE(results.size() == 6);
  CHECK(results[0] == "-11999999999");
  CHECK(results[1] == "-15");
  CHECK(results[2] == "0");
  CHECK(results[3] == "14");
  CHECK(results[4] == "11999999999");
  CHECK(results[5] == std::nullopt);
}

TEST_CASE("should insert and select back INTERVAL DAY TO SECOND values using parameter binding", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And A temporary table with INTERVAL DAY TO SECOND column is created
  conn.execute("CREATE TABLE interval_bind_dt (C1 INTERVAL DAY TO SECOND)");

  // When INTERVAL DAY TO SECOND values ['0 0:0:0.0', '12 3:4:5.678', '-1 2:3:4.567',
  //   '99999 23:59:59.999999', '-99999 23:59:59.999999', NULL] are inserted using parameter binding
  std::vector<std::optional<std::string>> values = {
      "0 0:0:0.0", "12 3:4:5.678", "-1 2:3:4.567", "99999 23:59:59.999999", "-99999 23:59:59.999999", std::nullopt};

  for (const auto& val : values) {
    auto insert_stmt = conn.createStatement();
    char buf[64] = {};
    SQLLEN ind = SQL_NULL_DATA;

    if (val.has_value()) {
      std::strncpy(buf, val->c_str(), sizeof(buf) - 1);
      ind = static_cast<SQLLEN>(val->size());
    }

    SQLRETURN ret = SQLBindParameter(insert_stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, sizeof(buf),
                                     0, buf, sizeof(buf), &ind);
    REQUIRE_ODBC(ret, insert_stmt);

    ret = SQLExecDirect(insert_stmt.getHandle(), sqlchar("INSERT INTO interval_bind_dt VALUES (?)"), SQL_NTS);
    REQUIRE_ODBC(ret, insert_stmt);
  }

  // And Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM interval_bind_dt ORDER BY C1 NULLS LAST"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then the result should contain the bound INTERVAL DAY TO SECOND values
  //   ['-99999 23:59:59.999999', '-1 2:3:4.567', '0 0:0:0.0', '12 3:4:5.678',
  //   '99999 23:59:59.999999', NULL]
  std::vector<std::optional<std::string>> results;
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) break;
    REQUIRE_ODBC(ret, stmt);
    results.push_back(get_data_optional<SQL_C_CHAR>(stmt, 1));
  }

  REQUIRE(results.size() == 6);
  CHECK(results[0] == "-8639999999999999.000");
  CHECK(results[1] == "-93784567000.000");
  CHECK(results[2] == "0.000");
  CHECK(results[3] == "1047845678000.000");
  CHECK(results[4] == "8639999999999999.000");
  CHECK(results[5] == std::nullopt);
}

TEST_CASE("should select INTERVAL YEAR TO MONTH values using parameter binding", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT ?::INTERVAL YEAR TO MONTH, ?::INTERVAL YEAR TO MONTH,
  //   ?::INTERVAL YEAR TO MONTH" is executed with bound string values ['0-0', '1-2', '999999999-11']
  auto stmt = conn.createStatement();

  char v1[] = "0-0";
  char v2[] = "1-2";
  char v3[] = "999999999-11";
  auto ind1 = static_cast<SQLLEN>(std::strlen(v1));
  auto ind2 = static_cast<SQLLEN>(std::strlen(v2));
  auto ind3 = static_cast<SQLLEN>(std::strlen(v3));

  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind1, 0, v1, sizeof(v1), &ind1);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind2, 0, v2, sizeof(v2), &ind2);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 3, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind3, 0, v3, sizeof(v3), &ind3);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(),
                      sqlchar("SELECT ?::INTERVAL YEAR TO MONTH, ?::INTERVAL YEAR TO MONTH, ?::INTERVAL YEAR TO MONTH"),
                      SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the result should contain:
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "0");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "14");
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "11999999999");
}

TEST_CASE("should select INTERVAL DAY TO SECOND values using parameter binding", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT ?::INTERVAL DAY TO SECOND, ?::INTERVAL DAY TO SECOND,
  //   ?::INTERVAL DAY TO SECOND" is executed with bound string values
  //   ['0 0:0:0.0', '12 3:4:5.678', '99999 23:59:59.999999']
  auto stmt = conn.createStatement();

  char v1[] = "0 0:0:0.0";
  char v2[] = "12 3:4:5.678";
  char v3[] = "99999 23:59:59.999999";
  auto ind1 = static_cast<SQLLEN>(std::strlen(v1));
  auto ind2 = static_cast<SQLLEN>(std::strlen(v2));
  auto ind3 = static_cast<SQLLEN>(std::strlen(v3));

  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind1, 0, v1, sizeof(v1), &ind1);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind2, 0, v2, sizeof(v2), &ind2);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 3, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind3, 0, v3, sizeof(v3), &ind3);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(),
                      sqlchar("SELECT ?::INTERVAL DAY TO SECOND, ?::INTERVAL DAY TO SECOND, ?::INTERVAL DAY TO SECOND"),
                      SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the result should contain:
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "0.000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "1047845678000.000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "8639999999999999.000");
}

TEST_CASE("should select NULL INTERVAL values using parameter binding", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT ?::INTERVAL YEAR TO MONTH, ?::INTERVAL DAY TO SECOND"
  //   is executed with bound NULL values
  auto stmt = conn.createStatement();

  char buf1[64] = {};
  char buf2[64] = {};
  SQLLEN ind1 = SQL_NULL_DATA, ind2 = SQL_NULL_DATA;

  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 64, 0, buf1, sizeof(buf1), &ind1);
  REQUIRE_ODBC(ret, stmt);
  ret =
      SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 64, 0, buf2, sizeof(buf2), &ind2);
  REQUIRE_ODBC(ret, stmt);

  ret =
      SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ?::INTERVAL YEAR TO MONTH, ?::INTERVAL DAY TO SECOND"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the result should contain:
  CHECK(get_data_optional<SQL_C_CHAR>(stmt, 1) == std::nullopt);
  CHECK(get_data_optional<SQL_C_CHAR>(stmt, 2) == std::nullopt);
}

TEST_CASE("should select INTERVAL YEAR values using parameter binding", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT ?::INTERVAL YEAR, ?::INTERVAL YEAR, ?::INTERVAL YEAR"
  //   is executed with bound string values ['0', '2', '-999999999']
  auto stmt = conn.createStatement();

  char v1[] = "0";
  char v2[] = "2";
  char v3[] = "-999999999";
  auto ind1 = static_cast<SQLLEN>(std::strlen(v1));
  auto ind2 = static_cast<SQLLEN>(std::strlen(v2));
  auto ind3 = static_cast<SQLLEN>(std::strlen(v3));

  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind1, 0, v1, sizeof(v1), &ind1);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind2, 0, v2, sizeof(v2), &ind2);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 3, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind3, 0, v3, sizeof(v3), &ind3);
  REQUIRE_ODBC(ret, stmt);

  ret =
      SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ?::INTERVAL YEAR, ?::INTERVAL YEAR, ?::INTERVAL YEAR"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the result should contain expected INTERVAL YEAR bound values in order
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "0.0");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "2.4");
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "-1199999998.8");
}

TEST_CASE("should select INTERVAL MONTH values using parameter binding", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT ?::INTERVAL MONTH, ?::INTERVAL MONTH, ?::INTERVAL MONTH"
  //   is executed with bound string values ['0', '5', '-999999999']
  auto stmt = conn.createStatement();

  char v1[] = "0";
  char v2[] = "5";
  char v3[] = "-999999999";
  auto ind1 = static_cast<SQLLEN>(std::strlen(v1));
  auto ind2 = static_cast<SQLLEN>(std::strlen(v2));
  auto ind3 = static_cast<SQLLEN>(std::strlen(v3));

  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind1, 0, v1, sizeof(v1), &ind1);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind2, 0, v2, sizeof(v2), &ind2);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 3, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind3, 0, v3, sizeof(v3), &ind3);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ?::INTERVAL MONTH, ?::INTERVAL MONTH, ?::INTERVAL MONTH"),
                      SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the result should contain expected INTERVAL MONTH bound values in order
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "0.00");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "0.05");
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "-9999999.99");
}

TEST_CASE("should select INTERVAL DAY values using parameter binding", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT ?::INTERVAL DAY, ?::INTERVAL DAY, ?::INTERVAL DAY"
  //   is executed with bound string values ['0', '1', '-999999999']
  auto stmt = conn.createStatement();

  char v1[] = "0";
  char v2[] = "1";
  char v3[] = "-999999999";
  auto ind1 = static_cast<SQLLEN>(std::strlen(v1));
  auto ind2 = static_cast<SQLLEN>(std::strlen(v2));
  auto ind3 = static_cast<SQLLEN>(std::strlen(v3));

  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind1, 0, v1, sizeof(v1), &ind1);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind2, 0, v2, sizeof(v2), &ind2);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 3, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind3, 0, v3, sizeof(v3), &ind3);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ?::INTERVAL DAY, ?::INTERVAL DAY, ?::INTERVAL DAY"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the result should contain expected INTERVAL DAY bound values in order
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "0.000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "86400000.000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "-86399999913600000.000000");
}

TEST_CASE("should select INTERVAL HOUR values using parameter binding", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT ?::INTERVAL HOUR, ?::INTERVAL HOUR, ?::INTERVAL HOUR"
  //   is executed with bound string values ['0', '5', '-999999999']
  auto stmt = conn.createStatement();

  char v1[] = "0";
  char v2[] = "5";
  char v3[] = "-999999999";
  auto ind1 = static_cast<SQLLEN>(std::strlen(v1));
  auto ind2 = static_cast<SQLLEN>(std::strlen(v2));
  auto ind3 = static_cast<SQLLEN>(std::strlen(v3));

  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind1, 0, v1, sizeof(v1), &ind1);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind2, 0, v2, sizeof(v2), &ind2);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 3, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind3, 0, v3, sizeof(v3), &ind3);
  REQUIRE_ODBC(ret, stmt);

  ret =
      SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ?::INTERVAL HOUR, ?::INTERVAL HOUR, ?::INTERVAL HOUR"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the result should contain expected INTERVAL HOUR bound values in order
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "0.000000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "18000.000000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "-3599999996400.000000000");
}

TEST_CASE("should select INTERVAL MINUTE values using parameter binding", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT ?::INTERVAL MINUTE, ?::INTERVAL MINUTE, ?::INTERVAL MINUTE"
  //   is executed with bound string values ['0', '4', '-999999999']
  auto stmt = conn.createStatement();

  char v1[] = "0";
  char v2[] = "4";
  char v3[] = "-999999999";
  auto ind1 = static_cast<SQLLEN>(std::strlen(v1));
  auto ind2 = static_cast<SQLLEN>(std::strlen(v2));
  auto ind3 = static_cast<SQLLEN>(std::strlen(v3));

  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind1, 0, v1, sizeof(v1), &ind1);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind2, 0, v2, sizeof(v2), &ind2);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 3, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind3, 0, v3, sizeof(v3), &ind3);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ?::INTERVAL MINUTE, ?::INTERVAL MINUTE, ?::INTERVAL MINUTE"),
                      SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the result should contain expected INTERVAL MINUTE bound values in order
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "0.00000000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "2.40000000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "-599999999.40000000000");
}

TEST_CASE("should select INTERVAL SECOND values using parameter binding", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT ?::INTERVAL SECOND, ?::INTERVAL SECOND, ?::INTERVAL SECOND"
  //   is executed with bound string values ['0', '8.5', '-999999999.999999']
  auto stmt = conn.createStatement();

  char v1[] = "0";
  char v2[] = "8.5";
  char v3[] = "-999999999.999999";
  auto ind1 = static_cast<SQLLEN>(std::strlen(v1));
  auto ind2 = static_cast<SQLLEN>(std::strlen(v2));
  auto ind3 = static_cast<SQLLEN>(std::strlen(v3));

  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind1, 0, v1, sizeof(v1), &ind1);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind2, 0, v2, sizeof(v2), &ind2);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 3, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind3, 0, v3, sizeof(v3), &ind3);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ?::INTERVAL SECOND, ?::INTERVAL SECOND, ?::INTERVAL SECOND"),
                      SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the result should contain expected INTERVAL SECOND bound values in order
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "0.000000000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "0.008500000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "-999999.999999999000");
}

TEST_CASE("should select INTERVAL DAY TO HOUR values using parameter binding", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT ?::INTERVAL DAY TO HOUR, ?::INTERVAL DAY TO HOUR"
  //   is executed with bound string values ['1 2', '-999999999 23']
  auto stmt = conn.createStatement();

  char v1[] = "1 2";
  char v2[] = "-999999999 23";
  auto ind1 = static_cast<SQLLEN>(std::strlen(v1));
  auto ind2 = static_cast<SQLLEN>(std::strlen(v2));

  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind1, 0, v1, sizeof(v1), &ind1);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind2, 0, v2, sizeof(v2), &ind2);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ?::INTERVAL DAY TO HOUR, ?::INTERVAL DAY TO HOUR"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the result should contain expected INTERVAL DAY TO HOUR bound values in order
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "936000000.00000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "-863999999964000000.00000");
}

TEST_CASE("should select INTERVAL DAY TO MINUTE values using parameter binding", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT ?::INTERVAL DAY TO MINUTE, ?::INTERVAL DAY TO MINUTE"
  //   is executed with bound string values ['1 2:30', '-999999999 23:59']
  auto stmt = conn.createStatement();

  char v1[] = "1 2:30";
  char v2[] = "-999999999 23:59";
  auto ind1 = static_cast<SQLLEN>(std::strlen(v1));
  auto ind2 = static_cast<SQLLEN>(std::strlen(v2));

  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind1, 0, v1, sizeof(v1), &ind1);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind2, 0, v2, sizeof(v2), &ind2);
  REQUIRE_ODBC(ret, stmt);

  ret =
      SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ?::INTERVAL DAY TO MINUTE, ?::INTERVAL DAY TO MINUTE"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the result should contain expected INTERVAL DAY TO MINUTE bound values in order
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "9540000000.0000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "-8639999999994000000.0000");
}

TEST_CASE("should select INTERVAL HOUR TO MINUTE values using parameter binding", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT ?::INTERVAL HOUR TO MINUTE, ?::INTERVAL HOUR TO MINUTE"
  //   is executed with bound string values ['1:30', '-999999999:59']
  auto stmt = conn.createStatement();

  char v1[] = "1:30";
  char v2[] = "-999999999:59";
  auto ind1 = static_cast<SQLLEN>(std::strlen(v1));
  auto ind2 = static_cast<SQLLEN>(std::strlen(v2));

  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind1, 0, v1, sizeof(v1), &ind1);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind2, 0, v2, sizeof(v2), &ind2);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ?::INTERVAL HOUR TO MINUTE, ?::INTERVAL HOUR TO MINUTE"),
                      SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the result should contain expected INTERVAL HOUR TO MINUTE bound values in order
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "54000.00000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "-35999999999400.00000000");
}

TEST_CASE("should select INTERVAL HOUR TO SECOND values using parameter binding", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT ?::INTERVAL HOUR TO SECOND, ?::INTERVAL HOUR TO SECOND"
  //   is executed with bound string values ['1:30:45.123', '-999999999:59:59.999999']
  auto stmt = conn.createStatement();

  char v1[] = "1:30:45.123";
  char v2[] = "-999999999:59:59.999999";
  auto ind1 = static_cast<SQLLEN>(std::strlen(v1));
  auto ind2 = static_cast<SQLLEN>(std::strlen(v2));

  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind1, 0, v1, sizeof(v1), &ind1);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind2, 0, v2, sizeof(v2), &ind2);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ?::INTERVAL HOUR TO SECOND, ?::INTERVAL HOUR TO SECOND"),
                      SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the result should contain expected INTERVAL HOUR TO SECOND bound values in order
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "544512.3000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "-359999999999999.9999000");
}

TEST_CASE("should select INTERVAL MINUTE TO SECOND values using parameter binding", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT ?::INTERVAL MINUTE TO SECOND, ?::INTERVAL MINUTE TO SECOND"
  //   is executed with bound string values ['30:45.123', '-999999999:59.999999']
  auto stmt = conn.createStatement();

  char v1[] = "30:45.123";
  char v2[] = "-999999999:59.999999";
  auto ind1 = static_cast<SQLLEN>(std::strlen(v1));
  auto ind2 = static_cast<SQLLEN>(std::strlen(v2));

  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind1, 0, v1, sizeof(v1), &ind1);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, ind2, 0, v2, sizeof(v2), &ind2);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ?::INTERVAL MINUTE TO SECOND, ?::INTERVAL MINUTE TO SECOND"),
                      SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the result should contain expected INTERVAL MINUTE TO SECOND bound values in order
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "184.5123000000");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "-5999999999.9999999000");
}

// ============================================================================
// MULTIPLE CHUNKS DOWNLOADING
// ============================================================================

TEST_CASE("should download INTERVAL YEAR TO MONTH data in multiple chunks", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '0-1'::INTERVAL YEAR TO MONTH * SEQ4() AS ym
  //   FROM TABLE(GENERATOR(ROWCOUNT => 50000)) v ORDER BY ym" is executed
  auto stmt = conn.createStatement();
  const auto sql =
      "SELECT '0-1'::INTERVAL YEAR TO MONTH * SEQ4() AS ym "
      "FROM TABLE(GENERATOR(ROWCOUNT => 50000)) v ORDER BY ym";
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar(sql), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then there are 50000 rows returned
  int row_count = 0;

  // And all returned INTERVAL YEAR TO MONTH values should form a sequential series of months starting at 0
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) break;
    REQUIRE_ODBC(ret, stmt);

    char buf[128] = {};
    SQLLEN indicator = 0;
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buf, sizeof(buf), &indicator);
    REQUIRE_ODBC(ret, stmt);

    std::string expected = std::to_string(row_count);
    CHECK(std::string(buf) == expected);

    row_count++;
  }

  REQUIRE(row_count == 50000);
}

TEST_CASE("should download INTERVAL DAY TO SECOND data in multiple chunks", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '0 0:0:1.0'::INTERVAL DAY TO SECOND * SEQ4() AS dt
  //   FROM TABLE(GENERATOR(ROWCOUNT => 50000)) v ORDER BY dt" is executed
  auto stmt = conn.createStatement();
  const auto sql =
      "SELECT '0 0:0:1.0'::INTERVAL DAY TO SECOND * SEQ4() AS dt "
      "FROM TABLE(GENERATOR(ROWCOUNT => 50000)) v ORDER BY dt";
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar(sql), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then there are 50000 rows returned
  int row_count = 0;

  // And all returned INTERVAL DAY TO SECOND values should form a sequential series of seconds starting at 0
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) break;
    REQUIRE_ODBC(ret, stmt);

    char buf[128] = {};
    SQLLEN indicator = 0;
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buf, sizeof(buf), &indicator);
    REQUIRE_ODBC(ret, stmt);

    std::string expected = std::to_string(static_cast<long long>(row_count) * 1000000LL) + ".000";
    CHECK(std::string(buf) == expected);

    row_count++;
  }

  REQUIRE(row_count == 50000);
}

// ============================================================================
// INTERVAL ARITHMETIC
// ============================================================================

TEST_CASE("should respect order of interval components in date arithmetic", "[interval]") {
  SKIP_NEW_DRIVER("INTERVAL", "07006: DATE + INTERVAL result cannot be read as SQL_C_CHAR");
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT TO_DATE('2019-02-28') + INTERVAL '1 day, 1 year' AS d1,
  //   TO_DATE('2019-02-28') + INTERVAL '1 year, 1 day' AS d2" is executed
  auto stmt = conn.execute_fetch(
      "SELECT TO_DATE('2019-02-28') + INTERVAL '1 day, 1 year' AS d1, "
      "TO_DATE('2019-02-28') + INTERVAL '1 year, 1 day' AS d2");

  // Then the result should contain:
  auto d1 = get_data<SQL_C_CHAR>(stmt, 1);
  auto d2 = get_data<SQL_C_CHAR>(stmt, 2);
  CHECK(d1 == "2020-03-01");
  CHECK(d2 == "2020-02-29");
}

TEST_CASE("should support complex INTERVAL with mixed units and abbreviations", "[interval]") {
  SKIP_NEW_DRIVER("INTERVAL", "07006: DATE + INTERVAL result cannot be read as SQL_C_CHAR");
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT TO_DATE('2025-01-17') + INTERVAL '1 y, 3 q, 4 mm, 5 w, 6 d, 7 h, 9 m, 8 s,
  //   1000 ms, 445343232 us, 898498273498 ns' AS complex_interval" is executed
  const auto stmt = conn.execute_fetch(
      "SELECT TO_DATE('2025-01-17') + INTERVAL "
      "'1 y, 3 q, 4 mm, 5 w, 6 d, 7 h, 9 m, 8 s, "
      "1000 ms, 445343232 us, 898498273498 ns' AS complex_interval");

  // Then the result should contain:
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "2027-03-30 07:31:32.841505498");
}

TEST_CASE("should add two INTERVAL YEAR TO MONTH values", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '1-2'::INTERVAL YEAR TO MONTH + '0-3'::INTERVAL YEAR TO MONTH AS i" is executed
  const auto stmt = conn.execute_fetch("SELECT '1-2'::INTERVAL YEAR TO MONTH + '0-3'::INTERVAL YEAR TO MONTH AS i");

  // Then the result should contain expected INTERVAL YEAR TO MONTH value '1-5'
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "17");
}

TEST_CASE("should add two INTERVAL DAY TO SECOND values", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '1 2:30:00.0'::INTERVAL DAY TO SECOND + '0 1:45:30.5'::INTERVAL DAY TO SECOND AS i" is executed
  const auto stmt =
      conn.execute_fetch("SELECT '1 2:30:00.0'::INTERVAL DAY TO SECOND + '0 1:45:30.5'::INTERVAL DAY TO SECOND AS i");

  // Then the result should contain expected INTERVAL DAY TO SECOND value '1 4:15:30.500000'
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "101730500000.000");
}

TEST_CASE("should negate an INTERVAL value", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT -('1-6'::INTERVAL YEAR TO MONTH) AS ym,
  //   -('3 12:0:0.0'::INTERVAL DAY TO SECOND) AS dt" is executed
  const auto stmt = conn.execute_fetch(
      "SELECT -('1-6'::INTERVAL YEAR TO MONTH) AS ym, "
      "-('3 12:0:0.0'::INTERVAL DAY TO SECOND) AS dt");

  // Then the result should contain expected negated INTERVAL values '-1-6' and '-3 12:0:0.000000'
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "-18");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "-302400000000.000");
}

TEST_CASE("should subtract two INTERVAL values", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '1-5'::INTERVAL YEAR TO MONTH - '0-3'::INTERVAL YEAR TO MONTH AS ym,
  //   '1 4:15:30.5'::INTERVAL DAY TO SECOND - '0 1:45:30.5'::INTERVAL DAY TO SECOND AS dt" is executed
  const auto stmt = conn.execute_fetch(
      "SELECT '1-5'::INTERVAL YEAR TO MONTH - '0-3'::INTERVAL YEAR TO MONTH AS ym, "
      "'1 4:15:30.5'::INTERVAL DAY TO SECOND - '0 1:45:30.5'::INTERVAL DAY TO SECOND AS dt");

  // Then the result should contain expected INTERVAL values '1-2' and '1 2:30:00.000000'
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "14");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "95400000000.000");
}

TEST_CASE("should multiply INTERVAL by a scalar", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '0-6'::INTERVAL YEAR TO MONTH * 3 AS ym,
  //   2 * '1 0:0:0.0'::INTERVAL DAY TO SECOND AS dt" is executed
  const auto stmt = conn.execute_fetch(
      "SELECT '0-6'::INTERVAL YEAR TO MONTH * 3 AS ym, "
      "2 * '1 0:0:0.0'::INTERVAL DAY TO SECOND AS dt");

  // Then the result should contain expected INTERVAL values '1-6' and '2 0:0:0.000000'
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "18");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "172800000000.000");
}

TEST_CASE("should divide INTERVAL by a scalar", "[interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '1-6'::INTERVAL YEAR TO MONTH / 3 AS ym,
  //   '2 0:0:0.0'::INTERVAL DAY TO SECOND / 2 AS dt" is executed
  const auto stmt = conn.execute_fetch(
      "SELECT '1-6'::INTERVAL YEAR TO MONTH / 3 AS ym, "
      "'2 0:0:0.0'::INTERVAL DAY TO SECOND / 2 AS dt");

  // Then the result should contain expected INTERVAL values '0-6' and '1 0:0:0.000000'
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "6");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "86400000000.000");
}
