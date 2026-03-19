// Float integer-valued (.0) C type conversion tests
// Tests that FLOAT values with no fractional part (.0) convert correctly
// to fixed-width integer C types, and that boundary values at i32/u32/i64
// limits are handled correctly by both drivers.

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_floating_point.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"

// ============================================================================
// Small .0 values to integer C types
// ============================================================================

TEST_CASE("should convert small integer-valued floats to all integer C types",
          "[datatype][float][conversion][integer]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Float values 0.0, 1.0, and -1.0 are queried for type conversion
  const std::string q_zero = "SELECT 0.0::FLOAT";
  const std::string q_one = "SELECT 1.0::FLOAT";
  const std::string q_neg = "SELECT -1.0::FLOAT";

  // Then 0.0 should convert to all integer C types without truncation
  CHECK(check_no_truncation<SQL_C_STINYINT>(conn.execute_fetch(q_zero), 1) == 0);
  CHECK(check_no_truncation<SQL_C_UTINYINT>(conn.execute_fetch(q_zero), 1) == 0);
  CHECK(check_no_truncation<SQL_C_SHORT>(conn.execute_fetch(q_zero), 1) == 0);
  CHECK(check_no_truncation<SQL_C_USHORT>(conn.execute_fetch(q_zero), 1) == 0);
  CHECK(check_no_truncation<SQL_C_LONG>(conn.execute_fetch(q_zero), 1) == 0);
  CHECK(check_no_truncation<SQL_C_ULONG>(conn.execute_fetch(q_zero), 1) == 0u);
  CHECK(check_no_truncation<SQL_C_SBIGINT>(conn.execute_fetch(q_zero), 1) == 0);
  CHECK(check_no_truncation<SQL_C_UBIGINT>(conn.execute_fetch(q_zero), 1) == 0u);

  // And 1.0 should convert to all integer C types without truncation
  CHECK(check_no_truncation<SQL_C_STINYINT>(conn.execute_fetch(q_one), 1) == 1);
  CHECK(check_no_truncation<SQL_C_UTINYINT>(conn.execute_fetch(q_one), 1) == 1);
  CHECK(check_no_truncation<SQL_C_SHORT>(conn.execute_fetch(q_one), 1) == 1);
  CHECK(check_no_truncation<SQL_C_USHORT>(conn.execute_fetch(q_one), 1) == 1);
  CHECK(check_no_truncation<SQL_C_LONG>(conn.execute_fetch(q_one), 1) == 1);
  CHECK(check_no_truncation<SQL_C_ULONG>(conn.execute_fetch(q_one), 1) == 1u);
  CHECK(check_no_truncation<SQL_C_SBIGINT>(conn.execute_fetch(q_one), 1) == 1);
  CHECK(check_no_truncation<SQL_C_UBIGINT>(conn.execute_fetch(q_one), 1) == 1u);

  // And -1.0 should convert to signed integer C types without truncation
  CHECK(check_no_truncation<SQL_C_STINYINT>(conn.execute_fetch(q_neg), 1) == -1);
  CHECK(check_no_truncation<SQL_C_SHORT>(conn.execute_fetch(q_neg), 1) == -1);
  CHECK(check_no_truncation<SQL_C_LONG>(conn.execute_fetch(q_neg), 1) == -1);
  CHECK(check_no_truncation<SQL_C_SBIGINT>(conn.execute_fetch(q_neg), 1) == -1);

  // And -1.0 should return 22003 for unsigned integer C types
  check_numeric_out_of_range<SQL_C_UTINYINT>(conn.execute_fetch(q_neg), 1);
  check_numeric_out_of_range<SQL_C_USHORT>(conn.execute_fetch(q_neg), 1);
  check_numeric_out_of_range<SQL_C_ULONG>(conn.execute_fetch(q_neg), 1);
  check_numeric_out_of_range<SQL_C_UBIGINT>(conn.execute_fetch(q_neg), 1);
}

// ============================================================================
// i32/u32 boundary .0 values
// ============================================================================

TEST_CASE("should handle i32 and u32 boundary values stored as float", "[datatype][float][conversion][integer][edge]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Boundary float values at i32 and u32 limits are queried
  const std::string q_i32_max = "SELECT 2147483647.0::FLOAT";
  const std::string q_i32_min = "SELECT -2147483648.0::FLOAT";
  const std::string q_u32_max = "SELECT 4294967295.0::FLOAT";
  const std::string q_2_31 = "SELECT 2147483648.0::FLOAT";
  const std::string q_2_32 = "SELECT 4294967296.0::FLOAT";

  // Then i32 max 2147483647.0 should succeed for SQL_C_LONG and wider types
  CHECK(check_no_truncation<SQL_C_LONG>(conn.execute_fetch(q_i32_max), 1) == 2147483647);
  CHECK(check_no_truncation<SQL_C_SBIGINT>(conn.execute_fetch(q_i32_max), 1) == 2147483647LL);
  CHECK(check_no_truncation<SQL_C_UBIGINT>(conn.execute_fetch(q_i32_max), 1) == 2147483647ULL);

  // And i32 min -2147483648.0 should succeed for SQL_C_LONG and wider signed types
  CHECK(check_no_truncation<SQL_C_LONG>(conn.execute_fetch(q_i32_min), 1) == (SQLINTEGER)-2147483648LL);
  CHECK(check_no_truncation<SQL_C_SBIGINT>(conn.execute_fetch(q_i32_min), 1) == -2147483648LL);

  // And u32 max 4294967295.0 should succeed for SQL_C_ULONG and wider types
  CHECK(check_no_truncation<SQL_C_ULONG>(conn.execute_fetch(q_u32_max), 1) == 4294967295u);
  CHECK(check_no_truncation<SQL_C_SBIGINT>(conn.execute_fetch(q_u32_max), 1) == 4294967295LL);
  CHECK(check_no_truncation<SQL_C_UBIGINT>(conn.execute_fetch(q_u32_max), 1) == 4294967295ULL);

  // And 2147483648.0 should succeed for SQL_C_ULONG and SQL_C_SBIGINT but overflow SQL_C_LONG
  CHECK(check_no_truncation<SQL_C_ULONG>(conn.execute_fetch(q_2_31), 1) == 2147483648u);
  CHECK(check_no_truncation<SQL_C_SBIGINT>(conn.execute_fetch(q_2_31), 1) == 2147483648LL);
  check_numeric_out_of_range<SQL_C_LONG>(conn.execute_fetch(q_2_31), 1);

  // And 4294967296.0 should succeed for SQL_C_SBIGINT but overflow SQL_C_LONG and SQL_C_ULONG
  CHECK(check_no_truncation<SQL_C_SBIGINT>(conn.execute_fetch(q_2_32), 1) == 4294967296LL);
  CHECK(check_no_truncation<SQL_C_UBIGINT>(conn.execute_fetch(q_2_32), 1) == 4294967296ULL);
  check_numeric_out_of_range<SQL_C_LONG>(conn.execute_fetch(q_2_32), 1);
  check_numeric_out_of_range<SQL_C_ULONG>(conn.execute_fetch(q_2_32), 1);
}

// ============================================================================
// Large .0 values to wider C types
// ============================================================================

TEST_CASE("should convert large integer-valued floats to wider types and strings",
          "[datatype][float][conversion][integer][edge]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Large integer-valued float values are queried
  const std::string q_2_53 = "SELECT 9007199254740992.0::FLOAT";
  const std::string q_i32_max = "SELECT 2147483647.0::FLOAT";
  const std::string q_u32_max = "SELECT 4294967295.0::FLOAT";

  // Then 2^53 should convert exactly to SQL_C_SBIGINT and SQL_C_UBIGINT
  CHECK(check_no_truncation<SQL_C_SBIGINT>(conn.execute_fetch(q_2_53), 1) == 9007199254740992LL);
  CHECK(check_no_truncation<SQL_C_UBIGINT>(conn.execute_fetch(q_2_53), 1) == 9007199254740992ULL);

  // And Large integer-valued floats should convert exactly to SQL_C_DOUBLE
  CHECK(check_no_truncation<SQL_C_DOUBLE>(conn.execute_fetch(q_i32_max), 1) == 2147483647.0);
  CHECK(check_no_truncation<SQL_C_DOUBLE>(conn.execute_fetch(q_u32_max), 1) == 4294967295.0);
  CHECK(check_no_truncation<SQL_C_DOUBLE>(conn.execute_fetch(q_2_53), 1) == 9007199254740992.0);

  // And Large integer-valued floats should render correctly as SQL_C_CHAR strings
  {
    std::string s = check_char_success(conn.execute_fetch(q_i32_max), 1);
    CHECK(std::stoll(s) == 2147483647LL);
  }
  {
    std::string s = check_char_success(conn.execute_fetch(q_u32_max), 1);
    CHECK(std::stoll(s) == 4294967295LL);
  }
  {
    std::string s = check_char_success(conn.execute_fetch(q_2_53), 1);
    CHECK_THAT(std::stod(s), Catch::Matchers::WithinRel(9007199254740992.0, 1e-14));
  }
}

// ============================================================================
// .0 values to SQL_C_FLOAT (f64 -> f32)
// ============================================================================

TEST_CASE("should convert integer-valued floats to SQL_C_FLOAT", "[datatype][float][conversion][edge]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Integer-valued float values are queried for f32 conversion
  const std::string q_zero = "SELECT 0.0::FLOAT";
  const std::string q_one = "SELECT 1.0::FLOAT";
  const std::string q_neg = "SELECT -1.0::FLOAT";
  const std::string q_100 = "SELECT 100.0::FLOAT";

  // Then Small integer-valued floats should convert exactly to SQL_C_FLOAT
  CHECK(check_no_truncation<SQL_C_FLOAT>(conn.execute_fetch(q_zero), 1) == 0.0f);
  CHECK(check_no_truncation<SQL_C_FLOAT>(conn.execute_fetch(q_one), 1) == 1.0f);
  CHECK(check_no_truncation<SQL_C_FLOAT>(conn.execute_fetch(q_neg), 1) == -1.0f);
  CHECK(check_no_truncation<SQL_C_FLOAT>(conn.execute_fetch(q_100), 1) == 100.0f);

  // And Power-of-two floats within f32 range should convert exactly to SQL_C_FLOAT
  CHECK(check_no_truncation<SQL_C_FLOAT>(conn.execute_fetch("SELECT 1024.0::FLOAT"), 1) == 1024.0f);
  CHECK(check_no_truncation<SQL_C_FLOAT>(conn.execute_fetch("SELECT 65536.0::FLOAT"), 1) == 65536.0f);
  CHECK(check_no_truncation<SQL_C_FLOAT>(conn.execute_fetch("SELECT 16777216.0::FLOAT"), 1) == 16777216.0f);
}

// ============================================================================
// .0 values to SQL_C_NUMERIC — boundary values
// ============================================================================

TEST_CASE("should encode large integer-valued floats correctly in SQL_C_NUMERIC",
          "[datatype][float][conversion][numeric][edge]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Large integer-valued float values are queried for SQL_C_NUMERIC conversion
  const std::string q_i32_max = "SELECT 2147483647.0::FLOAT";
  const std::string q_i32_min = "SELECT -2147483648.0::FLOAT";
  const std::string q_u32_max = "SELECT 4294967295.0::FLOAT";
  const std::string q_2_32 = "SELECT 4294967296.0::FLOAT";
  const std::string q_2_53 = "SELECT 9007199254740992.0::FLOAT";
  const std::string q_neg_2_53 = "SELECT -9007199254740992.0::FLOAT";

  // Then i32 max should encode correctly in SQL_NUMERIC_STRUCT
  {
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch(q_i32_max), 1);
    CHECK(numeric.sign == 1);
    CHECK(numeric_val_to_ull(numeric) == 2147483647ULL);
    check_numeric_val_zero_from(numeric, 4);
  }

  // And i32 min should encode as negative in SQL_NUMERIC_STRUCT
  {
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch(q_i32_min), 1);
    CHECK(numeric.sign == 0);
    CHECK(numeric_val_to_ull(numeric) == 2147483648ULL);
    check_numeric_val_zero_from(numeric, 4);
  }

  // And u32 max should encode correctly in SQL_NUMERIC_STRUCT
  {
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch(q_u32_max), 1);
    CHECK(numeric.sign == 1);
    CHECK(numeric_val_to_ull(numeric) == 4294967295ULL);
    check_numeric_val_zero_from(numeric, 4);
  }

  // And 2^32 and 2^53 should encode correctly in SQL_NUMERIC_STRUCT
  {
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch(q_2_32), 1);
    CHECK(numeric.sign == 1);
    CHECK(numeric_val_to_ull(numeric) == 4294967296ULL);
    check_numeric_val_zero_from(numeric, 5);
  }
  {
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch(q_2_53), 1);
    CHECK(numeric.sign == 1);
    CHECK(numeric_val_to_ull(numeric) == 9007199254740992ULL);
    check_numeric_val_zero_from(numeric, 7);
  }
  {
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch(q_neg_2_53), 1);
    CHECK(numeric.sign == 0);
    CHECK(numeric_val_to_ull(numeric) == 9007199254740992ULL);
    check_numeric_val_zero_from(numeric, 7);
  }
}
