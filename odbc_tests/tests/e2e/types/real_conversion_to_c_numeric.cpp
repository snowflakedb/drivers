#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "conversion_checks.hpp"

// ============================================================================
// SQL_C_NUMERIC conversion from REAL
// The old driver (via Simba SDK) supports SQL_C_NUMERIC for SQL_DOUBLE.
// ============================================================================

TEST_CASE("REAL to SQL_C_NUMERIC", "[e2e][types][real][numeric]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When REAL values are fetched as SQL_C_NUMERIC
  (void)0;  // Brace blocks below perform the fetch and assertions
  // Then SQL_NUMERIC_STRUCT fields match expected sign, val bytes, etc.
  {
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT 42.0::FLOAT"), 1);
    CHECK(numeric.sign == 1);
    CHECK(numeric.val[0] == 42);
    for (int i = 1; i < 16; ++i)
      CHECK(numeric.val[i] == 0);
  }

  // negative integer value
  {
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT -7.0::FLOAT"), 1);
    CHECK(numeric.sign == 0);
    CHECK(numeric.val[0] == 7);
    for (int i = 1; i < 16; ++i)
      CHECK(numeric.val[i] == 0);
  }

  // zero
  {
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT 0.0::FLOAT"), 1);
    CHECK(numeric.sign == 1);
    for (int i = 0; i < 16; ++i)
      CHECK(numeric.val[i] == 0);
  }

  // fractional value truncates with 01S07
  {
    auto numeric = check_fractional_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT 123.456::FLOAT"), 1);
    CHECK(numeric.sign == 1);
    unsigned long long stored = 0;
    for (int i = 7; i >= 0; --i) {
      stored = (stored << 8) | numeric.val[i];
    }
    CHECK(stored == 123);
  }

  // large integer value
  {
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT 1000000.0::FLOAT"), 1);
    CHECK(numeric.sign == 1);
    unsigned long long stored = 0;
    for (int i = 7; i >= 0; --i) {
      stored = (stored << 8) | numeric.val[i];
    }
    CHECK(stored == 1000000);
  }

  // negative fractional value truncates with 01S07
  {
    auto numeric = check_fractional_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT -99.9::FLOAT"), 1);
    CHECK(numeric.sign == 0);
    unsigned long long stored = 0;
    for (int i = 7; i >= 0; --i) {
      stored = (stored << 8) | numeric.val[i];
    }
    CHECK(stored == 99);
  }

  // value 1 has correct val bytes
  {
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT 1.0::FLOAT"), 1);
    CHECK(numeric.sign == 1);
    CHECK(numeric.val[0] == 1);
    for (int i = 1; i < 16; ++i)
      CHECK(numeric.val[i] == 0);
  }

  // value 255 uses single byte
  {
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT 255.0::FLOAT"), 1);
    CHECK(numeric.sign == 1);
    CHECK(numeric.val[0] == 255);
    for (int i = 1; i < 16; ++i)
      CHECK(numeric.val[i] == 0);
  }

  // value 256 spans two bytes
  {
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT 256.0::FLOAT"), 1);
    CHECK(numeric.sign == 1);
    CHECK(numeric.val[0] == 0);
    CHECK(numeric.val[1] == 1);
    for (int i = 2; i < 16; ++i)
      CHECK(numeric.val[i] == 0);
  }

  // NULL returns SQL_NULL_DATA
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::FLOAT"), 1, SQL_C_NUMERIC);
}

TEST_CASE("REAL SQL_C_NUMERIC negative zero", "[e2e][types][real][numeric][edge]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Negative fractional REAL values that truncate to zero are fetched as SQL_C_NUMERIC
  auto numeric1 = check_fractional_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT -0.5::FLOAT"), 1);
  // Then SQL_NUMERIC_STRUCT has sign=0 and val=0 (negative zero)
  CHECK(numeric1.sign == 0);
  CHECK(numeric1.val[0] == 0);

  // When -0.001 truncates to negative zero
  auto numeric2 = check_fractional_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT -0.001::FLOAT"), 1);
  // Then SQL_NUMERIC_STRUCT has sign=0 and val=0
  CHECK(numeric2.sign == 0);
  CHECK(numeric2.val[0] == 0);
}

TEST_CASE("REAL NaN to NUMERIC returns error", "[e2e][types][real][nan][edge]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When NaN is fetched as SQL_C_NUMERIC
  check_numeric_out_of_range<SQL_C_NUMERIC>(conn.execute_fetch("SELECT 'NaN'::FLOAT"), 1);
  // Then SQL_ERROR is returned with SQLSTATE 22003
  (void)0;  // check_numeric_out_of_range asserts 22003
}

TEST_CASE("REAL Infinity to NUMERIC returns 22003", "[e2e][types][real][infinity][edge]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Infinity is fetched as SQL_C_NUMERIC
  check_numeric_out_of_range<SQL_C_NUMERIC>(conn.execute_fetch("SELECT 'Infinity'::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_NUMERIC>(conn.execute_fetch("SELECT '-Infinity'::FLOAT"), 1);
  // Then SQL_ERROR is returned with SQLSTATE 22003
  (void)0;  // check_numeric_out_of_range asserts 22003
}

TEST_CASE("REAL NULL to SQL_C_NUMERIC", "[real][conversion][c_numeric][null]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When A NULL FLOAT value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::FLOAT");
  // Then NULL FLOAT values return SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_NUMERIC);
}
