#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"

TEST_CASE("should fail converting VARIANT to numeric C types", "[variant][conversion][negative]") {
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT PARSE_JSON('{\"a\":1}')");

  {
    SQLCHAR value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_BIT, &value, sizeof(value));
  }
  {
    SQLSCHAR value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_STINYINT, &value, sizeof(value));
  }
  {
    SQLCHAR value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_UTINYINT, &value, sizeof(value));
  }
  {
    SQLSMALLINT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_SSHORT, &value, sizeof(value));
  }
  {
    SQLUSMALLINT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_USHORT, &value, sizeof(value));
  }
  {
    SQLINTEGER value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_SLONG, &value, sizeof(value));
  }
  {
    SQLUINTEGER value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_ULONG, &value, sizeof(value));
  }
  {
    SQLBIGINT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_SBIGINT, &value, sizeof(value));
  }
  {
    SQLUBIGINT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_UBIGINT, &value, sizeof(value));
  }
  {
    SQLREAL value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_FLOAT, &value, sizeof(value));
  }
  {
    SQLDOUBLE value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_DOUBLE, &value, sizeof(value));
  }
  {
    SQL_NUMERIC_STRUCT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_NUMERIC, &value, sizeof(value));
  }
}

TEST_CASE("should fail converting VARIANT to temporal C types", "[variant][conversion][negative]") {
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT PARSE_JSON('{\"a\":1}')");

  {
    SQL_DATE_STRUCT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_TYPE_DATE, &value, sizeof(value));
  }
  {
    SQL_TIME_STRUCT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_TYPE_TIME, &value, sizeof(value));
  }
  {
    SQL_TIMESTAMP_STRUCT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_TYPE_TIMESTAMP, &value, sizeof(value));
  }
}

TEST_CASE("should fail converting VARIANT to interval C types", "[variant][conversion][negative]") {
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT PARSE_JSON('{\"a\":1}')");

  SQL_INTERVAL_STRUCT value = {};
  check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_YEAR, &value, sizeof(value));
  check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_MONTH, &value, sizeof(value));
  check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_DAY, &value, sizeof(value));
  check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_HOUR, &value, sizeof(value));
  check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_MINUTE, &value, sizeof(value));
  check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_SECOND, &value, sizeof(value));
  check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_YEAR_TO_MONTH, &value, sizeof(value));
  check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_DAY_TO_HOUR, &value, sizeof(value));
  check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_DAY_TO_MINUTE, &value, sizeof(value));
  check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_DAY_TO_SECOND, &value, sizeof(value));
  check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_HOUR_TO_MINUTE, &value, sizeof(value));
  check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_HOUR_TO_SECOND, &value, sizeof(value));
  check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_MINUTE_TO_SECOND, &value, sizeof(value));
}

TEST_CASE("should fail converting VARIANT to SQL_C_GUID", "[variant][conversion][negative]") {
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT PARSE_JSON('{\"a\":1}')");

  SQLGUID value = {};
  check_incompatible_conversion(stmt, 1, SQL_C_GUID, &value, sizeof(value));
}
