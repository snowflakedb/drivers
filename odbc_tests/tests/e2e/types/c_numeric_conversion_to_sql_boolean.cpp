#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"

TEST_CASE("should bind SQL_C_NUMERIC nonzero to SQL_BIT.", "[query][bind_parameter][c_numeric_to_boolean]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQL_NUMERIC_STRUCT param = {};
  param.precision = 10;
  param.scale = 0;
  param.sign = 1;
  param.val[0] = 1;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as SQL_BIT and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_NUMERIC, SQL_BIT, 1, 0, &param,
                                   sizeof(param), &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be TRUE
  CHECK(get_data<SQL_C_BIT>(stmt, 1) == 1);
}

TEST_CASE("should bind SQL_C_NUMERIC zero to SQL_BIT.", "[query][bind_parameter][c_numeric_to_boolean]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQL_NUMERIC_STRUCT param = {};
  param.precision = 10;
  param.scale = 0;
  param.sign = 1;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as SQL_BIT and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_NUMERIC, SQL_BIT, 1, 0, &param,
                                   sizeof(param), &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be FALSE
  CHECK(get_data<SQL_C_BIT>(stmt, 1) == 0);
}
