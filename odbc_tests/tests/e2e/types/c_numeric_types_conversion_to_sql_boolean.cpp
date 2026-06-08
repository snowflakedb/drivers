#include <cstring>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_SLONG one to SQL_BIT",
                 "[c_numeric_types][conversion][sql_boolean]") {
  // Given Snowflake client is logged in

  conn.execute("CREATE TEMPORARY TABLE t_long1 (col BOOLEAN)");

  SQLINTEGER val = 1;
  SQLLEN ind = 0;

  // When SQL_C_SLONG 1 is bound to SQL_BIT and inserted into BOOLEAN
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t_long1 VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_BIT, 1, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as SQL_C_BIT 1
  auto sel = conn.createStatement();
  ret = SQLExecDirect(sel.getHandle(), sqlchar("SELECT col FROM t_long1"), SQL_NTS);
  REQUIRE_ODBC(ret, sel);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_BIT>(sel, 1) == 1);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_SLONG zero to SQL_BIT",
                 "[c_numeric_types][conversion][sql_boolean]") {
  // Given Snowflake client is logged in

  conn.execute("CREATE TEMPORARY TABLE t_long0 (col BOOLEAN)");

  SQLINTEGER val = 0;
  SQLLEN ind = 0;

  // When SQL_C_SLONG 0 is bound to SQL_BIT and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t_long0 VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_BIT, 1, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as SQL_C_BIT 0
  auto sel = conn.createStatement();
  ret = SQLExecDirect(sel.getHandle(), sqlchar("SELECT col FROM t_long0"), SQL_NTS);
  REQUIRE_ODBC(ret, sel);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_BIT>(sel, 1) == 0);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_DOUBLE one to SQL_BIT",
                 "[c_numeric_types][conversion][sql_boolean]") {
  // Given Snowflake client is logged in

  conn.execute("CREATE TEMPORARY TABLE t_dbl1 (col BOOLEAN)");

  SQLDOUBLE val = 1.0;
  SQLLEN ind = 0;

  // When SQL_C_DOUBLE 1.0 is bound to SQL_BIT and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t_dbl1 VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_BIT, 1, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as SQL_C_BIT 1
  auto sel = conn.createStatement();
  ret = SQLExecDirect(sel.getHandle(), sqlchar("SELECT col FROM t_dbl1"), SQL_NTS);
  REQUIRE_ODBC(ret, sel);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_BIT>(sel, 1) == 1);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_DOUBLE zero to SQL_BIT",
                 "[c_numeric_types][conversion][sql_boolean]") {
  // Given Snowflake client is logged in

  conn.execute("CREATE TEMPORARY TABLE t_dbl0 (col BOOLEAN)");

  SQLDOUBLE val = 0.0;
  SQLLEN ind = 0;

  // When SQL_C_DOUBLE 0.0 is bound to SQL_BIT and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t_dbl0 VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_BIT, 1, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as SQL_C_BIT 0
  auto sel = conn.createStatement();
  ret = SQLExecDirect(sel.getHandle(), sqlchar("SELECT col FROM t_dbl0"), SQL_NTS);
  REQUIRE_ODBC(ret, sel);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_BIT>(sel, 1) == 0);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_BIT to SQL_BIT", "[c_numeric_types][conversion][sql_boolean]") {
  // Given Snowflake client is logged in

  conn.execute("CREATE TEMPORARY TABLE t_bbit (col BOOLEAN)");

  SQLCHAR val = 1;
  SQLLEN ind = 0;

  // When SQL_C_BIT 1 is bound to SQL_BIT and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t_bbit VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BIT, SQL_BIT, 1, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as SQL_C_BIT 1
  auto sel = conn.createStatement();
  ret = SQLExecDirect(sel.getHandle(), sqlchar("SELECT col FROM t_bbit"), SQL_NTS);
  REQUIRE_ODBC(ret, sel);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_BIT>(sel, 1) == 1);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_SLONG negative to SQL_BIT as true",
                 "[c_numeric_types][conversion][sql_boolean]") {
  // Given Snowflake client is logged in

  conn.execute("CREATE TEMPORARY TABLE t_long_neg (col BOOLEAN)");

  SQLINTEGER val = -1;
  SQLLEN ind = 0;

  // When SQL_C_SLONG -1 is bound to SQL_BIT and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t_long_neg VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_BIT, 1, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  OLD_DRIVER_ONLY("BD#35") {
    // Then the old driver rejects negative values with 22003
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22003"));
  }
  NEW_DRIVER_ONLY("BD#35") {
    // Then a nonzero value is stored as true (SQL_C_BIT 1)
    REQUIRE_ODBC(ret, stmt);
    auto sel = conn.createStatement();
    ret = SQLExecDirect(sel.getHandle(), sqlchar("SELECT col FROM t_long_neg"), SQL_NTS);
    REQUIRE_ODBC(ret, sel);
    ret = SQLFetch(sel.getHandle());
    REQUIRE_ODBC(ret, sel);
    CHECK(get_data<SQL_C_BIT>(sel, 1) == 1);
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_NUMERIC nonzero to SQL_BIT as true",
                 "[c_numeric_types][conversion][sql_boolean]") {
  // Given Snowflake client is logged in

  conn.execute("CREATE TEMPORARY TABLE t_num_bool (col BOOLEAN)");

  SQL_NUMERIC_STRUCT ns = {};
  ns.precision = 10;
  ns.scale = 0;
  ns.sign = 1;
  set_numeric_magnitude(ns, 42);
  SQLLEN ind = sizeof(ns);

  // When SQL_C_NUMERIC with value 42 is bound to SQL_BIT and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t_num_bool VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_NUMERIC, SQL_BIT, 10, 0, &ns, sizeof(ns), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  OLD_DRIVER_ONLY("BD#37") {
    // Then the old driver rejects nonzero SQL_C_NUMERIC values with 22003
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22003"));
  }
  NEW_DRIVER_ONLY("BD#37") {
    // Then a nonzero numeric is stored as true (SQL_C_BIT 1)
    REQUIRE_ODBC(ret, stmt);
    auto sel = conn.createStatement();
    ret = SQLExecDirect(sel.getHandle(), sqlchar("SELECT col FROM t_num_bool"), SQL_NTS);
    REQUIRE_ODBC(ret, sel);
    ret = SQLFetch(sel.getHandle());
    REQUIRE_ODBC(ret, sel);
    CHECK(get_data<SQL_C_BIT>(sel, 1) == 1);
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_NUMERIC zero to SQL_BIT as false",
                 "[c_numeric_types][conversion][sql_boolean]") {
  // Given Snowflake client is logged in

  conn.execute("CREATE TEMPORARY TABLE t_num_bool0 (col BOOLEAN)");

  SQL_NUMERIC_STRUCT ns = {};
  ns.precision = 10;
  ns.scale = 0;
  ns.sign = 1;
  set_numeric_magnitude(ns, 0);
  SQLLEN ind = sizeof(ns);

  // When SQL_C_NUMERIC with value 0 is bound to SQL_BIT and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t_num_bool0 VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_NUMERIC, SQL_BIT, 10, 0, &ns, sizeof(ns), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then a zero numeric is stored as false (SQL_C_BIT 0)
  auto sel = conn.createStatement();
  ret = SQLExecDirect(sel.getHandle(), sqlchar("SELECT col FROM t_num_bool0"), SQL_NTS);
  REQUIRE_ODBC(ret, sel);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_BIT>(sel, 1) == 0);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_NUMERIC negative to SQL_BIT as true",
                 "[c_numeric_types][conversion][sql_boolean]") {
  // Given Snowflake client is logged in

  conn.execute("CREATE TEMPORARY TABLE t_num_bool_neg (col BOOLEAN)");

  SQL_NUMERIC_STRUCT ns = {};
  ns.precision = 10;
  ns.scale = 0;
  ns.sign = 0;
  set_numeric_magnitude(ns, 7);
  SQLLEN ind = sizeof(ns);

  // When SQL_C_NUMERIC with negative value (sign=0, magnitude=7) is bound to SQL_BIT and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t_num_bool_neg VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_NUMERIC, SQL_BIT, 10, 0, &ns, sizeof(ns), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then a negative numeric is stored as true (SQL_C_BIT 1)
  OLD_DRIVER_ONLY("BD#37") {
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22003"));
  }
  NEW_DRIVER_ONLY("BD#37") {
    REQUIRE_ODBC(ret, stmt);
    auto sel = conn.createStatement();
    ret = SQLExecDirect(sel.getHandle(), sqlchar("SELECT col FROM t_num_bool_neg"), SQL_NTS);
    REQUIRE_ODBC(ret, sel);
    ret = SQLFetch(sel.getHandle());
    REQUIRE_ODBC(ret, sel);
    CHECK(get_data<SQL_C_BIT>(sel, 1) == 1);
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_DEFAULT to SQL_BIT",
                 "[c_numeric_types][conversion][sql_boolean]") {
  // Given Snowflake client is logged in

  conn.execute("CREATE TEMPORARY TABLE t_default (col BOOLEAN)");

  SQLCHAR val_true = 1;
  SQLCHAR val_false = 0;
  SQLLEN ind = 0;

  // When SQL_C_DEFAULT 1 is bound to SQL_BIT and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t_default VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DEFAULT, SQL_BIT, 1, 0, &val_true, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // And SQL_C_DEFAULT 0 is bound to SQL_BIT and inserted
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DEFAULT, SQL_BIT, 1, 0, &val_false, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the values are read back as true and false
  auto sel = conn.createStatement();
  ret = SQLExecDirect(sel.getHandle(), sqlchar("SELECT col FROM t_default ORDER BY col DESC"), SQL_NTS);
  REQUIRE_ODBC(ret, sel);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_BIT>(sel, 1) == 1);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data<SQL_C_BIT>(sel, 1) == 0);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_SLONG with NULL indicator to SQL_BIT",
                 "[c_numeric_types][conversion][sql_boolean]") {
  // Given Snowflake client is logged in

  conn.execute("CREATE TEMPORARY TABLE t_long_null (col BOOLEAN)");

  SQLINTEGER val = 0;
  SQLLEN ind = SQL_NULL_DATA;

  // When SQL_C_SLONG is bound with SQL_NULL_DATA and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t_long_null VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_BIT, 1, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the column is NULL when fetched as SQL_C_BIT
  auto sel = conn.createStatement();
  ret = SQLExecDirect(sel.getHandle(), sqlchar("SELECT col FROM t_long_null"), SQL_NTS);
  REQUIRE_ODBC(ret, sel);
  ret = SQLFetch(sel.getHandle());
  REQUIRE_ODBC(ret, sel);
  CHECK(get_data_optional<SQL_C_BIT>(sel, 1) == std::nullopt);
}
