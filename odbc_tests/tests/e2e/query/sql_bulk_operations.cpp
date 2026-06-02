#include <sql.h>
#include <sqlext.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("should return IM001 when SQLBulkOperations is called", "[query][sqlbulkoperations]") {
  // Given an executed statement positioned on a row (Snowflake's driver does
  //   not implement SQLBulkOperations)
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT 42 AS value");

  // When SQLBulkOperations is called on that statement
  SQLRETURN ret = SQLBulkOperations(stmt.getHandle(), SQL_ADD);

  // Then SQLBulkOperations fails under every DM
  NON_IODBC {
    // And the DM forwards the call and surfaces the
    //   driver's SQLSTATE IM001 (driver does not support this function)
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("IM001"));
  }
  IODBC_ONLY {
    // And the DM rejects the call before it reaches the driver. DM
    //   checks the driver's SQLFunctions bitmap, sees SQLBulkOperations is
    //   unsupported, and short-circuits with SQL_INVALID_HANDLE rather than
    //   the SQL_ERROR that the matcher's IsError() expects, so only the
    //   failure (not the specific return code or SQLSTATE) is asserted
    REQUIRE((ret == SQL_ERROR || ret == SQL_INVALID_HANDLE));
  }
}
