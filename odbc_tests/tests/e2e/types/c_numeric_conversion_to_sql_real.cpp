#include <catch2/catch_approx.hpp>
#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("should bind SQL_C_NUMERIC to SQL_DOUBLE and read back", "[c_numeric][conversion][sql_real]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  conn.execute("CREATE TABLE t_num_dbl (col FLOAT)");

  SQL_NUMERIC_STRUCT ns = {};
  ns.precision = 10;
  ns.scale = 0;
  ns.sign = 1;
  set_numeric_magnitude(ns, 42);
  SQLLEN ind = sizeof(ns);

  // When SQL_C_NUMERIC is bound to SQL_DOUBLE and inserted into FLOAT
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t_num_dbl VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_NUMERIC, SQL_DOUBLE, 15, 0, &ns, sizeof(ns), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as SQL_C_DOUBLE 42
  auto sel = conn.createStatement();
  ret = SQLExecDirect(sel.getHandle(), sqlchar("SELECT col FROM t_num_dbl"), SQL_NTS);
  REQUIRE_ODBC(ret, sel);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_DOUBLE>(sel, 1) == Catch::Approx(42.0));
}

TEST_CASE("should bind SQL_C_NUMERIC with scale to SQL_DOUBLE", "[c_numeric][conversion][sql_real]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  conn.execute("CREATE TABLE t_num_dbl_sc (col FLOAT)");

  SQL_NUMERIC_STRUCT ns = {};
  ns.precision = 10;
  ns.scale = 2;
  ns.sign = 1;
  set_numeric_magnitude(ns, 314);
  SQLLEN ind = sizeof(ns);

  // When SQL_C_NUMERIC with scale 2 (3.14) is bound to SQL_DOUBLE and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t_num_dbl_sc VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_NUMERIC, SQL_DOUBLE, 15, 0, &ns, sizeof(ns), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is approximately 3.14
  auto sel = conn.createStatement();
  ret = SQLExecDirect(sel.getHandle(), sqlchar("SELECT col FROM t_num_dbl_sc"), SQL_NTS);
  REQUIRE_ODBC(ret, sel);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  OLD_DRIVER_ONLY("BD#33") { CHECK(get_data<SQL_C_DOUBLE>(sel, 1) == Catch::Approx(314.0)); }
  NEW_DRIVER_ONLY("BD#33") { CHECK(get_data<SQL_C_DOUBLE>(sel, 1) == Catch::Approx(3.14)); }
}

TEST_CASE("should bind large SQL_C_NUMERIC exceeding 64-bit to SQL_DOUBLE", "[c_numeric][conversion][sql_real]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  conn.execute("CREATE TABLE t_num_dbl_big (col FLOAT)");

  // 10^20 = 100000000000000000000 which exceeds UINT64_MAX
  SQL_NUMERIC_STRUCT ns = {};
  ns.precision = 38;
  ns.scale = 0;
  ns.sign = 1;
  set_numeric_magnitude_128(ns, 0x6BC75E2D63100000ULL, 0x5ULL);
  SQLLEN ind = sizeof(ns);

  // When a large SQL_C_NUMERIC exceeding 64-bit range is bound to SQL_DOUBLE and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t_num_dbl_big VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_NUMERIC, SQL_DOUBLE, 38, 0, &ns, sizeof(ns), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is approximately 1e20
  auto sel = conn.createStatement();
  ret = SQLExecDirect(sel.getHandle(), sqlchar("SELECT col FROM t_num_dbl_big"), SQL_NTS);
  REQUIRE_ODBC(ret, sel);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_DOUBLE>(sel, 1) == Catch::Approx(1.0e20));
}

TEST_CASE("should bind SQL_C_NUMERIC with NULL indicator to SQL_DOUBLE", "[c_numeric][conversion][sql_real]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  conn.execute("CREATE TABLE t_num_dbl_null (col FLOAT)");

  SQL_NUMERIC_STRUCT ns = {};
  SQLLEN ind = SQL_NULL_DATA;

  // When SQL_C_NUMERIC is bound with SQL_NULL_DATA to SQL_DOUBLE and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t_num_dbl_null VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_NUMERIC, SQL_DOUBLE, 15, 0, &ns, sizeof(ns), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the column is NULL when fetched as SQL_C_DOUBLE
  auto sel = conn.createStatement();
  ret = SQLExecDirect(sel.getHandle(), sqlchar("SELECT col FROM t_num_dbl_null"), SQL_NTS);
  REQUIRE_ODBC(ret, sel);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data_optional<SQL_C_DOUBLE>(sel, 1) == std::nullopt);
}
