#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("should bind SQL_C_SLONG to SQL_VARCHAR and read back", "[c_numeric_types][conversion][sql_string]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  conn.execute("CREATE TABLE t_vc_long (col VARCHAR(100))");

  SQLINTEGER val = 42;
  SQLLEN ind = 0;

  // When SQL_C_SLONG 42 is bound to SQL_VARCHAR and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t_vc_long VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_VARCHAR, 100, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then fetching as SQL_C_CHAR yields 42
  auto sel = conn.createStatement();
  ret = SQLExecDirect(sel.getHandle(), sqlchar("SELECT col FROM t_vc_long"), SQL_NTS);
  REQUIRE_ODBC(ret, sel);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_CHAR>(sel, 1) == "42");
}

TEST_CASE("should bind SQL_C_DOUBLE to SQL_VARCHAR", "[c_numeric_types][conversion][sql_string]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  conn.execute("CREATE TABLE t_vc_dbl (col VARCHAR(100))");

  SQLDOUBLE val = 3.14;
  SQLLEN ind = 0;

  // When SQL_C_DOUBLE 3.14 is bound to SQL_VARCHAR and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t_vc_dbl VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_VARCHAR, 100, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the string representation contains 3.14
  auto sel = conn.createStatement();
  ret = SQLExecDirect(sel.getHandle(), sqlchar("SELECT col FROM t_vc_dbl"), SQL_NTS);
  REQUIRE_ODBC(ret, sel);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  std::string s = get_data<SQL_C_CHAR>(sel, 1);
  CHECK(s.find("3.14") != std::string::npos);
}

TEST_CASE("should bind SQL_C_BIT to SQL_VARCHAR", "[c_numeric_types][conversion][sql_string]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  conn.execute("CREATE TABLE t_vc_bit (col VARCHAR(100))");

  SQLCHAR val = 1;
  SQLLEN ind = 0;

  // When SQL_C_BIT 1 is bound to SQL_VARCHAR and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t_vc_bit VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BIT, SQL_VARCHAR, 100, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as the string 1
  auto sel = conn.createStatement();
  ret = SQLExecDirect(sel.getHandle(), sqlchar("SELECT col FROM t_vc_bit"), SQL_NTS);
  REQUIRE_ODBC(ret, sel);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_CHAR>(sel, 1) == "1");
}

TEST_CASE("should bind SQL_C_NUMERIC to SQL_VARCHAR", "[c_numeric_types][conversion][sql_string]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  conn.execute("CREATE TABLE t_vc_num (col VARCHAR(100))");

  SQL_NUMERIC_STRUCT ns = {};
  ns.precision = 10;
  ns.scale = 2;
  ns.sign = 1;
  set_numeric_magnitude(ns, 12345);
  SQLLEN ind = sizeof(ns);

  // When SQL_C_NUMERIC (123.45) is bound to SQL_VARCHAR and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t_vc_num VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_NUMERIC, SQL_VARCHAR, 100, 0, &ns, sizeof(ns), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as 123.45
  auto sel = conn.createStatement();
  ret = SQLExecDirect(sel.getHandle(), sqlchar("SELECT col FROM t_vc_num"), SQL_NTS);
  REQUIRE_ODBC(ret, sel);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  OLD_DRIVER_ONLY("BD#33") { CHECK(get_data<SQL_C_CHAR>(sel, 1) == "12345"); }
  NEW_DRIVER_ONLY("BD#33") { CHECK(get_data<SQL_C_CHAR>(sel, 1) == "123.45"); }
}

TEST_CASE("should bind SQL_C_SBIGINT to SQL_VARCHAR", "[c_numeric_types][conversion][sql_string]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  conn.execute("CREATE TABLE t_vc_sb (col VARCHAR(100))");

  SQLBIGINT val = 9999999999LL;
  SQLLEN ind = 0;

  // When SQL_C_SBIGINT 9999999999 is bound to SQL_VARCHAR and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t_vc_sb VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SBIGINT, SQL_VARCHAR, 100, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as 9999999999
  auto sel = conn.createStatement();
  ret = SQLExecDirect(sel.getHandle(), sqlchar("SELECT col FROM t_vc_sb"), SQL_NTS);
  REQUIRE_ODBC(ret, sel);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_CHAR>(sel, 1) == "9999999999");
}

TEST_CASE("should bind SQL_C_SLONG with NULL indicator to SQL_VARCHAR", "[c_numeric_types][conversion][sql_string]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  conn.execute("CREATE TABLE t_vc_null (col VARCHAR(100))");

  SQLINTEGER val = 0;
  SQLLEN ind = SQL_NULL_DATA;

  // When SQL_C_SLONG is bound with SQL_NULL_DATA and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t_vc_null VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_VARCHAR, 100, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the column is NULL when fetched as SQL_C_CHAR
  auto sel = conn.createStatement();
  ret = SQLExecDirect(sel.getHandle(), sqlchar("SELECT col FROM t_vc_null"), SQL_NTS);
  REQUIRE_ODBC(ret, sel);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data_optional<SQL_C_CHAR>(sel, 1) == std::nullopt);
}
