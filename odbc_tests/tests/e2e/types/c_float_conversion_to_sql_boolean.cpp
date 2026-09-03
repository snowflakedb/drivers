#include <limits>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("should bind SQL_C_DOUBLE 1.5 to SQL_BIT with 01S07.", "[query][bind_parameter][c_real_to_boolean]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLDOUBLE param = 1.5;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as SQL_BIT and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_BIT, 1, 0, &param,
                                   sizeof(param), &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  // Then the fractional part is truncated and 01S07 is reported
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccessWithInfo() && OdbcMatchers::HasSqlState("01S07"));
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_BIT>(stmt, 1) == 1);
}

TEST_CASE("should bind SQL_C_DOUBLE zero to SQL_BIT.", "[query][bind_parameter][c_real_to_boolean]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLDOUBLE param = 0.0;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as SQL_BIT and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_BIT, 1, 0, &param,
                                   sizeof(param), &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be FALSE
  CHECK(get_data<SQL_C_BIT>(stmt, 1) == 0);
}

TEST_CASE("should bind SQL_C_FLOAT 0.5 to SQL_BIT with 01S07.", "[query][bind_parameter][c_real_to_boolean]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLREAL param = 0.5f;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as SQL_BIT and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_FLOAT, SQL_BIT, 1, 0, &param,
                                   sizeof(param), &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  // Then the fractional part is truncated and 01S07 is reported
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsSuccessWithInfo() && OdbcMatchers::HasSqlState("01S07"));
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_BIT>(stmt, 1) == 0);
}

TEST_CASE("should reject SQL_C_DOUBLE negative bound to SQL_BIT.", "[query][bind_parameter][c_real_to_boolean]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLDOUBLE param = -3.14;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as SQL_BIT and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_BIT, 1, 0, &param,
                                   sizeof(param), &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  // Then the driver rejects values other than 0 or 1 with 22003
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22003"));
}

TEST_CASE("should bind SQL_C_FLOAT zero to SQL_BIT.", "[query][bind_parameter][c_real_to_boolean]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLREAL param = 0.0f;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as SQL_BIT and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_FLOAT, SQL_BIT, 1, 0, &param,
                                   sizeof(param), &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be FALSE
  CHECK(get_data<SQL_C_BIT>(stmt, 1) == 0);
}

TEST_CASE("should bind SQL_C_DOUBLE NULL to SQL_BIT.", "[query][bind_parameter][c_real_to_boolean]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLLEN indicator = SQL_NULL_DATA;
  // When SQL_C_DOUBLE with SQL_NULL_DATA is bound as SQL_BIT and SELECT ? is executed
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_BIT, 1, 0, nullptr, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be NULL
  CHECK(get_data_optional<SQL_C_BIT>(stmt, 1) == std::nullopt);
}

TEST_CASE("should bind SQL_C_DOUBLE NaN to SQL_BIT as true.", "[query][bind_parameter][c_real_to_boolean]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLDOUBLE param = std::numeric_limits<SQLDOUBLE>::quiet_NaN();
  SQLLEN indicator = sizeof(param);
  // When SQL_C_DOUBLE NaN is bound as SQL_BIT and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_BIT, 1, 0, &param,
                                   sizeof(param), &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then NaN is stored as true
  CHECK(get_data<SQL_C_BIT>(stmt, 1) == 1);
}

TEST_CASE("should reject SQL_C_DOUBLE infinity to SQL_BIT.", "[query][bind_parameter][c_real_to_boolean]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLDOUBLE param = std::numeric_limits<SQLDOUBLE>::infinity();
  SQLLEN indicator = sizeof(param);
  // When SQL_C_DOUBLE infinity is bound as SQL_BIT and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_BIT, 1, 0, &param,
                                   sizeof(param), &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  // Then the driver rejects values other than 0 or 1 with 22003
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22003"));
}

TEST_CASE("should bind SQL_C_FLOAT NaN to SQL_BIT as true.", "[query][bind_parameter][c_real_to_boolean]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLREAL param = std::numeric_limits<SQLREAL>::quiet_NaN();
  SQLLEN indicator = sizeof(param);
  // When SQL_C_FLOAT NaN is bound as SQL_BIT and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_FLOAT, SQL_BIT, 1, 0, &param,
                                   sizeof(param), &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then NaN is stored as true
  CHECK(get_data<SQL_C_BIT>(stmt, 1) == 1);
}
