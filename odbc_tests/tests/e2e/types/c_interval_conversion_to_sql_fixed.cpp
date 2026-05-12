// ODBC E2E: SQL_C_INTERVAL_* bound via SQLBindParameter to the
// "exact numeric" SQL targets (SQL_TINYINT / SQL_SMALLINT /
// SQL_INTEGER / SQL_BIGINT / SQL_DECIMAL / SQL_NUMERIC).
//
// Per ODBC Appendix D ("Converting Data from C to SQL Data Types",
// section "Interval"), only single-field interval C types
// (SQL_C_INTERVAL_YEAR / MONTH / DAY / HOUR / MINUTE / SECOND) have
// a well-defined integer mapping — the value carried by the leading
// (and only) datetime field is sent as the integer. Composite
// intervals (YEAR_TO_MONTH, DAY_TO_*, HOUR_TO_*, MINUTE_TO_SECOND)
// have no single-integer representation and the driver MUST reject
// them with SQLSTATE 07006 ("Restricted data type attribute
// violation"), matching the spec.
//
// Implementation lives in `odbc/src/conversion/number.rs`
// (`SnowflakeNumber::read_odbc`, the `Interval*` arms call
// `read_single_field_interval_i128`). Composite intervals fall
// through to `UnsupportedCDataTypeSnafu`.
//
// Each positive test exercises the round-trip:
//   SQLPrepare → SQLBindParameter → SQLExecute → SELECT → SQLGetData

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
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

SQL_INTERVAL_STRUCT dt_interval(SQLSMALLINT sign, SQLUINTEGER day, SQLUINTEGER hour, SQLUINTEGER minute,
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

// Bind a SQL_C_INTERVAL_* value to the parameter and execute.
// `column_size` / `decimal_digits` only matter for SQL_DECIMAL /
// SQL_NUMERIC; both are 0 for the integer SQL targets per ODBC.
void bind_and_exec(StatementHandleWrapper& stmt, SQLSMALLINT c_type, SQLSMALLINT sql_type, SQL_INTERVAL_STRUCT& val,
                   SQLLEN& ind, SQLULEN column_size = 0, SQLSMALLINT decimal_digits = 0) {
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, c_type, sql_type, column_size, decimal_digits,
                                   &val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
}

}  // namespace

// ============================================================================
// SUCCESSFUL CONVERSIONS - Single-field year/month interval C types
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_YEAR to all exact-numeric SQL targets",
                 "[c_interval][conversion][sql_fixed]") {
  // Given a NUMBER column wide enough to hold any of the integer SQL targets
  conn.execute("CREATE OR REPLACE TEMPORARY TABLE t (col NUMBER(38,0))");

  // When SQL_C_INTERVAL_YEAR carrying 7 years is bound as each exact-numeric SQL target
  for (auto sql_type : {SQL_TINYINT, SQL_SMALLINT, SQL_INTEGER, SQL_BIGINT}) {
    conn.execute("CREATE OR REPLACE TEMPORARY TABLE t (col NUMBER(38,0))");
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
    REQUIRE_ODBC(ret, stmt);
    SQL_INTERVAL_STRUCT val = ym_interval(SQL_FALSE, 7, 0);
    SQLLEN ind = sizeof(val);
    bind_and_exec(stmt, SQL_C_INTERVAL_YEAR, static_cast<SQLSMALLINT>(sql_type), val, ind);

    // Then the leading-field integer is read back unchanged
    auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
    INFO("sql_type=" << sql_type);
    CHECK(get_data<SQL_C_SBIGINT>(fetch_stmt, 1) == 7);
  }

  // And the same value round-trips through SQL_DECIMAL and SQL_NUMERIC with scale=0
  for (auto sql_type : {SQL_DECIMAL, SQL_NUMERIC}) {
    conn.execute("CREATE OR REPLACE TEMPORARY TABLE t (col NUMBER(38,0))");
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
    REQUIRE_ODBC(ret, stmt);
    SQL_INTERVAL_STRUCT val = ym_interval(SQL_FALSE, 7, 0);
    SQLLEN ind = sizeof(val);
    bind_and_exec(stmt, SQL_C_INTERVAL_YEAR, static_cast<SQLSMALLINT>(sql_type), val, ind, 10, 0);

    auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
    INFO("sql_type=" << sql_type);
    CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "7");
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_MONTH to all exact-numeric SQL targets",
                 "[c_interval][conversion][sql_fixed]") {
  // Given a NUMBER column
  conn.execute("CREATE OR REPLACE TEMPORARY TABLE t (col NUMBER(38,0))");

  // When SQL_C_INTERVAL_MONTH carrying 11 months is bound as each integer SQL target
  for (auto sql_type : {SQL_TINYINT, SQL_SMALLINT, SQL_INTEGER, SQL_BIGINT}) {
    conn.execute("CREATE OR REPLACE TEMPORARY TABLE t (col NUMBER(38,0))");
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
    REQUIRE_ODBC(ret, stmt);
    SQL_INTERVAL_STRUCT val = ym_interval(SQL_FALSE, 0, 11);
    SQLLEN ind = sizeof(val);
    bind_and_exec(stmt, SQL_C_INTERVAL_MONTH, static_cast<SQLSMALLINT>(sql_type), val, ind);

    // Then the month component is read back unchanged
    auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
    INFO("sql_type=" << sql_type);
    CHECK(get_data<SQL_C_SBIGINT>(fetch_stmt, 1) == 11);
  }
}

// ============================================================================
// SUCCESSFUL CONVERSIONS - Single-field day/time interval C types
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_DAY to all exact-numeric SQL targets",
                 "[c_interval][conversion][sql_fixed]") {
  // Given a NUMBER column
  conn.execute("CREATE OR REPLACE TEMPORARY TABLE t (col NUMBER(38,0))");

  // When SQL_C_INTERVAL_DAY carrying 31 days is bound as each integer SQL target
  for (auto sql_type : {SQL_TINYINT, SQL_SMALLINT, SQL_INTEGER, SQL_BIGINT}) {
    conn.execute("CREATE OR REPLACE TEMPORARY TABLE t (col NUMBER(38,0))");
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
    REQUIRE_ODBC(ret, stmt);
    SQL_INTERVAL_STRUCT val = dt_interval(SQL_FALSE, 31, 0, 0, 0, 0);
    SQLLEN ind = sizeof(val);
    bind_and_exec(stmt, SQL_C_INTERVAL_DAY, static_cast<SQLSMALLINT>(sql_type), val, ind);

    // Then the day component is read back unchanged
    auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
    INFO("sql_type=" << sql_type);
    CHECK(get_data<SQL_C_SBIGINT>(fetch_stmt, 1) == 31);
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_HOUR to all exact-numeric SQL targets",
                 "[c_interval][conversion][sql_fixed]") {
  // Given a NUMBER column
  conn.execute("CREATE OR REPLACE TEMPORARY TABLE t (col NUMBER(38,0))");

  // When SQL_C_INTERVAL_HOUR carrying 23 hours is bound as each integer SQL target
  for (auto sql_type : {SQL_TINYINT, SQL_SMALLINT, SQL_INTEGER, SQL_BIGINT}) {
    conn.execute("CREATE OR REPLACE TEMPORARY TABLE t (col NUMBER(38,0))");
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
    REQUIRE_ODBC(ret, stmt);
    SQL_INTERVAL_STRUCT val = dt_interval(SQL_FALSE, 0, 23, 0, 0, 0);
    SQLLEN ind = sizeof(val);
    bind_and_exec(stmt, SQL_C_INTERVAL_HOUR, static_cast<SQLSMALLINT>(sql_type), val, ind);

    // Then the hour component is read back unchanged
    auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
    INFO("sql_type=" << sql_type);
    CHECK(get_data<SQL_C_SBIGINT>(fetch_stmt, 1) == 23);
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_MINUTE to all exact-numeric SQL targets",
                 "[c_interval][conversion][sql_fixed]") {
  // Given a NUMBER column
  conn.execute("CREATE OR REPLACE TEMPORARY TABLE t (col NUMBER(38,0))");

  // When SQL_C_INTERVAL_MINUTE carrying 45 minutes is bound as each integer SQL target
  for (auto sql_type : {SQL_TINYINT, SQL_SMALLINT, SQL_INTEGER, SQL_BIGINT}) {
    conn.execute("CREATE OR REPLACE TEMPORARY TABLE t (col NUMBER(38,0))");
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
    REQUIRE_ODBC(ret, stmt);
    SQL_INTERVAL_STRUCT val = dt_interval(SQL_FALSE, 0, 0, 45, 0, 0);
    SQLLEN ind = sizeof(val);
    bind_and_exec(stmt, SQL_C_INTERVAL_MINUTE, static_cast<SQLSMALLINT>(sql_type), val, ind);

    // Then the minute component is read back unchanged
    auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
    INFO("sql_type=" << sql_type);
    CHECK(get_data<SQL_C_SBIGINT>(fetch_stmt, 1) == 45);
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_SECOND to all exact-numeric SQL targets",
                 "[c_interval][conversion][sql_fixed]") {
  // Given a NUMBER column
  conn.execute("CREATE OR REPLACE TEMPORARY TABLE t (col NUMBER(38,0))");

  // When SQL_C_INTERVAL_SECOND carrying 59 whole seconds is bound as each integer SQL target
  for (auto sql_type : {SQL_TINYINT, SQL_SMALLINT, SQL_INTEGER, SQL_BIGINT}) {
    conn.execute("CREATE OR REPLACE TEMPORARY TABLE t (col NUMBER(38,0))");
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
    REQUIRE_ODBC(ret, stmt);
    SQL_INTERVAL_STRUCT val = dt_interval(SQL_FALSE, 0, 0, 0, 59, 0);
    SQLLEN ind = sizeof(val);
    bind_and_exec(stmt, SQL_C_INTERVAL_SECOND, static_cast<SQLSMALLINT>(sql_type), val, ind);

    // Then the second component is read back unchanged
    auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
    INFO("sql_type=" << sql_type);
    CHECK(get_data<SQL_C_SBIGINT>(fetch_stmt, 1) == 59);
  }
}

// ============================================================================
// EDGE CASE - Negative interval magnitude survives through SQL_BIGINT
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should preserve negative sign when binding SQL_C_INTERVAL_DAY to SQL_BIGINT",
                 "[c_interval][conversion][sql_fixed]") {
  // Given a NUMBER column
  conn.execute("CREATE OR REPLACE TEMPORARY TABLE t (col NUMBER(38,0))");

  // When a negative SQL_C_INTERVAL_DAY (-100 days) is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = dt_interval(SQL_TRUE, 100, 0, 0, 0, 0);
  SQLLEN ind = sizeof(val);
  bind_and_exec(stmt, SQL_C_INTERVAL_DAY, SQL_BIGINT, val, ind);

  // Then the value is read back as -100
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_SBIGINT>(fetch_stmt, 1) == -100);
}

// ============================================================================
// EDGE CASE - Sub-second fraction is truncated toward zero (per Appendix D)
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture,
                 "should truncate sub-second fraction when binding SQL_C_INTERVAL_SECOND to SQL_INTEGER",
                 "[c_interval][conversion][sql_fixed]") {
  // Given a NUMBER column
  conn.execute("CREATE OR REPLACE TEMPORARY TABLE t (col NUMBER(38,0))");

  // When SQL_C_INTERVAL_SECOND carrying 12.500000s is bound as SQL_INTEGER
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  // 500_000 microseconds == .5s — must be truncated toward zero per
  // ODBC Appendix D rule for integer-valued numeric SQL targets.
  SQL_INTERVAL_STRUCT val = dt_interval(SQL_FALSE, 0, 0, 0, 12, 500000);
  SQLLEN ind = sizeof(val);
  bind_and_exec(stmt, SQL_C_INTERVAL_SECOND, SQL_INTEGER, val, ind);

  // Then only the integer part (12) is stored; the .5s is dropped
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_SBIGINT>(fetch_stmt, 1) == 12);
}

// ============================================================================
// EDGE CASE - Large day value through SQL_BIGINT
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_DAY large value to SQL_BIGINT",
                 "[c_interval][conversion][sql_fixed]") {
  // Given a NUMBER column
  conn.execute("CREATE OR REPLACE TEMPORARY TABLE t (col NUMBER(38,0))");

  // When SQL_C_INTERVAL_DAY carrying 1_000_000 days is bound as SQL_BIGINT
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = dt_interval(SQL_FALSE, 1000000, 0, 0, 0, 0);
  SQLLEN ind = sizeof(val);
  bind_and_exec(stmt, SQL_C_INTERVAL_DAY, SQL_BIGINT, val, ind);

  // Then the full magnitude is preserved
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_SBIGINT>(fetch_stmt, 1) == 1000000);
}

// ============================================================================
// NULL HANDLING - SQL_NULL_DATA propagates as SQL NULL
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_YEAR NULL to SQL_INTEGER as SQL NULL",
                 "[c_interval][conversion][sql_fixed]") {
  // Given a NUMBER column
  conn.execute("CREATE OR REPLACE TEMPORARY TABLE t (col NUMBER(38,0))");

  // When SQL_C_INTERVAL_YEAR is bound with SQL_NULL_DATA indicator
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ym_interval(SQL_FALSE, 0, 0);
  SQLLEN ind = SQL_NULL_DATA;
  bind_and_exec(stmt, SQL_C_INTERVAL_YEAR, SQL_INTEGER, val, ind);

  // Then the column reads back as SQL NULL
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  SQLLEN out_ind = 0;
  SQLBIGINT out = 0;
  ret = SQLGetData(fetch_stmt.getHandle(), 1, SQL_C_SBIGINT, &out, sizeof(out), &out_ind);
  REQUIRE_ODBC(ret, fetch_stmt);
  CHECK(out_ind == SQL_NULL_DATA);
}

// ============================================================================
// INCOMPATIBLE CONVERSIONS - Composite intervals must surface SQLSTATE 07006
// ============================================================================
//
// Composite SQL_C_INTERVAL_* types (YEAR_TO_MONTH, DAY_TO_HOUR,
// DAY_TO_MINUTE, DAY_TO_SECOND, HOUR_TO_MINUTE, HOUR_TO_SECOND,
// MINUTE_TO_SECOND) carry more than one datetime field and have no
// single-integer mapping per ODBC Appendix D. The driver MUST reject
// them with SQLSTATE 07006.

TEST_CASE_METHOD(ConnSchemaFixture, "should fail binding composite SQL_C_INTERVAL_* to SQL_INTEGER",
                 "[c_interval][conversion][sql_fixed][incompatible][negative]") {
  // Given a NUMBER column with a prepared INSERT
  conn.execute("CREATE OR REPLACE TEMPORARY TABLE t (col NUMBER(38,0))");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // When each composite interval C type is bound to SQL_INTEGER and executed
  // Then SQL_C_INTERVAL_YEAR_TO_MONTH is rejected with SQLSTATE 07006
  {
    SQL_INTERVAL_STRUCT v = ym_interval(SQL_FALSE, 1, 6);
    SQLLEN ind = sizeof(v);
    check_incompatible_bindparam(stmt, SQL_C_INTERVAL_YEAR_TO_MONTH, SQL_INTEGER, &v, sizeof(v), &ind);
  }
  // And SQL_C_INTERVAL_DAY_TO_HOUR is rejected with SQLSTATE 07006
  {
    SQL_INTERVAL_STRUCT v = dt_interval(SQL_FALSE, 2, 3, 0, 0, 0);
    SQLLEN ind = sizeof(v);
    check_incompatible_bindparam(stmt, SQL_C_INTERVAL_DAY_TO_HOUR, SQL_INTEGER, &v, sizeof(v), &ind);
  }
  // And SQL_C_INTERVAL_DAY_TO_MINUTE is rejected with SQLSTATE 07006
  {
    SQL_INTERVAL_STRUCT v = dt_interval(SQL_FALSE, 2, 3, 4, 0, 0);
    SQLLEN ind = sizeof(v);
    check_incompatible_bindparam(stmt, SQL_C_INTERVAL_DAY_TO_MINUTE, SQL_INTEGER, &v, sizeof(v), &ind);
  }
  // And SQL_C_INTERVAL_DAY_TO_SECOND is rejected with SQLSTATE 07006
  {
    SQL_INTERVAL_STRUCT v = dt_interval(SQL_FALSE, 2, 3, 4, 5, 0);
    SQLLEN ind = sizeof(v);
    check_incompatible_bindparam(stmt, SQL_C_INTERVAL_DAY_TO_SECOND, SQL_INTEGER, &v, sizeof(v), &ind);
  }
  // And SQL_C_INTERVAL_HOUR_TO_MINUTE is rejected with SQLSTATE 07006
  {
    SQL_INTERVAL_STRUCT v = dt_interval(SQL_FALSE, 0, 3, 4, 0, 0);
    SQLLEN ind = sizeof(v);
    check_incompatible_bindparam(stmt, SQL_C_INTERVAL_HOUR_TO_MINUTE, SQL_INTEGER, &v, sizeof(v), &ind);
  }
  // And SQL_C_INTERVAL_HOUR_TO_SECOND is rejected with SQLSTATE 07006
  {
    SQL_INTERVAL_STRUCT v = dt_interval(SQL_FALSE, 0, 3, 4, 5, 0);
    SQLLEN ind = sizeof(v);
    check_incompatible_bindparam(stmt, SQL_C_INTERVAL_HOUR_TO_SECOND, SQL_INTEGER, &v, sizeof(v), &ind);
  }
  // And SQL_C_INTERVAL_MINUTE_TO_SECOND is rejected with SQLSTATE 07006
  {
    SQL_INTERVAL_STRUCT v = dt_interval(SQL_FALSE, 0, 0, 4, 5, 0);
    SQLLEN ind = sizeof(v);
    check_incompatible_bindparam(stmt, SQL_C_INTERVAL_MINUTE_TO_SECOND, SQL_INTEGER, &v, sizeof(v), &ind);
  }
}

// NOTE: A symmetric `composite SQL_C_INTERVAL_* -> SQL_DECIMAL` rejection
// case is intentionally omitted here. SQL_DECIMAL/SQL_NUMERIC dispatch to
// `DecimalParamConverter` (see `odbc/src/conversion/param_binding.rs`),
// which currently surfaces a generic HY000 for unsupported C interval
// sources instead of the spec-mandated 07006 raised by the integer path
// in `SnowflakeNumber::read_odbc`. Once the decimal path is aligned to
// 07006, an analogous TEST_CASE should be added here.
