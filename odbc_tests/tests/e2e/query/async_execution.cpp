#include <sql.h>
#include <sqlext.h>

#include <chrono>
#include <string>
#include <thread>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "cross_thread_cancel.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "odbc_matchers.hpp"

namespace {

constexpr int kMaxPollIterations = 300;
constexpr auto kPollInterval = std::chrono::milliseconds(100);

// Long-running query that gives enough time to observe SQL_STILL_EXECUTING.
constexpr const char* kLongQuery = "SELECT COUNT(*) FROM TABLE(GENERATOR(TIMELIMIT => 30))";

// Fast query that should complete nearly instantly.
constexpr const char* kFastQuery = "SELECT 42 AS value";

SQLRETURN poll_until_complete(SQLHSTMT stmt, const char* query) {
  int polls = 0;
  SQLRETURN ret = SQL_STILL_EXECUTING;
  while (ret == SQL_STILL_EXECUTING && ++polls < kMaxPollIterations) {
    std::this_thread::sleep_for(kPollInterval);
    ret = SQLExecDirect(stmt, (SQLCHAR*)query, SQL_NTS);
  }
  return ret;
}

// SQLFetch is also async-capable: with async ON it returns SQL_STILL_EXECUTING.
// Poll by re-calling SQLFetch until it completes.
SQLRETURN poll_fetch(SQLHSTMT stmt) {
  int polls = 0;
  SQLRETURN ret = SQLFetch(stmt);
  while (ret == SQL_STILL_EXECUTING && ++polls < kMaxPollIterations) {
    std::this_thread::sleep_for(kPollInterval);
    ret = SQLFetch(stmt);
  }
  return ret;
}

}  // namespace

// =============================================================================
// ATTRIBUTE SETUP
// =============================================================================

TEST_CASE("should enable async execution on statement", "[query][async]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_ASYNC_ENABLE is set to ON
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);

  // Then the attribute is accepted
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
}

TEST_CASE("should disable async execution on statement", "[query][async]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in with async enabled
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // When SQL_ATTR_ASYNC_ENABLE is set to OFF
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, SQL_ASYNC_ENABLE_OFF, 0);

  // Then the attribute is accepted
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
}

TEST_CASE("should get async enable attribute value after setting it", "[query][async]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_ASYNC_ENABLE is set to ON and then read back
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  SQLULEN value = 0;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, &value, 0, nullptr);

  // Then the value should be SQL_ASYNC_ENABLE_ON
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
  CHECK(value == SQL_ASYNC_ENABLE_ON);
}

TEST_CASE("should reject connection-level async with HY092", "[query][async]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When SQL_ATTR_ASYNC_DBC_FUNCTIONS_ENABLE is set on the connection handle
  const SQLRETURN ret = SQLSetConnectAttr(conn.handleWrapper().getHandle(), SQL_ATTR_ASYNC_DBC_FUNCTIONS_ENABLE,
                                          reinterpret_cast<SQLPOINTER>(SQL_ASYNC_DBC_ENABLE_ON), 0);

  // Then the driver should reject it
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(conn.handleWrapper()) == "HY092");
}

// =============================================================================
// ASYNC EXECUTION (POLLING)
// =============================================================================

TEST_CASE("should return SQL_STILL_EXECUTING for long query", "[query][async]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in with async enabled
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // When a long-running query is executed
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)kLongQuery, SQL_NTS);

  // Then the first call should return SQL_STILL_EXECUTING
  REQUIRE(ret == SQL_STILL_EXECUTING);

  // Cleanup: poll to completion so the statement is usable for teardown
  poll_until_complete(stmt.getHandle(), kLongQuery);
}

TEST_CASE("should complete async execution via polling and retrieve data", "[query][async]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in with async enabled
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // When a query is executed asynchronously
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)kFastQuery, SQL_NTS);

  // Then the first call must return SQL_STILL_EXECUTING
  REQUIRE(ret == SQL_STILL_EXECUTING);

  // And polling eventually completes
  ret = poll_until_complete(stmt.getHandle(), kFastQuery);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // And data should be retrievable (SQLFetch is also async-capable)
  SQLRETURN fetch_ret = poll_fetch(stmt.getHandle());
  REQUIRE_THAT(OdbcResult(fetch_ret, stmt), OdbcMatchers::Succeeded());
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 42);
}

TEST_CASE("should execute and retrieve result set asynchronously", "[query][async]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in with async enabled
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // When a query returning multiple rows is executed asynchronously
  const char* query = "SELECT seq4() AS id FROM TABLE(GENERATOR(ROWCOUNT => 5)) ORDER BY id";
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)query, SQL_NTS);
  REQUIRE(ret == SQL_STILL_EXECUTING);

  ret = poll_until_complete(stmt.getHandle(), query);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // Then all rows should be fetchable after async completion
  // (SQLFetch is also async-capable, so we must poll it too)
  int row_count = 0;
  while (true) {
    SQLRETURN fetch_ret = poll_fetch(stmt.getHandle());
    if (fetch_ret == SQL_NO_DATA) break;
    REQUIRE_THAT(OdbcResult(fetch_ret, stmt), OdbcMatchers::Succeeded());

    auto id = get_data<SQL_C_LONG>(stmt, 1);
    CHECK(id == row_count);
    row_count++;
  }
  CHECK(row_count == 5);
}

// =============================================================================
// ASYNC + STATEMENT REUSE
// =============================================================================

TEST_CASE("should allow re-execution after async completion", "[query][async]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in with async enabled
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // When first async query completes
  const char* query1 = "SELECT 1 AS val";
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)query1, SQL_NTS);
  REQUIRE(ret == SQL_STILL_EXECUTING);
  ret = poll_until_complete(stmt.getHandle(), query1);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // And we close the cursor and execute a second query
  SQLCloseCursor(stmt.getHandle());

  const char* query2 = "SELECT 99 AS val";
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)query2, SQL_NTS);
  REQUIRE(ret == SQL_STILL_EXECUTING);
  ret = poll_until_complete(stmt.getHandle(), query2);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // Then the second query should produce correct results
  SQLRETURN fetch_ret = poll_fetch(stmt.getHandle());
  REQUIRE_THAT(OdbcResult(fetch_ret, stmt), OdbcMatchers::Succeeded());
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 99);
}

TEST_CASE("should allow disabling async after completion", "[query][async]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in with async enabled
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // When an async query completes
  const char* query = "SELECT 1";
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)query, SQL_NTS);
  REQUIRE(ret == SQL_STILL_EXECUTING);
  ret = poll_until_complete(stmt.getHandle(), query);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
  SQLCloseCursor(stmt.getHandle());

  // And async is disabled
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, SQL_ASYNC_ENABLE_OFF, 0);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // Then synchronous execution should work normally
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 77 AS val", SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
  REQUIRE(ret != SQL_STILL_EXECUTING);

  SQLRETURN fetch_ret = SQLFetch(stmt.getHandle());
  REQUIRE_THAT(OdbcResult(fetch_ret, stmt), OdbcMatchers::Succeeded());
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 77);
}

// =============================================================================
// ASYNC PREPARE + EXECUTE
// =============================================================================

TEST_CASE("should prepare asynchronously and poll to completion", "[query][async]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in with async enabled
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // When SQLPrepare is called asynchronously
  // Note: SQLPrepare may complete immediately (spec-valid) since the driver
  // can resolve metadata without a server round-trip for simple queries.
  const char* query = "SELECT ? AS val";
  ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)query, SQL_NTS);
  if (ret == SQL_STILL_EXECUTING) {
    int polls = 0;
    while (ret == SQL_STILL_EXECUTING && ++polls < kMaxPollIterations) {
      std::this_thread::sleep_for(kPollInterval);
      ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)query, SQL_NTS);
    }
  }

  // Then the prepare completes successfully
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
}

TEST_CASE("should execute prepared statement asynchronously", "[query][async]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in with async enabled and a prepared statement
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // Prepare (may complete immediately — spec-valid for simple queries)
  const char* query = "SELECT 123 AS val";
  ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)query, SQL_NTS);
  if (ret == SQL_STILL_EXECUTING) {
    int polls = 0;
    while (ret == SQL_STILL_EXECUTING && ++polls < kMaxPollIterations) {
      std::this_thread::sleep_for(kPollInterval);
      ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)query, SQL_NTS);
    }
  }
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // When SQLExecute is called asynchronously — must go async
  ret = SQLExecute(stmt.getHandle());
  REQUIRE(ret == SQL_STILL_EXECUTING);
  {
    int polls = 0;
    while (ret == SQL_STILL_EXECUTING && ++polls < kMaxPollIterations) {
      std::this_thread::sleep_for(kPollInterval);
      ret = SQLExecute(stmt.getHandle());
    }
  }

  // Then execution completes and results are retrievable
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
  SQLRETURN fetch_ret = poll_fetch(stmt.getHandle());
  REQUIRE_THAT(OdbcResult(fetch_ret, stmt), OdbcMatchers::Succeeded());
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 123);
}

TEST_CASE("should prepare and execute with bound parameters asynchronously", "[query][async]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in with async enabled
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // Prepare (may complete immediately)
  const char* query = "SELECT ? AS val";
  ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)query, SQL_NTS);
  if (ret == SQL_STILL_EXECUTING) {
    int polls = 0;
    while (ret == SQL_STILL_EXECUTING && ++polls < kMaxPollIterations) {
      std::this_thread::sleep_for(kPollInterval);
      ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)query, SQL_NTS);
    }
  }
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // Bind a parameter
  SQLINTEGER param_val = 456;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param_val, 0, &ind);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // When SQLExecute is called asynchronously — must go async
  ret = SQLExecute(stmt.getHandle());
  REQUIRE(ret == SQL_STILL_EXECUTING);
  {
    int polls = 0;
    while (ret == SQL_STILL_EXECUTING && ++polls < kMaxPollIterations) {
      std::this_thread::sleep_for(kPollInterval);
      ret = SQLExecute(stmt.getHandle());
    }
  }
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // Then it should return the bound value
  SQLRETURN fetch_ret = poll_fetch(stmt.getHandle());
  REQUIRE_THAT(OdbcResult(fetch_ret, stmt), OdbcMatchers::Succeeded());
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 456);
}

TEST_CASE("should re-execute prepared statement multiple times asynchronously", "[query][async]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in with async enabled and a prepared statement
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // Prepare once (may complete immediately)
  const char* query = "SELECT ? AS val";
  ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)query, SQL_NTS);
  if (ret == SQL_STILL_EXECUTING) {
    int polls = 0;
    while (ret == SQL_STILL_EXECUTING && ++polls < kMaxPollIterations) {
      std::this_thread::sleep_for(kPollInterval);
      ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)query, SQL_NTS);
    }
  }
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  SQLINTEGER param_val = 0;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param_val, 0, &ind);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // When the prepared statement is executed multiple times with different values
  for (int i = 1; i <= 3; i++) {
    param_val = i * 10;

    // SQLExecute must go async
    ret = SQLExecute(stmt.getHandle());
    REQUIRE(ret == SQL_STILL_EXECUTING);
    {
      int polls = 0;
      while (ret == SQL_STILL_EXECUTING && ++polls < kMaxPollIterations) {
        std::this_thread::sleep_for(kPollInterval);
        ret = SQLExecute(stmt.getHandle());
      }
    }
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

    SQLRETURN fetch_ret = poll_fetch(stmt.getHandle());
    REQUIRE_THAT(OdbcResult(fetch_ret, stmt), OdbcMatchers::Succeeded());

    // Then each execution should return the correct bound value
    CHECK(get_data<SQL_C_LONG>(stmt, 1) == i * 10);

    SQLCloseCursor(stmt.getHandle());
  }
}

// =============================================================================
// ASYNC CANCEL
// =============================================================================

TEST_CASE("should cancel async execution with HY008", "[query][async][cancel]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  SKIP_OLD_DRIVER("BD#34", "Async cancel does not interrupt in-progress operations on reference driver");

  // Given Snowflake client is logged in with async enabled
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // When a long query is started and then cancelled
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)kLongQuery, SQL_NTS);
  REQUIRE(ret == SQL_STILL_EXECUTING);

  SQLRETURN cancel_ret = SQLCancel(stmt.getHandle());
  REQUIRE_THAT(OdbcResult(cancel_ret, stmt), OdbcMatchers::Succeeded());

  // Then polling should eventually return HY008
  SQLRETURN poll_ret = poll_until_complete(stmt.getHandle(), kLongQuery);
  REQUIRE(poll_ret != SQL_STILL_EXECUTING);
  REQUIRE(poll_ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "HY008");
}

// =============================================================================
// CROSS-THREAD CANCEL
// =============================================================================

TEST_CASE("should cancel from another thread with HY008", "[query][async][cross_thread]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a long query is executed on one thread and cancelled from another
  odbc_test::CrossThreadCancel ctx;
  ctx.run(stmt.getHandle(), "SELECT COUNT(*) FROM TABLE(GENERATOR(TIMELIMIT => 60))", std::chrono::seconds(5));

  // Then the execution result should be HY008
  SQLRETURN exec_ret = ctx.exec_result.load();

  OLD_DRIVER_ONLY("BD#47") {
    // Old driver: cancel may return SQL_ERROR with HY008 (non-spec-compliant)
    REQUIRE((ctx.cancel_result == SQL_SUCCESS || ctx.cancel_result == SQL_ERROR));
  }
  NEW_DRIVER_ONLY("BD#47") { REQUIRE_THAT(OdbcResult(ctx.cancel_result, stmt), OdbcMatchers::Succeeded()); }

  REQUIRE(exec_ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "HY008");
}

// =============================================================================
// SQLGETINFO REPORTING
// =============================================================================

TEST_CASE("should report SQL_AM_STATEMENT for SQL_ASYNC_MODE", "[query][async]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When SQLGetInfo is called for SQL_ASYNC_MODE
  SQLUINTEGER mode = 0;
  SQLSMALLINT len = 0;
  SQLRETURN ret = SQLGetInfo(conn.handleWrapper().getHandle(), SQL_ASYNC_MODE, &mode, sizeof(mode), &len);

  // Then it should report statement-level async
  REQUIRE_THAT(OdbcResult(ret, conn.handleWrapper()), OdbcMatchers::Succeeded());
  CHECK(mode == SQL_AM_STATEMENT);
}

TEST_CASE("should report no limit for SQL_MAX_ASYNC_CONCURRENT_STATEMENTS", "[query][async]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();
  // Given Snowflake client is logged in
  Connection conn;

  // When SQLGetInfo is called for SQL_MAX_ASYNC_CONCURRENT_STATEMENTS
  SQLUINTEGER max_stmts = 999;
  SQLSMALLINT len = 0;
  SQLRETURN ret = SQLGetInfo(conn.handleWrapper().getHandle(), SQL_MAX_ASYNC_CONCURRENT_STATEMENTS, &max_stmts,
                             sizeof(max_stmts), &len);

  // Then the call should succeed and report 0 (no driver-imposed limit)
  REQUIRE_THAT(OdbcResult(ret, conn.handleWrapper()), OdbcMatchers::Succeeded());
  CHECK(max_stmts == 0);
}
