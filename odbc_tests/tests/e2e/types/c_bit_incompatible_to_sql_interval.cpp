// ODBC E2E: SQL_C_BIT bound via SQLBindParameter to a SQL_INTERVAL_* parameter
// type must be rejected with SQLSTATE 07006 ("Restricted data type attribute
// violation").
//
// Per ODBC Appendix D, the dedicated "C to SQL: Bit" conversion table lists no
// interval targets for SQL_C_BIT. This is unlike the exact numeric C types,
// which "C to SQL: Numeric" permits for single-field interval targets (covered
// by c_integer_conversion_to_sql_interval / c_numeric_conversion_to_sql_interval).
//
// The exhaustive per-code rejection (all 13 interval type codes, single-field
// AND compound) is pinned by the Rust unit test bit_to_interval_rejected_07006
// in interval.rs. This suite is a minimal end-to-end regression guard that the
// rejection actually surfaces through the DM: one single-field target (YEAR,
// year-month subtype) and one compound target (DAY_TO_SECOND, day-time subtype).

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

namespace {

void reject_bit_to_interval(Connection& conn, SQLSMALLINT interval_type) {
  // Given a VARCHAR column (sufficient to exercise the bound interval parameter type)
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQLCHAR val = 1;
  SQLLEN ind = 0;
  // When SQL_C_BIT is bound to an interval target
  // Then the driver rejects the conversion with SQLSTATE 07006
  check_incompatible_bindparam(stmt, SQL_C_BIT, interval_type, &val, 0, &ind);
}

}  // namespace

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_BIT bound to SQL_INTERVAL_YEAR",
                 "[c_bit][incompatible][sql_interval]") {
  SKIP_OLD_DRIVER(
      "BD#72",
      "Reference driver does not implement SQL_INTERVAL_* parameter binding, so it does not return the spec 07006 "
      "for SQL_C_BIT -> interval");
  // Given a SQL_C_BIT source and a single-field year-month interval target
  // When SQL_C_BIT is bound to SQL_INTERVAL_YEAR and executed
  // Then the driver rejects the conversion with SQLSTATE 07006
  reject_bit_to_interval(conn, SQL_INTERVAL_YEAR);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_BIT bound to SQL_INTERVAL_DAY_TO_SECOND",
                 "[c_bit][incompatible][sql_interval]") {
  SKIP_OLD_DRIVER(
      "BD#72",
      "Reference driver does not implement SQL_INTERVAL_* parameter binding, so it does not return the spec 07006 "
      "for SQL_C_BIT -> interval");
  // Given a SQL_C_BIT source and a compound day-time interval target
  // When SQL_C_BIT is bound to SQL_INTERVAL_DAY_TO_SECOND and executed
  // Then the driver rejects the conversion with SQLSTATE 07006
  reject_bit_to_interval(conn, SQL_INTERVAL_DAY_TO_SECOND);
}
