// ODBC C integer types bound to approximate SQL types (FLOAT / REAL / DOUBLE) via SQLBindParameter.

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_floating_point.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_SLONG to SQL_DOUBLE and read back",
                 "[c_integer][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When An integer value is bound as SQL_C_SLONG and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLINTEGER val = 42;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_DOUBLE, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The value should be read back as double
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == 42.0);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_SBIGINT to SQL_DOUBLE and read back",
                 "[c_integer][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When A 64-bit integer is bound as SQL_C_SBIGINT and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLBIGINT val = 1000000;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SBIGINT, SQL_DOUBLE, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The value should be read back as double
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == 1000000.0);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_SSHORT to SQL_REAL and read back",
                 "[c_integer][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col REAL)");

  // When A 16-bit integer is bound as SQL_C_SSHORT and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLSMALLINT val = -100;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SSHORT, SQL_REAL, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The value should be read back as double
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == -100.0);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_UTINYINT to SQL_DOUBLE and read back",
                 "[c_integer][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When An unsigned 8-bit integer is bound as SQL_C_UTINYINT and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLCHAR val = 255;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_UTINYINT, SQL_DOUBLE, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The value should be read back as double
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == 255.0);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_UBIGINT to SQL_DOUBLE and read back",
                 "[c_integer][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When A large unsigned 64-bit integer is bound as SQL_C_UBIGINT and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLUBIGINT val = 18446744073709551615ULL;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_UBIGINT, SQL_DOUBLE, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The value should round to double precision when read back
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  double expected = static_cast<double>(val);
  CHECK_THAT(get_data<SQL_C_DOUBLE>(fetch_stmt, 1), Catch::Matchers::WithinRel(expected, 1e-15));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_STINYINT to SQL_DOUBLE and read back",
                 "[c_integer][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When A signed 8-bit integer at minimum value is bound as SQL_C_STINYINT and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLSCHAR val = -128;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_STINYINT, SQL_DOUBLE, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The value should be read back as double
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == -128.0);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_USHORT to SQL_DOUBLE and read back",
                 "[c_integer][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When An unsigned 16-bit integer at maximum value is bound as SQL_C_USHORT and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLUSMALLINT val = 65535;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_USHORT, SQL_DOUBLE, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The value should be read back as double
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == 65535.0);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_SLONG zero to SQL_DOUBLE", "[c_integer][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When Zero is bound as SQL_C_SLONG and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLINTEGER val = 0;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_DOUBLE, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The value should be read back as double zero
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == 0.0);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_SLONG with NULL indicator to SQL_DOUBLE",
                 "[c_integer][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When SQL_NULL_DATA is used as the length/indicator for the bound parameter
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLINTEGER val = 0;
  SQLLEN ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_DOUBLE, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The column should be NULL when read back
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK_FALSE(get_data_optional<SQL_C_DOUBLE>(fetch_stmt, 1).has_value());
}
