// DECFLOAT incompatible C type conversion tests
// Tests that converting Snowflake DECFLOAT type to C types not listed in the
// ODBC spec conversion table returns the appropriate error.
// DECFLOAT maps to SQL_NUMERIC, so the same incompatible types apply as for
// NUMBER/DECIMAL: temporal and GUID C types.
// Expected SQLSTATE: 07006 (Restricted data type attribute violation)

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"

// ============================================================================
// INCOMPATIBLE CONVERSIONS - DECFLOAT to Temporal C Types
// ============================================================================

TEST_CASE("should fail converting decfloat to temporal C types", "[decfloat][conversion][negative]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT 42::DECFLOAT" is executed
  auto stmt = conn.execute_fetch("SELECT 42::DECFLOAT");

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
// INCOMPATIBLE CONVERSIONS - DECFLOAT to GUID C Type
// ============================================================================

TEST_CASE("should fail converting decfloat to SQL_C_GUID", "[decfloat][conversion][negative]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT 42::DECFLOAT" is executed
  auto stmt = conn.execute_fetch("SELECT 42::DECFLOAT");

  // Then SQL_C_GUID conversion should fail with SQLSTATE 07006 (or HYC00 on Windows)
  {
    SQLGUID value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_GUID, &value, sizeof(value));
  }
}
