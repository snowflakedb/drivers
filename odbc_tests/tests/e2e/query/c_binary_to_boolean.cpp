#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("should bind SQL_C_BINARY nonzero to SQL_BIT.", "[query][bind_parameter][c_binary_to_boolean]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLCHAR param[] = {0x01};
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as SQL_BIT and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_BIT, 1, 0, param,
                                   sizeof(param), &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be TRUE
  CHECK(get_data<SQL_C_BIT>(stmt, 1) == 1);
}

TEST_CASE("should bind SQL_C_BINARY zero to SQL_BIT.", "[query][bind_parameter][c_binary_to_boolean]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLCHAR param[] = {0x00};
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as SQL_BIT and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_BIT, 1, 0, param,
                                   sizeof(param), &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be FALSE
  CHECK(get_data<SQL_C_BIT>(stmt, 1) == 0);
}

TEST_CASE("should reject multi-byte SQL_C_BINARY for SQL_BIT.", "[query][bind_parameter][c_binary_to_boolean]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLCHAR param[] = {0x00, 0x01};
  SQLLEN indicator = sizeof(param);
  // When a multi-byte binary buffer is bound as SQL_BIT and executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_BIT, 1, 0, param,
                                   sizeof(param), &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  // Then the execution should fail with SQLSTATE 22003
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22003"));
}
