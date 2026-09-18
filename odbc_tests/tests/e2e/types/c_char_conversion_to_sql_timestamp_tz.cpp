// ODBC E2E: SQL_C_CHAR and SQL_C_WCHAR bound via SQLBindParameter with
// ParameterType SQL_SF_TIMESTAMP_TZ (2001), exercising both the
// space-separated and ISO 8601 'T'-separated date-time separator variants,
// with and without a UTC-offset suffix.
//
// `parse_tz_string_with_fallback` is the code path under test. It fires
// when ValueType=SQL_C_CHAR or SQL_C_WCHAR and ParameterType=SQL_SF_TIMESTAMP_TZ.
// The legacy 3.16.0 driver rejects all SQL_C_CHAR/SQL_C_WCHAR ->
// SQL_SF_TIMESTAMP_TZ bindings with HY000/40620 (BD#51); the new driver
// parses both separator styles and stores the timestamp with the original
// offset. The T-separator variants additionally document SNOW-3990013, the
// fix that added them to the fallback format list; that fix is folded into
// BD#51 (see BehaviorDifferences.yaml) rather than tracked as a separate BD.
//
// Each test uses NEW_DRIVER_ONLY / OLD_DRIVER_ONLY so old-driver behavior
// (HY000 at SQLExecute) is asserted directly rather than skipped.

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstddef>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>
#include <catch2/generators/catch_generators.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"
#include "snowflake_odbc_constants.hpp"

// ============================================================================
// SQL_C_CHAR — space-separated and ISO 8601 T-separated variants
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR TIMESTAMP_TZ string via SQL_SF_TIMESTAMP_TZ",
                 "[c_char][conversion][sql_timestamp_tz]") {
  auto [ts_in, scale, fmt, expected] =
      GENERATE(Catch::Generators::table<std::string, SQLSMALLINT, std::string, std::string>({
          // space-separated — already accepted before SNOW-3990013 fix
          {"2017-11-30 18:17:05.123 +08:00", 9, "YYYY-MM-DD HH24:MI:SS.FF3 TZH:TZM", "2017-11-30 18:17:05.123 +08:00"},
          {"2017-11-30 18:17:05.123", 9, "YYYY-MM-DD HH24:MI:SS.FF3 TZH:TZM", "2017-11-30 18:17:05.123 Z"},
          // ISO 8601 T-separated — SNOW-3990013
          {"2017-11-30T18:17:05.123+08:00", 9, "YYYY-MM-DD HH24:MI:SS.FF3 TZH:TZM", "2017-11-30 18:17:05.123 +08:00"},
          {"2017-11-30T18:17:05.123 +08:00", 9, "YYYY-MM-DD HH24:MI:SS.FF3 TZH:TZM", "2017-11-30 18:17:05.123 +08:00"},
          {"2017-11-30T18:17:05+08:00", 0, "YYYY-MM-DD HH24:MI:SS TZH:TZM", "2017-11-30 18:17:05 +08:00"},
          {"2017-11-30T18:17:05.123", 9, "YYYY-MM-DD HH24:MI:SS.FF3 TZH:TZM", "2017-11-30 18:17:05.123 Z"},
      }));
  CAPTURE(ts_in);

  // Given Snowflake client is logged in and a temporary TIMESTAMP_TZ(9) table
  conn.execute("CREATE TEMPORARY TABLE ts_tz_char (ts TIMESTAMP_TZ(9))");

  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO ts_tz_char VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // When the SQL_C_CHAR string is bound with ParameterType SQL_SF_TIMESTAMP_TZ and executed
  SQLLEN ind = static_cast<SQLLEN>(ts_in.size());
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_SF_TIMESTAMP_TZ, 35, scale,
                         const_cast<char*>(ts_in.data()), static_cast<SQLLEN>(ts_in.size()), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then the new driver parses and stores the timestamp; the legacy driver rejects with HY000/40620
  NEW_DRIVER_ONLY("BD#51") {
    REQUIRE_ODBC(ret, stmt);
    auto str_stmt = conn.execute_fetch("SELECT TO_VARCHAR(ts, '" + fmt + "') FROM ts_tz_char");
    CHECK(get_data<SQL_C_CHAR>(str_stmt, 1) == expected);
  }
  OLD_DRIVER_ONLY("BD#51") {
    REQUIRE_THAT(OdbcResult(ret, stmt),
                 OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("HY000") && OdbcMatchers::HasNativeError(40620));
  }
}

// ============================================================================
// SQL_C_WCHAR — exercises the same parse_tz_string_with_fallback via wchar path
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_WCHAR TIMESTAMP_TZ string via SQL_SF_TIMESTAMP_TZ",
                 "[c_char][conversion][sql_timestamp_tz]") {
  auto [ts_in, expected] = GENERATE(Catch::Generators::table<std::string, std::string>({
      {"2017-11-30 18:17:05.123 +08:00", "2017-11-30 18:17:05.123 +08:00"},
      {"2017-11-30T18:17:05.123+08:00", "2017-11-30 18:17:05.123 +08:00"},
  }));
  CAPTURE(ts_in);

  // Given Snowflake client is logged in and a temporary TIMESTAMP_TZ(9) table
  conn.execute("CREATE TEMPORARY TABLE ts_tz_char (ts TIMESTAMP_TZ(9))");

  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO ts_tz_char VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // When the SQL_C_WCHAR string is bound with ParameterType SQL_SF_TIMESTAMP_TZ and executed;
  // the buffer is built from the ASCII input by widening each byte to SQLWCHAR
  std::vector<SQLWCHAR> wbuf(ts_in.size() + 1, 0);
  for (std::size_t i = 0; i < ts_in.size(); ++i) {
    wbuf[i] = static_cast<SQLWCHAR>(ts_in[i]);
  }
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_WCHAR, SQL_SF_TIMESTAMP_TZ, 35, 9, wbuf.data(),
                         static_cast<SQLLEN>(wbuf.size() * sizeof(SQLWCHAR)), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then the new driver parses and stores the timestamp; the legacy driver rejects with HY000/40620
  NEW_DRIVER_ONLY("BD#51") {
    REQUIRE_ODBC(ret, stmt);
    auto str_stmt = conn.execute_fetch("SELECT TO_VARCHAR(ts, 'YYYY-MM-DD HH24:MI:SS.FF3 TZH:TZM') FROM ts_tz_char");
    CHECK(get_data<SQL_C_CHAR>(str_stmt, 1) == expected);
  }
  OLD_DRIVER_ONLY("BD#51") {
    REQUIRE_THAT(OdbcResult(ret, stmt),
                 OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("HY000") && OdbcMatchers::HasNativeError(40620));
  }
}
