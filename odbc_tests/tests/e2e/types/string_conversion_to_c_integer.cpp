// String to ODBC integer type conversions tests
// Tests converting Snowflake VARCHAR/STRING type to integer ODBC C types:
// SQL_C_LONG, SQL_C_SLONG, SQL_C_ULONG, SQL_C_SHORT, SQL_C_SSHORT, SQL_C_USHORT,
// SQL_C_TINYINT, SQL_C_STINYINT, SQL_C_UTINYINT, SQL_C_SBIGINT, SQL_C_UBIGINT,
// SQL_C_BIT, SQL_C_NUMERIC

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <limits>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "macros.hpp"
#include "test_setup.hpp"

// Helper to get raw data with error checking for expected failures
template <typename T>
static SQLRETURN get_data_raw(const StatementHandleWrapper& stmt, SQLUSMALLINT col, SQLSMALLINT target_type, T* value,
                              SQLLEN* indicator) {
  return SQLGetData(stmt.getHandle(), col, target_type, value, sizeof(*value), indicator);
}

// Helper to check SQLSTATE from diagnostic records
static std::string get_sqlstate(const StatementHandleWrapper& stmt) {
  auto records = get_diag_rec(stmt);
  if (!records.empty()) {
    return records[0].sqlState;
  }
  return "";
}

// ============================================================================
// SUCCESSFUL CONVERSIONS - String to Signed Integer Types
// ============================================================================

TEST_CASE("should convert string literals to signed integer types", "[datatype][string][conversion][integer]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting string literals representing integers is executed
  auto stmt = conn.execute_fetch(
      "SELECT '123' AS c1, '-456' AS c2, '0' AS c3, "
      "'2147483647' AS c4, '-2147483648' AS c5, "
      "'999' AS c6, '-999' AS c7, "
      "'32767' AS c8, '-32768' AS c9, "
      "'100' AS c10, '-100' AS c11, '127' AS c12, '-128' AS c13, "
      "'50' AS c14, '-50' AS c15, "
      "'9223372036854775807' AS c16, '-9223372036854775808' AS c17, '1234567890123456789' AS c18");

  // Then SQL_C_LONG conversions should work
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 123);
  CHECK(get_data<SQL_C_LONG>(stmt, 2) == -456);
  CHECK(get_data<SQL_C_LONG>(stmt, 3) == 0);
  CHECK(get_data<SQL_C_LONG>(stmt, 4) == 2147483647);
  CHECK(get_data<SQL_C_LONG>(stmt, 5) == -2147483648);

  // And SQL_C_SLONG conversions should work
  CHECK(get_data<SQL_C_SLONG>(stmt, 6) == 999);
  CHECK(get_data<SQL_C_SLONG>(stmt, 7) == -999);

  // And SQL_C_SHORT conversions should work
  CHECK(get_data<SQL_C_SHORT>(stmt, 8) == 32767);
  CHECK(get_data<SQL_C_SHORT>(stmt, 9) == -32768);

  // And SQL_C_TINYINT conversions should work
  CHECK(get_data<SQL_C_TINYINT>(stmt, 10) == 100);
  CHECK(get_data<SQL_C_TINYINT>(stmt, 11) == -100);
  CHECK(get_data<SQL_C_TINYINT>(stmt, 12) == 127);
  CHECK(get_data<SQL_C_TINYINT>(stmt, 13) == -128);

  // And SQL_C_STINYINT conversions should work
  CHECK(get_data<SQL_C_STINYINT>(stmt, 14) == 50);
  CHECK(get_data<SQL_C_STINYINT>(stmt, 15) == -50);

  // And SQL_C_SBIGINT conversions should work
  CHECK(get_data<SQL_C_SBIGINT>(stmt, 16) == 9223372036854775807LL);
  CHECK(get_data<SQL_C_SBIGINT>(stmt, 17) == (-9223372036854775807LL - 1));
  CHECK(get_data<SQL_C_SBIGINT>(stmt, 18) == 1234567890123456789LL);
}

// ============================================================================
// SUCCESSFUL CONVERSIONS - String to Unsigned Integer Types
// ============================================================================

TEST_CASE("should convert string literals to unsigned integer types", "[datatype][string][conversion][integer]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting string literals representing unsigned integers is executed
  auto stmt = conn.execute_fetch(
      "SELECT '123' AS c1, '0' AS c2, '4294967295' AS c3, "
      "'65535' AS c4, "
      "'255' AS c5, "
      "'18446744073709551615' AS c6, '12345678901234567890' AS c7, "
      "'100' AS c8, '200' AS c9");

  // Then SQL_C_ULONG conversions should work
  CHECK(get_data<SQL_C_ULONG>(stmt, 1) == 123);
  CHECK(get_data<SQL_C_ULONG>(stmt, 2) == 0);
  CHECK(get_data<SQL_C_ULONG>(stmt, 3) == 4294967295U);

  // And SQL_C_USHORT conversions should work
  CHECK(get_data<SQL_C_USHORT>(stmt, 4) == 65535);

  // And SQL_C_UTINYINT conversions should work
  CHECK(get_data<SQL_C_UTINYINT>(stmt, 5) == 255);

  // And SQL_C_UBIGINT conversions should work
  CHECK(get_data<SQL_C_UBIGINT>(stmt, 6) == 18446744073709551615ULL);
  CHECK(get_data<SQL_C_UBIGINT>(stmt, 7) == 12345678901234567890ULL);

  // And SQL_C_SSHORT conversions should work
  CHECK(get_data<SQL_C_SSHORT>(stmt, 8) == 100);
  CHECK(get_data<SQL_C_SSHORT>(stmt, 9) == 200);
}

// ============================================================================
// SUCCESSFUL CONVERSIONS - String to BIT Type
// ============================================================================

TEST_CASE("should convert string literals to SQL_C_BIT", "[datatype][string][conversion][integer]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting string literals representing boolean values is executed
  auto stmt = conn.execute_fetch("SELECT '1' AS true_val, '0' AS false_val, ' 1 ' AS c3, ' 0 ' AS c4");

  // Then the string values should be correctly converted to SQL_C_BIT
  CHECK(get_data<SQL_C_BIT>(stmt, 1) == 1);
  CHECK(get_data<SQL_C_BIT>(stmt, 2) == 0);
  CHECK(get_data<SQL_C_BIT>(stmt, 3) == 1);
  CHECK(get_data<SQL_C_BIT>(stmt, 4) == 0);
}

// ============================================================================
// FAILING CONVERSIONS - String to BIT Type
// ============================================================================

TEST_CASE("should fail converting string literals with to SQL_C_BIT", "[datatype][string][conversion][integer]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting string literals with leading/trailing whitespace is executed
  auto stmt = conn.execute_fetch("SELECT 'abc' AS invalid, '456' AS whole, '6' AS single_digit, '1.1' AS fractional");

  // Then the string values should fail to convert to SQL_C_BIT
  check_invalid_string<SQL_C_BIT>(stmt, 1);
  check_numeric_out_of_range<SQL_C_BIT>(stmt, 2);
  check_numeric_out_of_range<SQL_C_BIT>(stmt, 3);
  CHECK(check_fractional_truncation<SQL_C_BIT>(stmt, 4) == 1);
}

// ============================================================================
// TRUNCATION TESTS
// ============================================================================

TEST_CASE("should truncate decimal string literals with fractional part when converting to integer types",
          "[datatype][string][conversion][integer]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting string literals with decimal parts is executed
  auto stmt = conn.execute_fetch(
      "SELECT '123.999' AS round_down, '-456.001' AS neg_round, '0.9' AS less_than_one, "
      "'1.2345678901241242141241241e9' AS scientific_notation");

  SECTION("TEST") {
    auto value = get_data<SQL_C_SBIGINT>(stmt, 1);
    CHECK(value == 123);
  }
  // Then the string values should be truncated when converted to integer types
  SECTION("SQL_C_BIGINT") {
    CHECK(check_fractional_truncation<SQL_C_SBIGINT>(stmt, 1) == 123);
    CHECK(check_fractional_truncation<SQL_C_SBIGINT>(stmt, 2) == -456);
    CHECK(check_fractional_truncation<SQL_C_SBIGINT>(stmt, 3) == 0);
    CHECK(check_fractional_truncation<SQL_C_SBIGINT>(stmt, 4) == 1234567890);
  }
  SECTION("SQL_C_LONG") {
    CHECK(check_fractional_truncation<SQL_C_LONG>(stmt, 1) == 123);
    CHECK(check_fractional_truncation<SQL_C_LONG>(stmt, 2) == -456);
    CHECK(check_fractional_truncation<SQL_C_LONG>(stmt, 3) == 0);
    CHECK(check_fractional_truncation<SQL_C_LONG>(stmt, 4) == 1234567890);
  }
  SECTION("SQL_C_SHORT") {
    CHECK(check_fractional_truncation<SQL_C_SHORT>(stmt, 1) == 123);
    CHECK(check_fractional_truncation<SQL_C_SHORT>(stmt, 2) == -456);
    CHECK(check_fractional_truncation<SQL_C_SHORT>(stmt, 3) == 0);
    check_numeric_out_of_range<SQL_C_SHORT>(stmt, 4);
  }
  SECTION("SQL_C_TINYINT") {
    CHECK(check_fractional_truncation<SQL_C_TINYINT>(stmt, 1) == 123);
    check_numeric_out_of_range<SQL_C_TINYINT>(stmt, 2);
    CHECK(check_fractional_truncation<SQL_C_TINYINT>(stmt, 3) == 0);
    check_numeric_out_of_range<SQL_C_TINYINT>(stmt, 4);
  }
  SECTION("SQL_C_UBIGINT") {
    CHECK(check_fractional_truncation<SQL_C_UBIGINT>(stmt, 1) == 123);
    check_numeric_out_of_range<SQL_C_UBIGINT>(stmt, 2);
    CHECK(check_fractional_truncation<SQL_C_UBIGINT>(stmt, 3) == 0);
    CHECK(check_fractional_truncation<SQL_C_UBIGINT>(stmt, 4) == 1234567890);
  }
  SECTION("SQL_C_ULONG") {
    CHECK(check_fractional_truncation<SQL_C_ULONG>(stmt, 1) == 123);
    check_numeric_out_of_range<SQL_C_ULONG>(stmt, 2);
    CHECK(check_fractional_truncation<SQL_C_ULONG>(stmt, 3) == 0);
    CHECK(check_fractional_truncation<SQL_C_ULONG>(stmt, 4) == 1234567890);
  }
  SECTION("SQL_C_USHORT") {
    CHECK(check_fractional_truncation<SQL_C_USHORT>(stmt, 1) == 123);
    check_numeric_out_of_range<SQL_C_USHORT>(stmt, 2);
    CHECK(check_fractional_truncation<SQL_C_USHORT>(stmt, 3) == 0);
    check_numeric_out_of_range<SQL_C_USHORT>(stmt, 4);
  }
  SECTION("SQL_C_UTINYINT") {
    CHECK(check_fractional_truncation<SQL_C_UTINYINT>(stmt, 1) == 123);
    check_numeric_out_of_range<SQL_C_UTINYINT>(stmt, 2);
    CHECK(check_fractional_truncation<SQL_C_UTINYINT>(stmt, 3) == 0);
    check_numeric_out_of_range<SQL_C_UTINYINT>(stmt, 4);
  }
}

TEST_CASE("should truncate decimal string literals without fractional part when converting to integer types",
          "[datatype][string][conversion][integer]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  SECTION("SQL_C_BIGINT") {
    // When Query selecting string literals without fractional part is executed
    auto stmt = conn.execute_fetch(
        "SELECT '9223372036854775807' AS min, '-9223372036854775808' AS max, '9223372036854775808' AS more_than_max, "
        "'-9223372036854775809' AS less_than_min");
    // Then the string values should be truncated when converted to integer types
    CHECK(check_no_truncation<SQL_C_SBIGINT>(stmt, 1) == 9223372036854775807LL);
    CHECK(check_no_truncation<SQL_C_SBIGINT>(stmt, 2) == -9223372036854775808LL);
    check_numeric_out_of_range<SQL_C_SBIGINT>(stmt, 3);
    check_numeric_out_of_range<SQL_C_SBIGINT>(stmt, 4);
  }
  SECTION("SQL_C_LONG") {
    auto stmt = conn.execute_fetch(
        "SELECT '2147483647' AS max, '-2147483648' AS min, '2147483648' AS more_than_max, '-2147483649' AS "
        "less_than_min");
    CHECK(check_no_truncation<SQL_C_LONG>(stmt, 1) == 2147483647);
    CHECK(check_no_truncation<SQL_C_LONG>(stmt, 2) == -2147483648);
    check_numeric_out_of_range<SQL_C_LONG>(stmt, 3);
    check_numeric_out_of_range<SQL_C_LONG>(stmt, 4);
  }
  SECTION("SQL_C_SHORT") {
    auto stmt = conn.execute_fetch(
        "SELECT '32767' AS max, '-32768' AS min, '32768' AS more_than_max, '-32769' AS less_than_min");
    CHECK(check_no_truncation<SQL_C_SHORT>(stmt, 1) == 32767);
    CHECK(check_no_truncation<SQL_C_SHORT>(stmt, 2) == -32768);
    check_numeric_out_of_range<SQL_C_SHORT>(stmt, 3);
    check_numeric_out_of_range<SQL_C_SHORT>(stmt, 4);
  }
  SECTION("SQL_C_TINYINT") {
    auto stmt =
        conn.execute_fetch("SELECT '127' AS max, '-128' AS min, '128' AS more_than_max, '-129' AS less_than_min");
    CHECK(check_no_truncation<SQL_C_TINYINT>(stmt, 1) == 127);
    CHECK(check_no_truncation<SQL_C_TINYINT>(stmt, 2) == -128);
    check_numeric_out_of_range<SQL_C_TINYINT>(stmt, 3);
    check_numeric_out_of_range<SQL_C_TINYINT>(stmt, 4);
  }
  SECTION("SQL_C_UBIGINT") {
    auto stmt = conn.execute_fetch(
        "SELECT '18446744073709551615' AS max, '0' AS min, '18446744073709551616' AS more_than_max, '-1' AS "
        "less_than_min");
    CHECK(check_no_truncation<SQL_C_UBIGINT>(stmt, 1) == 18446744073709551615ULL);
    CHECK(check_no_truncation<SQL_C_UBIGINT>(stmt, 2) == 0ULL);
    check_numeric_out_of_range<SQL_C_UBIGINT>(stmt, 3);
    check_numeric_out_of_range<SQL_C_UBIGINT>(stmt, 4);
  }
  SECTION("SQL_C_ULONG") {
    auto stmt = conn.execute_fetch(
        "SELECT '4294967295' AS max, '0' AS min, '4294967296' AS more_than_max, '-1' AS less_than_min");
    CHECK(check_no_truncation<SQL_C_ULONG>(stmt, 1) == 4294967295U);
    CHECK(check_no_truncation<SQL_C_ULONG>(stmt, 2) == 0U);
    check_numeric_out_of_range<SQL_C_ULONG>(stmt, 3);
    check_numeric_out_of_range<SQL_C_ULONG>(stmt, 4);
  }
  SECTION("SQL_C_USHORT") {
    auto stmt =
        conn.execute_fetch("SELECT '65535' AS max, '0' AS min, '65536' AS more_than_max, '-1' AS less_than_min");
    CHECK(check_no_truncation<SQL_C_USHORT>(stmt, 1) == 65535);
    CHECK(check_no_truncation<SQL_C_USHORT>(stmt, 2) == 0);
    check_numeric_out_of_range<SQL_C_USHORT>(stmt, 3);
    check_numeric_out_of_range<SQL_C_USHORT>(stmt, 4);
  }
  SECTION("SQL_C_UTINYINT") {
    auto stmt = conn.execute_fetch("SELECT '255' AS max, '0' AS min, '256' AS more_than_max, '-1' AS less_than_min");
    CHECK(check_no_truncation<SQL_C_UTINYINT>(stmt, 1) == 255);
    CHECK(check_no_truncation<SQL_C_UTINYINT>(stmt, 2) == 0);
    check_numeric_out_of_range<SQL_C_UTINYINT>(stmt, 3);
    check_numeric_out_of_range<SQL_C_UTINYINT>(stmt, 4);
  }
}

// ============================================================================
// CONVERSION WITH SQLBindCol - Integer types
// ============================================================================

TEST_CASE("should convert strings to integer types using SQLBindCol", "[datatype][string][conversion][integer]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting string numeric value is executed with SQLBindCol for SQL_C_LONG
  // Then the bound integer value should match the string representation
  {
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT '12345' AS str_num", SQL_NTS);
    CHECK_ODBC(ret, stmt);

    SQLINTEGER value;
    SQLLEN indicator;
    ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
    CHECK_ODBC(ret, stmt);

    ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);

    CHECK(value == 12345);
    CHECK(indicator == sizeof(SQLINTEGER));
  }

  // And invalid string should fail binding with SQLSTATE 22018
  {
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 'not_a_number' AS str_val", SQL_NTS);
    CHECK_ODBC(ret, stmt);

    SQLINTEGER value;
    SQLLEN indicator;
    ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
    CHECK_ODBC(ret, stmt);

    ret = SQLFetch(stmt.getHandle());
    CHECK(ret == SQL_ERROR);
    CHECK(get_sqlstate(stmt) == "22018");
  }
}
