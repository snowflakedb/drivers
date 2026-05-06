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
// Format reference, per ODBC "Interval Data Type Length" (every
// non-leading datetime field is rendered as exactly two characters and
// the seconds component carries "1 plus the express or implied seconds
// precision" — defaulting to a 6-digit microsecond fraction):
//   DAY                : [-]<day>
//   HOUR               : [-]<hour>
//   MINUTE             : [-]<minute>
//   SECOND             : [-]<second>.<fraction(6)>
//   DAY_TO_HOUR        : [-]<day> <hour(2)>
//   DAY_TO_MINUTE      : [-]<day> <hour(2)>:<minute(2)>
//   DAY_TO_SECOND      : [-]<day> <hour(2)>:<minute(2)>:<second(2)>.<fraction(6)>
//   HOUR_TO_MINUTE     : [-]<hour>:<minute(2)>
//   HOUR_TO_SECOND     : [-]<hour>:<minute(2)>:<second(2)>.<fraction(6)>
//   MINUTE_TO_SECOND   : [-]<minute>:<second(2)>.<fraction(6)>
//
// `fraction` is in microseconds (matches the unit used elsewhere in the
// driver — see `numeric_helpers::compute_interval_fraction`) and is
// always emitted at the canonical 6-digit width with the decimal point,
// even when the value is zero. This matches both the spec literal width
// and the legacy 3.16.0 driver, so applications can round-trip a value
// through either driver and get an identical string.

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

// `decimal_digits` is the seconds-fractional precision per ODBC spec
// (Appendix D, "DecimalDigits parameter for SQLBindParameter"): 0 for
// non-second-bearing intervals, 6 (default) for SECOND / DAY_TO_SECOND /
// HOUR_TO_SECOND / MINUTE_TO_SECOND. Drivers are not required to consult
// this value when formatting (we always emit a fixed 6-digit fraction
// per the legacy 3.16.0 driver), but advertising the spec-correct
// precision through SQLBindParameter is what conformant applications do.
void bind_interval_and_execute(StatementHandleWrapper& stmt, SQLSMALLINT c_type, SQL_INTERVAL_STRUCT& val, SQLLEN& ind,
                               SQLSMALLINT decimal_digits = 0) {
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, c_type, SQL_VARCHAR, 200, decimal_digits, &val,
                                   sizeof(val), &ind);
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
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 15, 0, 0, 0, 0);
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
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 0, 8, 0, 0, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_HOUR, val, ind);

  // Then only the hour field is rendered without zero-padding for the leading field
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
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 0, 0, 30, 0, 0);
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

  // When SQL_C_INTERVAL_SECOND carrying 45 seconds is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 0, 0, 0, 45, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_SECOND, val, ind, 6);

  // Then the fraction is rendered at the canonical 6-digit width per ODBC "Interval Data Type Length"
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "45.000000");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_SECOND with microsecond fraction to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_SECOND carrying 45.500000s is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 0, 0, 0, 45, 500'000);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_SECOND, val, ind, 6);

  // Then the fraction is rendered at the canonical 6-digit width matching the legacy driver
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "45.500000");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_SECOND with one-microsecond fraction to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_SECOND carrying 1.000001s is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 0, 0, 0, 1, 1);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_SECOND, val, ind, 6);

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

  // When SQL_C_INTERVAL_DAY_TO_HOUR carrying 3 days 7 hours is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 3, 7, 0, 0, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_DAY_TO_HOUR, val, ind);

  // Then the "<day> <hour(2)>" form is stored with the hour zero-padded to 2 digits
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "3 07");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_DAY_TO_MINUTE to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_DAY_TO_MINUTE carrying 3 days 7:05 is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 3, 7, 5, 0, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_DAY_TO_MINUTE, val, ind);

  // Then both hour and minute sub-fields are zero-padded to 2 digits
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "3 07:05");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_DAY_TO_SECOND with fraction to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_DAY_TO_SECOND carrying 10 days 12:30:59.5 is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 10, 12, 30, 59, 500'000);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_DAY_TO_SECOND, val, ind, 6);

  // Then hour, minute and second are zero-padded and the seconds fraction is rendered at 6-digit microsecond width
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "10 12:30:59.500000");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind negative SQL_C_INTERVAL_DAY_TO_SECOND to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_DAY_TO_SECOND carrying -1 day 02:03:04 is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_TRUE, 1, 2, 3, 4, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_DAY_TO_SECOND, val, ind, 6);

  // Then the leading sign is applied once and the seconds fraction is emitted at the canonical 6-digit width even when
  // zero
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "-1 02:03:04.000000");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_HOUR_TO_MINUTE to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_HOUR_TO_MINUTE carrying 14:07 is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 0, 14, 7, 0, 0);
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

  // When SQL_C_INTERVAL_HOUR_TO_SECOND carrying 12:30:59.25 is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 0, 12, 30, 59, 250'000);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_HOUR_TO_SECOND, val, ind, 6);

  // Then minute and second are zero-padded and the fractional tail is rendered at 6-digit microsecond width
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "12:30:59.250000");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_MINUTE_TO_SECOND with no fraction to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_MINUTE_TO_SECOND carrying 30:07.000000 is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 0, 0, 30, 7, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_MINUTE_TO_SECOND, val, ind, 6);

  // Then the second sub-field is zero-padded to 2 digits and the fraction is emitted at the canonical 6-digit width
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "30:07.000000");
}

// ============================================================================
// Negative-sign coverage for the remaining single-field and composite types.
// `interval_sign != 0` must be applied exactly once before the leading field
// regardless of which variant is active.
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind negative SQL_C_INTERVAL_DAY to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_DAY carrying -15 days is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_TRUE, 15, 0, 0, 0, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_DAY, val, ind);

  // Then the leading sign is preserved
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "-15");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind negative SQL_C_INTERVAL_HOUR to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_HOUR carrying -8 hours is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_TRUE, 0, 8, 0, 0, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_HOUR, val, ind);

  // Then the leading sign is preserved
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "-8");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind negative SQL_C_INTERVAL_MINUTE to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_MINUTE carrying -30 minutes is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_TRUE, 0, 0, 30, 0, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_MINUTE, val, ind);

  // Then the leading sign is preserved
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "-30");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind negative SQL_C_INTERVAL_SECOND to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_SECOND carrying -45.500000s is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_TRUE, 0, 0, 0, 45, 500'000);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_SECOND, val, ind, 6);

  // Then the leading sign is applied once before the leading field and the fraction is rendered at 6-digit width
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "-45.500000");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind negative SQL_C_INTERVAL_DAY_TO_HOUR to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_DAY_TO_HOUR carrying -3 days 7 hours is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_TRUE, 3, 7, 0, 0, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_DAY_TO_HOUR, val, ind);

  // Then the leading sign is applied once before the day field and the hour stays zero-padded to 2 digits
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "-3 07");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind negative SQL_C_INTERVAL_DAY_TO_MINUTE to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_DAY_TO_MINUTE carrying -3 days 07:05 is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_TRUE, 3, 7, 5, 0, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_DAY_TO_MINUTE, val, ind);

  // Then the leading sign is applied once and both sub-fields are zero-padded to 2 digits
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "-3 07:05");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind negative SQL_C_INTERVAL_HOUR_TO_MINUTE to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_HOUR_TO_MINUTE carrying -14:07 is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_TRUE, 0, 14, 7, 0, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_HOUR_TO_MINUTE, val, ind);

  // Then the leading sign is applied once and the minute is zero-padded to 2 digits
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "-14:07");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind negative SQL_C_INTERVAL_HOUR_TO_SECOND to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_HOUR_TO_SECOND carrying -12:30:59.250000 is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_TRUE, 0, 12, 30, 59, 250'000);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_HOUR_TO_SECOND, val, ind, 6);

  // Then the leading sign is applied once and the trailing fields keep their canonical widths
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "-12:30:59.250000");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind negative SQL_C_INTERVAL_MINUTE_TO_SECOND to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_MINUTE_TO_SECOND carrying -30:07.000000 is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_TRUE, 0, 0, 30, 7, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_MINUTE_TO_SECOND, val, ind, 6);

  // Then the leading sign is applied once and the second sub-field stays zero-padded
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "-30:07.000000");
}

// ============================================================================
// Fraction boundaries — pin behaviour at the maximum representable
// microsecond and at the first out-of-spec value (>= 1 second).
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture,
                 "should bind SQL_C_INTERVAL_SECOND with maximum microsecond fraction to SQL_VARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_SECOND carrying 45.999999s (max valid microsecond fraction) is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 0, 0, 0, 45, 999'999);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_SECOND, val, ind, 6);

  // Then the upper boundary of the 6-digit width is rendered without truncation
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "45.999999");
}

// fraction == 1_000_000 is technically out-of-spec (the seconds-precision
// default is 6, so values above 999_999 should overflow into the seconds
// field rather than be passed verbatim). The current driver formatter
// emits `format!("{:06}", fraction)` which is min-width, not truncate, so
// values >= 1_000_000 produce 7 digits. This test pins that behaviour so
// future changes (truncation, overflow detection, or rejection) surface
// here as an explicit assertion failure.
TEST_CASE_METHOD(ConnSchemaFixture,
                 "should render seven-digit fraction for SQL_C_INTERVAL_SECOND when fraction equals one second",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_SECOND carrying second=45 fraction=1_000_000 is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 0, 0, 0, 45, 1'000'000);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_SECOND, val, ind, 6);

  // Then the formatter emits seven digits — documenting the current behaviour for an out-of-spec input
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "45.1000000");
}

// ============================================================================
// Alternate character SQL targets — the spec lists CHAR / VARCHAR / LONGVARCHAR
// (and their wide-character twins) as legal targets for SQL_C_INTERVAL_*.
// SnowflakeVarchar handles all of them identically; one representative
// composite interval per target is enough to exercise the routing.
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_DAY_TO_SECOND to SQL_CHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a CHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col CHAR(40))");

  // When SQL_C_INTERVAL_DAY_TO_SECOND carrying 10 12:30:59.500000 is bound to SQL_CHAR
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 10, 12, 30, 59, 500'000);
  SQLLEN ind = sizeof(val);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_INTERVAL_DAY_TO_SECOND, SQL_CHAR, 40, 6, &val,
                         sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE_ODBC(SQLExecute(stmt.getHandle()), stmt);

  // Then the canonical literal is stored
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "10 12:30:59.500000");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_DAY_TO_SECOND to SQL_WCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a CHAR column (Snowflake routes wide character targets through TEXT)
  conn.execute("CREATE TEMPORARY TABLE t (col CHAR(40))");

  // When SQL_C_INTERVAL_DAY_TO_SECOND carrying 10 12:30:59.500000 is bound to SQL_WCHAR
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 10, 12, 30, 59, 500'000);
  SQLLEN ind = sizeof(val);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_INTERVAL_DAY_TO_SECOND, SQL_WCHAR, 40, 6, &val,
                         sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE_ODBC(SQLExecute(stmt.getHandle()), stmt);

  // Then the canonical literal is stored
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "10 12:30:59.500000");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_DAY_TO_SECOND to SQL_LONGVARCHAR",
                 "[c_interval][conversion][sql_string]") {
  // Given a VARCHAR column (Snowflake routes LONGVARCHAR through TEXT)
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When SQL_C_INTERVAL_DAY_TO_SECOND carrying 10 12:30:59.500000 is bound to SQL_LONGVARCHAR
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 10, 12, 30, 59, 500'000);
  SQLLEN ind = sizeof(val);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_INTERVAL_DAY_TO_SECOND, SQL_LONGVARCHAR, 200, 6,
                         &val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE_ODBC(SQLExecute(stmt.getHandle()), stmt);

  // Then the canonical literal is stored
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "10 12:30:59.500000");
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
