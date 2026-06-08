#include <sql.h>
#include <sqlext.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("should return IM001 when SQLSetPos is called", "[query][sqlsetpos]") {
  // Given an executed statement positioned on a row
  Connection conn;
  auto stmt = conn.execute_fetch("SELECT 42 AS value");

  // When SQLSetPos is called on that statement
  SQLRETURN ret = SQLSetPos(stmt.getHandle(), 1, SQL_POSITION, SQL_LOCK_NO_CHANGE);

  // Then SQLSetPos fails under every DM
  NON_IODBC {
    // And the DM forwards the call and surfaces the
    //   driver's SQLSTATE IM001 (driver does not support this function)
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("IM001"));
  }
  IODBC_ONLY {
    // And the DM rejects the call before it reaches the driver. DM
    //   checks the driver's SQLFunctions bitmap, sees SQLSetPos is unsupported,
    //   and short-circuits with SQL_INVALID_HANDLE
    REQUIRE((ret == SQL_ERROR || ret == SQL_INVALID_HANDLE));
  }
}
