// String to ODBC floating point type conversions tests
// Tests converting Snowflake VARCHAR/STRING type to floating point ODBC C types:
// SQL_C_FLOAT, SQL_C_DOUBLE

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cmath>
#include <cstring>
#include <limits>
#include <string>
#include <vector>

#include <catch2/catch_approx.hpp>
#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "odbc_matchers.hpp"
#include "test_setup.hpp"

static long long numeric_val_to_int(const SQL_NUMERIC_STRUCT& num) {
  long long result = 0;
  long long multiplier = 1;
  for (int i = 0; i < SQL_MAX_NUMERIC_LEN; i++) {
    result += static_cast<long long>(num.val[i]) * multiplier;
    multiplier *= 256;
  }
  return result;
}

static unsigned int to_unsigned_int(char c) { return static_cast<unsigned int>((unsigned char)c); }

// ============================================================================
// SUCCESSFUL CONVERSIONS - String to Floating Point Types
// ============================================================================

TEST_CASE("should convert string literals to floating point types", "[datatype][string][conversion][real]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting string literals representing floating point numbers is executed
  auto stmt = conn.execute_fetch(
      "SELECT '123.456' AS c1, '-789.012' AS c2, '0.0' AS c3, "
      "'3.14159' AS c4, '1.5e10' AS c5, "
      "'123.456789012345' AS c6, '-1.7976931348623157e308' AS c7, "
      "'2.2250738585072014e-308' AS c8, "
      "'42' AS c9, '-100' AS c10, '  123.456  ' AS c11, '    -789.012  ' AS c12");

  // Then SQL_C_FLOAT conversions should work
  CHECK(get_data<SQL_C_FLOAT>(stmt, 1) == Catch::Approx(123.456f).epsilon(0.001f));
  CHECK(get_data<SQL_C_FLOAT>(stmt, 2) == Catch::Approx(-789.012f).epsilon(0.001f));
  CHECK(get_data<SQL_C_FLOAT>(stmt, 3) == Catch::Approx(0.0f));
  CHECK(get_data<SQL_C_FLOAT>(stmt, 4) == Catch::Approx(3.14159f).epsilon(0.00001f));
  CHECK(get_data<SQL_C_FLOAT>(stmt, 5) == Catch::Approx(1.5e10f).margin(1e6f));

  // And SQL_C_DOUBLE conversions should work
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 6) == Catch::Approx(123.456789012345).epsilon(1e-12));
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 7) == Catch::Approx(-1.7976931348623157e308));
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 8) == Catch::Approx(2.2250738585072014e-308));

  // And integer strings should convert to floating point
  CHECK(get_data<SQL_C_FLOAT>(stmt, 9) == Catch::Approx(42.0f));
  CHECK(get_data<SQL_C_FLOAT>(stmt, 10) == Catch::Approx(-100.0f));
  CHECK(get_data<SQL_C_FLOAT>(stmt, 11) == Catch::Approx(123.456f).epsilon(0.001f));
  CHECK(get_data<SQL_C_FLOAT>(stmt, 12) == Catch::Approx(-789.012f).epsilon(0.001f));
}

// ============================================================================
// DATA OUT OF RANGE - String to Floating Point Types
// ============================================================================

TEST_CASE("should fail converting string literals to floating point types when data is out of range",
          "[datatype][string][conversion][real]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting string literals representing floating point numbers is executed
  {
    INFO("SQL_C_DOUBLE");
    auto stmt = conn.execute_fetch(
        "SELECT '1.7976931348623157e308' AS max_double, '1.7976931348623157e309' AS more_than_max_double, "
        "'-1.7976931348623157E+308' AS min_double, '-1.7976931348623158e308' AS less_than_min_double");
    // Then values within range should convert successfully and values exceeding SQL_C_DOUBLE range should fail
    CHECK(check_no_truncation<SQL_C_DOUBLE>(stmt, 1) == 1.7976931348623157e308);
    check_numeric_out_of_range<SQL_C_DOUBLE>(stmt, 2);
    CHECK(check_no_truncation<SQL_C_DOUBLE>(stmt, 3) == -1.7976931348623157e308);
    CHECK(check_no_truncation<SQL_C_DOUBLE>(stmt, 4) == -1.7976931348623157e308);
  }
  // And values exceeding SQL_C_FLOAT range should fail with numeric out of range
  {
    INFO("SQL_C_FLOAT");
    // NOTE: This is the behavior of the old ODBC driver
    // It does not convert max_float and min_float correctly so we have almost max_float and min_float values
    auto stmt = conn.execute_fetch(
        "SELECT '3.4028234e+38' AS almost_max_float, '3.4028235e38' AS max_float, '3.4028236e38' AS "
        "more_than_max_float, '-3.4028234e38' AS almost_min_float, '-3.4028235e38' AS min_float, '-3.4028234e39' AS "
        "less_than_min_float");
    CHECK(check_no_truncation<SQL_C_FLOAT>(stmt, 1) == 3.4028235e38f);
    NEW_DRIVER_ONLY("BD#8") { CHECK(check_no_truncation<SQL_C_FLOAT>(stmt, 2) == 3.4028235e38f); }
    OLD_DRIVER_ONLY("BD#8") { check_numeric_out_of_range<SQL_C_FLOAT>(stmt, 2); }
    check_numeric_out_of_range<SQL_C_FLOAT>(stmt, 3);
    CHECK(check_no_truncation<SQL_C_FLOAT>(stmt, 4) == -3.4028235e38f);
    NEW_DRIVER_ONLY("BD#8") { CHECK(check_no_truncation<SQL_C_FLOAT>(stmt, 5) == -3.4028235e38f); }
    OLD_DRIVER_ONLY("BD#8") { check_numeric_out_of_range<SQL_C_FLOAT>(stmt, 5); }
    check_numeric_out_of_range<SQL_C_FLOAT>(stmt, 6);
  }
}

TEST_CASE("should handle special floating point string conversions", "[datatype][string][conversion][real][edge]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting special float strings is executed
  const auto query = "SELECT 'inf' AS pos_inf, '-inf' AS neg_inf, 'NaN' AS nan";

  // Then inf conversion either succeeds with infinity or fails
  {
    INFO("SQL_C_DOUBLE");
    auto stmt = conn.execute_fetch(query);
    CHECK(check_no_truncation<SQL_C_DOUBLE>(stmt, 1) == std::numeric_limits<SQLDOUBLE>::infinity());
    CHECK(check_no_truncation<SQL_C_DOUBLE>(stmt, 2) == -std::numeric_limits<SQLDOUBLE>::infinity());
    auto val = get_data<SQL_C_DOUBLE>(stmt, 3);
    CHECK(std::isnan(val));
  }

  {
    INFO("SQL_C_FLOAT");
    auto stmt = conn.execute_fetch(query);
    CHECK(check_no_truncation<SQL_C_FLOAT>(stmt, 1) == std::numeric_limits<SQLFLOAT>::infinity());
    CHECK(check_no_truncation<SQL_C_FLOAT>(stmt, 2) == -std::numeric_limits<SQLFLOAT>::infinity());
    auto val = get_data<SQL_C_FLOAT>(stmt, 3);
    CHECK(std::isnan(val));
  }
}

// ============================================================================
// EDGE CASES - Numeric strings with special formatting
// ============================================================================

TEST_CASE("should handle edge case floating point string formats", "[datatype][string][conversion][real][edge]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting strings with special formatting is executed
  auto stmt = conn.execute_fetch(
      "SELECT '+456.789' AS c1, '0.00000001' AS c2, '1e-10' AS c3, "
      "'1.5E10' AS c4, '2.5E-5' AS c5");

  // Then explicit plus sign should be handled for floats
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 1) == Catch::Approx(456.789).epsilon(0.001));

  // And very small decimal values should convert
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 2) == Catch::Approx(0.00000001).epsilon(1e-12));
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 3) == Catch::Approx(1e-10).epsilon(1e-15));

  // And uppercase E in scientific notation should work
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 4) == Catch::Approx(1.5e10).margin(1e6));
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 5) == Catch::Approx(2.5e-5).epsilon(1e-9));
}

// ============================================================================
// FAILING CONVERSIONS - Non-numeric strings to floating point types
// ============================================================================

TEST_CASE("should fail converting non-numeric strings to floating point types",
          "[datatype][string][conversion][real][failure]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting various non-numeric strings is executed
  auto stmt = conn.execute_fetch("SELECT 'not a number' AS c1, 'abc123' AS c2");

  // Then text should fail for SQL_C_FLOAT
  check_invalid_string<SQL_C_FLOAT>(stmt, 1);
  // And non-numeric text should fail for SQL_C_DOUBLE
  check_invalid_string<SQL_C_DOUBLE>(stmt, 2);
}

// ============================================================================
// FAILING CONVERSIONS - Malformed numeric strings
// ============================================================================

TEST_CASE("should fail converting malformed numeric strings to floating point types",
          "[datatype][string][conversion][real][failure]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting various malformed numeric strings is executed
  auto stmt = conn.execute_fetch("SELECT '123.456.789' AS c1, '123,456' AS c2");

  // Then multiple decimal points should fail for SQL_C_DOUBLE
  check_invalid_string<SQL_C_DOUBLE>(stmt, 1);
  // And comma as decimal separator should fail for SQL_C_DOUBLE
  check_invalid_string<SQL_C_DOUBLE>(stmt, 2);
}

// ============================================================================
// NULL VALUE HANDLING
// ============================================================================

TEST_CASE("should handle NULL string when converting to floating point types",
          "[datatype][string][conversion][real][null]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting NULL is executed
  auto stmt = conn.execute_fetch("SELECT NULL::STRING AS null_double");

  // Then SQL_C_DOUBLE should return SQL_NULL_DATA indicator
  {
    SQLDOUBLE value = 999.0;
    SQLLEN indicator;
    SQLRETURN ret = get_data_raw(stmt, 1, SQL_C_DOUBLE, &value, &indicator);
    REQUIRE_ODBC(ret, stmt);
    CHECK(indicator == SQL_NULL_DATA);
  }
}

// ============================================================================
// CONVERSION WITH SQLBindCol - Floating point types
// ============================================================================

TEST_CASE("should convert strings to floating point types using SQLBindCol", "[datatype][string][conversion][real]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting string numeric value is executed with SQLBindCol for SQL_C_DOUBLE
  {
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT '987.654' AS str_num", SQL_NTS);
    REQUIRE_ODBC(ret, stmt);

    SQLDOUBLE value;
    SQLLEN indicator;
    ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_DOUBLE, &value, sizeof(value), &indicator);
    REQUIRE_ODBC(ret, stmt);

    ret = SQLFetch(stmt.getHandle());
    REQUIRE_ODBC(ret, stmt);

    // Then the bound double value should match the string representation
    CHECK(value == Catch::Approx(987.654).epsilon(0.001));
    CHECK(indicator == sizeof(SQLDOUBLE));
  }
}

TEST_CASE("should convert string literals to SQL_C_NUMERIC", "[datatype][string][conversion][real]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting various numeric string formats is executed
  auto stmt = conn.execute_fetch(
      "SELECT '12345' AS c1, '-67890' AS c2, '0' AS c3, "
      "'123.456' AS c4, '  999  ' AS c5, '+42' AS c6, "
      "'00123' AS c7, '1.5432e3' AS c8, '123456789012345678901234567890' AS c9, NULL::STRING AS c10");

  // Then positive integer '12345' should convert correctly
  {
    auto num = get_data<SQL_C_NUMERIC>(stmt, 1);
    CHECK(num.precision == 38);
    CHECK(num.scale == 0);
    CHECK(num.sign == 1);  // Positive
    CHECK(numeric_val_to_int(num) == 12345);
  }

  // And negative integer '-67890' should convert correctly
  {
    auto num = get_data<SQL_C_NUMERIC>(stmt, 2);
    CHECK(num.precision == 38);
    CHECK(num.scale == 0);
    CHECK(num.sign == 0);  // Negative
    CHECK(numeric_val_to_int(num) == 67890);
  }

  // And zero '0' should convert correctly
  {
    auto num = get_data<SQL_C_NUMERIC>(stmt, 3);
    CHECK(num.precision == 38);
    CHECK(num.scale == 0);
    CHECK(num.sign == 1);  // Zero is positive
    CHECK(numeric_val_to_int(num) == 0);
  }

  // And decimal '123.456' should convert correctly with appropriate scale
  {
    // NOTE: This is the behavior of the old ODBC driver
    auto num = check_fractional_truncation<SQL_C_NUMERIC>(stmt, 4);
    CHECK(num.precision == 38);
    CHECK(num.scale == 0);
    CHECK(num.sign == 1);  // Positive
    CHECK(numeric_val_to_int(num) == 123);
  }

  // And whitespace '  999  ' should be stripped
  {
    auto num = get_data<SQL_C_NUMERIC>(stmt, 5);
    CHECK(num.precision == 38);
    CHECK(num.scale == 0);
    CHECK(num.sign == 1);  // Positive
    CHECK(numeric_val_to_int(num) == 999);
  }

  // And explicit plus sign '+42' should be handled
  {
    auto num = get_data<SQL_C_NUMERIC>(stmt, 6);
    CHECK(num.precision == 38);
    CHECK(num.scale == 0);
    CHECK(num.sign == 1);  // Positive
    CHECK(numeric_val_to_int(num) == 42);
  }

  // And leading zeros '00123' should be handled
  {
    auto num = get_data<SQL_C_NUMERIC>(stmt, 7);
    CHECK(num.precision == 38);
    CHECK(num.scale == 0);
    CHECK(num.sign == 1);  // Positive
    CHECK(numeric_val_to_int(num) == 123);
  }

  // And scientific notation '1.5432e3' should convert correctly (1.5432e3 = 1543)
  {
    // NOTE: This is the behavior of the old ODBC driver
    auto num = check_fractional_truncation<SQL_C_NUMERIC>(stmt, 8);
    CHECK(num.precision == 38);
    CHECK(num.scale == 0);
    CHECK(num.sign == 1);  // Positive
    CHECK(numeric_val_to_int(num) == 1543);
  }

  // And very large integer '123456789012345678901234567890' should convert correctly to 18EE90FF6C373E0EE4E3F0AD2
  {
    auto num = get_data<SQL_C_NUMERIC>(stmt, 9);
    CHECK(num.precision == 38);
    CHECK(num.scale == 0);
    CHECK(num.sign == 1);  // Positive
    CHECK(to_unsigned_int(num.val[0]) == 0xD2);
    CHECK(to_unsigned_int(num.val[1]) == 0x0A);
    CHECK(to_unsigned_int(num.val[2]) == 0x3F);
    CHECK(to_unsigned_int(num.val[3]) == 0x4E);
    CHECK(to_unsigned_int(num.val[4]) == 0xEE);
    CHECK(to_unsigned_int(num.val[5]) == 0xE0);
    CHECK(to_unsigned_int(num.val[6]) == 0x73);
    CHECK(to_unsigned_int(num.val[7]) == 0xC3);
    CHECK(to_unsigned_int(num.val[8]) == 0xF6);
    CHECK(to_unsigned_int(num.val[9]) == 0x0F);
    CHECK(to_unsigned_int(num.val[10]) == 0xE9);
    CHECK(to_unsigned_int(num.val[11]) == 0x8E);
    CHECK(to_unsigned_int(num.val[12]) == 0x01);
    CHECK(to_unsigned_int(num.val[13]) == 0x00);
    CHECK(to_unsigned_int(num.val[14]) == 0x00);
    CHECK(to_unsigned_int(num.val[15]) == 0x00);
  }

  // And NULL should return SQL_NULL_DATA indicator
  {
    SQL_NUMERIC_STRUCT num;
    SQLLEN indicator;
    SQLRETURN ret = get_data_raw(stmt, 10, SQL_C_NUMERIC, &num, &indicator);
    REQUIRE_ODBC(ret, stmt);
    CHECK(indicator == SQL_NULL_DATA);
  }
}
