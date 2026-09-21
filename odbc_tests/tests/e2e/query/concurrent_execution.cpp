#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <memory>
#include <string>
#include <thread>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "odbc_cast.hpp"

constexpr int kConcurrency = 8;

struct MarkerResult {
  SQLRETURN exec = SQL_ERROR;
  SQLRETURN fetch = SQL_ERROR;
  SQLINTEGER value = -1;
};

void execute_marker(SQLHSTMT stmt, int marker, MarkerResult& out) {
  std::string sql = "SELECT " + std::to_string(marker) + " AS marker";
  out.exec = SQLExecDirect(stmt, sqlchar(sql.c_str()), SQL_NTS);
  if (!SQL_SUCCEEDED(out.exec)) {
    return;
  }
  out.fetch = SQLFetch(stmt);
  if (!SQL_SUCCEEDED(out.fetch)) {
    return;
  }
  SQLLEN indicator = 0;
  out.value = -1;
  const SQLRETURN get_ret = SQLGetData(stmt, 1, SQL_C_LONG, &out.value, sizeof(out.value), &indicator);
  if (!SQL_SUCCEEDED(get_ret) || indicator == SQL_NULL_DATA) {
    out.value = -1;
  }
}

TEST_CASE("should return independent correct results for overlapping queries on one connection",
          "[query][concurrent_execution]") {
  // Given Snowflake client is logged in
  Connection conn;
  std::vector<StatementHandleWrapper> stmts;
  stmts.reserve(kConcurrency);
  for (int i = 0; i < kConcurrency; ++i) {
    stmts.push_back(conn.createStatement());
  }

  // When 8 queries with distinct markers are executed concurrently on the same connection
  std::vector<MarkerResult> results(kConcurrency);
  std::vector<std::thread> threads;
  threads.reserve(kConcurrency);
  for (int i = 0; i < kConcurrency; ++i) {
    threads.emplace_back([&stmts, &results, i]() { execute_marker(stmts[i].getHandle(), i, results[i]); });
  }
  for (auto& thread : threads) {
    thread.join();
  }

  // Then each query returns its own marker
  for (int i = 0; i < kConcurrency; ++i) {
    INFO("marker=" << i);
    REQUIRE(SQL_SUCCEEDED(results[i].exec));
    REQUIRE(SQL_SUCCEEDED(results[i].fetch));
    CHECK(results[i].value == i);
  }
}

TEST_CASE("should return independent correct results for overlapping queries on distinct connections",
          "[query][concurrent_execution]") {
  // Given 8 Snowflake clients are logged in
  std::vector<std::unique_ptr<Connection>> conns;
  conns.reserve(kConcurrency);
  for (int i = 0; i < kConcurrency; ++i) {
    conns.push_back(std::make_unique<Connection>());
  }
  std::vector<StatementHandleWrapper> stmts;
  stmts.reserve(kConcurrency);
  for (int i = 0; i < kConcurrency; ++i) {
    stmts.push_back(conns[i]->createStatement());
  }

  // When 8 queries with distinct markers are executed concurrently on distinct connections
  std::vector<MarkerResult> results(kConcurrency);
  std::vector<std::thread> threads;
  threads.reserve(kConcurrency);
  for (int i = 0; i < kConcurrency; ++i) {
    threads.emplace_back([&stmts, &results, i]() { execute_marker(stmts[i].getHandle(), i, results[i]); });
  }
  for (auto& thread : threads) {
    thread.join();
  }

  // Then each query returns its own marker
  for (int i = 0; i < kConcurrency; ++i) {
    INFO("marker=" << i);
    REQUIRE(SQL_SUCCEEDED(results[i].exec));
    REQUIRE(SQL_SUCCEEDED(results[i].fetch));
    CHECK(results[i].value == i);
  }
}
