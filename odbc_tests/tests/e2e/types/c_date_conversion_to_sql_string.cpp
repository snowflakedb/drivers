// ODBC E2E: SQL_C_TYPE_DATE bound via SQLBindParameter to the character
// SQL target types (SQL_CHAR / SQL_VARCHAR / SQL_LONGVARCHAR and their
// wide SQL_WCHAR / SQL_WVARCHAR / SQL_WLONGVARCHAR counterparts).
//
// Per ODBC Appendix D ("Converting Data from C to SQL Data Types",
// section "C to SQL: Date") a SQL_C_TYPE_DATE bound to a character
// target is rendered as the ANSI date literal "yyyy-mm-dd". The leading
// year field is zero-padded to four digits and month/day to two digits.
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

SQL_DATE_STRUCT make_date(SQLSMALLINT year, SQLUSMALLINT month, SQLUSMALLINT day) {
  SQL_DATE_STRUCT d = {};
  d.year = year;
  d.month = month;
  d.day = day;
  return d;
}

// Binds the SQL_C_TYPE_DATE value as `sql_type`, projects it through
// `SELECT ? AS val`, and fetches the single row so the converted string is
// available via get_data on `stmt`.
void bind_date_and_select(StatementHandleWrapper& stmt, SQLSMALLINT sql_type, SQL_DATE_STRUCT& val, SQLLEN& ind) {
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_DATE, sql_type, 100, 0, &val,
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

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_DATE to SQL_VARCHAR and read back",
                 "[c_date][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_DATE 2024-03-15 is bound to SQL_VARCHAR and projected via SELECT ?
  SQL_DATE_STRUCT val = make_date(2024, 3, 15);
  SQLLEN ind = sizeof(val);
  bind_date_and_select(stmt, SQL_VARCHAR, val, ind);

  // Then the value is rendered as the ANSI date literal "2024-03-15"
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "2024-03-15");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should zero-pad single-digit month and day for SQL_C_TYPE_DATE",
                 "[c_date][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_DATE 2024-01-05 is bound to SQL_VARCHAR and projected via SELECT ?
  SQL_DATE_STRUCT val = make_date(2024, 1, 5);
  SQLLEN ind = sizeof(val);
  bind_date_and_select(stmt, SQL_VARCHAR, val, ind);

  // Then month and day are zero-padded to two digits
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "2024-01-05");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind leap-day SQL_C_TYPE_DATE to SQL_VARCHAR",
                 "[c_date][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_DATE 2024-02-29 (a leap day) is bound and projected via SELECT ?
  SQL_DATE_STRUCT val = make_date(2024, 2, 29);
  SQLLEN ind = sizeof(val);
  bind_date_and_select(stmt, SQL_VARCHAR, val, ind);

  // Then the leap day is rendered as "2024-02-29"
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "2024-02-29");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind end-of-year SQL_C_TYPE_DATE to SQL_VARCHAR",
                 "[c_date][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_DATE 2023-12-31 is bound and projected via SELECT ?
  SQL_DATE_STRUCT val = make_date(2023, 12, 31);
  SQLLEN ind = sizeof(val);
  bind_date_and_select(stmt, SQL_VARCHAR, val, ind);

  // Then the value is rendered as "2023-12-31"
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "2023-12-31");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind maximum-year SQL_C_TYPE_DATE to SQL_VARCHAR",
                 "[c_date][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_DATE 9999-12-31 (the ODBC maximum year) is bound
  SQL_DATE_STRUCT val = make_date(9999, 12, 31);
  SQLLEN ind = sizeof(val);
  bind_date_and_select(stmt, SQL_VARCHAR, val, ind);

  // Then the four-digit year is preserved as "9999-12-31"
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "9999-12-31");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should zero-pad the year for early SQL_C_TYPE_DATE values",
                 "[c_date][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_DATE 0001-01-01 is bound and projected via SELECT ?
  SQL_DATE_STRUCT val = make_date(1, 1, 1);
  SQLLEN ind = sizeof(val);
  bind_date_and_select(stmt, SQL_VARCHAR, val, ind);

  // Then the year is zero-padded to four digits as "0001-01-01"
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "0001-01-01");
}

// ============================================================================
// Character target-type coverage (same value, every SQL string concise type)
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_DATE to SQL_CHAR target",
                 "[c_date][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_DATE 2024-03-15 is bound to a SQL_CHAR parameter
  SQL_DATE_STRUCT val = make_date(2024, 3, 15);
  SQLLEN ind = sizeof(val);
  bind_date_and_select(stmt, SQL_CHAR, val, ind);

  // Then the rendered literal matches the SQL_VARCHAR rendering
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "2024-03-15");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_DATE to SQL_LONGVARCHAR target",
                 "[c_date][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_DATE 2024-03-15 is bound to a SQL_LONGVARCHAR parameter
  SQL_DATE_STRUCT val = make_date(2024, 3, 15);
  SQLLEN ind = sizeof(val);
  bind_date_and_select(stmt, SQL_LONGVARCHAR, val, ind);

  // Then the rendered literal matches the SQL_VARCHAR rendering
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "2024-03-15");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_DATE to SQL_WCHAR target",
                 "[c_date][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_DATE 2024-03-15 is bound to a SQL_WCHAR parameter
  SQL_DATE_STRUCT val = make_date(2024, 3, 15);
  SQLLEN ind = sizeof(val);
  bind_date_and_select(stmt, SQL_WCHAR, val, ind);

  // Then the rendered literal matches the SQL_VARCHAR rendering
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "2024-03-15");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_DATE to SQL_WVARCHAR target",
                 "[c_date][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_DATE 2024-03-15 is bound to a SQL_WVARCHAR parameter
  SQL_DATE_STRUCT val = make_date(2024, 3, 15);
  SQLLEN ind = sizeof(val);
  bind_date_and_select(stmt, SQL_WVARCHAR, val, ind);

  // Then the rendered literal matches the SQL_VARCHAR rendering
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "2024-03-15");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_DATE to SQL_WLONGVARCHAR target",
                 "[c_date][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_DATE 2024-03-15 is bound to a SQL_WLONGVARCHAR parameter
  SQL_DATE_STRUCT val = make_date(2024, 3, 15);
  SQLLEN ind = sizeof(val);
  bind_date_and_select(stmt, SQL_WLONGVARCHAR, val, ind);

  // Then the rendered literal matches the SQL_VARCHAR rendering
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "2024-03-15");
}

// ============================================================================
// NULL handling
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_DATE with NULL indicator to SQL_VARCHAR",
                 "[c_date][conversion][sql_string]") {
  auto stmt = conn.createStatement();

  // When SQL_C_TYPE_DATE is bound with SQL_NULL_DATA and projected via SELECT ?
  SQLLEN ind = SQL_NULL_DATA;
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_DATE, SQL_VARCHAR, 100, 0, nullptr, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the projected value should be NULL
  CHECK(get_data_optional<SQL_C_CHAR>(stmt, 1) == std::nullopt);
}
