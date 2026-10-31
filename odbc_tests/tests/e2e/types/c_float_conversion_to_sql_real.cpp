// ODBC E2E: SQL_C_FLOAT / SQL_C_DOUBLE bound via SQLBindParameter to SQL real types (SQL_REAL, SQL_FLOAT, SQL_DOUBLE)

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cfloat>
#include <cmath>
#include <cstring>
#include <optional>

#include <catch2/catch_approx.hpp>
#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_DOUBLE to SQL_DOUBLE and read back",
                 "[e2e][types][c_float][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When A double value is bound with SQL_C_DOUBLE and SQL_DOUBLE and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLDOUBLE val = 3.14;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_DOUBLE, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The value should be read back correctly as SQL_C_DOUBLE
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == Catch::Approx(3.14));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_FLOAT to SQL_REAL and read back",
                 "[e2e][types][c_float][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When A float value is bound with SQL_C_FLOAT and SQL_REAL and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLREAL val = 1.5f;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_FLOAT, SQL_REAL, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The value should be read back as SQL_C_DOUBLE
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == Catch::Approx(1.5).epsilon(1e-6));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_DOUBLE negative zero",
                 "[e2e][types][c_float][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When Negative zero is bound as SQL_C_DOUBLE and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLDOUBLE val = -0.0;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_DOUBLE, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The fetched value should be floating-point zero
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  double read = get_data<SQL_C_DOUBLE>(fetch_stmt, 1);
  CHECK(read == Catch::Approx(0.0));
  CHECK(std::fpclassify(read) == FP_ZERO);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_DOUBLE large value",
                 "[e2e][types][c_float][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When A large double near DBL_MAX is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLDOUBLE val = 1.7e308;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_DOUBLE, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The value should round-trip within floating-point precision
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == Catch::Approx(1.7e308).epsilon(1e-12));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_DOUBLE small value",
                 "[e2e][types][c_float][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When A very small positive double is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLDOUBLE val = 2.2e-308;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_DOUBLE, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The value should round-trip within floating-point precision
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  OLD_DRIVER_ONLY("BD#36") { CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == 0.0); }
  NEW_DRIVER_ONLY("BD#36") { CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == Catch::Approx(2.2e-308).epsilon(1e-12)); }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_FLOAT max value",
                 "[e2e][types][c_float][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When FLT_MAX is bound with SQL_C_FLOAT and SQL_REAL and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLREAL val = FLT_MAX;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_FLOAT, SQL_REAL, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The value should read back matching FLT_MAX
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == Catch::Approx(static_cast<double>(FLT_MAX)).epsilon(1e-6));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_DOUBLE with NULL indicator",
                 "[e2e][types][c_float][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When SQL_NULL_DATA is used for the SQL_C_DOUBLE parameter
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLLEN ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_DOUBLE, 0, 0, nullptr, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The column value should be NULL
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data_optional<SQL_C_DOUBLE>(fetch_stmt, 1) == std::nullopt);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_DEFAULT to SQL_DOUBLE and read back",
                 "[e2e][types][c_float][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When A double value is bound with SQL_C_DEFAULT and SQL_DOUBLE and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLDOUBLE val = 2.718;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DEFAULT, SQL_DOUBLE, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The value should be read back correctly as SQL_C_DOUBLE
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == Catch::Approx(2.718));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_DEFAULT to SQL_REAL as SQL_C_FLOAT",
                 "[e2e][types][c_float][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When A 4-byte float followed by non-zero bytes is bound with SQL_C_DEFAULT and SQL_REAL and inserted
  struct {
    SQLREAL value;
    unsigned char trailing[4];
  } buf;
  std::memset(&buf, 0xAA, sizeof(buf));
  buf.value = 1.25f;
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DEFAULT, SQL_REAL, 0, 0, &buf.value, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then Only the 4-byte float is read, so the value round-trips exactly
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == 1.25);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_FLOAT parameter array to SQL_REAL with zero BufferLength",
                 "[e2e][types][c_float][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When A column-wise array of 4-byte floats is bound with SQL_C_FLOAT, SQL_REAL and BufferLength 0
  constexpr SQLULEN num_rows = 3;
  SQLREAL values[num_rows] = {1.25f, 2.5f, -3.75f};
  SQLLEN indicators[num_rows] = {0, 0, 0};
  SQLULEN params_processed = 0;
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_BIND_TYPE, SQL_PARAM_BIND_BY_COLUMN, 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, reinterpret_cast<SQLPOINTER>(num_rows), 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMS_PROCESSED_PTR, &params_processed, 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_FLOAT, SQL_REAL, 0, 0, values, 0, indicators);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(params_processed == num_rows);

  // Then Each row is inserted
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t ORDER BY col");
  CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == -3.75);
  REQUIRE_ODBC(SQLFetch(fetch_stmt.getHandle()), fetch_stmt);
  CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == 1.25);
  REQUIRE_ODBC(SQLFetch(fetch_stmt.getHandle()), fetch_stmt);
  CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == 2.5);
  REQUIRE_ODBC_NO_DATA(SQLFetch(fetch_stmt.getHandle()), fetch_stmt);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_DEFAULT parameter array to SQL_REAL with zero BufferLength",
                 "[e2e][types][c_float][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When A column-wise array of 4-byte floats is bound with SQL_C_DEFAULT, SQL_REAL and BufferLength 0
  constexpr SQLULEN num_rows = 3;
  SQLREAL values[num_rows] = {1.25f, 2.5f, -3.75f};
  SQLLEN indicators[num_rows] = {0, 0, 0};
  SQLULEN params_processed = 0;
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_BIND_TYPE, SQL_PARAM_BIND_BY_COLUMN, 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, reinterpret_cast<SQLPOINTER>(num_rows), 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMS_PROCESSED_PTR, &params_processed, 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DEFAULT, SQL_REAL, 0, 0, values, 0, indicators);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(params_processed == num_rows);

  // Then The new driver inserts each row and the old driver inserts the first element for every row
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t ORDER BY col");
  NEW_DRIVER_ONLY("BD#sql-c-default-real-parameter-array") {
    CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == -3.75);
    REQUIRE_ODBC(SQLFetch(fetch_stmt.getHandle()), fetch_stmt);
    CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == 1.25);
    REQUIRE_ODBC(SQLFetch(fetch_stmt.getHandle()), fetch_stmt);
    CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == 2.5);
  }
  OLD_DRIVER_ONLY("BD#sql-c-default-real-parameter-array") {
    CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == 1.25);
    REQUIRE_ODBC(SQLFetch(fetch_stmt.getHandle()), fetch_stmt);
    CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == 1.25);
    REQUIRE_ODBC(SQLFetch(fetch_stmt.getHandle()), fetch_stmt);
    CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == 1.25);
  }
  REQUIRE_ODBC_NO_DATA(SQLFetch(fetch_stmt.getHandle()), fetch_stmt);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_DOUBLE zero", "[e2e][types][c_float][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When Zero is bound as SQL_C_DOUBLE and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLDOUBLE val = 0.0;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_DOUBLE, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The fetched value should be zero
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == Catch::Approx(0.0));
}
