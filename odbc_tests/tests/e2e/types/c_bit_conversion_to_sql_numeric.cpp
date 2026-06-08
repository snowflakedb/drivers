#include <catch2/catch_approx.hpp>
#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_BIT true to SQL_INTEGER", "[c_bit][conversion][sql_numeric]") {
  // Given Snowflake client is logged in

  conn.execute("CREATE TEMPORARY TABLE t_bit_int (col INT)");

  SQLCHAR val = 1;
  SQLLEN ind = 0;

  // When SQL_C_BIT 1 is bound to SQL_INTEGER and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t_bit_int VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BIT, SQL_INTEGER, 10, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as SQL_C_SBIGINT 1
  auto sel = conn.createStatement();
  ret = SQLExecDirect(sel.getHandle(), sqlchar("SELECT col FROM t_bit_int"), SQL_NTS);
  REQUIRE_ODBC(ret, sel);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_SBIGINT>(sel, 1) == 1);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_BIT false to SQL_INTEGER", "[c_bit][conversion][sql_numeric]") {
  // Given Snowflake client is logged in

  conn.execute("CREATE TEMPORARY TABLE t_bit_int0 (col INT)");

  SQLCHAR val = 0;
  SQLLEN ind = 0;

  // When SQL_C_BIT 0 is bound to SQL_INTEGER and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t_bit_int0 VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BIT, SQL_INTEGER, 10, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as SQL_C_SBIGINT 0
  auto sel = conn.createStatement();
  ret = SQLExecDirect(sel.getHandle(), sqlchar("SELECT col FROM t_bit_int0"), SQL_NTS);
  REQUIRE_ODBC(ret, sel);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_SBIGINT>(sel, 1) == 0);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_BIT to SQL_DOUBLE", "[c_bit][conversion][sql_numeric]") {
  // Given Snowflake client is logged in

  conn.execute("CREATE TEMPORARY TABLE t_bit_flt (col FLOAT)");

  SQLCHAR val = 1;
  SQLLEN ind = 0;

  // When SQL_C_BIT 1 is bound to SQL_DOUBLE and inserted into FLOAT
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t_bit_flt VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BIT, SQL_DOUBLE, 15, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as SQL_C_DOUBLE 1.0
  auto sel = conn.createStatement();
  ret = SQLExecDirect(sel.getHandle(), sqlchar("SELECT col FROM t_bit_flt"), SQL_NTS);
  REQUIRE_ODBC(ret, sel);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_DOUBLE>(sel, 1) == Catch::Approx(1.0));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_BIT to SQL_BIT", "[c_bit][conversion][sql_numeric]") {
  // Given Snowflake client is logged in

  conn.execute("CREATE TEMPORARY TABLE t_bit_bool (col BOOLEAN)");

  SQLCHAR val = 1;
  SQLLEN ind = 0;

  // When SQL_C_BIT 1 is bound to SQL_BIT and inserted into BOOLEAN
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t_bit_bool VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BIT, SQL_BIT, 1, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as SQL_C_BIT 1
  auto sel = conn.createStatement();
  ret = SQLExecDirect(sel.getHandle(), sqlchar("SELECT col FROM t_bit_bool"), SQL_NTS);
  REQUIRE_ODBC(ret, sel);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_BIT>(sel, 1) == 1);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_BIT with NULL indicator", "[c_bit][conversion][sql_numeric]") {
  // Given Snowflake client is logged in

  conn.execute("CREATE TEMPORARY TABLE t_bit_null (col INT)");

  SQLCHAR val = 0;
  SQLLEN ind = SQL_NULL_DATA;

  // When SQL_C_BIT is bound with SQL_NULL_DATA and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t_bit_null VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BIT, SQL_INTEGER, 10, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the column is NULL
  auto sel = conn.createStatement();
  ret = SQLExecDirect(sel.getHandle(), sqlchar("SELECT col FROM t_bit_null"), SQL_NTS);
  REQUIRE_ODBC(ret, sel);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data_optional<SQL_C_SBIGINT>(sel, 1) == std::nullopt);
}
