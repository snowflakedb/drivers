#include <sql.h>
#include <sqlext.h>

#include <atomic>
#include <thread>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "WiremockClient.hpp"
#include "compatibility.hpp"
#include "odbc_matchers.hpp"

/// Connect directly to a WireMock instance without using the RAII Connection
/// class, whose destructor calls SQLDisconnect.  For logout tests we need to
/// control exactly when and how many times SQLDisconnect is called.
static ConnectionHandleWrapper connect_to_wiremock(EnvironmentHandleWrapper& env, const WiremockClient& wm) {
  ConnectionHandleWrapper dbc = env.createConnectionHandle();
  auto conn_str = get_wiremock_connection_string(wm);
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), nullptr, (SQLCHAR*)conn_str.c_str(), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_NOPROMPT);
  REQUIRE_ODBC(ret, dbc);
  return dbc;
}

TEST_CASE("should be idempotent when close called multiple times", "[session][logout]") {
  SKIP_OLD_DRIVER("BD#000", "Old driver does not support WireMock-based logout testing");

  WiremockClient wm;
  wm.add_mapping_file("auth/login_success_jwt.json");
  wm.add_mapping_file("session/logout_success.json");

  // Given Snowflake client is logged in
  auto env = Connection::initEnv();
  auto dbc = connect_to_wiremock(env, wm);

  // When Connection is closed
  SQLRETURN first_ret = SQLDisconnect(dbc.getHandle());
  REQUIRE_ODBC(first_ret, dbc);

  // And Connection is closed again
  SQLDisconnect(dbc.getHandle());  // ODBC spec: returns SQL_ERROR/08003 — expected

  // And Connection is closed a third time
  SQLDisconnect(dbc.getHandle());  // ODBC spec: returns SQL_ERROR/08003 — expected

  // Then Only one logout request is sent
  CHECK(wm.get_request_count("POST", "/session") == 1);

  // Per ODBC spec (08003), SQLDisconnect on an already-disconnected handle returns
  // SQL_ERROR — not a bug.  The first close is the meaningful one; verify it succeeded.
  // And No errors are thrown
  CHECK(first_ret == SQL_SUCCESS);
}

TEST_CASE("should handle concurrent close calls safely", "[session][logout]") {
  SKIP_OLD_DRIVER("BD#000", "Old driver does not support WireMock-based logout testing");

  WiremockClient wm;
  wm.add_mapping_file("auth/login_success_jwt.json");
  wm.add_mapping_file("session/logout_success.json");

  // Given Snowflake client is logged in
  auto env = Connection::initEnv();
  auto dbc = connect_to_wiremock(env, wm);

  constexpr int num_threads = 5;
  std::vector<SQLRETURN> results(num_threads, SQL_ERROR);
  std::atomic<int> ready_count{0};

  // When Connection is closed from multiple threads concurrently
  std::vector<std::thread> threads;
  for (int i = 0; i < num_threads; ++i) {
    threads.emplace_back([&, i]() {
      // Spin-wait until all threads are ready, then call SQLDisconnect simultaneously.
      ready_count.fetch_add(1, std::memory_order_release);
      while (ready_count.load(std::memory_order_acquire) < num_threads) {
      }
      results[i] = SQLDisconnect(dbc.getHandle());
    });
  }
  for (auto& t : threads)
    t.join();

  // Then Only one logout request is sent
  CHECK(wm.get_request_count("POST", "/session") == 1);

  // Per ODBC spec: exactly one thread gets SQL_SUCCESS; others get SQL_ERROR/08003
  // (connection already closed).  All threads returned — no crash, no hang.
  // And All close calls return successfully
  int success_count = 0;
  for (auto r : results) {
    if (r == SQL_SUCCESS) success_count++;
  }
  CHECK(success_count == 1);
}
