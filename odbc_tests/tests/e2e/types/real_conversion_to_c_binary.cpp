#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "SchemaFixtures.hpp"
#include "conversion_checks.hpp"
#include "get_diag_rec.hpp"

static double get_binary_as_double(const StatementHandleWrapper& stmt, SQLUSMALLINT col) {
  double val;
  std::memset(&val, 0xFF, sizeof(val));
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), col, SQL_C_BINARY, &val, sizeof(val), &indicator);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(indicator == sizeof(double));
  return val;
}

TEST_CASE_METHOD(ConnSchemaFixture, "REAL to SQL_C_BINARY", "[e2e][types][real][binary]") {
  // Given A Snowflake connection is established
  // When REAL values are fetched as SQL_C_BINARY
  // Then The result is the native 8-byte IEEE 754 double
  CHECK(get_binary_as_double(conn.execute_fetch("SELECT 42.0::FLOAT"), 1) == 42.0);
  CHECK(get_binary_as_double(conn.execute_fetch("SELECT -7.0::FLOAT"), 1) == -7.0);
  CHECK(get_binary_as_double(conn.execute_fetch("SELECT 0.0::FLOAT"), 1) == 0.0);
  CHECK(get_binary_as_double(conn.execute_fetch("SELECT 123.456::FLOAT"), 1) == 123.456);
  CHECK(get_binary_as_double(conn.execute_fetch("SELECT 1000000.0::FLOAT"), 1) == 1000000.0);
  CHECK(get_binary_as_double(conn.execute_fetch("SELECT -99.9::FLOAT"), 1) == -99.9);
  CHECK(get_binary_as_double(conn.execute_fetch("SELECT 255.0::FLOAT"), 1) == 255.0);
  CHECK(get_binary_as_double(conn.execute_fetch("SELECT 256.0::FLOAT"), 1) == 256.0);
}

TEST_CASE_METHOD(ConnSchemaFixture, "REAL SQL_C_BINARY buffer too small returns 22003",
                 "[e2e][types][real][binary][22003]") {
  // Given A Snowflake connection is established
  // When A REAL value is fetched as SQL_C_BINARY into a buffer smaller than sizeof(double)
  auto stmt = conn.execute_fetch("SELECT 42.0::FLOAT");
  char tiny_buffer[4];
  std::memset(tiny_buffer, 0xFF, sizeof(tiny_buffer));
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, tiny_buffer, sizeof(tiny_buffer), &indicator);

  // Then SQL_ERROR is returned with SQLSTATE 22003 and the buffer is untouched
  CHECK(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "22003");
  CHECK(std::memcmp(tiny_buffer, "\xFF\xFF\xFF\xFF", sizeof(tiny_buffer)) == 0);
}

TEST_CASE_METHOD(ConnSchemaFixture, "REAL NULL to SQL_C_BINARY", "[real][conversion][c_binary][null]") {
  // Given A Snowflake connection is established
  // When A NULL FLOAT value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::FLOAT");
  // Then NULL FLOAT values return SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_BINARY);
}

TEST_CASE_METHOD(ConnSchemaFixture, "REAL to SQL_C_DOUBLE, SQL_C_NUMERIC and SQL_C_BINARY",
                 "[e2e][types][real][binary]") {
  // Given A Snowflake connection is established
  // When A FLOAT value is fetched as SQL_C_DOUBLE, SQL_C_NUMERIC, and SQL_C_BINARY
  // Then DOUBLE and BINARY keep the fractional value; NUMERIC truncates with 01S07
  {
    auto stmt = conn.execute_fetch("SELECT 42.5::FLOAT");
    char buffer[100];
    std::memset(buffer, 0xFF, sizeof(buffer));
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DOUBLE, buffer, sizeof(buffer), &indicator);
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(indicator == sizeof(double));
    double val;
    std::memcpy(&val, buffer, sizeof(double));
    CHECK(val == 42.5);
    CHECK(buffer[8] == static_cast<char>(0xFF));
  }

  {
    auto numeric = check_fractional_truncation<SQL_C_NUMERIC>(conn.execute_fetch("SELECT 42.5::FLOAT"), 1);
    CHECK(numeric.sign == 1);
    CHECK(numeric.val[0] == 42);
  }

  {
    auto stmt = conn.execute_fetch("SELECT 42.5::FLOAT");
    char buffer[100];
    std::memset(buffer, 0xFF, sizeof(buffer));
    SQLLEN indicator = 0;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(indicator == sizeof(double));
    double val;
    std::memcpy(&val, buffer, sizeof(double));
    CHECK(val == 42.5);
    CHECK(buffer[8] == static_cast<char>(0xFF));
  }
}
