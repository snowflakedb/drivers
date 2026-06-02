// ODBC E2E: SQL_C_TYPE_TIMESTAMP bound via SQLBindParameter to the
// character SQL target types (SQL_CHAR / SQL_VARCHAR / SQL_LONGVARCHAR and
// their wide SQL_WCHAR / SQL_WVARCHAR / SQL_WLONGVARCHAR counterparts).
//
// Per ODBC Appendix D ("Converting Data from C to SQL Data Types",
// section "C to SQL: Timestamp") a SQL_C_TYPE_TIMESTAMP bound to a
// character target is rendered as the ANSI timestamp literal
// "yyyy-mm-dd hh:mm:ss[.fffffffff]". The fractional-seconds part is only
// emitted when SQL_TIMESTAMP_STRUCT.fraction is non-zero, and the driver
// renders it with nanosecond (nine-digit) precision.
//
// The bound parameter is projected through `SELECT ? AS val` (mirroring
// the existing c_temporal_to_varchar suite) so the assertion observes the
// exact character rendering produced by the C-to-SQL parameter conversion
// without a typed result column coercing the value.

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

namespace {

SQL_TIMESTAMP_STRUCT make_timestamp(SQLSMALLINT year, SQLUSMALLINT month, SQLUSMALLINT day, SQLUSMALLINT hour,
                                    SQLUSMALLINT minute, SQLUSMALLINT second, SQLUINTEGER fraction) {
  SQL_TIMESTAMP_STRUCT ts = {};
  ts.year = year;
  ts.month = month;
  ts.day = day;
  ts.hour = hour;
  ts.minute = minute;
  ts.second = second;
  ts.fraction = fraction;
  return ts;
}

// Binds the SQL_C_TYPE_TIMESTAMP value as `sql_type`, projects it through
// `SELECT ? AS val`, and fetches the single row so the converted string is
// available via get_data on `stmt`.
void bind_timestamp_and_select(StatementHandleWrapper& stmt, SQLSMALLINT sql_type, SQL_TIMESTAMP_STRUCT& val,
                               SQLLEN& ind) {
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, sql_type, 100, 0, &val,
                                   sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
}

}  // namespace

// ============================================================================
// Value coverage (SQL_VARCHAR target)
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_TIMESTAMP without fraction to SQL_VARCHAR",
                 "[c_timestamp][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_TIMESTAMP 2024-03-15 13:45:30 (no fraction) is bound and projected via SELECT ?
  SQL_TIMESTAMP_STRUCT val = make_timestamp(2024, 3, 15, 13, 45, 30, 0);
  SQLLEN ind = sizeof(val);
  bind_timestamp_and_select(stmt, SQL_VARCHAR, val, ind);

  // Then no fractional part is emitted: "2024-03-15 13:45:30"
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "2024-03-15 13:45:30");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should render nanosecond fraction for SQL_C_TYPE_TIMESTAMP to SQL_VARCHAR",
                 "[c_timestamp][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_TIMESTAMP 2024-03-15 13:45:30.123456789 is bound and projected via SELECT ?
  SQL_TIMESTAMP_STRUCT val = make_timestamp(2024, 3, 15, 13, 45, 30, 123456789);
  SQLLEN ind = sizeof(val);
  bind_timestamp_and_select(stmt, SQL_VARCHAR, val, ind);

  // Then nine-digit nanoseconds are rendered: "2024-03-15 13:45:30.123456789"
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "2024-03-15 13:45:30.123456789");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should zero-pad the nanosecond fraction for SQL_C_TYPE_TIMESTAMP",
                 "[c_timestamp][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_TIMESTAMP with fraction 1000 nanoseconds is bound and projected via SELECT ?
  SQL_TIMESTAMP_STRUCT val = make_timestamp(2024, 3, 15, 13, 45, 30, 1000);
  SQLLEN ind = sizeof(val);
  bind_timestamp_and_select(stmt, SQL_VARCHAR, val, ind);

  // Then the fraction is zero-padded to nine digits: "2024-03-15 13:45:30.000001000"
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "2024-03-15 13:45:30.000001000");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind midnight SQL_C_TYPE_TIMESTAMP to SQL_VARCHAR",
                 "[c_timestamp][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_TIMESTAMP 2024-01-01 00:00:00 (midnight) is bound and projected via SELECT ?
  SQL_TIMESTAMP_STRUCT val = make_timestamp(2024, 1, 1, 0, 0, 0, 0);
  SQLLEN ind = sizeof(val);
  bind_timestamp_and_select(stmt, SQL_VARCHAR, val, ind);

  // Then the value is rendered as "2024-01-01 00:00:00"
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "2024-01-01 00:00:00");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should zero-pad single-digit fields for SQL_C_TYPE_TIMESTAMP",
                 "[c_timestamp][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_TIMESTAMP 2024-01-05 01:02:03 is bound and projected via SELECT ?
  SQL_TIMESTAMP_STRUCT val = make_timestamp(2024, 1, 5, 1, 2, 3, 0);
  SQLLEN ind = sizeof(val);
  bind_timestamp_and_select(stmt, SQL_VARCHAR, val, ind);

  // Then all fields are zero-padded as "2024-01-05 01:02:03"
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "2024-01-05 01:02:03");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind maximum-year SQL_C_TYPE_TIMESTAMP to SQL_VARCHAR",
                 "[c_timestamp][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_TIMESTAMP 9999-12-31 23:59:59 is bound and projected via SELECT ?
  SQL_TIMESTAMP_STRUCT val = make_timestamp(9999, 12, 31, 23, 59, 59, 0);
  SQLLEN ind = sizeof(val);
  bind_timestamp_and_select(stmt, SQL_VARCHAR, val, ind);

  // Then the four-digit year is preserved as "9999-12-31 23:59:59"
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "9999-12-31 23:59:59");
}

// ============================================================================
// Character target-type coverage (same value, every SQL string concise type)
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_TIMESTAMP to SQL_CHAR target",
                 "[c_timestamp][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_TIMESTAMP 2024-03-15 13:45:30 is bound to SQL_CHAR
  SQL_TIMESTAMP_STRUCT val = make_timestamp(2024, 3, 15, 13, 45, 30, 0);
  SQLLEN ind = sizeof(val);
  bind_timestamp_and_select(stmt, SQL_CHAR, val, ind);

  // Then the rendered literal matches the SQL_VARCHAR rendering
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "2024-03-15 13:45:30");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_TIMESTAMP to SQL_LONGVARCHAR target",
                 "[c_timestamp][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_TIMESTAMP 2024-03-15 13:45:30 is bound to SQL_LONGVARCHAR
  SQL_TIMESTAMP_STRUCT val = make_timestamp(2024, 3, 15, 13, 45, 30, 0);
  SQLLEN ind = sizeof(val);
  bind_timestamp_and_select(stmt, SQL_LONGVARCHAR, val, ind);

  // Then the rendered literal matches the SQL_VARCHAR rendering
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "2024-03-15 13:45:30");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_TIMESTAMP to SQL_WCHAR target",
                 "[c_timestamp][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_TIMESTAMP 2024-03-15 13:45:30 is bound to SQL_WCHAR
  SQL_TIMESTAMP_STRUCT val = make_timestamp(2024, 3, 15, 13, 45, 30, 0);
  SQLLEN ind = sizeof(val);
  bind_timestamp_and_select(stmt, SQL_WCHAR, val, ind);

  // Then the rendered literal matches the SQL_VARCHAR rendering
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "2024-03-15 13:45:30");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_TIMESTAMP to SQL_WVARCHAR target",
                 "[c_timestamp][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_TIMESTAMP 2024-03-15 13:45:30 is bound to SQL_WVARCHAR
  SQL_TIMESTAMP_STRUCT val = make_timestamp(2024, 3, 15, 13, 45, 30, 0);
  SQLLEN ind = sizeof(val);
  bind_timestamp_and_select(stmt, SQL_WVARCHAR, val, ind);

  // Then the rendered literal matches the SQL_VARCHAR rendering
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "2024-03-15 13:45:30");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_TIMESTAMP to SQL_WLONGVARCHAR target",
                 "[c_timestamp][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_TIMESTAMP 2024-03-15 13:45:30 is bound to SQL_WLONGVARCHAR
  SQL_TIMESTAMP_STRUCT val = make_timestamp(2024, 3, 15, 13, 45, 30, 0);
  SQLLEN ind = sizeof(val);
  bind_timestamp_and_select(stmt, SQL_WLONGVARCHAR, val, ind);

  // Then the rendered literal matches the SQL_VARCHAR rendering
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "2024-03-15 13:45:30");
}

// ============================================================================
// NULL handling
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_TIMESTAMP with NULL indicator to SQL_VARCHAR",
                 "[c_timestamp][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_TIMESTAMP is bound with SQL_NULL_DATA and projected via SELECT ?
  SQLLEN ind = SQL_NULL_DATA;
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_VARCHAR, 100, 0,
                                   nullptr, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the projected value should be NULL
  CHECK(get_data_optional<SQL_C_CHAR>(stmt, 1) == std::nullopt);
}
