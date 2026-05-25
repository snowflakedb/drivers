// ODBC E2E: SQL_C_INTERVAL_YEAR / SQL_C_INTERVAL_MONTH /
// SQL_C_INTERVAL_YEAR_TO_MONTH bound via SQLBindParameter to SQL_VARCHAR.
//
// Snowflake has no native INTERVAL column type, so per ODBC Appendix D
// ("Converting Data from C to SQL Data Types") all SQL_C_INTERVAL_*
// parameters are routed to a VARCHAR target and formatted as the ANSI
// SQL interval literal text. These tests exercise the round-trip:
// SQLPrepare → SQLBindParameter → SQLExecute → SELECT → SQLGetData.
//
// Format reference (ODBC Appendix D, "C to SQL: Interval"):
//   YEAR              : [-]<year>
//   MONTH             : [-]<month>
//   YEAR_TO_MONTH     : [-]<year>-<month>
//
// The driver writes the chosen sub-fields based on the C type bound on
// the parameter — the struct's `interval_type` field is intentionally
// ignored (Appendix D requires conformance to the bound C type).

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

namespace {

SQL_INTERVAL_STRUCT ym_interval(SQLSMALLINT sign, SQLUINTEGER year, SQLUINTEGER month) {
  SQL_INTERVAL_STRUCT iv = {};
  iv.interval_sign = sign;
  iv.intval.year_month.year = year;
  iv.intval.year_month.month = month;
  return iv;
}

void bind_interval_and_execute(StatementHandleWrapper& stmt, SQLSMALLINT c_type, SQL_INTERVAL_STRUCT& val,
                               SQLLEN& ind) {
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, c_type, SQL_VARCHAR, 200, 0, &val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
}

}  // namespace

// ============================================================================
// SQL_C_INTERVAL_YEAR
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_YEAR to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_YEAR carrying 5 years is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ym_interval(SQL_FALSE, 5, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_YEAR, val, ind);

  // Then the formatted literal is stored
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "5");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind negative SQL_C_INTERVAL_YEAR to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_YEAR carrying -7 years is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ym_interval(SQL_TRUE, 7, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_YEAR, val, ind);

  // Then the leading "-" sign is preserved
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "-7");
}

// ============================================================================
// SQL_C_INTERVAL_MONTH
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_MONTH to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_MONTH carrying 11 months is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ym_interval(SQL_FALSE, 0, 11);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_MONTH, val, ind);

  // Then the formatted literal is stored without zero-padding for the leading field
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "11");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind negative SQL_C_INTERVAL_MONTH to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_MONTH carrying -8 months is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ym_interval(SQL_TRUE, 0, 8);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_MONTH, val, ind);

  // Then the leading "-" sign is preserved on the leading field
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "-8");
}

// ============================================================================
// SQL_C_INTERVAL_YEAR_TO_MONTH
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_YEAR_TO_MONTH to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_YEAR_TO_MONTH carrying 5 years 11 months is bound
  // and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ym_interval(SQL_FALSE, 5, 11);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_YEAR_TO_MONTH, val, ind);

  // Then the "<year>-<month(2)>" form is stored
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "5-11");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_YEAR_TO_MONTH with single-digit month to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_YEAR_TO_MONTH carrying 4 years 7 months is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ym_interval(SQL_FALSE, 4, 7);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_YEAR_TO_MONTH, val, ind);

  // Then the trailing month sub-field is zero-padded to 2 digits per ODBC "Interval Data Type Length"
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "4-07");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind negative SQL_C_INTERVAL_YEAR_TO_MONTH to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_YEAR_TO_MONTH carrying -2 years 3 months is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ym_interval(SQL_TRUE, 2, 3);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_YEAR_TO_MONTH, val, ind);

  // Then the leading sign is applied once before the year and the trailing month is zero-padded to 2 digits
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "-2-03");
}

// ============================================================================
// Alternate character SQL targets — the spec lists CHAR / VARCHAR / LONGVARCHAR
// (and their wide-character twins) as legal targets for SQL_C_INTERVAL_*.
// SnowflakeVarchar handles all of them identically; one representative
// interval per target is enough to exercise the routing.
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_YEAR_TO_MONTH to SQL_CHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a CHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col CHAR(20))");

  // When SQL_C_INTERVAL_YEAR_TO_MONTH carrying 4 years 7 months is bound to SQL_CHAR
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ym_interval(SQL_FALSE, 4, 7);
  SQLLEN ind = sizeof(val);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_INTERVAL_YEAR_TO_MONTH, SQL_CHAR, 20, 0, &val,
                         sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE_ODBC(SQLExecute(stmt.getHandle()), stmt);

  // Then the canonical "<year>-<month(2)>" form is stored
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "4-07");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_YEAR_TO_MONTH to SQL_WCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a CHAR column (Snowflake routes wide character targets through TEXT)
  conn.execute("CREATE TEMPORARY TABLE t (col CHAR(20))");

  // When SQL_C_INTERVAL_YEAR_TO_MONTH carrying 4 years 7 months is bound to SQL_WCHAR
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ym_interval(SQL_FALSE, 4, 7);
  SQLLEN ind = sizeof(val);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_INTERVAL_YEAR_TO_MONTH, SQL_WCHAR, 20, 0, &val,
                         sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE_ODBC(SQLExecute(stmt.getHandle()), stmt);

  // Then the canonical "<year>-<month(2)>" form is stored
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "4-07");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_YEAR_TO_MONTH to SQL_LONGVARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column (Snowflake routes LONGVARCHAR through TEXT)
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_YEAR_TO_MONTH carrying 4 years 7 months is bound to SQL_LONGVARCHAR
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ym_interval(SQL_FALSE, 4, 7);
  SQLLEN ind = sizeof(val);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_INTERVAL_YEAR_TO_MONTH, SQL_LONGVARCHAR, 200, 0,
                         &val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE_ODBC(SQLExecute(stmt.getHandle()), stmt);

  // Then the canonical "<year>-<month(2)>" form is stored
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "4-07");
}

// ============================================================================
// NULL indicator
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_YEAR with NULL indicator to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_YEAR is bound with SQL_NULL_DATA and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLLEN ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_INTERVAL_YEAR, SQL_VARCHAR, 200, 0, nullptr, 0,
                         &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the stored value should be NULL
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data_optional<SQL_C_CHAR>(fetch_stmt, 1) == std::nullopt);
}
