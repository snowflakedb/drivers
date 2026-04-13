#include <limits>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("should bind SQL_C_DOUBLE nonzero to SQL_BIT.", "[query][bind_parameter][c_real_to_boolean]") {
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
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be TRUE
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

TEST_CASE("should bind SQL_C_FLOAT nonzero to SQL_BIT.", "[query][bind_parameter][c_real_to_boolean]") {
  SKIP_OLD_DRIVER("BD-35", "Old driver has limited SQL_C_FLOAT/DOUBLE support for SQL_BIT");
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
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be TRUE
  CHECK(get_data<SQL_C_BIT>(stmt, 1) == 1);
}

TEST_CASE("should bind SQL_C_DOUBLE negative to SQL_BIT.", "[query][bind_parameter][c_real_to_boolean]") {
  SKIP_OLD_DRIVER("BD-35", "Old driver rejects negative SQL_C_DOUBLE for SQL_BIT");
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
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be TRUE (negative nonzero)
  CHECK(get_data<SQL_C_BIT>(stmt, 1) == 1);
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

TEST_CASE("should reject SQL_C_DOUBLE NaN to SQL_BIT.", "[query][bind_parameter][c_real_to_boolean]") {
  SKIP_OLD_DRIVER("BD-35", "Old driver silently converts NaN to false instead of SQL_ERROR");
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
  // Then the execution should fail with SQLSTATE 22018 (invalid character value for cast)
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22018"));
}

TEST_CASE("should reject SQL_C_DOUBLE infinity to SQL_BIT.", "[query][bind_parameter][c_real_to_boolean]") {
  SKIP_OLD_DRIVER("BD-35", "Old driver may return different SQLSTATE for infinity");
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
  // Then the execution should fail with SQLSTATE 22018 (invalid character value for cast)
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22018"));
}

TEST_CASE("should reject SQL_C_FLOAT NaN to SQL_BIT.", "[query][bind_parameter][c_real_to_boolean]") {
  SKIP_OLD_DRIVER("BD-35", "Old driver has limited SQL_C_FLOAT/DOUBLE support for SQL_BIT");
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
  // Then the execution should fail with SQLSTATE 22018 (invalid character value for cast)
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22018"));
}
