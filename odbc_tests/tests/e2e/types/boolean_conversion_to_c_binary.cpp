#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "odbc_cast.hpp"
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

// ============================================================================
// NULL handling
// ============================================================================

TEST_CASE("should handle NULL boolean with SQL_C_BINARY", "[datatype][boolean][conversion][binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT NULL::BOOLEAN" is executed
  const auto stmt = conn.execute_fetch("SELECT NULL::BOOLEAN");

  // Then SQL_C_BINARY should return SQL_NULL_DATA indicator
  SQLCHAR buffer[16] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(indicator == SQL_NULL_DATA);
}

// ============================================================================
// SQLBindCol
// ============================================================================

TEST_CASE("should convert boolean using SQLBindCol for SQL_C_BINARY", "[datatype][boolean][conversion][binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed with SQLBindCol for SQL_C_BINARY
  const auto stmt = conn.createStatement();
  SQLCHAR true_buf = 0xFF;
  SQLCHAR false_buf = 0xFF;
  SQLLEN true_ind = 0;
  SQLLEN false_ind = 0;

  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_BINARY, &true_buf, sizeof(true_buf), &true_ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindCol(stmt.getHandle(), 2, SQL_C_BINARY, &false_buf, sizeof(false_buf), &false_ind);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT TRUE::BOOLEAN, FALSE::BOOLEAN"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then Bound buffers should contain 0x01 for TRUE and 0x00 for FALSE
  REQUIRE(true_buf == 0x01);
  REQUIRE(false_buf == 0x00);
  REQUIRE(true_ind == 1);
  REQUIRE(false_ind == 1);
}
