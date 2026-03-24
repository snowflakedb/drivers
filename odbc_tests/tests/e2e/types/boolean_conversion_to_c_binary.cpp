#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "odbc_matchers.hpp"

// ============================================================================
// SQL_C_BINARY
// ============================================================================

TEST_CASE("should convert boolean to SQL_C_BINARY", "[datatype][boolean][conversion][binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed
  const auto stmt = conn.execute_fetch("SELECT TRUE::BOOLEAN, FALSE::BOOLEAN");

  // Then SQL_C_BINARY should return byte 0x01 for TRUE and 0x00 for FALSE
  SQLCHAR true_buf[16] = {};
  SQLLEN true_ind = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, true_buf, sizeof(true_buf), &true_ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(true_ind == 1);
  REQUIRE(true_buf[0] == 0x01);

  SQLCHAR false_buf[16] = {};
  SQLLEN false_ind = 0;
  ret = SQLGetData(stmt.getHandle(), 2, SQL_C_BINARY, false_buf, sizeof(false_buf), &false_ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(false_ind == 1);
  REQUIRE(false_buf[0] == 0x00);
}
