#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "get_diag_rec.hpp"

// =============================================================================
// QUERY TIMEOUT E2E TESTS
// =============================================================================

TEST_CASE("query completes within timeout", "[query][query_timeout][e2e]") {
  // Given a connection with query timeout set
  Connection conn;
  auto stmt = conn.createStatement();

  // Set a generous timeout (60s) that won't fire for a simple query
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_QUERY_TIMEOUT, reinterpret_cast<SQLPOINTER>(60), SQL_IS_UINTEGER);
  REQUIRE(ret == SQL_SUCCESS);

  // When executing a fast query
  ret = SQLExecDirect(stmt.getHandle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT 1")), SQL_NTS);
  // Then it should succeed
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE("query timeout triggers for long-running query", "[query][query_timeout][e2e]") {
  // Given a connection with a very short timeout
  Connection conn;
  auto stmt = conn.createStatement();

  // Set a 1-second timeout
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_QUERY_TIMEOUT, reinterpret_cast<SQLPOINTER>(1), SQL_IS_UINTEGER);
  REQUIRE(ret == SQL_SUCCESS);

  // When executing a query that takes longer than the timeout
  ret = SQLExecDirect(stmt.getHandle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("CALL SYSTEM$WAIT(10)")), SQL_NTS);
  // Then it should fail with an error
  REQUIRE(ret == SQL_ERROR);
}

TEST_CASE("query timeout zero means no timeout", "[query][query_timeout][e2e]") {
  // Given a connection with timeout explicitly set to 0 (disabled)
  Connection conn;
  auto stmt = conn.createStatement();

  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_QUERY_TIMEOUT, reinterpret_cast<SQLPOINTER>(0), SQL_IS_UINTEGER);
  REQUIRE(ret == SQL_SUCCESS);

  // When executing a query
  ret = SQLExecDirect(stmt.getHandle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT 1")), SQL_NTS);
  // Then it should succeed (no timeout applied)
  REQUIRE(ret == SQL_SUCCESS);
}
