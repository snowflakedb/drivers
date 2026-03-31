#include <sql.h>
#include <sqlext.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("should return IM001 when SQLBulkOperations is called", "[query][sqlbulkoperations]") {
  // Given A query is executed and a row is fetched
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT 42 AS value");

  // When SQLBulkOperations is called
  SQLRETURN ret = SQLBulkOperations(stmt.getHandle(), SQL_ADD);

  // Then The driver should report that it does not support this function
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("IM001"));
}
