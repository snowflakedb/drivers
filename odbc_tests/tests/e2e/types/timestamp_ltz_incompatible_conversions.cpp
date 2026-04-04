#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "timestamp_incompatible_checks.hpp"

TEST_CASE("should fail converting TIMESTAMP_LTZ to numeric C types", "[timestamp_ltz][conversion][negative]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '2024-01-15 14:30:45'::TIMESTAMP_LTZ" is executed
  auto stmt = conn.execute_fetch("SELECT '2024-01-15 14:30:45'::TIMESTAMP_LTZ");

  // Then SQL_C_SLONG conversion should fail with SQLSTATE 07006
  check_incompat<SQLINTEGER>(stmt, 1, SQL_C_SLONG);

  // And SQL_C_DOUBLE conversion should fail with SQLSTATE 07006
  check_incompat<SQLDOUBLE>(stmt, 1, SQL_C_DOUBLE);

  // And SQL_C_FLOAT conversion should fail with SQLSTATE 07006
  check_incompat<SQLREAL>(stmt, 1, SQL_C_FLOAT);

  // And SQL_C_NUMERIC conversion should fail with SQLSTATE 07006
  check_incompat<SQL_NUMERIC_STRUCT>(stmt, 1, SQL_C_NUMERIC);

  // And SQL_C_BIT conversion should fail with SQLSTATE 07006
  check_incompat<SQLCHAR>(stmt, 1, SQL_C_BIT);
}

TEST_CASE("should fail converting TIMESTAMP_LTZ to additional numeric C types",
          "[timestamp_ltz][conversion][negative]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '2024-01-15 14:30:45'::TIMESTAMP_LTZ" is executed
  auto stmt = conn.execute_fetch("SELECT '2024-01-15 14:30:45'::TIMESTAMP_LTZ");

  // Then SQL_C_STINYINT conversion should fail with SQLSTATE 07006
  check_incompat<SQLSCHAR>(stmt, 1, SQL_C_STINYINT);

  // And SQL_C_SSHORT conversion should fail with SQLSTATE 07006
  check_incompat<SQLSMALLINT>(stmt, 1, SQL_C_SSHORT);

  // And SQL_C_SBIGINT conversion should fail with SQLSTATE 07006
  check_incompat<SQLBIGINT>(stmt, 1, SQL_C_SBIGINT);

  // And SQL_C_UTINYINT conversion should fail with SQLSTATE 07006
  check_incompat<SQLCHAR>(stmt, 1, SQL_C_UTINYINT);

  // And SQL_C_USHORT conversion should fail with SQLSTATE 07006
  check_incompat<SQLUSMALLINT>(stmt, 1, SQL_C_USHORT);

  // And SQL_C_ULONG conversion should fail with SQLSTATE 07006
  check_incompat<SQLUINTEGER>(stmt, 1, SQL_C_ULONG);

  // And SQL_C_UBIGINT conversion should fail with SQLSTATE 07006
  check_incompat<SQLUBIGINT>(stmt, 1, SQL_C_UBIGINT);
}

TEST_CASE("should fail converting TIMESTAMP_LTZ to single-component interval C types",
          "[timestamp_ltz][conversion][negative]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '2024-01-15 14:30:45'::TIMESTAMP_LTZ" is executed
  auto stmt = conn.execute_fetch("SELECT '2024-01-15 14:30:45'::TIMESTAMP_LTZ");

  // Then SQL_C_INTERVAL_YEAR conversion should fail with SQLSTATE 07006
  check_incompat<SQL_INTERVAL_STRUCT>(stmt, 1, SQL_C_INTERVAL_YEAR);

  // And SQL_C_INTERVAL_MONTH conversion should fail with SQLSTATE 07006
  check_incompat<SQL_INTERVAL_STRUCT>(stmt, 1, SQL_C_INTERVAL_MONTH);

  // And SQL_C_INTERVAL_DAY conversion should fail with SQLSTATE 07006
  check_incompat<SQL_INTERVAL_STRUCT>(stmt, 1, SQL_C_INTERVAL_DAY);

  // And SQL_C_INTERVAL_HOUR conversion should fail with SQLSTATE 07006
  check_incompat<SQL_INTERVAL_STRUCT>(stmt, 1, SQL_C_INTERVAL_HOUR);

  // And SQL_C_INTERVAL_MINUTE conversion should fail with SQLSTATE 07006
  check_incompat<SQL_INTERVAL_STRUCT>(stmt, 1, SQL_C_INTERVAL_MINUTE);

  // And SQL_C_INTERVAL_SECOND conversion should fail with SQLSTATE 07006
  check_incompat<SQL_INTERVAL_STRUCT>(stmt, 1, SQL_C_INTERVAL_SECOND);
}

TEST_CASE("should fail converting TIMESTAMP_LTZ to compound interval C types",
          "[timestamp_ltz][conversion][negative]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '2024-01-15 14:30:45'::TIMESTAMP_LTZ" is executed
  auto stmt = conn.execute_fetch("SELECT '2024-01-15 14:30:45'::TIMESTAMP_LTZ");

  // Then SQL_C_INTERVAL_YEAR_TO_MONTH conversion should fail with SQLSTATE 07006
  check_incompat<SQL_INTERVAL_STRUCT>(stmt, 1, SQL_C_INTERVAL_YEAR_TO_MONTH);

  // And SQL_C_INTERVAL_DAY_TO_HOUR conversion should fail with SQLSTATE 07006
  check_incompat<SQL_INTERVAL_STRUCT>(stmt, 1, SQL_C_INTERVAL_DAY_TO_HOUR);

  // And SQL_C_INTERVAL_DAY_TO_MINUTE conversion should fail with SQLSTATE 07006
  check_incompat<SQL_INTERVAL_STRUCT>(stmt, 1, SQL_C_INTERVAL_DAY_TO_MINUTE);

  // And SQL_C_INTERVAL_DAY_TO_SECOND conversion should fail with SQLSTATE 07006
  check_incompat<SQL_INTERVAL_STRUCT>(stmt, 1, SQL_C_INTERVAL_DAY_TO_SECOND);

  // And SQL_C_INTERVAL_HOUR_TO_MINUTE conversion should fail with SQLSTATE 07006
  check_incompat<SQL_INTERVAL_STRUCT>(stmt, 1, SQL_C_INTERVAL_HOUR_TO_MINUTE);

  // And SQL_C_INTERVAL_HOUR_TO_SECOND conversion should fail with SQLSTATE 07006
  check_incompat<SQL_INTERVAL_STRUCT>(stmt, 1, SQL_C_INTERVAL_HOUR_TO_SECOND);

  // And SQL_C_INTERVAL_MINUTE_TO_SECOND conversion should fail with SQLSTATE 07006
  check_incompat<SQL_INTERVAL_STRUCT>(stmt, 1, SQL_C_INTERVAL_MINUTE_TO_SECOND);
}

TEST_CASE("should fail converting TIMESTAMP_LTZ to SQL_C_GUID", "[timestamp_ltz][conversion][negative]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT '2024-01-15 14:30:45'::TIMESTAMP_LTZ" is executed
  auto stmt = conn.execute_fetch("SELECT '2024-01-15 14:30:45'::TIMESTAMP_LTZ");

  // Then SQL_C_GUID conversion should fail with SQLSTATE 07006 (or HYC00 on Windows)
  check_incompat<SQLGUID>(stmt, 1, SQL_C_GUID);
}
