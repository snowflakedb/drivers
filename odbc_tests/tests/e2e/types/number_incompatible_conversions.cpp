// Number incompatible C type conversion tests
// Tests that converting Snowflake NUMBER/DECIMAL/NUMERIC SQL type to C types
// not listed in the ODBC spec conversion table returns the appropriate error.
// Per the ODBC spec (Appendix D, "SQL to C: Numeric"), exact numeric types
// (SQL_DECIMAL, SQL_NUMERIC) cannot be converted to temporal or GUID C types.

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"

// ============================================================================
// INCOMPATIBLE CONVERSIONS - Number to Temporal C Types
// ============================================================================

TEST_CASE("should fail converting number to temporal C types", "[datatype][number][conversion][negative]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT 42::NUMBER(10,0)" is executed
  auto stmt = conn.execute_fetch("SELECT 42::NUMBER(10,0)");

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
// INCOMPATIBLE CONVERSIONS - Number to GUID C Type
// ============================================================================

TEST_CASE("should fail converting number to SQL_C_GUID", "[datatype][number][conversion][negative]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT 42::NUMBER(10,0)" is executed
  auto stmt = conn.execute_fetch("SELECT 42::NUMBER(10,0)");

  // Then SQL_C_GUID conversion should fail with SQLSTATE 07006 (or HYC00 on Windows)
  {
    SQLGUID value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_GUID, &value, sizeof(value));
  }
}
