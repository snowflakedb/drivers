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
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

namespace {

constexpr int kMaxPollIterations = 300;
constexpr auto kPollInterval = std::chrono::milliseconds(100);

// Long-running query that gives enough time to observe SQL_STILL_EXECUTING.
constexpr const char* kLongQuery = "SELECT COUNT(*) FROM TABLE(GENERATOR(TIMELIMIT => 30))";

// Fast query that should complete nearly instantly.
constexpr const char* kFastQuery = "SELECT 42 AS value";

SQLRETURN poll_exec_direct(SQLHSTMT stmt, const char* query) {
  int polls = 0;
  SQLRETURN ret = SQL_STILL_EXECUTING;
  while (ret == SQL_STILL_EXECUTING && ++polls < kMaxPollIterations) {
    std::this_thread::sleep_for(kPollInterval);
    ret = SQLExecDirect(stmt, sqlchar(query), SQL_NTS);
  }
  return ret;
}

SQLRETURN poll_fetch(SQLHSTMT stmt) {
  int polls = 0;
  SQLRETURN ret = SQLFetch(stmt);
  while (ret == SQL_STILL_EXECUTING && ++polls < kMaxPollIterations) {
    std::this_thread::sleep_for(kPollInterval);
    ret = SQLFetch(stmt);
  }
  return ret;
}

SQLRETURN poll_prepare(SQLHSTMT stmt, const char* query) {
  SQLRETURN ret = SQLPrepare(stmt, sqlchar(query), SQL_NTS);
  int polls = 0;
  while (ret == SQL_STILL_EXECUTING && ++polls < kMaxPollIterations) {
    std::this_thread::sleep_for(kPollInterval);
    ret = SQLPrepare(stmt, sqlchar(query), SQL_NTS);
  }
  return ret;
}

SQLRETURN poll_execute(SQLHSTMT stmt) {
  SQLRETURN ret = SQLExecute(stmt);
  int polls = 0;
  while (ret == SQL_STILL_EXECUTING && ++polls < kMaxPollIterations) {
    std::this_thread::sleep_for(kPollInterval);
    ret = SQLExecute(stmt);
  }
  return ret;
}

}  // namespace

// =============================================================================
// ATTRIBUTE SETUP
// =============================================================================

TEST_CASE("should enable async execution on statement", "[query][async]") {
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
  // Given Snowflake client is logged in with async enabled
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // When SQL_ATTR_ASYNC_ENABLE is set to OFF
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_OFF), 0);

  // Then the attribute is accepted
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
}

TEST_CASE("should get async enable attribute value after setting it", "[query][async]") {
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
  // Given Snowflake client is logged in
  Connection conn;

  // When SQL_ATTR_ASYNC_DBC_FUNCTIONS_ENABLE is set on the connection handle
  const SQLRETURN ret = SQLSetConnectAttr(conn.handleWrapper().getHandle(), SQL_ATTR_ASYNC_DBC_FUNCTIONS_ENABLE,
                                          reinterpret_cast<SQLPOINTER>(SQL_ASYNC_DBC_ENABLE_ON), 0);

  // Then the driver should reject it
  REQUIRE_THAT(OdbcResult(ret, conn.handleWrapper()), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("HY092"));
}

// =============================================================================
// ASYNC EXECUTION (POLLING)
// =============================================================================

// [flaky]: the old reference driver can abort (simba_abort / pthread_mutex_lock)
// when this long-running async query races with parallel ctest workers.
TEST_CASE("should return SQL_STILL_EXECUTING for long query", "[query][async][flaky]") {
  // Given Snowflake client is logged in with async enabled
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // When a long-running query is executed
  ret = SQLExecDirect(stmt.getHandle(), sqlchar(kLongQuery), SQL_NTS);

  // Then the first call should return SQL_STILL_EXECUTING
  REQUIRE(ret == SQL_STILL_EXECUTING);

  // Cleanup: poll to completion so the statement is usable for teardown
  poll_exec_direct(stmt.getHandle(), kLongQuery);
}

TEST_CASE("should complete async execution via polling and retrieve data", "[query][async]") {
  // Given Snowflake client is logged in with async enabled
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // When a query is executed asynchronously
  ret = SQLExecDirect(stmt.getHandle(), sqlchar(kFastQuery), SQL_NTS);

  // Then the first call must return SQL_STILL_EXECUTING
  REQUIRE(ret == SQL_STILL_EXECUTING);

  // And polling eventually completes
  ret = poll_exec_direct(stmt.getHandle(), kFastQuery);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // And data should be retrievable (SQLFetch is also async-capable)
  SQLRETURN fetch_ret = poll_fetch(stmt.getHandle());
  REQUIRE_THAT(OdbcResult(fetch_ret, stmt), OdbcMatchers::Succeeded());
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 42);
}

TEST_CASE("should execute and retrieve result set asynchronously", "[query][async]") {
  // Given Snowflake client is logged in with async enabled
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // When a query returning multiple rows is executed asynchronously
  const char* query = "SELECT seq4() AS id FROM TABLE(GENERATOR(ROWCOUNT => 5)) ORDER BY id";
  ret = SQLExecDirect(stmt.getHandle(), sqlchar(query), SQL_NTS);
  REQUIRE(ret == SQL_STILL_EXECUTING);

  ret = poll_exec_direct(stmt.getHandle(), query);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // Then all rows should be fetchable after async completion
  int row_count = 0;
  while (true) {
    SQLRETURN fetch_ret = poll_fetch(stmt.getHandle());
    if (fetch_ret == SQL_NO_DATA) break;
    CHECK_THAT(OdbcResult(fetch_ret, stmt), OdbcMatchers::Succeeded());

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
  // Given Snowflake client is logged in with async enabled
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // When first async query completes
  const char* query1 = "SELECT 1 AS val";
  ret = SQLExecDirect(stmt.getHandle(), sqlchar(query1), SQL_NTS);
  REQUIRE(ret == SQL_STILL_EXECUTING);
  ret = poll_exec_direct(stmt.getHandle(), query1);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // And we close the cursor and execute a second query
  ret = SQLCloseCursor(stmt.getHandle());
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  const char* query2 = "SELECT 99 AS val";
  ret = SQLExecDirect(stmt.getHandle(), sqlchar(query2), SQL_NTS);
  REQUIRE(ret == SQL_STILL_EXECUTING);
  ret = poll_exec_direct(stmt.getHandle(), query2);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // Then the second query should produce correct results
  SQLRETURN fetch_ret = poll_fetch(stmt.getHandle());
  REQUIRE_THAT(OdbcResult(fetch_ret, stmt), OdbcMatchers::Succeeded());
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 99);
}

TEST_CASE("should allow disabling async after completion", "[query][async]") {
  // Given Snowflake client is logged in with async enabled
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // When an async query completes
  const char* query = "SELECT 1";
  ret = SQLExecDirect(stmt.getHandle(), sqlchar(query), SQL_NTS);
  REQUIRE(ret == SQL_STILL_EXECUTING);
  ret = poll_exec_direct(stmt.getHandle(), query);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
  ret = SQLCloseCursor(stmt.getHandle());
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // And async is disabled
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_OFF), 0);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // Then synchronous execution should work normally
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT 77 AS val"), SQL_NTS);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  SQLRETURN fetch_ret = SQLFetch(stmt.getHandle());
  REQUIRE_THAT(OdbcResult(fetch_ret, stmt), OdbcMatchers::Succeeded());
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 77);
}

// =============================================================================
// ASYNC PREPARE + EXECUTE
// =============================================================================

TEST_CASE("should prepare asynchronously and poll to completion", "[query][async]") {
  // Given Snowflake client is logged in with async enabled
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // When SQLPrepare is called with async enabled
  // Note: SQLPrepare may complete immediately (spec-valid) since the driver
  // can resolve metadata without a server round-trip for simple queries.
  ret = poll_prepare(stmt.getHandle(), "SELECT ? AS val");

  // Then the prepare completes successfully
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
}

TEST_CASE("should execute prepared statement asynchronously", "[query][async]") {
  // Given Snowflake client is logged in with async enabled and a prepared statement
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret;
  IODBC_ONLY {
    ret = poll_prepare(stmt.getHandle(), "SELECT 123 AS val");
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
    ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
  }
  NON_IODBC {
    ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
    ret = poll_prepare(stmt.getHandle(), "SELECT 123 AS val");
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
  }

  // When SQLExecute is called asynchronously — must go async
  ret = SQLExecute(stmt.getHandle());
  REQUIRE(ret == SQL_STILL_EXECUTING);
  ret = poll_execute(stmt.getHandle());

  // Then execution completes and results are retrievable
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
  SQLRETURN fetch_ret = poll_fetch(stmt.getHandle());
  REQUIRE_THAT(OdbcResult(fetch_ret, stmt), OdbcMatchers::Succeeded());
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 123);
}

TEST_CASE("should prepare and execute with bound parameters asynchronously", "[query][async]") {
  // Given Snowflake client is logged in with async enabled
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret;
  IODBC_ONLY {
    ret = poll_prepare(stmt.getHandle(), "SELECT ? AS val");
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
    ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
  }
  NON_IODBC {
    ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
    ret = poll_prepare(stmt.getHandle(), "SELECT ? AS val");
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
  }

  // Bind a parameter
  SQLINTEGER param_val = 456;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param_val, 0, &ind);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // When SQLExecute is called asynchronously — must go async
  ret = SQLExecute(stmt.getHandle());
  REQUIRE(ret == SQL_STILL_EXECUTING);
  ret = poll_execute(stmt.getHandle());
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // Then it should return the bound value
  SQLRETURN fetch_ret = poll_fetch(stmt.getHandle());
  REQUIRE_THAT(OdbcResult(fetch_ret, stmt), OdbcMatchers::Succeeded());
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 456);
}

TEST_CASE("should re-execute prepared statement multiple times asynchronously", "[query][async]") {
  // Given Snowflake client is logged in with async enabled and a prepared statement
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret;
  IODBC_ONLY {
    ret = poll_prepare(stmt.getHandle(), "SELECT ? AS val");
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
    ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
  }
  NON_IODBC {
    ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
    ret = poll_prepare(stmt.getHandle(), "SELECT ? AS val");
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
  }

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
    ret = poll_execute(stmt.getHandle());
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

    SQLRETURN fetch_ret = poll_fetch(stmt.getHandle());
    CHECK_THAT(OdbcResult(fetch_ret, stmt), OdbcMatchers::Succeeded());

    // Then each execution should return the correct bound value
    CHECK(get_data<SQL_C_LONG>(stmt, 1) == i * 10);

    ret = SQLCloseCursor(stmt.getHandle());
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
  }
}

TEST_CASE("should preserve prepared state after async execute and cursor close", "[query][async]") {
  // Given Snowflake client is logged in with async enabled and a prepared SELECT
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret;
  IODBC_ONLY {
    ret = poll_prepare(stmt.getHandle(), "SELECT 1 AS col1, 2 AS col2");
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
    ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
  }
  NON_IODBC {
    ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
    ret = poll_prepare(stmt.getHandle(), "SELECT 1 AS col1, 2 AS col2");
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
  }

  // Verify prepared state reports column count before execution
  SQLSMALLINT num_cols_before = 0;
  ret = SQLNumResultCols(stmt.getHandle(), &num_cols_before);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
  REQUIRE(num_cols_before == 2);

  // When executed asynchronously and cursor is closed
  ret = SQLExecute(stmt.getHandle());
  REQUIRE(ret == SQL_STILL_EXECUTING);
  ret = poll_execute(stmt.getHandle());
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  ret = SQLCloseCursor(stmt.getHandle());
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // Then the statement should still be in Prepared state (not Created),
  // so SQLNumResultCols should still report the column count.
  SQLSMALLINT num_cols_after = 0;
  ret = SQLNumResultCols(stmt.getHandle(), &num_cols_after);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
  NEW_DRIVER_ONLY("BD#58") { CHECK(num_cols_after == 2); }
  OLD_DRIVER_ONLY("BD#58") { CHECK(num_cols_after == 0); }
}

// =============================================================================
// ASYNC CANCEL
// =============================================================================

TEST_CASE("should cancel async execution with HY008", "[query][async][cancel]") {
  SKIP_OLD_DRIVER("BD#32", "Async cancel does not interrupt in-progress operations on reference driver");

  // Given Snowflake client is logged in with async enabled
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // When a long query is started and then cancelled
  ret = SQLExecDirect(stmt.getHandle(), sqlchar(kLongQuery), SQL_NTS);
  REQUIRE(ret == SQL_STILL_EXECUTING);

  SQLRETURN cancel_ret = SQLCancel(stmt.getHandle());
  REQUIRE_THAT(OdbcResult(cancel_ret, stmt), OdbcMatchers::Succeeded());

  // Then polling should eventually return HY008
  SQLRETURN poll_ret = poll_exec_direct(stmt.getHandle(), kLongQuery);
  REQUIRE(poll_ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "HY008");
}

TEST_CASE("should treat SQLCancel on idle async-enabled statement as no-op", "[query][async][cancel]") {
  // Given Snowflake client is logged in with async enabled but no query in progress
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // When SQLCancel is called on an idle statement
  ret = SQLCancel(stmt.getHandle());

  // Then it should succeed with no side effects (ODBC 3.5+ no-op semantics)
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // And the statement should still be usable for execution
  ret = SQLExecDirect(stmt.getHandle(), sqlchar(kFastQuery), SQL_NTS);
  REQUIRE(ret == SQL_STILL_EXECUTING);
  ret = poll_exec_direct(stmt.getHandle(), kFastQuery);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
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

  OLD_DRIVER_ONLY("BD#47") { REQUIRE((ctx.cancel_result == SQL_SUCCESS || ctx.cancel_result == SQL_ERROR)); }
  NEW_DRIVER_ONLY("BD#47") { REQUIRE_THAT(OdbcResult(ctx.cancel_result, stmt), OdbcMatchers::Succeeded()); }

  REQUIRE(exec_ret == SQL_ERROR);
  OLD_IODBC_ONLY("BD#60") {
    // iODBC's DM catches the cross-thread cancel as a function-sequence event
    //   on the busy async statement and surfaces ODBC 2.x "S1010" instead of
    //   the spec-mandated "HY008" the new driver maps inside.
    CHECK(get_sqlstate(stmt) == "S1010");
  }
  else {
    CHECK(get_sqlstate(stmt) == "HY008");
  }
}

// =============================================================================
// ASYNC ERROR PATHS
// =============================================================================

TEST_CASE("should return SQL_ERROR asynchronously for invalid SQL", "[query][async]") {
  // Given Snowflake client is logged in with async enabled
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // When an invalid query is executed asynchronously
  const char* bad_query = "SELCT INVALID SYNTAX GIBBERISH";
  ret = SQLExecDirect(stmt.getHandle(), sqlchar(bad_query), SQL_NTS);
  REQUIRE(ret == SQL_STILL_EXECUTING);

  // Then polling should eventually return SQL_ERROR with a syntax error SQLSTATE
  ret = poll_exec_direct(stmt.getHandle(), bad_query);
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "42000");
}

// [flaky]: the old reference driver can abort (simba_abort / pthread_mutex_lock)
// when a non-permitted call races the still-executing async query under parallel
// ctest workers, crashing the test subprocess. Same class as the long-query async
// case above. Tagged flaky so the blocking reference run is not destabilized.
TEST_CASE("should reject non-permitted function call during async execution", "[query][async][flaky]") {
  // Given Snowflake client is logged in with async enabled and a query in progress
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  ret = SQLExecDirect(stmt.getHandle(), sqlchar(kLongQuery), SQL_NTS);
  REQUIRE(ret == SQL_STILL_EXECUTING);

  // When a non-permitted function is called on the busy statement
  SQLSMALLINT num_cols = 0;
  SQLRETURN bad_ret = SQLNumResultCols(stmt.getHandle(), &num_cols);

  // Then it should return HY010 (function sequence error)
  CHECK(bad_ret == SQL_ERROR);
  IODBC_ONLY {
    // iODBC's DM catches the non-permitted call against the still-executing
    //   async statement and surfaces the ODBC 2.x form "S1010" before the
    //   driver gets to map it to the spec "HY010".
    CHECK(get_sqlstate(stmt) == "S1010");
  }
  else {
    CHECK(get_sqlstate(stmt) == "HY010");
  }

  // Cleanup: poll to completion
  poll_exec_direct(stmt.getHandle(), kLongQuery);
}

TEST_CASE("should clear diagnostic records between async polls", "[query][async]") {
  // Given Snowflake client is logged in with async enabled
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());

  // And a prior query has failed, leaving diagnostics populated
  const char* bad_query = "SELCT INVALID";
  ret = SQLExecDirect(stmt.getHandle(), sqlchar(bad_query), SQL_NTS);
  REQUIRE(ret == SQL_STILL_EXECUTING);
  ret = poll_exec_direct(stmt.getHandle(), bad_query);
  REQUIRE(ret == SQL_ERROR);
  auto error_records = get_diag_rec(SQL_HANDLE_STMT, stmt.getHandle());
  REQUIRE_FALSE(error_records.empty());

  // When a new async query is started
  ret = SQLExecDirect(stmt.getHandle(), sqlchar(kFastQuery), SQL_NTS);
  REQUIRE(ret == SQL_STILL_EXECUTING);

  // Then diagnostic records should be cleared during SQL_STILL_EXECUTING
  auto records = get_diag_rec(SQL_HANDLE_STMT, stmt.getHandle());
  CHECK(records.empty());

  // And after polling to completion, diagnostics reflect the final state
  ret = poll_exec_direct(stmt.getHandle(), kFastQuery);
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded());
}

// =============================================================================
// SQLGETINFO REPORTING
// =============================================================================

TEST_CASE("should report SQL_AM_STATEMENT for SQL_ASYNC_MODE", "[query][async]") {
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

TEST_CASE("should report SQL_ASYNC_DBC_CAPABLE for SQL_ASYNC_DBC_FUNCTIONS", "[query][async]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When SQLGetInfo is called for SQL_ASYNC_DBC_FUNCTIONS
  SQLUINTEGER dbc_funcs = 0;
  SQLSMALLINT len = 0;
  SQLRETURN ret =
      SQLGetInfo(conn.handleWrapper().getHandle(), SQL_ASYNC_DBC_FUNCTIONS, &dbc_funcs, sizeof(dbc_funcs), &len);

  // Then it should report capable (so the DM passes DBC attr calls through)
  REQUIRE_THAT(OdbcResult(ret, conn.handleWrapper()), OdbcMatchers::Succeeded());
  CHECK(dbc_funcs == SQL_ASYNC_DBC_CAPABLE);
}

TEST_CASE("should report SQL_ASYNC_NOTIFICATION_NOT_CAPABLE", "[query][async]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When SQLGetInfo is called for SQL_ASYNC_NOTIFICATION
  SQLUINTEGER notif = 999;
  SQLSMALLINT len = 0;
  SQLRETURN ret = SQLGetInfo(conn.handleWrapper().getHandle(), SQL_ASYNC_NOTIFICATION, &notif, sizeof(notif), &len);

  // Then it should report not capable (polling only, no notification support)
  REQUIRE_THAT(OdbcResult(ret, conn.handleWrapper()), OdbcMatchers::Succeeded());
  CHECK(notif == SQL_ASYNC_NOTIFICATION_NOT_CAPABLE);
}
