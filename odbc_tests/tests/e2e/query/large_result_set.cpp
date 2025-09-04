#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"

class LargeResultSetTest {
 public:
  Connection get_connection() { return Connection(); }

  StatementHandleWrapper execute_large_result_query(Connection& conn) {
    auto stmt = conn.createStatement();
    const auto sql = "SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 1000000)) v ORDER BY id";
    SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)sql, SQL_NTS);
    CHECK_ODBC(ret, stmt);
    return stmt;
  }

  void verify_column_count(StatementHandleWrapper& stmt, int expected_count) {
    SQLSMALLINT num_cols;
    SQLRETURN ret = SQLNumResultCols(stmt.getHandle(), &num_cols);
    CHECK_ODBC(ret, stmt);
    REQUIRE(num_cols == expected_count);
  }

  void verify_row_count_and_sequential_numbering(StatementHandleWrapper& stmt,
                                                 int expected_row_count) {
    int row_count = 0;
    int expected_value = 0;

    while (true) {
      SQLRETURN ret = SQLFetch(stmt.getHandle());
      if (ret == SQL_NO_DATA) {
        break;  // No more data
      }
      CHECK_ODBC(ret, stmt);

      SQLINTEGER result = 0;
      ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &result, sizeof(result), NULL);
      CHECK_ODBC(ret, stmt);

      // Verify sequential numbering
      REQUIRE(result == expected_value);
      expected_value++;
      row_count++;
    }

    // Verify we got exactly the expected number of rows
    REQUIRE(row_count == expected_row_count);
  }
};

TEST_CASE("should process one million row result set", "[large_result_set]") {
  LargeResultSetTest test;

  // Given Snowflake client is logged in
  auto conn = test.get_connection();

  // When Query "SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 1000000)) v ORDER BY id" is
  // executed
  auto stmt = test.execute_large_result_query(conn);

  // Then there are 1000000 numbered sequentially rows returned
  SQLSMALLINT num_cols;
  SQLRETURN ret = SQLNumResultCols(stmt.getHandle(), &num_cols);
  CHECK_ODBC(ret, stmt);
  REQUIRE(num_cols == 1);
  test.verify_row_count_and_sequential_numbering(stmt, 1000000);
}
