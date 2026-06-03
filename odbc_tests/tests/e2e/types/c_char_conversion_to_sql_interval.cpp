// ODBC E2E: SQL_C_CHAR / SQL_C_WCHAR bound via SQLBindParameter to the
// SQL_INTERVAL_* parameter types.
//
// Per ODBC Appendix D ("Converting Data from C to SQL Data Types"),
// SQL_C_CHAR and SQL_C_WCHAR are legal source types for every SQL_INTERVAL_*
// target (single-field and compound). The driver treats a character source as
// always legal and forwards the literal verbatim as the parameter value, so it
// round-trips through a VARCHAR column unchanged. (Snowflake has native
// INTERVAL columns as of 2026, but a VARCHAR target is sufficient to exercise
// the C->SQL parameter conversion and lets us assert the exact stored text.)
//
// The legacy driver forwards the ODBC interval type code (101-113) as the
// Snowflake bind-variable type, which the server rejects with HY000
// ("Unsupported data type for bind variable"), so these cases run on the new
// driver only -- documented as BD#73.

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

namespace {

constexpr const char* kIntervalSkipReason =
    "Legacy driver rejects SQL_INTERVAL_* bind-variable type codes server-side (HY000)";

// Binds the SQL_C_CHAR literal `value` as parameter type `sql_interval_type`,
// inserts it into a fresh VARCHAR column, and returns the round-tripped text.
std::string insert_char_interval(Connection& conn, SQLSMALLINT sql_interval_type, const char* value) {
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  std::string buf(value);
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, sql_interval_type, 0, 0, buf.data(),
                         static_cast<SQLLEN>(buf.size() + 1), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  return get_data<SQL_C_CHAR>(fetch_stmt, 1);
}

}  // namespace

// ============================================================================
// SQL_C_CHAR -> single-field SQL_INTERVAL_* targets
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR to SQL_INTERVAL_YEAR",
                 "[c_char][conversion][sql_interval]") {
  SKIP_OLD_DRIVER("BD#73", kIntervalSkipReason);
  // When SQL_C_CHAR "5" is bound as SQL_INTERVAL_YEAR and inserted
  auto stored = insert_char_interval(conn, SQL_INTERVAL_YEAR, "5");
  // Then the literal round-trips through the VARCHAR column
  CHECK(stored == "5");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR to SQL_INTERVAL_MONTH",
                 "[c_char][conversion][sql_interval]") {
  SKIP_OLD_DRIVER("BD#73", kIntervalSkipReason);
  // When SQL_C_CHAR "11" is bound as SQL_INTERVAL_MONTH and inserted
  auto stored = insert_char_interval(conn, SQL_INTERVAL_MONTH, "11");
  // Then the literal round-trips through the VARCHAR column
  CHECK(stored == "11");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR to SQL_INTERVAL_DAY",
                 "[c_char][conversion][sql_interval]") {
  SKIP_OLD_DRIVER("BD#73", kIntervalSkipReason);
  // When SQL_C_CHAR "10" is bound as SQL_INTERVAL_DAY and inserted
  auto stored = insert_char_interval(conn, SQL_INTERVAL_DAY, "10");
  // Then the literal round-trips through the VARCHAR column
  CHECK(stored == "10");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR to SQL_INTERVAL_HOUR",
                 "[c_char][conversion][sql_interval]") {
  SKIP_OLD_DRIVER("BD#73", kIntervalSkipReason);
  // When SQL_C_CHAR "3" is bound as SQL_INTERVAL_HOUR and inserted
  auto stored = insert_char_interval(conn, SQL_INTERVAL_HOUR, "3");
  // Then the literal round-trips through the VARCHAR column
  CHECK(stored == "3");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR to SQL_INTERVAL_MINUTE",
                 "[c_char][conversion][sql_interval]") {
  SKIP_OLD_DRIVER("BD#73", kIntervalSkipReason);
  // When SQL_C_CHAR "30" is bound as SQL_INTERVAL_MINUTE and inserted
  auto stored = insert_char_interval(conn, SQL_INTERVAL_MINUTE, "30");
  // Then the literal round-trips through the VARCHAR column
  CHECK(stored == "30");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR to SQL_INTERVAL_SECOND",
                 "[c_char][conversion][sql_interval]") {
  SKIP_OLD_DRIVER("BD#73", kIntervalSkipReason);
  // When SQL_C_CHAR "45" is bound as SQL_INTERVAL_SECOND and inserted
  auto stored = insert_char_interval(conn, SQL_INTERVAL_SECOND, "45");
  // Then the literal round-trips through the VARCHAR column
  CHECK(stored == "45");
}

// ============================================================================
// SQL_C_CHAR -> compound SQL_INTERVAL_* targets
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR to SQL_INTERVAL_YEAR_TO_MONTH",
                 "[c_char][conversion][sql_interval]") {
  SKIP_OLD_DRIVER("BD#73", kIntervalSkipReason);
  // When SQL_C_CHAR "1-6" is bound as SQL_INTERVAL_YEAR_TO_MONTH and inserted
  auto stored = insert_char_interval(conn, SQL_INTERVAL_YEAR_TO_MONTH, "1-6");
  // Then the literal round-trips through the VARCHAR column
  CHECK(stored == "1-6");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR to SQL_INTERVAL_DAY_TO_SECOND",
                 "[c_char][conversion][sql_interval]") {
  SKIP_OLD_DRIVER("BD#73", kIntervalSkipReason);
  // When SQL_C_CHAR "1 02:03:04" is bound as SQL_INTERVAL_DAY_TO_SECOND and inserted
  auto stored = insert_char_interval(conn, SQL_INTERVAL_DAY_TO_SECOND, "1 02:03:04");
  // Then the literal round-trips through the VARCHAR column
  CHECK(stored == "1 02:03:04");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR to SQL_INTERVAL_HOUR_TO_MINUTE",
                 "[c_char][conversion][sql_interval]") {
  SKIP_OLD_DRIVER("BD#73", kIntervalSkipReason);
  // When SQL_C_CHAR "10:30" is bound as SQL_INTERVAL_HOUR_TO_MINUTE and inserted
  auto stored = insert_char_interval(conn, SQL_INTERVAL_HOUR_TO_MINUTE, "10:30");
  // Then the literal round-trips through the VARCHAR column
  CHECK(stored == "10:30");
}

// ============================================================================
// SQL_C_WCHAR -> SQL_INTERVAL_* targets
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_WCHAR to SQL_INTERVAL_YEAR",
                 "[c_char][conversion][sql_interval]") {
  SKIP_OLD_DRIVER("BD#73", kIntervalSkipReason);

  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When a wide-character year literal is bound as SQL_INTERVAL_YEAR and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLWCHAR val[] = {'7', 0};
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_WCHAR, SQL_INTERVAL_YEAR, 0, 0, val, sizeof(val),
                         &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then the literal is accepted and stored verbatim
  REQUIRE_ODBC(ret, stmt);
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "7");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_WCHAR to SQL_INTERVAL_DAY_TO_SECOND",
                 "[c_char][conversion][sql_interval]") {
  SKIP_OLD_DRIVER("BD#73", kIntervalSkipReason);

  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When a wide-character day-to-second literal is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLWCHAR val[] = {'2', ' ', '0', '3', ':', '0', '4', ':', '0', '5', 0};
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_WCHAR, SQL_INTERVAL_DAY_TO_SECOND, 0, 0, val,
                         sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then the literal is accepted and stored verbatim
  REQUIRE_ODBC(ret, stmt);
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "2 03:04:05");
}

// ============================================================================
// NULL handling
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR with NULL indicator to SQL_INTERVAL_YEAR",
                 "[c_char][conversion][sql_interval]") {
  SKIP_OLD_DRIVER("BD#73", kIntervalSkipReason);

  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When a NULL parameter is bound as SQL_INTERVAL_YEAR using SQL_NULL_DATA
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLLEN ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_INTERVAL_YEAR, 0, 0, nullptr, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the stored value should be NULL
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK_FALSE(get_data_optional<SQL_C_CHAR>(fetch_stmt, 1).has_value());
}
