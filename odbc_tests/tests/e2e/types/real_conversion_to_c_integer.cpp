// REAL (FLOAT/DOUBLE) to integer C type conversion tests
// Migrated from odbc_tests/tests/datatype_tests/real_tests.cpp

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_diag_rec.hpp"

TEST_CASE("REAL explicit integer conversions truncate fractional part", "[e2e][types][real][conversion][c_integer]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When A fractional FLOAT value 123.789 is fetched as each integer C type
  const std::string query = "SELECT 123.789::FLOAT";

  // Then All integer C types return 123 with fractional truncation
  CHECK(check_fractional_truncation<SQL_C_LONG>(conn.execute_fetch(query), 1) == 123);
  CHECK(check_fractional_truncation<SQL_C_SLONG>(conn.execute_fetch(query), 1) == 123);
  CHECK(check_fractional_truncation<SQL_C_ULONG>(conn.execute_fetch(query), 1) == 123);
  CHECK(check_fractional_truncation<SQL_C_SHORT>(conn.execute_fetch(query), 1) == 123);
  CHECK(check_fractional_truncation<SQL_C_SSHORT>(conn.execute_fetch(query), 1) == 123);
  CHECK(check_fractional_truncation<SQL_C_USHORT>(conn.execute_fetch(query), 1) == 123);
  CHECK(check_fractional_truncation<SQL_C_TINYINT>(conn.execute_fetch(query), 1) == 123);
  CHECK(check_fractional_truncation<SQL_C_STINYINT>(conn.execute_fetch(query), 1) == 123);
  CHECK(check_fractional_truncation<SQL_C_UTINYINT>(conn.execute_fetch(query), 1) == 123);
  CHECK(check_fractional_truncation<SQL_C_SBIGINT>(conn.execute_fetch(query), 1) == 123);
  CHECK(check_fractional_truncation<SQL_C_UBIGINT>(conn.execute_fetch(query), 1) == 123);
}

TEST_CASE("REAL explicit integer conversions - negative value", "[e2e][types][real][conversion][c_integer]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When A negative fractional FLOAT value -42.9 is fetched as signed integer C types
  const std::string query = "SELECT -42.9::FLOAT";

  // Then All signed integer C types return -42 with fractional truncation
  CHECK(check_fractional_truncation<SQL_C_LONG>(conn.execute_fetch(query), 1) == -42);
  CHECK(check_fractional_truncation<SQL_C_SLONG>(conn.execute_fetch(query), 1) == -42);
  CHECK(check_fractional_truncation<SQL_C_SHORT>(conn.execute_fetch(query), 1) == -42);
  CHECK(check_fractional_truncation<SQL_C_SSHORT>(conn.execute_fetch(query), 1) == -42);
  CHECK(check_fractional_truncation<SQL_C_TINYINT>(conn.execute_fetch(query), 1) == -42);
  CHECK(check_fractional_truncation<SQL_C_STINYINT>(conn.execute_fetch(query), 1) == -42);
  CHECK(check_fractional_truncation<SQL_C_SBIGINT>(conn.execute_fetch(query), 1) == -42);
}

TEST_CASE("REAL explicit SQL_C_SBIGINT with large values", "[e2e][types][real][conversion][c_integer]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When The largest integer exactly representable as f64 (2^53) is fetched as SQL_C_SBIGINT
  auto stmt = conn.execute_fetch("SELECT 9007199254740992::FLOAT");

  // Then The value 9007199254740992 is returned without truncation
  CHECK(check_no_truncation<SQL_C_SBIGINT>(stmt, 1) == 9007199254740992LL);
}

TEST_CASE("REAL fractional truncation returns 01S07", "[e2e][types][real][conversion][c_integer][01S07]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Fractional FLOAT values are fetched as integer C types
  (void)0;
  // Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01S07
  {
    auto stmt = conn.execute_fetch("SELECT 123.45::FLOAT");
    auto value = check_fractional_truncation<SQL_C_LONG>(stmt, 1);
    CHECK(value == 123);
  }

  {
    auto stmt = conn.execute_fetch("SELECT 9.99::FLOAT");
    auto value = check_fractional_truncation<SQL_C_SHORT>(stmt, 1);
    CHECK(value == 9);
  }

  {
    auto stmt = conn.execute_fetch("SELECT 1.5::FLOAT");
    auto value = check_fractional_truncation<SQL_C_STINYINT>(stmt, 1);
    CHECK(value == 1);
  }

  {
    auto stmt = conn.execute_fetch("SELECT 999.001::FLOAT");
    auto value = check_fractional_truncation<SQL_C_SBIGINT>(stmt, 1);
    CHECK(value == 999);
  }

  {
    auto stmt = conn.execute_fetch("SELECT 100.0::FLOAT");
    auto value = check_no_truncation<SQL_C_LONG>(stmt, 1);
    CHECK(value == 100);
  }

  {
    auto stmt = conn.execute_fetch("SELECT -42.9::FLOAT");
    auto value = check_fractional_truncation<SQL_C_LONG>(stmt, 1);
    CHECK(value == -42);
  }
}

TEST_CASE("REAL overflow returns 22003", "[e2e][types][real][conversion][c_integer][22003]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Out-of-range FLOAT values are fetched as narrow integer C types
  (void)0;
  // Then SQL_ERROR is returned with SQLSTATE 22003
  check_numeric_out_of_range<SQL_C_STINYINT>(conn.execute_fetch("SELECT 128.0::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_STINYINT>(conn.execute_fetch("SELECT -129.0::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_UTINYINT>(conn.execute_fetch("SELECT -1.0::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_UTINYINT>(conn.execute_fetch("SELECT 256.0::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_SHORT>(conn.execute_fetch("SELECT 32768.0::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_USHORT>(conn.execute_fetch("SELECT -1.0::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_SHORT>(conn.execute_fetch("SELECT -32769.0::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_USHORT>(conn.execute_fetch("SELECT 65536.0::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_LONG>(conn.execute_fetch("SELECT 2147483648.0::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_LONG>(conn.execute_fetch("SELECT -2147483649.0::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_ULONG>(conn.execute_fetch("SELECT -1.0::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_ULONG>(conn.execute_fetch("SELECT 4294967296.0::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_UBIGINT>(conn.execute_fetch("SELECT -1.0::FLOAT"), 1);
}

TEST_CASE("REAL explicit unsigned integer conversions", "[e2e][types][real][conversion][c_integer]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  const std::string q_exact = "SELECT 42.0::FLOAT";
  const std::string q_frac = "SELECT 42.9::FLOAT";
  const std::string q_zero = "SELECT 0.0::FLOAT";

  // When Positive FLOAT values are fetched as unsigned integer C types
  (void)0;
  // Then Exact values succeed, fractional values truncate with 01S07, zero succeeds
  {
    CHECK(check_no_truncation<SQL_C_ULONG>(conn.execute_fetch(q_exact), 1) == 42u);
    CHECK(check_no_truncation<SQL_C_USHORT>(conn.execute_fetch(q_exact), 1) == 42u);
    CHECK(check_no_truncation<SQL_C_UTINYINT>(conn.execute_fetch(q_exact), 1) == 42u);
    CHECK(check_no_truncation<SQL_C_UBIGINT>(conn.execute_fetch(q_exact), 1) == 42u);
  }

  {
    CHECK(check_fractional_truncation<SQL_C_ULONG>(conn.execute_fetch(q_frac), 1) == 42u);
    CHECK(check_fractional_truncation<SQL_C_USHORT>(conn.execute_fetch(q_frac), 1) == 42u);
    CHECK(check_fractional_truncation<SQL_C_UTINYINT>(conn.execute_fetch(q_frac), 1) == 42u);
    CHECK(check_fractional_truncation<SQL_C_UBIGINT>(conn.execute_fetch(q_frac), 1) == 42u);
  }

  {
    CHECK(check_no_truncation<SQL_C_ULONG>(conn.execute_fetch(q_zero), 1) == 0u);
    CHECK(check_no_truncation<SQL_C_USHORT>(conn.execute_fetch(q_zero), 1) == 0u);
    CHECK(check_no_truncation<SQL_C_UTINYINT>(conn.execute_fetch(q_zero), 1) == 0u);
    CHECK(check_no_truncation<SQL_C_UBIGINT>(conn.execute_fetch(q_zero), 1) == 0u);
  }
}

TEST_CASE("REAL integer boundary values for overflow", "[e2e][types][real][conversion][c_integer][edge][22003]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When FLOAT values at or just past integer type boundaries are fetched
  (void)0;
  // Then Values at boundaries succeed, values past boundaries return 22003
  CHECK(check_no_truncation<SQL_C_STINYINT>(conn.execute_fetch("SELECT 127.0::FLOAT"), 1) == 127);
  CHECK(check_no_truncation<SQL_C_STINYINT>(conn.execute_fetch("SELECT -128.0::FLOAT"), 1) == -128);
  CHECK(check_no_truncation<SQL_C_SHORT>(conn.execute_fetch("SELECT 32767.0::FLOAT"), 1) == 32767);
  CHECK(check_no_truncation<SQL_C_SHORT>(conn.execute_fetch("SELECT -32768.0::FLOAT"), 1) == -32768);
  CHECK(check_no_truncation<SQL_C_UTINYINT>(conn.execute_fetch("SELECT 255.0::FLOAT"), 1) == 255);
  CHECK(check_no_truncation<SQL_C_USHORT>(conn.execute_fetch("SELECT 65535.0::FLOAT"), 1) == 65535);
  check_numeric_out_of_range<SQL_C_STINYINT>(conn.execute_fetch("SELECT 128.0::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_STINYINT>(conn.execute_fetch("SELECT -129.0::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_UTINYINT>(conn.execute_fetch("SELECT 256.0::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_USHORT>(conn.execute_fetch("SELECT 65536.0::FLOAT"), 1);
  CHECK(check_fractional_truncation<SQL_C_STINYINT>(conn.execute_fetch("SELECT 126.9::FLOAT"), 1) == 126);
  check_numeric_out_of_range<SQL_C_STINYINT>(conn.execute_fetch("SELECT 128.1::FLOAT"), 1);
}

TEST_CASE("REAL negative fraction to unsigned integer types",
          "[e2e][types][real][conversion][c_integer][unsigned][edge]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Negative fractional FLOAT values are fetched as unsigned integer C types
  (void)0;
  // Then Either 01S07 or 22003 is returned depending on implementation
  (void)0;
  // SQL_C_UTINYINT with -0.1
  {
    auto stmt = conn.execute_fetch("SELECT -0.1::FLOAT");
    typename MetaOfSqlCType<SQL_C_UTINYINT>::type value{};
    SQLLEN indicator = -999;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_UTINYINT, &value, sizeof(value), &indicator);
    INFO("ret=" << ret << " value=" << (int)value << " indicator=" << indicator);
    if (ret == SQL_SUCCESS_WITH_INFO) {
      CHECK(value == 0);
      auto records = get_diag_rec(stmt);
      CHECK(records[0].sqlState == "01S07");
    } else {
      REQUIRE(ret == SQL_ERROR);
      auto records = get_diag_rec(stmt);
      CHECK(records[0].sqlState == "22003");
    }
  }

  // SQL_C_USHORT with -0.1
  {
    auto stmt = conn.execute_fetch("SELECT -0.1::FLOAT");
    typename MetaOfSqlCType<SQL_C_USHORT>::type value{};
    SQLLEN indicator = -999;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_USHORT, &value, sizeof(value), &indicator);
    INFO("ret=" << ret << " value=" << value << " indicator=" << indicator);
    if (ret == SQL_SUCCESS_WITH_INFO) {
      CHECK(value == 0);
      auto records = get_diag_rec(stmt);
      CHECK(records[0].sqlState == "01S07");
    } else {
      REQUIRE(ret == SQL_ERROR);
      auto records = get_diag_rec(stmt);
      CHECK(records[0].sqlState == "22003");
    }
  }

  // SQL_C_ULONG with -0.1
  {
    auto stmt = conn.execute_fetch("SELECT -0.1::FLOAT");
    typename MetaOfSqlCType<SQL_C_ULONG>::type value{};
    SQLLEN indicator = -999;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_ULONG, &value, sizeof(value), &indicator);
    INFO("ret=" << ret << " value=" << value << " indicator=" << indicator);
    if (ret == SQL_SUCCESS_WITH_INFO) {
      CHECK(value == 0);
      auto records = get_diag_rec(stmt);
      CHECK(records[0].sqlState == "01S07");
    } else {
      REQUIRE(ret == SQL_ERROR);
      auto records = get_diag_rec(stmt);
      CHECK(records[0].sqlState == "22003");
    }
  }

  // SQL_C_UBIGINT with -0.1
  {
    auto stmt = conn.execute_fetch("SELECT -0.1::FLOAT");
    typename MetaOfSqlCType<SQL_C_UBIGINT>::type value{};
    SQLLEN indicator = -999;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_UBIGINT, &value, sizeof(value), &indicator);
    INFO("ret=" << ret << " value=" << value << " indicator=" << indicator);
    if (ret == SQL_SUCCESS_WITH_INFO) {
      CHECK(value == 0);
      auto records = get_diag_rec(stmt);
      CHECK(records[0].sqlState == "01S07");
    } else {
      REQUIRE(ret == SQL_ERROR);
      auto records = get_diag_rec(stmt);
      CHECK(records[0].sqlState == "22003");
    }
  }

  // SQL_C_UTINYINT with -0.9
  {
    auto stmt = conn.execute_fetch("SELECT -0.9::FLOAT");
    typename MetaOfSqlCType<SQL_C_UTINYINT>::type value{};
    SQLLEN indicator = -999;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_UTINYINT, &value, sizeof(value), &indicator);
    INFO("ret=" << ret << " value=" << (int)value << " indicator=" << indicator);
    if (ret == SQL_SUCCESS_WITH_INFO) {
      CHECK(value == 0);
      auto records = get_diag_rec(stmt);
      CHECK(records[0].sqlState == "01S07");
    } else {
      REQUIRE(ret == SQL_ERROR);
      auto records = get_diag_rec(stmt);
      CHECK(records[0].sqlState == "22003");
    }
  }

  // SQL_C_USHORT with -0.9
  {
    auto stmt = conn.execute_fetch("SELECT -0.9::FLOAT");
    typename MetaOfSqlCType<SQL_C_USHORT>::type value{};
    SQLLEN indicator = -999;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_USHORT, &value, sizeof(value), &indicator);
    INFO("ret=" << ret << " value=" << value << " indicator=" << indicator);
    if (ret == SQL_SUCCESS_WITH_INFO) {
      CHECK(value == 0);
      auto records = get_diag_rec(stmt);
      CHECK(records[0].sqlState == "01S07");
    } else {
      REQUIRE(ret == SQL_ERROR);
      auto records = get_diag_rec(stmt);
      CHECK(records[0].sqlState == "22003");
    }
  }
}

TEST_CASE("REAL NaN to integer types returns error", "[e2e][types][real][conversion][c_integer][nan][edge]") {
  SKIP_OLD_DRIVER("BD#16", "Old driver silently converts NaN to 0 for integer targets");
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When NaN FLOAT value is fetched as integer C types
  (void)0;
  // Then SQL_ERROR is returned with SQLSTATE 22003
  check_numeric_out_of_range<SQL_C_SLONG>(conn.execute_fetch("SELECT 'NaN'::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_ULONG>(conn.execute_fetch("SELECT 'NaN'::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_SHORT>(conn.execute_fetch("SELECT 'NaN'::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_USHORT>(conn.execute_fetch("SELECT 'NaN'::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_STINYINT>(conn.execute_fetch("SELECT 'NaN'::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_UTINYINT>(conn.execute_fetch("SELECT 'NaN'::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_SBIGINT>(conn.execute_fetch("SELECT 'NaN'::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_UBIGINT>(conn.execute_fetch("SELECT 'NaN'::FLOAT"), 1);
}

TEST_CASE("REAL Infinity to integer types returns 22003", "[e2e][types][real][conversion][c_integer][infinity][edge]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Infinity FLOAT values are fetched as integer C types
  (void)0;
  // Then SQL_ERROR is returned with SQLSTATE 22003
  check_numeric_out_of_range<SQL_C_SLONG>(conn.execute_fetch("SELECT 'Infinity'::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_SHORT>(conn.execute_fetch("SELECT 'Infinity'::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_STINYINT>(conn.execute_fetch("SELECT 'Infinity'::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_SBIGINT>(conn.execute_fetch("SELECT 'Infinity'::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_ULONG>(conn.execute_fetch("SELECT 'Infinity'::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_USHORT>(conn.execute_fetch("SELECT 'Infinity'::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_UTINYINT>(conn.execute_fetch("SELECT 'Infinity'::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_UBIGINT>(conn.execute_fetch("SELECT 'Infinity'::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_SLONG>(conn.execute_fetch("SELECT '-Infinity'::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_SBIGINT>(conn.execute_fetch("SELECT '-Infinity'::FLOAT"), 1);
}

TEST_CASE("REAL NULL to SQL_C_INTEGER types", "[real][conversion][c_integer][null]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When NULL FLOAT values are queried
  const auto query = "SELECT NULL::FLOAT";
  // Then NULL FLOAT values return SQL_NULL_DATA
  check_null_via_get_data(conn.execute_fetch(query), 1, SQL_C_LONG);
  check_null_via_get_data(conn.execute_fetch(query), 1, SQL_C_SBIGINT);
}
