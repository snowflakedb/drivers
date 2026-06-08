// ODBC E2E: exact-numeric C types (the signed/unsigned SQL_C_STINYINT ..
// SQL_C_UBIGINT codes plus the un-suffixed ODBC 2.x aliases SQL_C_TINYINT /
// SQL_C_SHORT / SQL_C_LONG) bound via SQLBindParameter to the single-field
// SQL_INTERVAL_* parameter types.
//
// Per ODBC Appendix D ("C to SQL: Numeric"), the exact numeric C data types may
// be converted to the single-field interval SQL types (YEAR, MONTH, DAY, HOUR,
// MINUTE, SECOND). The numeric value is interpreted as the count of that single
// leading field. (Approximate numerics — SQL_C_FLOAT / SQL_C_DOUBLE — and the
// multi-field/compound interval targets are NOT permitted; those are covered by
// the c_*_incompatible_to_sql_interval suites.)
//
// The conversion under test is keyed by the bound SQL_INTERVAL_* parameter
// type, not the column type, so the parameter is bound as SQL_INTERVAL_* and
// inserted into a VARCHAR column. (Snowflake does have native INTERVAL columns
// as of 2026, but a VARCHAR target is sufficient to exercise the C->SQL
// parameter conversion.) Because the column is VARCHAR, the driver's rendered
// interval literal is stored verbatim, so these tests assert the exact
// round-tripped text; the driver-side formatting itself is additionally pinned
// by the Rust unit tests in interval.rs.

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_SLONG to SQL_INTERVAL_YEAR",
                 "[c_integer][conversion][sql_interval]") {
  SKIP_OLD_DRIVER("BD#72",
                  "Reference driver does not implement ODBC Appendix D numeric C -> single-field SQL_INTERVAL bind "
                  "semantics");

  // Given a VARCHAR column (sufficient to exercise the bound interval parameter type)
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When a 32-bit integer year count is bound as SQL_INTERVAL_YEAR and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLINTEGER val = 5;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTERVAL_YEAR, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then the year count is stored as the interval literal "5"
  REQUIRE_ODBC(ret, stmt);
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "5");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind negative SQL_C_SLONG to SQL_INTERVAL_YEAR",
                 "[c_integer][conversion][sql_interval]") {
  SKIP_OLD_DRIVER("BD#72",
                  "Reference driver does not implement ODBC Appendix D numeric C -> single-field SQL_INTERVAL bind "
                  "semantics");

  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When a negative year count is bound as SQL_INTERVAL_YEAR and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLINTEGER val = -3;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTERVAL_YEAR, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then the negative year count is stored as the interval literal "-3"
  REQUIRE_ODBC(ret, stmt);
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "-3");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_SBIGINT to SQL_INTERVAL_MONTH",
                 "[c_integer][conversion][sql_interval]") {
  SKIP_OLD_DRIVER("BD#72",
                  "Reference driver does not implement ODBC Appendix D numeric C -> single-field SQL_INTERVAL bind "
                  "semantics");

  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When a 64-bit integer month count is bound as SQL_INTERVAL_MONTH and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLBIGINT val = 11;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SBIGINT, SQL_INTERVAL_MONTH, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then the month count is stored as the interval literal "11"
  REQUIRE_ODBC(ret, stmt);
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "11");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_SSHORT to SQL_INTERVAL_DAY",
                 "[c_integer][conversion][sql_interval]") {
  SKIP_OLD_DRIVER("BD#72",
                  "Reference driver does not implement ODBC Appendix D numeric C -> single-field SQL_INTERVAL bind "
                  "semantics");

  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When a 16-bit integer day count is bound as SQL_INTERVAL_DAY and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLSMALLINT val = 7;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SSHORT, SQL_INTERVAL_DAY, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then the day count is stored as the interval literal "7"
  REQUIRE_ODBC(ret, stmt);
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "7");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_UTINYINT to SQL_INTERVAL_HOUR",
                 "[c_integer][conversion][sql_interval]") {
  SKIP_OLD_DRIVER("BD#72",
                  "Reference driver does not implement ODBC Appendix D numeric C -> single-field SQL_INTERVAL bind "
                  "semantics");

  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When an unsigned 8-bit integer hour count is bound as SQL_INTERVAL_HOUR and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLCHAR val = 12;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_UTINYINT, SQL_INTERVAL_HOUR, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then the hour count is stored as the interval literal "12"
  REQUIRE_ODBC(ret, stmt);
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "12");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_ULONG to SQL_INTERVAL_MINUTE",
                 "[c_integer][conversion][sql_interval]") {
  SKIP_OLD_DRIVER("BD#72",
                  "Reference driver does not implement ODBC Appendix D numeric C -> single-field SQL_INTERVAL bind "
                  "semantics");

  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When an unsigned 32-bit integer minute count is bound as SQL_INTERVAL_MINUTE and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLUINTEGER val = 30;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_ULONG, SQL_INTERVAL_MINUTE, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then the minute count is stored as the interval literal "30"
  REQUIRE_ODBC(ret, stmt);
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "30");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_STINYINT to SQL_INTERVAL_SECOND",
                 "[c_integer][conversion][sql_interval]") {
  SKIP_OLD_DRIVER("BD#72",
                  "Reference driver does not implement ODBC Appendix D numeric C -> single-field SQL_INTERVAL bind "
                  "semantics");

  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When a signed 8-bit integer second count is bound as SQL_INTERVAL_SECOND and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLSCHAR val = 45;
  SQLLEN ind = 0;
  ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_STINYINT, SQL_INTERVAL_SECOND, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then the second count is stored as the canonical interval literal "45.000000"
  REQUIRE_ODBC(ret, stmt);
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "45.000000");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_SLONG with NULL indicator to SQL_INTERVAL_YEAR",
                 "[c_integer][conversion][sql_interval]") {
  SKIP_OLD_DRIVER("BD#72",
                  "Reference driver does not implement ODBC Appendix D numeric C -> single-field SQL_INTERVAL bind "
                  "semantics");

  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When a NULL parameter is bound as SQL_INTERVAL_YEAR using SQL_NULL_DATA
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLINTEGER val = 0;
  SQLLEN ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTERVAL_YEAR, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the stored value should be NULL
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK_FALSE(get_data_optional<SQL_C_CHAR>(fetch_stmt, 1).has_value());
}

// ---------------------------------------------------------------------------
// ODBC 2.x un-suffixed C type aliases. SQL_C_TINYINT / SQL_C_SHORT / SQL_C_LONG
// share the exact-numeric conversion arms with their signed counterparts
// (CDataType::TinyInt|STinyInt, Short|SShort, Long|SLong in interval.rs), so an
// alias bind must round-trip identically. Pinned end-to-end so the alias->arm
// mapping is verified through the DM, not just for the suffixed codes.
// ---------------------------------------------------------------------------

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TINYINT (unsuffixed alias) to SQL_INTERVAL_HOUR",
                 "[c_integer][conversion][sql_interval]") {
  SKIP_OLD_DRIVER("BD#72",
                  "Reference driver does not implement ODBC Appendix D numeric C -> single-field SQL_INTERVAL bind "
                  "semantics");

  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When a tinyint hour count is bound via the un-suffixed SQL_C_TINYINT alias and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLSCHAR val = 9;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TINYINT, SQL_INTERVAL_HOUR, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then the hour count is stored as the interval literal "9"
  REQUIRE_ODBC(ret, stmt);
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "9");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_SHORT (unsuffixed alias) to SQL_INTERVAL_MONTH",
                 "[c_integer][conversion][sql_interval]") {
  SKIP_OLD_DRIVER("BD#72",
                  "Reference driver does not implement ODBC Appendix D numeric C -> single-field SQL_INTERVAL bind "
                  "semantics");

  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When a short month count is bound via the un-suffixed SQL_C_SHORT alias and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLSMALLINT val = 6;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SHORT, SQL_INTERVAL_MONTH, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then the month count is stored as the interval literal "6"
  REQUIRE_ODBC(ret, stmt);
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "6");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_LONG (unsuffixed alias) to SQL_INTERVAL_DAY",
                 "[c_integer][conversion][sql_interval]") {
  SKIP_OLD_DRIVER("BD#72",
                  "Reference driver does not implement ODBC Appendix D numeric C -> single-field SQL_INTERVAL bind "
                  "semantics");

  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When a 32-bit day count is bound via the un-suffixed SQL_C_LONG alias and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLINTEGER val = 21;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_LONG, SQL_INTERVAL_DAY, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then the day count is stored as the interval literal "21"
  REQUIRE_ODBC(ret, stmt);
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "21");
}
