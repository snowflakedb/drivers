// REAL (FLOAT/DOUBLE) to SQL_C_BIT conversion tests
// Migrated from odbc_tests/tests/datatype_tests/real_tests.cpp

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"

TEST_CASE("REAL explicit SQL_C_BIT - basic", "[e2e][types][real][conversion][c_bit]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Various FLOAT values are fetched as SQL_C_BIT
  (void)0;
  // Then 0 and 1 succeed, fractions truncate with 01S07, out-of-range returns 22003
  CHECK(check_no_truncation<SQL_C_BIT>(conn.execute_fetch("SELECT 0.0::FLOAT"), 1) == 0);
  CHECK(check_no_truncation<SQL_C_BIT>(conn.execute_fetch("SELECT 1.0::FLOAT"), 1) == 1);
  CHECK(check_fractional_truncation<SQL_C_BIT>(conn.execute_fetch("SELECT 0.5::FLOAT"), 1) == 0);
  CHECK(check_fractional_truncation<SQL_C_BIT>(conn.execute_fetch("SELECT 1.5::FLOAT"), 1) == 1);
  check_numeric_out_of_range<SQL_C_BIT>(conn.execute_fetch("SELECT 5.5::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_BIT>(conn.execute_fetch("SELECT -1.5::FLOAT"), 1);
}

TEST_CASE("REAL SQL_C_BIT spec compliance", "[e2e][types][real][conversion][c_bit]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When FLOAT values are fetched as SQL_C_BIT per ODBC spec
  (void)0;
  // Then 0 and 1 succeed, value 2 returns 22003, negative returns 22003, fractions truncate with 01S07
  {
    auto stmt = conn.execute_fetch("SELECT 0.0::FLOAT, 1.0::FLOAT");
    CHECK(check_no_truncation<SQL_C_BIT>(stmt, 1) == 0);
    CHECK(check_no_truncation<SQL_C_BIT>(stmt, 2) == 1);
  }

  check_numeric_out_of_range<SQL_C_BIT>(conn.execute_fetch("SELECT 2.0::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_BIT>(conn.execute_fetch("SELECT -1.0::FLOAT"), 1);

  {
    auto value = check_fractional_truncation<SQL_C_BIT>(conn.execute_fetch("SELECT 0.5::FLOAT"), 1);
    CHECK(value == 0);
  }

  {
    auto value = check_fractional_truncation<SQL_C_BIT>(conn.execute_fetch("SELECT 1.5::FLOAT"), 1);
    CHECK(value == 1);
  }

  check_numeric_out_of_range<SQL_C_BIT>(conn.execute_fetch("SELECT 100.0::FLOAT"), 1);
}

TEST_CASE("REAL SQL_C_BIT rejects negative fractions", "[e2e][types][real][conversion][c_bit][edge]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Negative fractional FLOAT values are fetched as SQL_C_BIT
  (void)0;
  // Then SQL_ERROR is returned with SQLSTATE 22003
  (void)0;
  check_numeric_out_of_range<SQL_C_BIT>(conn.execute_fetch("SELECT -0.5::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_BIT>(conn.execute_fetch("SELECT -0.001::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_BIT>(conn.execute_fetch("SELECT -0.9999::FLOAT"), 1);
}

TEST_CASE("REAL NaN to BIT returns error", "[e2e][types][real][conversion][c_bit][nan][edge]") {
  SKIP_OLD_DRIVER("BD#16", "Old driver silently converts NaN to 0 for BIT target");
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When NaN FLOAT value is fetched as SQL_C_BIT
  (void)0;
  // Then SQL_ERROR is returned with SQLSTATE 22003
  check_numeric_out_of_range<SQL_C_BIT>(conn.execute_fetch("SELECT 'NaN'::FLOAT"), 1);
}

TEST_CASE("REAL Infinity to BIT returns 22003", "[e2e][types][real][conversion][c_bit][infinity][edge]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Infinity FLOAT values are fetched as SQL_C_BIT
  (void)0;
  // Then SQL_ERROR is returned with SQLSTATE 22003
  check_numeric_out_of_range<SQL_C_BIT>(conn.execute_fetch("SELECT 'Infinity'::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_BIT>(conn.execute_fetch("SELECT '-Infinity'::FLOAT"), 1);
}

TEST_CASE("REAL NULL to SQL_C_BIT", "[real][conversion][c_bit][null]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When A NULL FLOAT value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::FLOAT");
  // Then NULL FLOAT values return SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_BIT);
}
