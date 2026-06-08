// ODBC E2E: SQL_C_TYPE_TIME bound via SQLBindParameter to the character
// SQL target types (SQL_CHAR / SQL_VARCHAR / SQL_LONGVARCHAR and their
// wide SQL_WCHAR / SQL_WVARCHAR / SQL_WLONGVARCHAR counterparts).
//
// Per ODBC Appendix D ("Converting Data from C to SQL Data Types",
// section "C to SQL: Time") a SQL_C_TYPE_TIME bound to a character target
// is rendered as the ANSI time literal "hh:mm:ss". SQL_TIME_STRUCT has no
// fractional-seconds field, so the rendering never carries a fraction.
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

SQL_TIME_STRUCT make_time(SQLUSMALLINT hour, SQLUSMALLINT minute, SQLUSMALLINT second) {
  SQL_TIME_STRUCT t = {};
  t.hour = hour;
  t.minute = minute;
  t.second = second;
  return t;
}

// Binds the SQL_C_TYPE_TIME value as `sql_type`, projects it through
// `SELECT ? AS val`, and fetches the single row so the converted string is
// available via get_data on `stmt`.
void bind_time_and_select(StatementHandleWrapper& stmt, SQLSMALLINT sql_type, SQL_TIME_STRUCT& val, SQLLEN& ind) {
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_TIME, sql_type, 100, 0, &val,
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

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_TIME to SQL_VARCHAR and read back",
                 "[c_time][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_TIME 13:45:30 is bound to SQL_VARCHAR and projected via SELECT ?
  SQL_TIME_STRUCT val = make_time(13, 45, 30);
  SQLLEN ind = sizeof(val);
  bind_time_and_select(stmt, SQL_VARCHAR, val, ind);

  // Then the value is rendered as the ANSI time literal "13:45:30"
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "13:45:30");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind midnight SQL_C_TYPE_TIME to SQL_VARCHAR",
                 "[c_time][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_TIME 00:00:00 (midnight) is bound and projected via SELECT ?
  SQL_TIME_STRUCT val = make_time(0, 0, 0);
  SQLLEN ind = sizeof(val);
  bind_time_and_select(stmt, SQL_VARCHAR, val, ind);

  // Then every field is zero-padded as "00:00:00"
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "00:00:00");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind end-of-day SQL_C_TYPE_TIME to SQL_VARCHAR",
                 "[c_time][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_TIME 23:59:59 is bound and projected via SELECT ?
  SQL_TIME_STRUCT val = make_time(23, 59, 59);
  SQLLEN ind = sizeof(val);
  bind_time_and_select(stmt, SQL_VARCHAR, val, ind);

  // Then the value is rendered as "23:59:59"
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "23:59:59");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should zero-pad single-digit fields for SQL_C_TYPE_TIME",
                 "[c_time][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_TIME 01:02:03 is bound and projected via SELECT ?
  SQL_TIME_STRUCT val = make_time(1, 2, 3);
  SQLLEN ind = sizeof(val);
  bind_time_and_select(stmt, SQL_VARCHAR, val, ind);

  // Then each field is zero-padded to two digits as "01:02:03"
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "01:02:03");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind noon SQL_C_TYPE_TIME to SQL_VARCHAR",
                 "[c_time][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_TIME 12:00:00 (noon) is bound and projected via SELECT ?
  SQL_TIME_STRUCT val = make_time(12, 0, 0);
  SQLLEN ind = sizeof(val);
  bind_time_and_select(stmt, SQL_VARCHAR, val, ind);

  // Then the value is rendered as "12:00:00"
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "12:00:00");
}

// ============================================================================
// Character target-type coverage (same value, every SQL string concise type)
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_TIME to SQL_CHAR target",
                 "[c_time][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_TIME 13:45:30 is bound to a SQL_CHAR parameter
  SQL_TIME_STRUCT val = make_time(13, 45, 30);
  SQLLEN ind = sizeof(val);
  bind_time_and_select(stmt, SQL_CHAR, val, ind);

  // Then the rendered literal matches the SQL_VARCHAR rendering
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "13:45:30");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_TIME to SQL_LONGVARCHAR target",
                 "[c_time][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_TIME 13:45:30 is bound to a SQL_LONGVARCHAR parameter
  SQL_TIME_STRUCT val = make_time(13, 45, 30);
  SQLLEN ind = sizeof(val);
  bind_time_and_select(stmt, SQL_LONGVARCHAR, val, ind);

  // Then the rendered literal matches the SQL_VARCHAR rendering
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "13:45:30");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_TIME to SQL_WCHAR target",
                 "[c_time][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_TIME 13:45:30 is bound to a SQL_WCHAR parameter
  SQL_TIME_STRUCT val = make_time(13, 45, 30);
  SQLLEN ind = sizeof(val);
  bind_time_and_select(stmt, SQL_WCHAR, val, ind);

  // Then the rendered literal matches the SQL_VARCHAR rendering
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "13:45:30");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_TIME to SQL_WVARCHAR target",
                 "[c_time][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_TIME 13:45:30 is bound to a SQL_WVARCHAR parameter
  SQL_TIME_STRUCT val = make_time(13, 45, 30);
  SQLLEN ind = sizeof(val);
  bind_time_and_select(stmt, SQL_WVARCHAR, val, ind);

  // Then the rendered literal matches the SQL_VARCHAR rendering
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "13:45:30");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_TIME to SQL_WLONGVARCHAR target",
                 "[c_time][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_TIME 13:45:30 is bound to a SQL_WLONGVARCHAR parameter
  SQL_TIME_STRUCT val = make_time(13, 45, 30);
  SQLLEN ind = sizeof(val);
  bind_time_and_select(stmt, SQL_WLONGVARCHAR, val, ind);

  // Then the rendered literal matches the SQL_VARCHAR rendering
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "13:45:30");
}

// ============================================================================
// NULL handling
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_TIME with NULL indicator to SQL_VARCHAR",
                 "[c_time][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_TIME is bound with SQL_NULL_DATA and projected via SELECT ?
  SQLLEN ind = SQL_NULL_DATA;
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_TIME, SQL_VARCHAR, 100, 0, nullptr, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the projected value should be NULL
  CHECK(get_data_optional<SQL_C_CHAR>(stmt, 1) == std::nullopt);
}
