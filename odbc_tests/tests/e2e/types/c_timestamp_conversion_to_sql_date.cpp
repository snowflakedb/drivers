// ODBC E2E: SQL_C_TYPE_TIMESTAMP bound via SQLBindParameter to ODBC 3.x
// SQL_TYPE_DATE.
//
// Per ODBC Appendix D ("Converting Data from C to SQL Data Types"), binding
// a TIMESTAMP source to a DATE target succeeds only when the discarded time
// portion (hour, minute, second, fraction) is exactly zero — otherwise the
// driver must return SQL_ERROR with SQLSTATE 22008 ("Datetime field
// overflow"). These tests cover both the happy path (zero time) and the
// spec-mandated overflow cases.

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

namespace {

void bind_timestamp_and_execute(StatementHandleWrapper& stmt, SQL_TIMESTAMP_STRUCT& val, SQLLEN& ind) {
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_TYPE_DATE, 0, 0,
                                   &val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
}

// Like bind_timestamp_and_execute but returns the SQLExecute return code so
// the caller can assert on the diagnostic SQLSTATE instead of REQUIRE'ing
// success.
SQLRETURN bind_timestamp_and_try_execute(StatementHandleWrapper& stmt, SQL_TIMESTAMP_STRUCT& val, SQLLEN& ind) {
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_TYPE_DATE, 0, 0,
                                   &val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  return SQLExecute(stmt.getHandle());
}

}  // namespace

// ============================================================================
// SQL_C_TYPE_TIMESTAMP → DATE — happy paths (time portion = 00:00:00.0)
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind midnight SQL_C_TYPE_TIMESTAMP to SQL_TYPE_DATE without info loss",
                 "[c_timestamp][conversion][sql_date]") {
  // Given a DATE column
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

  // When SQL_C_TYPE_TIMESTAMP at exactly midnight is bound to a DATE target
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIMESTAMP_STRUCT val = {2026, 4, 13, 0, 0, 0, 0};
  SQLLEN ind = sizeof(val);
  bind_timestamp_and_execute(stmt, val, ind);

  // Then the date round-trips exactly
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  SQL_DATE_STRUCT result = get_data<SQL_C_TYPE_DATE>(fetch_stmt, 1);
  CHECK(result.year == 2026);
  CHECK(result.month == 4);
  CHECK(result.day == 13);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind leap-day SQL_C_TYPE_TIMESTAMP at midnight to SQL_TYPE_DATE",
                 "[c_timestamp][conversion][sql_date]") {
  // Given a DATE column
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

  // When the leap day 2024-02-29 at exactly midnight is bound to DATE
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIMESTAMP_STRUCT val = {2024, 2, 29, 0, 0, 0, 0};
  SQLLEN ind = sizeof(val);
  bind_timestamp_and_execute(stmt, val, ind);

  // Then the leap date is preserved
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  SQL_DATE_STRUCT result = get_data<SQL_C_TYPE_DATE>(fetch_stmt, 1);
  CHECK(result.year == 2024);
  CHECK(result.month == 2);
  CHECK(result.day == 29);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_TIMESTAMP with NULL indicator to SQL_TYPE_DATE",
                 "[c_timestamp][conversion][sql_date]") {
  // Given a DATE column
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

  // When SQL_C_TYPE_TIMESTAMP is bound with SQL_NULL_DATA and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLLEN ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_TYPE_DATE, 0, 0, nullptr, 0,
                         &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the stored value should be NULL
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data_optional<SQL_C_TYPE_DATE>(fetch_stmt, 1) == std::nullopt);
}

// ============================================================================
// SQL_C_TYPE_TIMESTAMP → DATE — datetime field overflow (SQLSTATE 22008)
//
// Per ODBC Appendix D, the conversion must fail when the timestamp's time
// portion is non-zero. The driver must NOT silently truncate or round.
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_TYPE_TIMESTAMP with non-zero time bound to SQL_TYPE_DATE",
                 "[c_timestamp][conversion][sql_date]") {
  // Given a DATE column
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

  // When SQL_C_TYPE_TIMESTAMP 2026-04-13 14:30:45 is bound (non-zero h/m/s)
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIMESTAMP_STRUCT val = {2026, 4, 13, 14, 30, 45, 0};
  SQLLEN ind = sizeof(val);
  ret = bind_timestamp_and_try_execute(stmt, val, ind);

  // Then SQLExecute fails with SQLSTATE 22008 (Datetime field overflow)
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22008"));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_TYPE_TIMESTAMP with non-zero fraction bound to SQL_TYPE_DATE",
                 "[c_timestamp][conversion][sql_date]") {
  // Given a DATE column
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

  // When the timestamp carries a non-zero nanosecond fraction (whole seconds
  // are zero, so only `fraction` triggers the overflow)
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIMESTAMP_STRUCT val = {2026, 4, 13, 0, 0, 0, 1};
  SQLLEN ind = sizeof(val);
  ret = bind_timestamp_and_try_execute(stmt, val, ind);

  // Then SQLExecute fails with SQLSTATE 22008
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22008"));
}

TEST_CASE_METHOD(ConnSchemaFixture,
                 "should reject end-of-day SQL_C_TYPE_TIMESTAMP bound to SQL_TYPE_DATE (no rollover)",
                 "[c_timestamp][conversion][sql_date]") {
  // Given a DATE column
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

  // When 23:59:59 on 2026-04-13 is bound: per spec the conversion must NOT
  // silently round up to 2026-04-14, it must error
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIMESTAMP_STRUCT val = {2026, 4, 13, 23, 59, 59, 0};
  SQLLEN ind = sizeof(val);
  ret = bind_timestamp_and_try_execute(stmt, val, ind);

  // Then SQLExecute fails with SQLSTATE 22008
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22008"));
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

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_TYPE_TIMESTAMP with month=13 bound to SQL_TYPE_DATE",
                 "[c_timestamp][conversion][sql_date][invalid]") {
  // Given a DATE column
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

  // When the timestamp carries month=13 (out of legal 1..12 range)
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIMESTAMP_STRUCT val = {2024, 13, 1, 0, 0, 0, 0};
  SQLLEN ind = sizeof(val);
  ret = bind_timestamp_and_try_execute(stmt, val, ind);

  // Then SQLExecute fails with SQLSTATE 22007 (Invalid datetime format)
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22007"));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_TYPE_TIMESTAMP with day=32 bound to SQL_TYPE_DATE",
                 "[c_timestamp][conversion][sql_date][invalid]") {
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

  // When the timestamp carries day=32 (no month has 32 days)
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIMESTAMP_STRUCT val = {2024, 1, 32, 0, 0, 0, 0};
  SQLLEN ind = sizeof(val);
  ret = bind_timestamp_and_try_execute(stmt, val, ind);

  // Then SQLExecute fails with SQLSTATE 22007
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22007"));
}

TEST_CASE_METHOD(
    ConnSchemaFixture,
    "should prefer SQLSTATE 22007 over 22008 when SQL_C_TYPE_TIMESTAMP has invalid month and non-zero time",
    "[c_timestamp][conversion][sql_date][invalid]") {
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

  // When the timestamp has BOTH an invalid date field AND a non-zero time
  // portion (which would otherwise trigger 22008): the struct-validity
  // error must take precedence.
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIMESTAMP_STRUCT val = {2024, 13, 1, 14, 30, 45, 500000000};
  SQLLEN ind = sizeof(val);
  ret = bind_timestamp_and_try_execute(stmt, val, ind);

  // Then SQLExecute fails with SQLSTATE 22007, NOT 22008
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22007"));
}
