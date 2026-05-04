// ODBC E2E: SQL_C_TYPE_TIMESTAMP bound via SQLBindParameter to ODBC 3.x
// SQL_TYPE_TIME.
//
// Per ODBC Appendix D ("Converting Data from C to SQL Data Types"), binding
// a TIMESTAMP source to a TIME target silently discards the date fields and
// preserves the whole-second h/m/s, but only when the discarded fractional-
// seconds portion is exactly zero — otherwise the driver must return
// SQL_ERROR with SQLSTATE 22008 ("Datetime field overflow"). These tests
// cover both the happy path (zero fraction) and the spec-mandated overflow
// case.

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

namespace {

void bind_timestamp_and_execute(StatementHandleWrapper& stmt, SQL_TIMESTAMP_STRUCT& val, SQLLEN& ind) {
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_TYPE_TIME, 0, 0,
                                   &val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
}

// Like bind_timestamp_and_execute but returns the SQLExecute return code so
// the caller can assert on the diagnostic SQLSTATE instead of REQUIRE'ing
// success.
SQLRETURN bind_timestamp_and_try_execute(StatementHandleWrapper& stmt, SQL_TIMESTAMP_STRUCT& val, SQLLEN& ind) {
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_TYPE_TIME, 0, 0,
                                   &val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  return SQLExecute(stmt.getHandle());
}

}  // namespace

// ============================================================================
// SQL_C_TYPE_TIMESTAMP → TIME — happy paths
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_TIMESTAMP to SQL_TYPE_TIME and discard date component",
                 "[c_timestamp][conversion][sql_time]") {
  // Given a TIME column
  conn.execute("CREATE TEMPORARY TABLE t (col TIME)");

  // When SQL_C_TYPE_TIMESTAMP 2026-04-13 14:30:45 is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIMESTAMP_STRUCT val = {2026, 4, 13, 14, 30, 45, 0};
  SQLLEN ind = sizeof(val);
  bind_timestamp_and_execute(stmt, val, ind);

  // Then only the time is preserved and the date is silently discarded
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  SQL_TIME_STRUCT result = get_data<SQL_C_TYPE_TIME>(fetch_stmt, 1);
  CHECK(result.hour == 14);
  CHECK(result.minute == 30);
  CHECK(result.second == 45);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_TYPE_TIMESTAMP with non-zero fraction bound to SQL_TYPE_TIME",
                 "[c_timestamp][conversion][sql_time]") {
  // Given a TIME column
  conn.execute("CREATE TEMPORARY TABLE t (col TIME)");

  // When the timestamp carries a half-second fractional component
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIMESTAMP_STRUCT val = {2026, 4, 13, 14, 30, 45, 500000000};
  SQLLEN ind = sizeof(val);
  ret = bind_timestamp_and_try_execute(stmt, val, ind);

  // Then SQLExecute fails with SQLSTATE 22008
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22008"));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind midnight SQL_C_TYPE_TIMESTAMP to SQL_TYPE_TIME",
                 "[c_timestamp][conversion][sql_time]") {
  // Given a TIME column
  conn.execute("CREATE TEMPORARY TABLE t (col TIME)");

  // When the timestamp's time portion is 00:00:00
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIMESTAMP_STRUCT val = {2026, 4, 13, 0, 0, 0, 0};
  SQLLEN ind = sizeof(val);
  bind_timestamp_and_execute(stmt, val, ind);

  // Then the stored value is the zero time
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  SQL_TIME_STRUCT result = get_data<SQL_C_TYPE_TIME>(fetch_stmt, 1);
  CHECK(result.hour == 0);
  CHECK(result.minute == 0);
  CHECK(result.second == 0);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind end-of-day SQL_C_TYPE_TIMESTAMP to SQL_TYPE_TIME",
                 "[c_timestamp][conversion][sql_time]") {
  // Given a TIME column
  conn.execute("CREATE TEMPORARY TABLE t (col TIME)");

  // When 23:59:59 on any date is bound
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIMESTAMP_STRUCT val = {2026, 4, 13, 23, 59, 59, 0};
  SQLLEN ind = sizeof(val);
  bind_timestamp_and_execute(stmt, val, ind);

  // Then the upper-bound time is preserved
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  SQL_TIME_STRUCT result = get_data<SQL_C_TYPE_TIME>(fetch_stmt, 1);
  CHECK(result.hour == 23);
  CHECK(result.minute == 59);
  CHECK(result.second == 59);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_TIMESTAMP at epoch date to SQL_TYPE_TIME",
                 "[c_timestamp][conversion][sql_time]") {
  // Given a TIME column
  conn.execute("CREATE TEMPORARY TABLE t (col TIME)");

  // When the timestamp date is the Unix epoch but the time is non-zero
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIMESTAMP_STRUCT val = {1970, 1, 1, 12, 0, 0, 0};
  SQLLEN ind = sizeof(val);
  bind_timestamp_and_execute(stmt, val, ind);

  // Then only the time matters and the epoch date is irrelevant
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  SQL_TIME_STRUCT result = get_data<SQL_C_TYPE_TIME>(fetch_stmt, 1);
  CHECK(result.hour == 12);
  CHECK(result.minute == 0);
  CHECK(result.second == 0);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_TIMESTAMP with NULL indicator to SQL_TYPE_TIME",
                 "[c_timestamp][conversion][sql_time]") {
  // Given a TIME column
  conn.execute("CREATE TEMPORARY TABLE t (col TIME)");

  // When SQL_C_TYPE_TIMESTAMP is bound with SQL_NULL_DATA and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLLEN ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_TYPE_TIME, 0, 0, nullptr, 0,
                         &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the stored value should be NULL
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data_optional<SQL_C_TYPE_TIME>(fetch_stmt, 1) == std::nullopt);
}

// ============================================================================
// Invalid-struct-field rejection — SQLSTATE 22007 (Invalid datetime format)
//
// Per ODBC Appendix D ("C to SQL: Timestamp"), a SQL_C_TYPE_TIMESTAMP
// struct whose fields are outside their legal range must surface
// SQLSTATE 22007. 22007 takes precedence over the narrowing 22008
// diagnostic — the *_invalid_*_takes_precedence_over_22008 unit tests
// in odbc/src/conversion/param_binding.rs pin this for the conversion
// layer; these e2e tests pin the same contract end-to-end.
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_TYPE_TIMESTAMP with hour=24 bound to SQL_TYPE_TIME",
                 "[c_timestamp][conversion][sql_time][invalid]") {
  // Given a TIME column
  conn.execute("CREATE TEMPORARY TABLE t (col TIME)");

  // When the timestamp carries hour=24 which is out of the legal range
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIMESTAMP_STRUCT val = {2026, 4, 13, 24, 0, 0, 0};
  SQLLEN ind = sizeof(val);
  ret = bind_timestamp_and_try_execute(stmt, val, ind);

  // Then SQLExecute fails with SQLSTATE 22007
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22007"));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_TYPE_TIMESTAMP with minute=60 bound to SQL_TYPE_TIME",
                 "[c_timestamp][conversion][sql_time][invalid]") {
  // Given a TIME column
  conn.execute("CREATE TEMPORARY TABLE t (col TIME)");

  // When the timestamp carries minute=60 which is out of the legal range
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIMESTAMP_STRUCT val = {2026, 4, 13, 12, 60, 0, 0};
  SQLLEN ind = sizeof(val);
  ret = bind_timestamp_and_try_execute(stmt, val, ind);

  // Then SQLExecute fails with SQLSTATE 22007
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22007"));
}

TEST_CASE_METHOD(ConnSchemaFixture,
                 "should reject SQL_C_TYPE_TIMESTAMP with out-of-range fraction bound to SQL_TYPE_TIME",
                 "[c_timestamp][conversion][sql_time][invalid]") {
  // Given a TIME column
  conn.execute("CREATE TEMPORARY TABLE t (col TIME)");

  // When the timestamp carries fraction=3000000000 ns which is out of the legal range
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIMESTAMP_STRUCT val = {2026, 4, 13, 12, 0, 0, 3000000000U};
  SQLLEN ind = sizeof(val);
  ret = bind_timestamp_and_try_execute(stmt, val, ind);

  // Then SQLExecute fails with SQLSTATE 22007
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22007"));
}

TEST_CASE_METHOD(
    ConnSchemaFixture,
    "should prefer SQLSTATE 22007 over 22008 when SQL_C_TYPE_TIMESTAMP has invalid hour and non-zero fraction",
    "[c_timestamp][conversion][sql_time][invalid]") {
  // Given a TIME column
  conn.execute("CREATE TEMPORARY TABLE t (col TIME)");

  // When the timestamp has both an invalid hour and a non-zero fraction
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIMESTAMP_STRUCT val = {2026, 4, 13, 25, 0, 0, 500000000};
  SQLLEN ind = sizeof(val);
  ret = bind_timestamp_and_try_execute(stmt, val, ind);

  // Then SQLExecute fails with SQLSTATE 22007 not 22008
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22007"));
}
