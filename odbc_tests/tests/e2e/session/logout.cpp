#include <sql.h>
#include <sqlext.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_matchers.hpp"

// WiremockClient is POSIX-only (fork/exec); on Windows these tests are skipped.
#ifndef _WIN32
#include <atomic>
#include <chrono>
#include <thread>
#include <vector>

#include "WiremockClient.hpp"
#endif

/// Connect directly to a WireMock instance without using the RAII Connection
/// class, whose destructor calls SQLDisconnect.  For logout tests we need to
/// control exactly when and how many times SQLDisconnect is called.
#ifndef _WIN32
static ConnectionHandleWrapper connect_to_wiremock(EnvironmentHandleWrapper& env, const WiremockClient& wm) {
  ConnectionHandleWrapper dbc = env.createConnectionHandle();
  auto conn_str = get_wiremock_connection_string(wm);
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), nullptr, (SQLCHAR*)conn_str.c_str(), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_NOPROMPT);
  REQUIRE_ODBC(ret, dbc);
  return dbc;
}
#endif

TEST_CASE("should be idempotent when close called multiple times", "[session][logout]") {
  SKIP_OLD_DRIVER("BD#000", "Old driver does not support WireMock-based logout testing");
#ifdef _WIN32
  SKIP("WireMock subprocess requires POSIX (fork/exec)");
#else

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
  SQLRETURN second_ret = SQLDisconnect(dbc.getHandle());
  CHECK(second_ret == SQL_ERROR);
  CHECK(get_sqlstate(dbc) == "08003");

  // And Connection is closed a third time
  SQLRETURN third_ret = SQLDisconnect(dbc.getHandle());
  CHECK(third_ret == SQL_ERROR);
  CHECK(get_sqlstate(dbc) == "08003");

  // Then Only one logout request is sent
  CHECK(wm.get_request_count("POST", "/session") == 1);

  // The only errors from repeated disconnect are 08003 (connection not open),
  // which is the ODBC-mandated response — not an application error.
  // And No errors are thrown
  CHECK(get_sqlstate(dbc) == "08003");

#endif  // !_WIN32
}

TEST_CASE("should handle concurrent close calls safely", "[session][logout]") {
  SKIP_OLD_DRIVER("BD#000", "Old driver does not support WireMock-based logout testing");
#ifdef _WIN32
  SKIP("WireMock subprocess requires POSIX (fork/exec)");
#else

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
      ready_count.fetch_add(1, std::memory_order_release);
      // Timeout prevents infinite hang on slow CI runners.
      auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(10);
      while (ready_count.load(std::memory_order_acquire) < num_threads) {
        if (std::chrono::steady_clock::now() > deadline) break;
      }
      results[i] = SQLDisconnect(dbc.getHandle());
    });
  }
  for (auto& t : threads)
    t.join();

  // Then Only one logout request is sent
  CHECK(wm.get_request_count("POST", "/session") == 1);

  // "All return successfully" in ODBC: every thread returned a valid ODBC code
  // (no hang, no crash). One got SQL_SUCCESS (logout sent), rest got SQL_ERROR/08003.
  // And All close calls return successfully
  int success_count = 0;
  int expected_error_count = 0;
  for (auto r : results) {
    if (r == SQL_SUCCESS) {
      success_count++;
    } else if (r == SQL_ERROR) {
      expected_error_count++;
    }
  }
  CHECK(success_count + expected_error_count == num_threads);
  CHECK(success_count == 1);

#endif  // !_WIN32
}
