// Boolean incompatible C type conversion tests
// Tests that converting Snowflake BOOLEAN SQL type to C types not listed in the
// ODBC spec conversion table returns the appropriate error.
// Per the ODBC spec (Appendix D, "SQL to C: Bit"), SQL_BIT can be converted to
// SQL_C_BIT, SQL_C_CHAR, SQL_C_WCHAR, SQL_C_BINARY, all integer C types,
// SQL_C_FLOAT, SQL_C_DOUBLE, and SQL_C_NUMERIC (plus SQL_C_DEFAULT = SQL_C_BIT).
// All other C types are incompatible and must return SQLSTATE 07006.

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"

// ============================================================================
// INCOMPATIBLE CONVERSIONS - Boolean to Temporal C Types
// ============================================================================

TEST_CASE("should fail converting boolean to temporal C types", "[datatype][boolean][conversion][negative]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT TRUE::BOOLEAN" is executed
  auto stmt = conn.execute_fetch("SELECT TRUE::BOOLEAN");

  // Then SQL_C_TYPE_DATE conversion should fail with SQLSTATE 07006
  {
    SQL_DATE_STRUCT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_TYPE_DATE, &value, sizeof(value));
  }

  // And SQL_C_TYPE_TIME conversion should fail with SQLSTATE 07006
  {
    SQL_TIME_STRUCT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_TYPE_TIME, &value, sizeof(value));
  }

  // And SQL_C_TYPE_TIMESTAMP conversion should fail with SQLSTATE 07006
  {
    SQL_TIMESTAMP_STRUCT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_TYPE_TIMESTAMP, &value, sizeof(value));
  }
}

// ============================================================================
// INCOMPATIBLE CONVERSIONS - Boolean to Single-Component Interval C Types
// ============================================================================

TEST_CASE("should fail converting boolean to single-component interval C types",
          "[datatype][boolean][conversion][negative]") {
  SKIP_OLD_DRIVER("BD#31", "Old driver (Simba SDK) accepts boolean-to-interval conversion instead of rejecting");
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT TRUE::BOOLEAN" is executed
  auto stmt = conn.execute_fetch("SELECT TRUE::BOOLEAN");

  // Then SQL_C_INTERVAL_YEAR conversion should fail with SQLSTATE 07006
  {
    SQL_INTERVAL_STRUCT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_YEAR, &value, sizeof(value));
  }

  // And SQL_C_INTERVAL_MONTH conversion should fail with SQLSTATE 07006
  {
    SQL_INTERVAL_STRUCT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_MONTH, &value, sizeof(value));
  }

  // And SQL_C_INTERVAL_DAY conversion should fail with SQLSTATE 07006
  {
    SQL_INTERVAL_STRUCT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_DAY, &value, sizeof(value));
  }

  // And SQL_C_INTERVAL_HOUR conversion should fail with SQLSTATE 07006
  {
    SQL_INTERVAL_STRUCT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_HOUR, &value, sizeof(value));
  }

  // And SQL_C_INTERVAL_MINUTE conversion should fail with SQLSTATE 07006
  {
    SQL_INTERVAL_STRUCT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_MINUTE, &value, sizeof(value));
  }

  // And SQL_C_INTERVAL_SECOND conversion should fail with SQLSTATE 07006
  {
    SQL_INTERVAL_STRUCT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_SECOND, &value, sizeof(value));
  }
}

// ============================================================================
// INCOMPATIBLE CONVERSIONS - Boolean to Compound Interval C Types
// ============================================================================

TEST_CASE("should fail converting boolean to compound interval C types", "[datatype][boolean][conversion][negative]") {
  SKIP_OLD_DRIVER("BD#31", "Old driver (Simba SDK) accepts boolean-to-interval conversion instead of rejecting");
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT TRUE::BOOLEAN" is executed
  auto stmt = conn.execute_fetch("SELECT TRUE::BOOLEAN");

  // Then SQL_C_INTERVAL_YEAR_TO_MONTH conversion should fail with SQLSTATE 07006
  {
    SQL_INTERVAL_STRUCT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_YEAR_TO_MONTH, &value, sizeof(value));
  }

  // And SQL_C_INTERVAL_DAY_TO_HOUR conversion should fail with SQLSTATE 07006
  {
    SQL_INTERVAL_STRUCT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_DAY_TO_HOUR, &value, sizeof(value));
  }

  // And SQL_C_INTERVAL_DAY_TO_MINUTE conversion should fail with SQLSTATE 07006
  {
    SQL_INTERVAL_STRUCT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_DAY_TO_MINUTE, &value, sizeof(value));
  }

  // And SQL_C_INTERVAL_DAY_TO_SECOND conversion should fail with SQLSTATE 07006
  {
    SQL_INTERVAL_STRUCT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_DAY_TO_SECOND, &value, sizeof(value));
  }

  // And SQL_C_INTERVAL_HOUR_TO_MINUTE conversion should fail with SQLSTATE 07006
  {
    SQL_INTERVAL_STRUCT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_HOUR_TO_MINUTE, &value, sizeof(value));
  }

  // And SQL_C_INTERVAL_HOUR_TO_SECOND conversion should fail with SQLSTATE 07006
  {
    SQL_INTERVAL_STRUCT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_HOUR_TO_SECOND, &value, sizeof(value));
  }

  // And SQL_C_INTERVAL_MINUTE_TO_SECOND conversion should fail with SQLSTATE 07006
  {
    SQL_INTERVAL_STRUCT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_INTERVAL_MINUTE_TO_SECOND, &value, sizeof(value));
  }
}

// ============================================================================
// INCOMPATIBLE CONVERSIONS - Boolean to GUID C Type
// ============================================================================

TEST_CASE("should fail converting boolean to SQL_C_GUID", "[datatype][boolean][conversion][negative]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT TRUE::BOOLEAN" is executed
  auto stmt = conn.execute_fetch("SELECT TRUE::BOOLEAN");

  // Then SQL_C_GUID conversion should fail with SQLSTATE 07006 (or HYC00 on Windows)
  {
    SQLGUID value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_GUID, &value, sizeof(value));
  }
}
