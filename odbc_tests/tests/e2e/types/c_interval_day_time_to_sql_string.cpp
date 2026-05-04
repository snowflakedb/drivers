// ODBC E2E: SQL_C_INTERVAL_DAY / HOUR / MINUTE / SECOND and the
// composite DAY_TO_HOUR / DAY_TO_MINUTE / DAY_TO_SECOND /
// HOUR_TO_MINUTE / HOUR_TO_SECOND / MINUTE_TO_SECOND C types bound via
// SQLBindParameter to SQL_VARCHAR.
//
// Snowflake has no native INTERVAL column type, so per ODBC Appendix D
// ("Converting Data from C to SQL Data Types") all SQL_C_INTERVAL_*
// parameters are routed to a VARCHAR target and formatted as the ANSI
// SQL interval literal text. These tests exercise the round-trip:
// SQLPrepare → SQLBindParameter → SQLExecute → SELECT → SQLGetData.
//
// Format reference (ODBC Appendix D, "C to SQL: Interval"):
//   DAY                : [-]<day>
//   HOUR               : [-]<hour>
//   MINUTE             : [-]<minute>
//   SECOND             : [-]<second>[.<fraction>]
//   DAY_TO_HOUR        : [-]<day> <hour>
//   DAY_TO_MINUTE      : [-]<day> <hour>:<minute(2)>
//   DAY_TO_SECOND      : [-]<day> <hour>:<minute(2)>:<second(2)>[.<fraction>]
//   HOUR_TO_MINUTE     : [-]<hour>:<minute(2)>
//   HOUR_TO_SECOND     : [-]<hour>:<minute(2)>:<second(2)>[.<fraction>]
//   MINUTE_TO_SECOND   : [-]<minute>:<second(2)>[.<fraction>]
//
// `fraction` is in microseconds (matches the unit used elsewhere in the
// driver — see `numeric_helpers::compute_interval_fraction`). Trailing
// zeros are trimmed and the dot is omitted entirely when fraction == 0.

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

namespace {

SQL_INTERVAL_STRUCT ds_interval(SQLSMALLINT sign, SQLUINTEGER day, SQLUINTEGER hour, SQLUINTEGER minute,
                                SQLUINTEGER second, SQLUINTEGER fraction) {
  SQL_INTERVAL_STRUCT iv = {};
  iv.interval_sign = sign;
  iv.intval.day_second.day = day;
  iv.intval.day_second.hour = hour;
  iv.intval.day_second.minute = minute;
  iv.intval.day_second.second = second;
  iv.intval.day_second.fraction = fraction;
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
// Single-field day/time intervals
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_DAY to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_DAY carrying 15 days is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(0, 15, 0, 0, 0, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_DAY, val, ind);

  // Then only the day field is rendered
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "15");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_HOUR to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_HOUR carrying 8 hours is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(0, 0, 8, 0, 0, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_HOUR, val, ind);

  // Then only the hour field is rendered (no zero-padding for the
  // leading field, per Appendix D)
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "8");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_MINUTE to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_MINUTE carrying 30 minutes is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(0, 0, 0, 30, 0, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_MINUTE, val, ind);

  // Then only the minute field is rendered
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "30");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_SECOND with no fraction to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_SECOND carrying 45 seconds (zero fraction) is
  // bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(0, 0, 0, 0, 45, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_SECOND, val, ind);

  // Then no decimal point appears when the fraction is zero
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "45");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_SECOND with microsecond fraction to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_SECOND carrying 45.5s — fraction=500_000 microseconds
  // (this is the unit produced by `numeric_helpers::compute_interval_fraction`
  // and consumed by the formatter; see #980 review)
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(0, 0, 0, 0, 45, 500'000);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_SECOND, val, ind);

  // Then trailing zeros are trimmed and only the significant fractional
  // digit is rendered
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "45.5");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_SECOND with one-microsecond fraction to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_SECOND carrying 1.000001s — verifies the
  // fractional component is rendered with 6-digit microsecond precision
  // (would have rendered as "1.000000001" if the formatter still used
  // the buggy 9-digit nanosecond width caught in #980 review).
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(0, 0, 0, 0, 1, 1);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_SECOND, val, ind);

  // Then leading-zero microseconds are preserved up to 6 digits
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "1.000001");
}

// ============================================================================
// Composite day/time intervals
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_DAY_TO_HOUR to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_DAY_TO_HOUR carrying 3 days 7 hours is bound and
  // inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(0, 3, 7, 0, 0, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_DAY_TO_HOUR, val, ind);

  // Then the "<day> <hour>" form is stored, no zero-padding on the hour
  // field (Appendix D: only sub-fields after a `:` separator are
  // zero-padded)
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "3 7");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_DAY_TO_MINUTE to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_DAY_TO_MINUTE carrying 3 days 7:05 is bound and
  // inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(0, 3, 7, 5, 0, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_DAY_TO_MINUTE, val, ind);

  // Then the minute sub-field is zero-padded to 2 digits while the hour
  // (still a `<space>`-separated leading-of-tail field) is not
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "3 7:05");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_DAY_TO_SECOND with fraction to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_DAY_TO_SECOND carrying 10 days 12:30:59.5 is
  // bound and inserted (fraction in microseconds)
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(0, 10, 12, 30, 59, 500'000);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_DAY_TO_SECOND, val, ind);

  // Then minute and second sub-fields are zero-padded; the fractional
  // tail is microsecond-scaled and trailing zeros are trimmed
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "10 12:30:59.5");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind negative SQL_C_INTERVAL_DAY_TO_SECOND to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_DAY_TO_SECOND carrying -1 day 02:03:04 is bound
  // and inserted (sign=1 means the whole interval is negative; per
  // Appendix D the leading `-` is applied once before the day field)
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(1, 1, 2, 3, 4, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_DAY_TO_SECOND, val, ind);

  // Then the leading sign is applied once
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "-1 2:03:04");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_HOUR_TO_MINUTE to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_HOUR_TO_MINUTE carrying 14:07 is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(0, 0, 14, 7, 0, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_HOUR_TO_MINUTE, val, ind);

  // Then the minute sub-field is zero-padded to 2 digits
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "14:07");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_HOUR_TO_SECOND with fraction to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_HOUR_TO_SECOND carrying 12:30:59.25 is bound and
  // inserted (250_000 microseconds = 0.25 seconds)
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(0, 0, 12, 30, 59, 250'000);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_HOUR_TO_SECOND, val, ind);

  // Then minute and second are zero-padded and the fractional tail is
  // rendered with microsecond precision (trailing zeros trimmed)
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "12:30:59.25");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_MINUTE_TO_SECOND no fraction to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_MINUTE_TO_SECOND carrying 30:07 is bound and
  // inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(0, 0, 0, 30, 7, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_MINUTE_TO_SECOND, val, ind);

  // Then the second sub-field is zero-padded to 2 digits and the
  // leading minute is not (it's the leading field of the interval)
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "30:07");
}

// ============================================================================
// NULL indicator
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_DAY_TO_SECOND with NULL indicator to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_DAY_TO_SECOND is bound with SQL_NULL_DATA and
  // inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLLEN ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_INTERVAL_DAY_TO_SECOND, SQL_VARCHAR, 200, 0,
                         nullptr, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the stored value should be NULL
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data_optional<SQL_C_CHAR>(fetch_stmt, 1) == std::nullopt);
}
