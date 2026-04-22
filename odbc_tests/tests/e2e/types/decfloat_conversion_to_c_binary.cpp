#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_diag_rec.hpp"

// ============================================================================
// BASIC BINARY CONVERSIONS
// ============================================================================

TEST_CASE("DECFLOAT to SQL_C_BINARY", "[decfloat][conversion][c_binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A DECFLOAT integer value is fetched as SQL_C_BINARY
  auto numeric = get_binary_as_numeric(conn.execute_fetch("SELECT 42::DECFLOAT"), 1);

  // Then SQL_C_BINARY returns SQL_NUMERIC_STRUCT bytes with correct sign and value
  CHECK(numeric.sign == 1);
  CHECK(numeric.val[0] == 42);
  check_numeric_val_zero_from(numeric, 1);
}

TEST_CASE("DECFLOAT fractional to SQL_C_BINARY", "[decfloat][conversion][c_binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A fractional DECFLOAT value is fetched as SQL_C_BINARY
  auto numeric = get_binary_as_numeric(conn.execute_fetch("SELECT 123.456::DECFLOAT"), 1);

  // Then SQL_C_BINARY returns SQL_NUMERIC_STRUCT with integer part
  CHECK(numeric.sign == 1);
  CHECK(numeric_val_to_ull(numeric) == 123);
}

TEST_CASE("DECFLOAT negative to SQL_C_BINARY", "[decfloat][conversion][c_binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A negative DECFLOAT value is fetched as SQL_C_BINARY
  auto numeric = get_binary_as_numeric(conn.execute_fetch("SELECT '-7'::DECFLOAT"), 1);

  // Then SQL_NUMERIC_STRUCT has sign=0 for negative and correct magnitude
  CHECK(numeric.sign == 0);
  CHECK(numeric.val[0] == 7);
  check_numeric_val_zero_from(numeric, 1);
}

TEST_CASE("DECFLOAT zero to SQL_C_BINARY", "[decfloat][conversion][c_binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Zero DECFLOAT value is fetched as SQL_C_BINARY
  auto numeric = get_binary_as_numeric(conn.execute_fetch("SELECT 0::DECFLOAT"), 1);

  // Then SQL_NUMERIC_STRUCT has sign=1 and all val bytes zero
  CHECK(numeric.sign == 1);
  check_numeric_val_zero_from(numeric, 0);
}

// ============================================================================
// BUFFER TOO SMALL
// ============================================================================

TEST_CASE("DECFLOAT SQL_C_BINARY buffer too small returns 22003", "[decfloat][conversion][c_binary][22003]") {
  SKIP_OLD_DRIVER("BD#12",
                  "Old driver does not return SQL_ERROR (22003) when SQL_C_BINARY buffer is too small for "
                  "SQL_NUMERIC_STRUCT");
  // Given Snowflake client is logged in
  Connection conn;

  // When A DECFLOAT value is fetched as SQL_C_BINARY into a buffer smaller than SQL_NUMERIC_STRUCT
  auto stmt = conn.execute_fetch("SELECT 42::DECFLOAT");
  char small_buffer[4];
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, small_buffer, sizeof(small_buffer), &indicator);

  // Then SQL_ERROR is returned with SQLSTATE 22003
  CHECK(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "22003");
}

// ============================================================================
// OVERFLOW (extreme exponent)
// ============================================================================

TEST_CASE("DECFLOAT extreme exponent to SQL_C_BINARY returns 22003", "[decfloat][conversion][c_binary][22003]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A DECFLOAT value with exponent exceeding i128 range is fetched as SQL_C_BINARY
  (void)0;
  // Then SQL_ERROR is returned with SQLSTATE 22003
  {
    INFO("1e100 overflows i128 integer part");
    auto stmt = conn.execute_fetch("SELECT '1e100'::DECFLOAT");
    char buffer[100] = {};
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
    OLD_DRIVER_ONLY("BD#29") { CHECK(ret == SQL_SUCCESS); }
    NEW_DRIVER_ONLY("BD#29") {
      CHECK(ret == SQL_ERROR);
      CHECK(get_sqlstate(stmt) == "22003");
    }
  }

  {
    INFO("negative extreme exponent");
    auto stmt = conn.execute_fetch("SELECT '-1e100'::DECFLOAT");
    char buffer[100] = {};
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
    OLD_DRIVER_ONLY("BD#29") { CHECK(ret == SQL_SUCCESS); }
    NEW_DRIVER_ONLY("BD#29") {
      CHECK(ret == SQL_ERROR);
      CHECK(get_sqlstate(stmt) == "22003");
    }
  }
}

// ============================================================================
// NULL handling
// ============================================================================

TEST_CASE("DECFLOAT NULL to SQL_C_BINARY", "[decfloat][conversion][c_binary][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL DECFLOAT value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::DECFLOAT");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_BINARY);
}
