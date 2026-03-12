// Float to illegal C type conversion tests
// Tests that converting Snowflake FLOAT/DOUBLE/REAL SQL type to C types
// not listed in the ODBC spec conversion table returns the appropriate error.
// Per the ODBC spec (Appendix D, "SQL to C: Numeric"), approximate numeric
// types (SQL_REAL, SQL_FLOAT, SQL_DOUBLE) cannot be converted to temporal,
// interval, or GUID C types.

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "macros.hpp"

static void check_restricted_conversion(const StatementHandleWrapper& stmt, SQLUSMALLINT col, SQLSMALLINT target_type,
                                        void* buffer, SQLLEN buffer_size) {
  SQLLEN indicator = -999;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), col, target_type, buffer, buffer_size, &indicator);
  auto records = get_diag_rec(stmt);
  std::string sqlstate = records.empty() ? "(no diag)" : records[0].sqlState;
  INFO("target_type=" << target_type << " ret=" << ret << " sqlstate=" << sqlstate);
  REQUIRE(ret == SQL_ERROR);
  REQUIRE(!records.empty());
  CHECK((sqlstate == "07006" || sqlstate == "HY003" || sqlstate == "HYC00"));
}

static void check_single_interval_conversion(Connection& conn, const char* query, SQLSMALLINT target_type) {
  auto stmt = conn.execute_fetch(query);
  SQL_INTERVAL_STRUCT value = {};
  SQLLEN indicator = -999;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, target_type, &value, sizeof(value), &indicator);
  auto records = get_diag_rec(stmt);
  std::string sqlstate = records.empty() ? "(no diag)" : records[0].sqlState;
  INFO("target_type=" << target_type << " ret=" << ret << " sqlstate=" << sqlstate);

  OLD_DRIVER_ONLY("BD#20") { CHECK((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO)); }

  NEW_DRIVER_ONLY("BD#20") {
    REQUIRE(ret == SQL_ERROR);
    REQUIRE(!records.empty());
    CHECK((sqlstate == "07006" || sqlstate == "HY003"));
  }
}

static void check_compound_interval_conversion(Connection& conn, const char* query, SQLSMALLINT target_type) {
  auto stmt = conn.execute_fetch(query);
  SQL_INTERVAL_STRUCT value = {};
  SQLLEN indicator = -999;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, target_type, &value, sizeof(value), &indicator);
  auto records = get_diag_rec(stmt);
  std::string sqlstate = records.empty() ? "(no diag)" : records[0].sqlState;
  INFO("target_type=" << target_type << " ret=" << ret << " sqlstate=" << sqlstate);

  REQUIRE(ret == SQL_ERROR);
  REQUIRE(!records.empty());

  OLD_DRIVER_ONLY("BD#21") { CHECK(sqlstate == "22015"); }

  NEW_DRIVER_ONLY("BD#21") { CHECK((sqlstate == "07006" || sqlstate == "HY003")); }
}

// ============================================================================
// ILLEGAL CONVERSIONS - Float to Temporal C Types
// ============================================================================

TEST_CASE("should fail converting float to temporal C types", "[datatype][float][conversion][negative]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT 42.5::FLOAT" is executed
  auto stmt = conn.execute_fetch("SELECT 42.5::FLOAT");

  // Then SQL_C_TYPE_DATE conversion should fail with restricted data type error
  {
    SQL_DATE_STRUCT value = {};
    check_restricted_conversion(stmt, 1, SQL_C_TYPE_DATE, &value, sizeof(value));
  }

  // And SQL_C_TYPE_TIME conversion should fail with restricted data type error
  {
    SQL_TIME_STRUCT value = {};
    check_restricted_conversion(stmt, 1, SQL_C_TYPE_TIME, &value, sizeof(value));
  }

  // And SQL_C_TYPE_TIMESTAMP conversion should fail with restricted data type error
  {
    SQL_TIMESTAMP_STRUCT value = {};
    check_restricted_conversion(stmt, 1, SQL_C_TYPE_TIMESTAMP, &value, sizeof(value));
  }
}

// ============================================================================
// ILLEGAL CONVERSIONS - Float to Single-Component Interval C Types
// ============================================================================

TEST_CASE("should fail converting float to single-component interval C types",
          "[datatype][float][conversion][negative]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT 42.5::FLOAT" is executed
  const auto* query = "SELECT 42.5::FLOAT";

  // Then SQL_C_INTERVAL_YEAR conversion should fail with restricted data type error
  check_single_interval_conversion(conn, query, SQL_C_INTERVAL_YEAR);

  // And SQL_C_INTERVAL_MONTH conversion should fail with restricted data type error
  check_single_interval_conversion(conn, query, SQL_C_INTERVAL_MONTH);

  // And SQL_C_INTERVAL_DAY conversion should fail with restricted data type error
  check_single_interval_conversion(conn, query, SQL_C_INTERVAL_DAY);

  // And SQL_C_INTERVAL_HOUR conversion should fail with restricted data type error
  check_single_interval_conversion(conn, query, SQL_C_INTERVAL_HOUR);

  // And SQL_C_INTERVAL_MINUTE conversion should fail with restricted data type error
  check_single_interval_conversion(conn, query, SQL_C_INTERVAL_MINUTE);

  // And SQL_C_INTERVAL_SECOND conversion should fail with restricted data type error
  check_single_interval_conversion(conn, query, SQL_C_INTERVAL_SECOND);
}

// ============================================================================
// ILLEGAL CONVERSIONS - Float to Compound Interval C Types
// ============================================================================

TEST_CASE("should fail converting float to compound interval C types", "[datatype][float][conversion][negative]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT 42.5::FLOAT" is executed
  const auto* query = "SELECT 42.5::FLOAT";

  // Then SQL_C_INTERVAL_YEAR_TO_MONTH conversion should fail with error
  check_compound_interval_conversion(conn, query, SQL_C_INTERVAL_YEAR_TO_MONTH);

  // And SQL_C_INTERVAL_DAY_TO_HOUR conversion should fail with error
  check_compound_interval_conversion(conn, query, SQL_C_INTERVAL_DAY_TO_HOUR);

  // And SQL_C_INTERVAL_DAY_TO_MINUTE conversion should fail with error
  check_compound_interval_conversion(conn, query, SQL_C_INTERVAL_DAY_TO_MINUTE);

  // And SQL_C_INTERVAL_DAY_TO_SECOND conversion should fail with error
  check_compound_interval_conversion(conn, query, SQL_C_INTERVAL_DAY_TO_SECOND);

  // And SQL_C_INTERVAL_HOUR_TO_MINUTE conversion should fail with error
  check_compound_interval_conversion(conn, query, SQL_C_INTERVAL_HOUR_TO_MINUTE);

  // And SQL_C_INTERVAL_HOUR_TO_SECOND conversion should fail with error
  check_compound_interval_conversion(conn, query, SQL_C_INTERVAL_HOUR_TO_SECOND);

  // And SQL_C_INTERVAL_MINUTE_TO_SECOND conversion should fail with error
  check_compound_interval_conversion(conn, query, SQL_C_INTERVAL_MINUTE_TO_SECOND);
}

// ============================================================================
// ILLEGAL CONVERSIONS - Float to GUID C Type
// ============================================================================

TEST_CASE("should fail converting float to SQL_C_GUID", "[datatype][float][conversion][negative]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT 42.5::FLOAT" is executed
  auto stmt = conn.execute_fetch("SELECT 42.5::FLOAT");

  // Then SQL_C_GUID conversion should fail with restricted data type error
  {
    SQLGUID value = {};
    check_restricted_conversion(stmt, 1, SQL_C_GUID, &value, sizeof(value));
  }
}
