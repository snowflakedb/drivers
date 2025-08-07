#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"

TEST_CASE("Test large single column integer set", "[large_result_tests]") {
  // This is a placeholder for the actual test logic.
  Connection conn;
  auto stmt = conn.createStatement();
  const auto sql =
      "SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => "
      "1000000)) v ORDER BY id";
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)sql, SQL_NTS);
  CHECK_ODBC(ret, stmt);

  SQLSMALLINT num_cols;
  ret = SQLNumResultCols(stmt.getHandle(), &num_cols);
  CHECK_ODBC(ret, stmt);
  REQUIRE(num_cols == 1);

  int row_index = 0;
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) {
      break;  // No more data
    }
    CHECK_ODBC(ret, stmt);
    SQLINTEGER result = 0;
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &result, sizeof(result), NULL);
    CHECK_ODBC(ret, stmt);
    REQUIRE(result == row_index);
    row_index++;
  }
  REQUIRE(true);
}
