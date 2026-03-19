#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_diag_rec.hpp"
#include "macros.hpp"

TEST_CASE("SQL_DECIMAL to SQL_C_BINARY", "[fixed][conversion][c_binary]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  {
    INFO("integer value");
    // When NUMBER values are fetched as SQL_C_BINARY
    auto num = get_binary_as_numeric(conn.execute_fetch("SELECT 42::NUMBER(10,0)"), 1);
    // Then The result is a SQL_NUMERIC_STRUCT with correct sign, scale, and val bytes
    CHECK(num.sign == 1);
    CHECK(num.val[0] == 42);
    check_numeric_val_zero_from(num, 1);
  }

  {
    INFO("scaled value");
    // When NUMBER values with scale are fetched as SQL_C_BINARY
    auto num = get_binary_as_numeric(conn.execute_fetch("SELECT 123.45::NUMBER(10,2)"), 1);
    // Then The result is a SQL_NUMERIC_STRUCT with correct sign, scale, and val bytes
    CHECK(num.sign == 1);
    CHECK(num.scale == 0);
    CHECK(num.val[0] == 123);
  }

  {
    INFO("negative value");
    // When Negative NUMBER values are fetched as SQL_C_BINARY
    auto num = get_binary_as_numeric(conn.execute_fetch("SELECT -7::NUMBER(10,0)"), 1);
    // Then The result is a SQL_NUMERIC_STRUCT with correct sign, scale, and val bytes
    CHECK(num.sign == 0);
    CHECK(num.val[0] == 7);
  }
}

TEST_CASE("SQL_DECIMAL SQL_C_BINARY buffer too small returns 22003", "[fixed][conversion][c_binary][22003]") {
  SKIP_OLD_DRIVER("BD#12",
                  "Old driver does not return SQL_ERROR (22003) when SQL_C_BINARY buffer is too small for "
                  "SQL_NUMERIC_STRUCT");
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When A NUMBER value is fetched as SQL_C_BINARY into a buffer smaller than SQL_NUMERIC_STRUCT
  auto stmt = conn.execute_fetch("SELECT 42::NUMBER(10,0)");
  char small_buffer[4];
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, small_buffer, sizeof(small_buffer), &indicator);

  // Then SQL_ERROR is returned with SQLSTATE 22003
  CHECK(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "22003");
}

TEST_CASE("NUMBER NULL to SQL_C_BINARY", "[fixed][conversion][c_binary][null]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When A NULL NUMBER value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::NUMBER(10,0)");
  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_BINARY);
}
