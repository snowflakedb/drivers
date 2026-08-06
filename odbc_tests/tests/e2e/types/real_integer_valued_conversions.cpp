// Float integer-valued (.0) C type conversion tests
// Tests that FLOAT values with no fractional part (.0) convert correctly
// to fixed-width integer C types, and that boundary values at i32/u32
// limits and 2^53 (f64 exact-integer limit) are handled correctly.

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_floating_point.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"
#include "test_setup.hpp"

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
  const std::string q_i32_max = "SELECT 2147483647.0::FLOAT";
  const std::string q_u32_max = "SELECT 4294967295.0::FLOAT";
  const std::string q_2_53 = "SELECT 9007199254740992.0::FLOAT";

  // Then Large integer-valued floats should convert exactly to SQL_C_DOUBLE
  CHECK(check_no_truncation<SQL_C_DOUBLE>(conn.execute_fetch(q_i32_max), 1) == 2147483647.0);
  CHECK(check_no_truncation<SQL_C_DOUBLE>(conn.execute_fetch(q_u32_max), 1) == 4294967295.0);

  // And Large integer-valued floats should render correctly as SQL_C_CHAR strings
  {
    std::string s = check_char_success(conn.execute_fetch(q_i32_max), 1);
    CHECK(std::stoll(s) == 2147483647LL);
  }
  {
    std::string s = check_char_success(conn.execute_fetch(q_u32_max), 1);
    CHECK(std::stoll(s) == 4294967295LL);
  }
  // 2^53 renders within 1e-14 relative even over JSON, whose shorter decimal
  // representation costs only the last two units here.
  {
    std::string s = check_char_success(conn.execute_fetch(q_2_53), 1);
    CHECK_THAT(std::stod(s), Catch::Matchers::WithinRel(9007199254740992.0, 1e-14));
  }
}

TEST_CASE("should convert 2^53 exactly to wider integer types", "[datatype][float][conversion][integer][edge]") {
  // Only the 2^53 cases need this skip — the i32/u32 boundary values in the
  // test above are short enough to survive JSON intact.
  SKIP_FOR_JSON_RESULT_SET("JSON truncates DOUBLE below f64 fidelity, so 2^53 arrives as 9007199254740990");

  // Given Snowflake client is logged in
  Connection conn;

  // When 2^53, the largest integer exactly representable as f64, is queried
  const std::string q_2_53 = "SELECT 9007199254740992.0::FLOAT";

  // Then 2^53 should convert exactly to SQL_C_SBIGINT and SQL_C_UBIGINT
  CHECK(check_no_truncation<SQL_C_SBIGINT>(conn.execute_fetch(q_2_53), 1) == 9007199254740992LL);
  CHECK(check_no_truncation<SQL_C_UBIGINT>(conn.execute_fetch(q_2_53), 1) == 9007199254740992ULL);

  // And exactly to SQL_C_DOUBLE
  CHECK(check_no_truncation<SQL_C_DOUBLE>(conn.execute_fetch(q_2_53), 1) == 9007199254740992.0);
}

TEST_CASE("should lose the low digits of 2^53 in wider integer types in JSON",
          "[datatype][float][conversion][integer][edge]") {
  RUN_ONLY_FOR_JSON_RESULT_SET("Arrow carries exact IEEE-754 doubles, so 2^53 survives there");

  // Given Snowflake client is logged in
  Connection conn;

  // When 2^53 is queried over JSON
  //
  // The server sends DOUBLE as a decimal string with fewer significant digits
  // than an f64 carries, so 9007199254740992 arrives as "9.00719925474099e+15".
  // The digits are already gone on the wire, so each conversion below is
  // reported as exact.
  const std::string q_2_53 = "SELECT 9007199254740992.0::FLOAT";

  // Then Every wider type sees 9007199254740990 instead
  CHECK(check_no_truncation<SQL_C_SBIGINT>(conn.execute_fetch(q_2_53), 1) == 9007199254740990LL);
  CHECK(check_no_truncation<SQL_C_UBIGINT>(conn.execute_fetch(q_2_53), 1) == 9007199254740990ULL);
  CHECK(check_no_truncation<SQL_C_DOUBLE>(conn.execute_fetch(q_2_53), 1) == 9007199254740990.0);
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

  // And 2^32 should encode correctly in SQL_NUMERIC_STRUCT
  {
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch(q_2_32), 1);
    CHECK(numeric.sign == 1);
    CHECK(numeric_val_to_ull(numeric) == 4294967296ULL);
    check_numeric_val_zero_from(numeric, 5);
  }
}

TEST_CASE("should encode 2^53 exactly in SQL_C_NUMERIC", "[datatype][float][conversion][numeric][edge]") {
  // Only the 2^53 cases need this skip — the i32/u32/2^32 boundary values in
  // the test above are short enough to survive JSON intact.
  SKIP_FOR_JSON_RESULT_SET("JSON truncates DOUBLE below f64 fidelity, so 2^53 arrives as 9007199254740990");

  // Given Snowflake client is logged in
  Connection conn;

  // When ±2^53 are queried for SQL_C_NUMERIC conversion
  const std::string q_2_53 = "SELECT 9007199254740992.0::FLOAT";
  const std::string q_neg_2_53 = "SELECT -9007199254740992.0::FLOAT";

  // Then 2^53 should encode correctly in SQL_NUMERIC_STRUCT
  {
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch(q_2_53), 1);
    CHECK(numeric.sign == 1);
    CHECK(numeric_val_to_ull(numeric) == 9007199254740992ULL);
    check_numeric_val_zero_from(numeric, 7);
  }

  // And -2^53 should encode as negative in SQL_NUMERIC_STRUCT
  {
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch(q_neg_2_53), 1);
    CHECK(numeric.sign == 0);
    CHECK(numeric_val_to_ull(numeric) == 9007199254740992ULL);
    check_numeric_val_zero_from(numeric, 7);
  }
}

TEST_CASE("should encode 2^53 with its low digits lost in SQL_C_NUMERIC in JSON",
          "[datatype][float][conversion][numeric][edge]") {
  RUN_ONLY_FOR_JSON_RESULT_SET("Arrow carries exact IEEE-754 doubles, so 2^53 survives there");

  // Given Snowflake client is logged in
  Connection conn;

  // When ±2^53 are queried over JSON
  //
  // The server sends DOUBLE as a decimal string with fewer significant digits
  // than an f64 carries, so 9007199254740992 arrives as "9.00719925474099e+15".
  // The sign and the zero-padding of the struct are unaffected — only the
  // mantissa digits are lost.
  const std::string q_2_53 = "SELECT 9007199254740992.0::FLOAT";
  const std::string q_neg_2_53 = "SELECT -9007199254740992.0::FLOAT";

  // Then The struct encodes 9007199254740990 instead, with the sign preserved
  {
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch(q_2_53), 1);
    CHECK(numeric.sign == 1);
    CHECK(numeric_val_to_ull(numeric) == 9007199254740990ULL);
    check_numeric_val_zero_from(numeric, 7);
  }
  {
    auto numeric = check_no_truncation<SQL_C_NUMERIC>(conn.execute_fetch(q_neg_2_53), 1);
    CHECK(numeric.sign == 0);
    CHECK(numeric_val_to_ull(numeric) == 9007199254740990ULL);
    check_numeric_val_zero_from(numeric, 7);
  }
}
