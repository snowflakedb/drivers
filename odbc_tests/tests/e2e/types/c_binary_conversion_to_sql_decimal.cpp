// ODBC E2E: SQL_C_BINARY bound via SQLBindParameter to SQL_DECIMAL / SQL_NUMERIC
// The binary buffer is interpreted as a raw SQL_NUMERIC_STRUCT (19 bytes).

#include <cstddef>
#include <cstdint>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

static SQL_NUMERIC_STRUCT make_numeric(SQLCHAR precision, SQLSCHAR scale, SQLCHAR sign, uint64_t magnitude) {
  SQL_NUMERIC_STRUCT ns = {};
  ns.precision = precision;
  ns.scale = scale;
  ns.sign = sign;
  // SQL_NUMERIC_STRUCT::val is defined by the ODBC spec as a little-endian byte
  // array regardless of host endianness. Populate it explicitly so the test
  // remains correct on big-endian targets (where memcpy of a uint64_t would
  // produce big-endian bytes).
  for (size_t i = 0; i < sizeof(magnitude); ++i) {
    ns.val[i] = static_cast<SQLCHAR>((magnitude >> (i * 8)) & 0xFF);
  }
  return ns;
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_BINARY numeric struct to SQL_DECIMAL and read back",
                 "[c_binary][conversion][sql_decimal]") {
  SKIP_OLD_DRIVER("BD#46", "Old driver does not support SQL_C_BINARY as source for SQL_DECIMAL");
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col NUMBER(10,2))");

  // When A 19-byte binary buffer containing a SQL_NUMERIC_STRUCT is bound as SQL_DECIMAL
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_NUMERIC_STRUCT ns = make_numeric(10, 2, 1, 12345);
  SQLLEN ind = sizeof(ns);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_DECIMAL, 10, 2, &ns, sizeof(ns), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The value 123.45 should be read back correctly
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "123.45");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_BINARY numeric struct integer to SQL_NUMERIC",
                 "[c_binary][conversion][sql_decimal]") {
  SKIP_OLD_DRIVER("BD#46", "Old driver does not support SQL_C_BINARY as source for SQL_NUMERIC");
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col NUMBER(10,0))");

  // When A SQL_NUMERIC_STRUCT with scale=0 is bound as SQL_NUMERIC
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_NUMERIC_STRUCT ns = make_numeric(10, 0, 1, 42);
  SQLLEN ind = sizeof(ns);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_NUMERIC, 10, 0, &ns, sizeof(ns), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The integer value should be read back correctly
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_SBIGINT>(fetch_stmt, 1) == 42);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_BINARY with wrong size for SQL_DECIMAL",
                 "[c_binary][conversion][sql_decimal]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col NUMBER(10,2))");

  // When A 10-byte buffer (not 19) is bound as SQL_DECIMAL
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  unsigned char buf[10] = {};
  SQLLEN ind = sizeof(buf);
  ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_DECIMAL, 10, 2, buf, sizeof(buf), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then The execution should fail with SQLSTATE 22003
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22003"));
}
