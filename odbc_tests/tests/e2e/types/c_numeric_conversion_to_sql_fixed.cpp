#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("should bind SQL_C_NUMERIC to SQL_INTEGER and read back", "[c_numeric][conversion][sql_fixed]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  conn.execute("CREATE TABLE t_num_int (col NUMBER)");

  SQL_NUMERIC_STRUCT ns = {};
  ns.precision = 10;
  ns.scale = 0;
  ns.sign = 1;
  set_numeric_magnitude(ns, 42);
  SQLLEN ind = sizeof(ns);

  // When SQL_C_NUMERIC is bound to SQL_INTEGER and a row is inserted into NUMBER
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t_num_int VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_NUMERIC, SQL_INTEGER, 10, 0, &ns, sizeof(ns), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the stored value is read back as SQL_C_SBIGINT 42
  auto sel = conn.createStatement();
  ret = SQLExecDirect(sel.getHandle(), sqlchar("SELECT col FROM t_num_int"), SQL_NTS);
  REQUIRE_ODBC(ret, sel);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_SBIGINT>(sel, 1) == 42);
}

TEST_CASE("should bind negative SQL_C_NUMERIC to SQL_BIGINT", "[c_numeric][conversion][sql_fixed]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  conn.execute("CREATE TABLE t_num_big (col BIGINT)");

  SQL_NUMERIC_STRUCT ns = {};
  ns.precision = 10;
  ns.scale = 0;
  ns.sign = 0;
  set_numeric_magnitude(ns, 99);

  SQLLEN ind = sizeof(ns);

  // When a negative SQL_C_NUMERIC (sign 0, magnitude 99) is bound to SQL_BIGINT and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t_num_big VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_NUMERIC, SQL_BIGINT, 19, 0, &ns, sizeof(ns), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as -99
  auto sel = conn.createStatement();
  ret = SQLExecDirect(sel.getHandle(), sqlchar("SELECT col FROM t_num_big"), SQL_NTS);
  REQUIRE_ODBC(ret, sel);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_SBIGINT>(sel, 1) == -99);
}

TEST_CASE("should bind SQL_C_NUMERIC with scale to SQL_DECIMAL", "[c_numeric][conversion][sql_fixed]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  conn.execute("CREATE TABLE t_num_dec (col DECIMAL(10,2))");

  SQL_NUMERIC_STRUCT ns = {};
  ns.precision = 10;
  ns.scale = 2;
  ns.sign = 1;
  set_numeric_magnitude(ns, 12345);
  SQLLEN ind = sizeof(ns);

  // When SQL_C_NUMERIC with scale 2 (123.45) is bound to SQL_DECIMAL and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t_num_dec VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_NUMERIC, SQL_DECIMAL, 10, 2, &ns, sizeof(ns), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then fetching as SQL_C_CHAR yields 123.45
  auto sel = conn.createStatement();
  ret = SQLExecDirect(sel.getHandle(), sqlchar("SELECT col FROM t_num_dec"), SQL_NTS);
  REQUIRE_ODBC(ret, sel);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  OLD_DRIVER_ONLY("BD#33") { CHECK(get_data<SQL_C_CHAR>(sel, 1) == "12345.00"); }
  NEW_DRIVER_ONLY("BD#33") { CHECK(get_data<SQL_C_CHAR>(sel, 1) == "123.45"); }
}

TEST_CASE("should bind SQL_C_NUMERIC zero", "[c_numeric][conversion][sql_fixed]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  conn.execute("CREATE TABLE t_num_zero (col NUMBER)");

  SQL_NUMERIC_STRUCT ns = {};
  ns.precision = 10;
  ns.scale = 0;
  ns.sign = 1;
  set_numeric_magnitude(ns, 0);
  SQLLEN ind = sizeof(ns);

  // When SQL_C_NUMERIC zero is bound to SQL_INTEGER and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t_num_zero VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_NUMERIC, SQL_INTEGER, 10, 0, &ns, sizeof(ns), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as 0
  auto sel = conn.createStatement();
  ret = SQLExecDirect(sel.getHandle(), sqlchar("SELECT col FROM t_num_zero"), SQL_NTS);
  REQUIRE_ODBC(ret, sel);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_SBIGINT>(sel, 1) == 0);
}

TEST_CASE("should bind large SQL_C_NUMERIC exceeding 64-bit range", "[c_numeric][conversion][sql_fixed]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  conn.execute("CREATE TABLE t_num_big128 (col NUMBER(38,0))");

  // 10^20 = 100000000000000000000 which exceeds UINT64_MAX
  SQL_NUMERIC_STRUCT ns = {};
  ns.precision = 38;
  ns.scale = 0;
  ns.sign = 1;
  set_numeric_magnitude_128(ns, 0x6BC75E2D63100000ULL, 0x5ULL);
  SQLLEN ind = sizeof(ns);

  // When a large SQL_C_NUMERIC exceeding 64-bit range is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t_num_big128 VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_NUMERIC, SQL_DECIMAL, 38, 0, &ns, sizeof(ns), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as the string 100000000000000000000
  auto sel = conn.createStatement();
  ret = SQLExecDirect(sel.getHandle(), sqlchar("SELECT col FROM t_num_big128"), SQL_NTS);
  REQUIRE_ODBC(ret, sel);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_CHAR>(sel, 1) == "100000000000000000000");
}

TEST_CASE("should reject SQL_C_NUMERIC overflow into NUMBER(3,0)", "[c_numeric][conversion][sql_fixed]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  conn.execute("CREATE TABLE t_num_ovf (col NUMBER(3,0))");

  SQL_NUMERIC_STRUCT ns = {};
  ns.precision = 10;
  ns.scale = 0;
  ns.sign = 1;
  set_numeric_magnitude(ns, 99999);
  SQLLEN ind = sizeof(ns);

  // When SQL_C_NUMERIC with value 99999 is bound to a NUMBER(3,0) column and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t_num_ovf VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_NUMERIC, SQL_DECIMAL, 10, 0, &ns, sizeof(ns), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then the server rejects the value with an error
  CHECK(ret == SQL_ERROR);
}

TEST_CASE("should bind SQL_C_NUMERIC with NULL indicator", "[c_numeric][conversion][sql_fixed]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  conn.execute("CREATE TABLE t_num_null (col NUMBER)");

  SQL_NUMERIC_STRUCT ns = {};
  SQLLEN ind = SQL_NULL_DATA;

  // When SQL_C_NUMERIC is bound with SQL_NULL_DATA and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t_num_null VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_NUMERIC, SQL_INTEGER, 10, 0, &ns, sizeof(ns), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the column is NULL when fetched
  auto sel = conn.createStatement();
  ret = SQLExecDirect(sel.getHandle(), sqlchar("SELECT col FROM t_num_null"), SQL_NTS);
  REQUIRE_ODBC(ret, sel);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data_optional<SQL_C_SBIGINT>(sel, 1) == std::nullopt);
}
