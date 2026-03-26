// Binary incompatible C type conversion tests
// Tests that converting Snowflake BINARY SQL type to C types not listed in the
// ODBC spec conversion table returns the appropriate error.
// Per the ODBC spec (Appendix D, "SQL to C: Binary"), SQL_BINARY /
// SQL_VARBINARY / SQL_LONGVARBINARY can only be converted to SQL_C_CHAR,
// SQL_C_WCHAR, and SQL_C_BINARY (plus SQL_C_DEFAULT which maps to SQL_C_BINARY).
// All other C types are incompatible and must return SQLSTATE 07006.

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"

// ============================================================================
// INCOMPATIBLE CONVERSIONS - Binary to Integer C Types
// ============================================================================

TEST_CASE("should fail converting binary to integer C types", "[datatype][binary][conversion][negative]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT X'48656C6C6F'::BINARY" is executed
  auto stmt = conn.execute_fetch("SELECT X'48656C6C6F'::BINARY");

  // Then SQL_C_BIT conversion should fail with SQLSTATE 07006
  {
    SQLCHAR value = 0;
    check_incompatible_conversion(stmt, 1, SQL_C_BIT, &value, sizeof(value));
  }

  // And SQL_C_TINYINT conversion should fail with SQLSTATE 07006
  {
    SQLSCHAR value = 0;
    check_incompatible_conversion(stmt, 1, SQL_C_TINYINT, &value, sizeof(value));
  }

  // And SQL_C_STINYINT conversion should fail with SQLSTATE 07006
  {
    SQLSCHAR value = 0;
    check_incompatible_conversion(stmt, 1, SQL_C_STINYINT, &value, sizeof(value));
  }

  // And SQL_C_UTINYINT conversion should fail with SQLSTATE 07006
  {
    SQLCHAR value = 0;
    check_incompatible_conversion(stmt, 1, SQL_C_UTINYINT, &value, sizeof(value));
  }

  // And SQL_C_SHORT conversion should fail with SQLSTATE 07006
  {
    SQLSMALLINT value = 0;
    check_incompatible_conversion(stmt, 1, SQL_C_SHORT, &value, sizeof(value));
  }

  // And SQL_C_SSHORT conversion should fail with SQLSTATE 07006
  {
    SQLSMALLINT value = 0;
    check_incompatible_conversion(stmt, 1, SQL_C_SSHORT, &value, sizeof(value));
  }

  // And SQL_C_USHORT conversion should fail with SQLSTATE 07006
  {
    SQLUSMALLINT value = 0;
    check_incompatible_conversion(stmt, 1, SQL_C_USHORT, &value, sizeof(value));
  }

  // And SQL_C_LONG conversion should fail with SQLSTATE 07006
  {
    SQLINTEGER value = 0;
    check_incompatible_conversion(stmt, 1, SQL_C_LONG, &value, sizeof(value));
  }

  // And SQL_C_SLONG conversion should fail with SQLSTATE 07006
  {
    SQLINTEGER value = 0;
    check_incompatible_conversion(stmt, 1, SQL_C_SLONG, &value, sizeof(value));
  }

  // And SQL_C_ULONG conversion should fail with SQLSTATE 07006
  {
    SQLUINTEGER value = 0;
    check_incompatible_conversion(stmt, 1, SQL_C_ULONG, &value, sizeof(value));
  }

  // And SQL_C_SBIGINT conversion should fail with SQLSTATE 07006
  {
    SQLBIGINT value = 0;
    check_incompatible_conversion(stmt, 1, SQL_C_SBIGINT, &value, sizeof(value));
  }

  // And SQL_C_UBIGINT conversion should fail with SQLSTATE 07006
  {
    SQLUBIGINT value = 0;
    check_incompatible_conversion(stmt, 1, SQL_C_UBIGINT, &value, sizeof(value));
  }
}

// ============================================================================
// INCOMPATIBLE CONVERSIONS - Binary to Floating-Point C Types
// ============================================================================

TEST_CASE("should fail converting binary to floating-point C types", "[datatype][binary][conversion][negative]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT X'48656C6C6F'::BINARY" is executed
  auto stmt = conn.execute_fetch("SELECT X'48656C6C6F'::BINARY");

  // Then SQL_C_FLOAT conversion should fail with SQLSTATE 07006
  {
    SQLREAL value = 0;
    check_incompatible_conversion(stmt, 1, SQL_C_FLOAT, &value, sizeof(value));
  }

  // And SQL_C_DOUBLE conversion should fail with SQLSTATE 07006
  {
    SQLDOUBLE value = 0;
    check_incompatible_conversion(stmt, 1, SQL_C_DOUBLE, &value, sizeof(value));
  }
}

// ============================================================================
// INCOMPATIBLE CONVERSIONS - Binary to SQL_C_NUMERIC
// ============================================================================

TEST_CASE("should fail converting binary to SQL_C_NUMERIC", "[datatype][binary][conversion][negative]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT X'48656C6C6F'::BINARY" is executed
  auto stmt = conn.execute_fetch("SELECT X'48656C6C6F'::BINARY");

  // Then SQL_C_NUMERIC conversion should fail with SQLSTATE 07006
  {
    SQL_NUMERIC_STRUCT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_NUMERIC, &value, sizeof(value));
  }
}

// ============================================================================
// INCOMPATIBLE CONVERSIONS - Binary to Temporal C Types
// ============================================================================

TEST_CASE("should fail converting binary to temporal C types", "[datatype][binary][conversion][negative]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT X'48656C6C6F'::BINARY" is executed
  auto stmt = conn.execute_fetch("SELECT X'48656C6C6F'::BINARY");

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
// INCOMPATIBLE CONVERSIONS - Binary to Single-Component Interval C Types
// ============================================================================

TEST_CASE("should fail converting binary to single-component interval C types",
          "[datatype][binary][conversion][negative]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT X'48656C6C6F'::BINARY" is executed
  auto stmt = conn.execute_fetch("SELECT X'48656C6C6F'::BINARY");

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
// INCOMPATIBLE CONVERSIONS - Binary to Compound Interval C Types
// ============================================================================

TEST_CASE("should fail converting binary to compound interval C types", "[datatype][binary][conversion][negative]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT X'48656C6C6F'::BINARY" is executed
  auto stmt = conn.execute_fetch("SELECT X'48656C6C6F'::BINARY");

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
// INCOMPATIBLE CONVERSIONS - Binary to GUID C Type
// ============================================================================

TEST_CASE("should fail converting binary to SQL_C_GUID", "[datatype][binary][conversion][negative]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT X'48656C6C6F'::BINARY" is executed
  auto stmt = conn.execute_fetch("SELECT X'48656C6C6F'::BINARY");

  // Then SQL_C_GUID conversion should fail with SQLSTATE 07006 (or HYC00 on Windows)
  {
    SQLGUID value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_GUID, &value, sizeof(value));
  }
}
