#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_diag_rec.hpp"

// ============================================================================
// SQL_C_BINARY conversion from REAL
// Per ODBC spec, SQL_C_BINARY for SQL_REAL/SQL_FLOAT/SQL_DOUBLE writes the
// value as SQL_NUMERIC_STRUCT into the buffer.
// ============================================================================

TEST_CASE("REAL to SQL_C_BINARY", "[e2e][types][real][binary]") {
  // Given A Snowflake connection is established
  SKIP_OLD_DRIVER("BD#14",
                  "Old driver returns raw f64 bytes instead of SQL_NUMERIC_STRUCT for SQL_C_BINARY on FLOAT columns");
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When REAL values are fetched as SQL_C_BINARY
  (void)0;  // Brace blocks below perform the fetch and assertions
  // Then The result is a SQL_NUMERIC_STRUCT with correct sign, scale, and val bytes
  {
    auto num = get_binary_as_numeric(conn.execute_fetch("SELECT 42.0::FLOAT"), 1);
    CHECK(num.sign == 1);
    CHECK(num.val[0] == 42);
    check_numeric_val_zero_from(num, 1);
  }

  // negative integer value
  {
    auto num = get_binary_as_numeric(conn.execute_fetch("SELECT -7.0::FLOAT"), 1);
    CHECK(num.sign == 0);
    CHECK(num.val[0] == 7);
    check_numeric_val_zero_from(num, 1);
  }

  // zero
  {
    auto num = get_binary_as_numeric(conn.execute_fetch("SELECT 0.0::FLOAT"), 1);
    CHECK(num.sign == 1);
    check_numeric_val_zero_from(num, 0);
  }

  // fractional value truncates with 01S07
  {
    auto num = get_binary_as_numeric_with_truncation(conn.execute_fetch("SELECT 123.456::FLOAT"), 1);
    CHECK(num.sign == 1);
    CHECK(num.scale == 0);
    CHECK(numeric_val_to_ull(num) == 123);
    check_numeric_val_zero_from(num, 1);
  }

  // large integer value
  {
    auto num = get_binary_as_numeric(conn.execute_fetch("SELECT 1000000.0::FLOAT"), 1);
    CHECK(num.sign == 1);
    CHECK(numeric_val_to_ull(num) == 1000000);
    check_numeric_val_zero_from(num, 3);
  }

  // negative fractional truncates with 01S07
  {
    auto num = get_binary_as_numeric_with_truncation(conn.execute_fetch("SELECT -99.9::FLOAT"), 1);
    CHECK(num.sign == 0);
    CHECK(numeric_val_to_ull(num) == 99);
    check_numeric_val_zero_from(num, 1);
  }

  // value 255 uses single byte
  {
    auto num = get_binary_as_numeric(conn.execute_fetch("SELECT 255.0::FLOAT"), 1);
    CHECK(num.sign == 1);
    CHECK(num.val[0] == 255);
    check_numeric_val_zero_from(num, 1);
  }

  // value 256 spans two bytes
  {
    auto num = get_binary_as_numeric(conn.execute_fetch("SELECT 256.0::FLOAT"), 1);
    CHECK(num.sign == 1);
    CHECK(num.val[0] == 0);
    CHECK(num.val[1] == 1);
    check_numeric_val_zero_from(num, 2);
  }

  // NULL returns SQL_NULL_DATA
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::FLOAT"), 1, SQL_C_BINARY);
}

TEST_CASE("REAL SQL_C_BINARY buffer too small returns 22003", "[e2e][types][real][binary][22003]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When A REAL value is fetched as SQL_C_BINARY into a buffer smaller than SQL_NUMERIC_STRUCT
  auto stmt = conn.execute_fetch("SELECT 42.0::FLOAT");
  char tiny_buffer[4];
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, tiny_buffer, sizeof(tiny_buffer), &indicator);

  // Then SQL_ERROR is returned with SQLSTATE 22003
  CHECK(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "22003");
}

TEST_CASE("REAL SQL_C_BINARY negative zero", "[e2e][types][real][binary][edge]") {
  // Given A Snowflake connection is established
  SKIP_OLD_DRIVER("BD#14", "Old driver returns raw f64 bytes instead of SQL_NUMERIC_STRUCT for FLOAT columns");
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When -0.5 is fetched as SQL_C_BINARY
  auto stmt = conn.execute_fetch("SELECT -0.5::FLOAT");
  char buffer[100] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
  // Then SQL_SUCCESS_WITH_INFO with 01S07 and SQL_NUMERIC_STRUCT has sign=0, val=0
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  CHECK(get_sqlstate(stmt) == "01S07");
  REQUIRE(indicator == sizeof(SQL_NUMERIC_STRUCT));
  SQL_NUMERIC_STRUCT numeric;
  std::memcpy(&numeric, buffer, sizeof(SQL_NUMERIC_STRUCT));
  CHECK(numeric.sign == 0);
  CHECK(numeric.val[0] == 0);
}

TEST_CASE("REAL NULL to SQL_C_BINARY", "[real][conversion][c_binary][null]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When A NULL FLOAT value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::FLOAT");
  // Then NULL FLOAT values return SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_BINARY);
}
