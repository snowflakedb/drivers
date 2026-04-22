#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"

TEST_CASE("should fail converting ARRAY to numeric C types", "[array][conversion][negative]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When An ARRAY value is fetched with numeric C types
  auto stmt = conn.execute_fetch("SELECT ARRAY_CONSTRUCT(1,2,3)");

  // Then Each conversion should fail with SQLSTATE 07006
  {
    SQLCHAR value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_BIT, &value, sizeof(value), true);
  }
  {
    SQLSCHAR value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_STINYINT, &value, sizeof(value), true);
  }
  {
    SQLCHAR value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_UTINYINT, &value, sizeof(value), true);
  }
  {
    SQLSMALLINT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_SSHORT, &value, sizeof(value), true);
  }
  {
    SQLUSMALLINT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_USHORT, &value, sizeof(value), true);
  }
  {
    SQLINTEGER value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_SLONG, &value, sizeof(value), true);
  }
  {
    SQLUINTEGER value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_ULONG, &value, sizeof(value), true);
  }
  {
    SQLBIGINT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_SBIGINT, &value, sizeof(value), true);
  }
  {
    SQLUBIGINT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_UBIGINT, &value, sizeof(value), true);
  }
  {
    SQLREAL value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_FLOAT, &value, sizeof(value), true);
  }
  {
    SQLDOUBLE value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_DOUBLE, &value, sizeof(value), true);
  }
  {
    SQL_NUMERIC_STRUCT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_NUMERIC, &value, sizeof(value), true);
  }
}

TEST_CASE("should fail converting ARRAY to temporal C types", "[array][conversion][negative]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When An ARRAY value is fetched with temporal C types
  auto stmt = conn.execute_fetch("SELECT ARRAY_CONSTRUCT(1,2,3)");

  // Then Each conversion should fail with SQLSTATE 07006
  {
    SQL_DATE_STRUCT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_TYPE_DATE, &value, sizeof(value), true);
  }
  {
    SQL_TIME_STRUCT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_TYPE_TIME, &value, sizeof(value), true);
  }
  {
    SQL_TIMESTAMP_STRUCT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_TYPE_TIMESTAMP, &value, sizeof(value), true);
  }
}

TEST_CASE("should fail converting ARRAY to interval C types", "[array][conversion][negative]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When An ARRAY value is fetched with interval C types
  auto stmt = conn.execute_fetch("SELECT ARRAY_CONSTRUCT(1,2,3)");

  // Then Each conversion should fail with SQLSTATE 07006
  SQL_INTERVAL_STRUCT value = {};
  check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_YEAR, &value, sizeof(value), true);
  check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_MONTH, &value, sizeof(value), true);
  check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_DAY, &value, sizeof(value), true);
  check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_HOUR, &value, sizeof(value), true);
  check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_MINUTE, &value, sizeof(value), true);
  check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_SECOND, &value, sizeof(value), true);
  check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_YEAR_TO_MONTH, &value, sizeof(value), true);
  check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_DAY_TO_HOUR, &value, sizeof(value), true);
  check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_DAY_TO_MINUTE, &value, sizeof(value), true);
  check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_DAY_TO_SECOND, &value, sizeof(value), true);
  check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_HOUR_TO_MINUTE, &value, sizeof(value), true);
  check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_HOUR_TO_SECOND, &value, sizeof(value), true);
  check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_MINUTE_TO_SECOND, &value, sizeof(value), true);
}

TEST_CASE("should fail converting ARRAY to SQL_C_GUID", "[array][conversion][negative]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When An ARRAY value is fetched as SQL_C_GUID
  auto stmt = conn.execute_fetch("SELECT ARRAY_CONSTRUCT(1,2,3)");

  // Then Conversion should fail with SQLSTATE 07006
  SQLGUID value = {};
  check_incompatible_conversion(stmt, 1, SQL_C_GUID, &value, sizeof(value), true);
}
