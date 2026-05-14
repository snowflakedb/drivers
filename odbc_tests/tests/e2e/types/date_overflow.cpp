#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include "Connection.hpp"
#include "overflow_helpers.hpp"

// SQL DATE values are bounded to year 0001..9999. Pushing ADD_MONTHS far
// outside that range (~88,000 years here) lets the server return a
// wrapped/sentinel day-count on the wire; the driver must surface this as
// a datetime-field-overflow error rather than render the wrapped value as
// if it were a real date.
namespace {
constexpr const char* kAddMonthsOverflowQuery =
    "WITH t(c_int4, c_date) AS (SELECT 128000, DATE '2001-09-08') "
    "SELECT ADD_MONTHS(c_date, c_int4 - 1184021) FROM t";
}

TEST_CASE("ADD_MONTHS DATE overflow surfaces a datetime overflow error", "[date][overflow][22007]") {
  // Given a connection
  Connection conn;
  auto stmt = conn.createStatement();

  // When the overflow query is run end-to-end (exec -> fetch -> SQL_C_CHAR get)
  auto result = run_overflow_query(stmt, kAddMonthsOverflowQuery);

  // Then the driver reports SQL_ERROR with SQLSTATE 22007 (invalid datetime format)
  // matching the legacy ODBC driver, which detects the out-of-SQL-range value at SQLGetData time
  REQUIRE(result.ret == SQL_ERROR);
  CHECK(result.sqlstate == "22007");
}
