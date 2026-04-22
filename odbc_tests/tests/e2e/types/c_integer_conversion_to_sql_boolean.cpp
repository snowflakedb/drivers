#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_SLONG one to SQL_BIT via integer",
                 "[c_integer][conversion][sql_boolean]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col BOOLEAN)");

  // When SQL_C_SLONG 1 is bound to SQL_BIT and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLINTEGER val = 1;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_BIT, 1, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as SQL_C_BIT 1
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_BIT>(fetch_stmt, 1) == 1);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_SLONG zero to SQL_BIT via integer",
                 "[c_integer][conversion][sql_boolean]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col BOOLEAN)");

  // When SQL_C_SLONG 0 is bound to SQL_BIT and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLINTEGER val = 0;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_BIT, 1, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as SQL_C_BIT 0
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_BIT>(fetch_stmt, 1) == 0);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_SLONG nonzero >1 to SQL_BIT via integer",
                 "[c_integer][conversion][sql_boolean]") {
  SKIP_OLD_DRIVER("BD-35", "Old driver rejects integer values other than 0/1 for SQL_BIT");
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col BOOLEAN)");

  // When SQL_C_SLONG 42 is bound to SQL_BIT and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLINTEGER val = 42;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_BIT, 1, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as SQL_C_BIT 1
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_BIT>(fetch_stmt, 1) == 1);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_SLONG negative to SQL_BIT via integer",
                 "[c_integer][conversion][sql_boolean]") {
  SKIP_OLD_DRIVER("BD-35", "Old driver rejects negative integers for SQL_BIT");
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col BOOLEAN)");

  // When SQL_C_SLONG -99 is bound to SQL_BIT and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLINTEGER val = -99;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_BIT, 1, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as SQL_C_BIT 1
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_BIT>(fetch_stmt, 1) == 1);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_SBIGINT to SQL_BIT", "[c_integer][conversion][sql_boolean]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col BOOLEAN)");

  // When SQL_C_SBIGINT 1 is bound to SQL_BIT and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLBIGINT val = 1;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SBIGINT, SQL_BIT, 1, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as SQL_C_BIT 1
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_BIT>(fetch_stmt, 1) == 1);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_SSHORT to SQL_BIT", "[c_integer][conversion][sql_boolean]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col BOOLEAN)");

  // When SQL_C_SSHORT 1 is bound to SQL_BIT and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLSMALLINT val = 1;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SSHORT, SQL_BIT, 1, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as SQL_C_BIT 1
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_BIT>(fetch_stmt, 1) == 1);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_UTINYINT to SQL_BIT", "[c_integer][conversion][sql_boolean]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col BOOLEAN)");

  // When SQL_C_UTINYINT 1 is bound to SQL_BIT and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLCHAR val = 1;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_UTINYINT, SQL_BIT, 1, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as SQL_C_BIT 1
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_BIT>(fetch_stmt, 1) == 1);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_ULONG to SQL_BIT", "[c_integer][conversion][sql_boolean]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col BOOLEAN)");

  // When SQL_C_ULONG 1 is bound to SQL_BIT and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLUINTEGER val = 1;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_ULONG, SQL_BIT, 1, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as SQL_C_BIT 1
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_BIT>(fetch_stmt, 1) == 1);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_STINYINT to SQL_BIT", "[c_integer][conversion][sql_boolean]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col BOOLEAN)");

  // When SQL_C_STINYINT 1 is bound to SQL_BIT and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLSCHAR val = 1;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_STINYINT, SQL_BIT, 1, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as SQL_C_BIT 1
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_BIT>(fetch_stmt, 1) == 1);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_USHORT to SQL_BIT", "[c_integer][conversion][sql_boolean]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col BOOLEAN)");

  // When SQL_C_USHORT 1 is bound to SQL_BIT and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLUSMALLINT val = 1;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_USHORT, SQL_BIT, 1, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as SQL_C_BIT 1
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_BIT>(fetch_stmt, 1) == 1);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_UBIGINT to SQL_BIT", "[c_integer][conversion][sql_boolean]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col BOOLEAN)");

  // When SQL_C_UBIGINT 1 is bound to SQL_BIT and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLUBIGINT val = 1;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_UBIGINT, SQL_BIT, 1, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as SQL_C_BIT 1
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_BIT>(fetch_stmt, 1) == 1);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_SLONG with NULL indicator to SQL_BIT via integer",
                 "[c_integer][conversion][sql_boolean]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col BOOLEAN)");

  // When SQL_C_SLONG is bound with SQL_NULL_DATA and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLINTEGER val = 0;
  SQLLEN ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_BIT, 1, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the column is NULL when fetched
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data_optional<SQL_C_BIT>(fetch_stmt, 1) == std::nullopt);
}
