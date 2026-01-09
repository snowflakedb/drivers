#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"

Connection get_connection() { return Connection(); }

TEST_CASE("should return empty result when query produces no rows", "[empty_result]") {
  // Given Snowflake client is logged in
  auto conn = get_connection();

  // When Query "SELECT 1 WHERE FALSE" is executed
  auto stmt = conn.createStatement();
  const auto sql = "SELECT 1 WHERE FALSE";
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)sql, SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then empty result set is returned
  SQLSMALLINT num_cols;
  ret = SQLNumResultCols(stmt.getHandle(), &num_cols);
  CHECK_ODBC(ret, stmt);
  REQUIRE(num_cols == 1);

  // Verify first fetch returns SQL_NO_DATA (no rows)
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_NO_DATA);
}
