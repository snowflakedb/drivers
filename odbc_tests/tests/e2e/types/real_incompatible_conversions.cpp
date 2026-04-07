// Float incompatible C type conversion tests
// Tests that converting Snowflake FLOAT/DOUBLE/REAL SQL type to C types
// not listed in the ODBC spec conversion table returns the appropriate error.

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"
#include "get_data.hpp"

// ============================================================================
// INCOMPATIBLE CONVERSIONS - Float to Temporal C Types
// ============================================================================

TEST_CASE("should fail converting float to temporal C types", "[datatype][float][conversion][negative]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT 42.5::FLOAT" is executed
  auto stmt = conn.execute_fetch("SELECT 42.5::FLOAT");

  // Then SQL_C_TYPE_DATE conversion should fail with restricted data type error
  {
    SQL_DATE_STRUCT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_TYPE_DATE, &value, sizeof(value));
  }

  // And SQL_C_TYPE_TIME conversion should fail with restricted data type error
  {
    SQL_TIME_STRUCT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_TYPE_TIME, &value, sizeof(value));
  }

  // And SQL_C_TYPE_TIMESTAMP conversion should fail with restricted data type error
  {
    SQL_TIMESTAMP_STRUCT value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_TYPE_TIMESTAMP, &value, sizeof(value));
  }
}

// ============================================================================
// INCOMPATIBLE CONVERSIONS - Float to GUID C Type
// ============================================================================

TEST_CASE("should fail converting float to SQL_C_GUID", "[datatype][float][conversion][negative]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT 42.5::FLOAT" is executed
  auto stmt = conn.execute_fetch("SELECT 42.5::FLOAT");

  // Then SQL_C_GUID conversion should fail with restricted data type error (07006, or HYC00 on Windows)
  {
    SQLGUID value = {};
    check_incompatible_conversion(stmt, 1, SQL_C_GUID, &value, sizeof(value));
  }
}
