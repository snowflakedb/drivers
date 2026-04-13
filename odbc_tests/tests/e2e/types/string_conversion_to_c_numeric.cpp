// String to SQL_C_NUMERIC conversion tests
// Tests converting Snowflake VARCHAR/STRING type to SQL_C_NUMERIC

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "conversion_checks.hpp"
#include "get_data.hpp"

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

TEST_CASE("should convert string literals to SQL_C_NUMERIC", "[datatype][string][conversion][numeric]") {
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
