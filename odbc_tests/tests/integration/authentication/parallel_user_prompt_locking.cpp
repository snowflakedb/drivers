#include <sql.h>
#include <sqlext.h>

#include <algorithm>
#include <atomic>
#include <chrono>
#include <mutex>
#include <sstream>
#include <string>
#include <thread>

#ifndef _WIN32
#include <arpa/inet.h>
#endif

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_all.hpp>

#include "Connection.hpp"
#include "EnvOverride.hpp"
#include "HandleWrapper.hpp"
#include "WiremockClient.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"
#include "platform.hpp"
#include "test_setup.hpp"

using Catch::Matchers::ContainsSubstring;

// =============================================================================
// Helpers
// =============================================================================

static std::string eb_conn_str_with_caching(const WiremockClient& wm) {
  std::ostringstream ss;
  configure_driver_string(ss);
  ss << "SERVER=localhost;";
  ss << "PORT=" << wm.port() << ";";
  ss << "ACCOUNT=testaccount;";
  ss << "UID=eb_lock_user;";
  ss << "AUTHENTICATOR=EXTERNALBROWSER;";
  ss << "CLIENT_STORE_TEMPORARY_CREDENTIAL=true;";
  ss << "SSL=off;";
  ss << "DisableOCSPCheck=true;";
  return ss.str();
}

static std::string eb_conn_str_with_locking_disabled(const WiremockClient& wm) {
  std::ostringstream ss;
  configure_driver_string(ss);
  ss << "SERVER=localhost;";
  ss << "PORT=" << wm.port() << ";";
  ss << "ACCOUNT=testaccount;";
  ss << "UID=eb_nolock_user;";
  ss << "AUTHENTICATOR=EXTERNALBROWSER;";
  ss << "CLIENT_STORE_TEMPORARY_CREDENTIAL=true;";
  ss << "DISABLE_PARALLEL_USER_PROMPT=false;";
  ss << "SSL=off;";
  ss << "DisableOCSPCheck=true;";
  return ss.str();
}

static std::string eb_conn_str_no_caching(const WiremockClient& wm) {
  std::ostringstream ss;
  configure_driver_string(ss);
  ss << "SERVER=localhost;";
  ss << "PORT=" << wm.port() << ";";
  ss << "ACCOUNT=testaccount;";
  ss << "UID=eb_nocache_user;";
  ss << "AUTHENTICATOR=EXTERNALBROWSER;";
  // CLIENT_STORE_TEMPORARY_CREDENTIAL intentionally not set → locking disabled
  ss << "SSL=off;";
  ss << "DisableOCSPCheck=true;";
  return ss.str();
}

/// Poll WireMock for the n-th authenticator-request (0-indexed), extract the
/// redirect port from its body, then send a fake token to that loopback
/// callback listener.  Using the index lets caller threads route each callback
/// to the correct connection's listener when multiple concurrent connections
/// each bind their own loopback port.
// wm_mutex serializes concurrent find_requests calls when multiple callback
// threads share the same WiremockClient.  Pass nullptr for single-threaded use.
static void simulate_browser_callback_nth(const WiremockClient& wm, const std::string& token, size_t n,
                                          std::mutex* wm_mutex = nullptr, int timeout_ms = 15000) {
  auto deadline = std::chrono::steady_clock::now() + std::chrono::milliseconds(timeout_ms);
  while (std::chrono::steady_clock::now() < deadline) {
    std::vector<picojson::value> requests;
    if (wm_mutex) {
      std::lock_guard<std::mutex> lock(*wm_mutex);
      requests = wm.find_requests("/session/authenticator-request.*");
    } else {
      requests = wm.find_requests("/session/authenticator-request.*");
    }
    if (requests.size() > n) {
      const auto& req_obj = requests[n].get<picojson::object>();
      auto body_it = req_obj.find("body");
      if (body_it == req_obj.end() || !body_it->second.is<std::string>())
        throw std::runtime_error("authenticator-request has no body string");

      picojson::value body_json;
      std::string err = picojson::parse(body_json, body_it->second.get<std::string>());
      if (!err.empty()) throw std::runtime_error("Failed to parse authenticator-request body: " + err);

      auto& data = body_json.get<picojson::object>()["data"].get<picojson::object>();
      auto port_str = data["BROWSER_MODE_REDIRECT_PORT"].get<std::string>();
      int port = std::stoi(port_str);

      // Send the fake browser callback using platform-appropriate socket APIs.
      // platform.hpp already pulled in the socket headers for each platform.
#ifdef _WIN32
      SOCKET sock = ::socket(AF_INET, SOCK_STREAM, 0);
      if (sock == INVALID_SOCKET) throw std::runtime_error("socket() failed");
#else
      int sock = (int)::socket(AF_INET, SOCK_STREAM, 0);
      if (sock < 0) throw std::runtime_error("socket() failed");
#endif
      struct sockaddr_in addr = {};
      addr.sin_family = AF_INET;
      addr.sin_port = htons((uint16_t)port);
      inet_pton(AF_INET, "127.0.0.1", &addr.sin_addr);
      ::connect(sock, (struct sockaddr*)&addr, sizeof(addr));
      std::string req = "GET /?token=" + token + " HTTP/1.1\r\nHost: localhost\r\n\r\n";
      ::send(sock, req.c_str(), (int)req.size(), 0);
      char buf[512] = {};
      ::recv(sock, buf, sizeof(buf) - 1, 0);
#ifdef _WIN32
      ::closesocket(sock);
#else
      ::close(sock);
#endif
      return;
    }
    std::this_thread::sleep_for(std::chrono::milliseconds(100));
  }
  throw std::runtime_error("Timed out waiting for authenticator-request #" + std::to_string(n));
}

/// Convenience wrapper: deliver a browser callback for the first (index 0)
/// authenticator-request in flight.
static void simulate_browser_callback(const WiremockClient& wm, const std::string& token, int timeout_ms = 15000) {
  simulate_browser_callback_nth(wm, token, 0, nullptr, timeout_ms);
}

// =============================================================================
// Scenario: should show only one external browser prompt when multiple
//           connections authenticate concurrently
// =============================================================================

TEST_CASE("should show only one external browser prompt when multiple connections authenticate concurrently",
          "[parallel_user_prompt_locking]") {
#ifdef _WIN32
  SKIP("WireMock tests not yet validated on Windows");
#endif
  SKIP_OLD_DRIVER("", "New-driver-only: prompt-lock serialization");
  EnvOverride browser_env("SF_TEST_BROWSER_OPENER", "noop");

  // Given clientStoreTemporaryCredential is enabled and DISABLE_PARALLEL_USER_PROMPT is true
  WiremockClient wm;
  // And Wiremock returns valid ssoUrl and proofKey for authenticator-request
  wm.add_mapping_file("auth/external_browser_authenticator_request.json");

  // First connection gets idToken in its response; after the lock is released the second
  // connection finds the cached idToken and logs in with AUTHENTICATOR=ID_TOKEN.
  // And Login endpoint returns success
  wm.add_mapping_file("auth/login_success_external_browser_with_id_token.json");
  wm.add_mapping_file("auth/login_success_cached_id_token.json");

  // When Multiple connections attempt external browser login concurrently
  auto conn_str = eb_conn_str_with_caching(wm);
  SQLRETURN ret1{SQL_SUCCESS}, ret2{SQL_SUCCESS};

  auto env1 = Connection::initEnv();
  ConnectionHandleWrapper dbc1 = env1.createConnectionHandle();
  auto env2 = Connection::initEnv();
  ConnectionHandleWrapper dbc2 = env2.createConnectionHandle();

  // Deliver the callback once the first authenticator-request appears (the
  // second connection will block on the prompt-lock and then reuse the cache).
  std::thread callback_thread([&wm]() { simulate_browser_callback(wm, "browser_sso_token_locked"); });

  std::thread conn2_thread([&]() {
    ret2 = SQLDriverConnect(dbc2.getHandle(), nullptr, sqlchar(conn_str.c_str()), SQL_NTS, nullptr, 0, nullptr,
                            SQL_DRIVER_NOPROMPT);
  });
  ret1 = SQLDriverConnect(dbc1.getHandle(), nullptr, sqlchar(conn_str.c_str()), SQL_NTS, nullptr, 0, nullptr,
                          SQL_DRIVER_NOPROMPT);
  conn2_thread.join();
  callback_thread.join();

  // Then Only one authenticator-request is sent to the server
  auto authn_reqs = wm.find_requests("/session/authenticator-request.*");
  REQUIRE(authn_reqs.size() == 1);

  // And All connections succeed
  REQUIRE_ODBC(ret1, dbc1);
  REQUIRE_ODBC(ret2, dbc2);

  SQLRETURN rdis1 = SQLDisconnect(dbc1.getHandle());
  REQUIRE_ODBC(rdis1, dbc1);
  SQLRETURN rdis2 = SQLDisconnect(dbc2.getHandle());
  REQUIRE_ODBC(rdis2, dbc2);
}

// =============================================================================
// Scenario: should show only one MFA prompt when multiple connections
//           authenticate concurrently
// =============================================================================

TEST_CASE("should show only one MFA prompt when multiple connections authenticate concurrently",
          "[parallel_user_prompt_locking]") {
#ifdef _WIN32
  SKIP("WireMock tests not yet validated on Windows");
#endif
  SKIP_OLD_DRIVER("", "New-driver-only: prompt-lock serialization");

  // Given clientStoreTemporaryCredential is enabled and DISABLE_PARALLEL_USER_PROMPT is true
  WiremockClient wm;
  // And Wiremock returns successful login with MFA token for the first connection
  wm.add_mapping_file("auth/mfa_login_success_with_mfa_token.json");
  wm.add_mapping_file("auth/mfa_login_success_with_cached_token.json");

  std::ostringstream mfa_ss;
  configure_driver_string(mfa_ss);
  mfa_ss << "SERVER=localhost;PORT=" << wm.port() << ";ACCOUNT=testaccount;";
  mfa_ss << "UID=mfa_lock_user;PWD=test_password;";  // pragma: allowlist secret
  mfa_ss << "AUTHENTICATOR=USERNAME_PASSWORD_MFA;";
  mfa_ss << "CLIENT_STORE_TEMPORARY_CREDENTIAL=true;";
  mfa_ss << "SSL=off;DisableOCSPCheck=true;";
  auto mfa_conn_str = mfa_ss.str();

  SQLRETURN ret1{SQL_SUCCESS}, ret2{SQL_SUCCESS};
  auto env1 = Connection::initEnv();
  ConnectionHandleWrapper dbc1 = env1.createConnectionHandle();
  auto env2 = Connection::initEnv();
  ConnectionHandleWrapper dbc2 = env2.createConnectionHandle();

  // When Multiple connections attempt username_password_mfa login concurrently
  std::thread conn2_thread([&]() {
    ret2 = SQLDriverConnect(dbc2.getHandle(), nullptr, sqlchar(mfa_conn_str.c_str()), SQL_NTS, nullptr, 0, nullptr,
                            SQL_DRIVER_NOPROMPT);
  });
  ret1 = SQLDriverConnect(dbc1.getHandle(), nullptr, sqlchar(mfa_conn_str.c_str()), SQL_NTS, nullptr, 0, nullptr,
                          SQL_DRIVER_NOPROMPT);
  conn2_thread.join();

  // Interactive MFA logins lack a TOKEN field; cached-token logins include TOKEN — exclude them.
  // Then Only one interactive MFA login-request is sent to the server
  auto all_login_reqs = wm.find_requests("/session/v1/login-request.*");
  int interactive_count = 0;
  for (const auto& r : all_login_reqs) {
    const auto& obj = r.get<picojson::object>();
    auto body_it = obj.find("body");
    if (body_it != obj.end() && body_it->second.is<std::string>()) {
      picojson::value jv;
      picojson::parse(jv, body_it->second.get<std::string>());
      auto& data = jv.get<picojson::object>()["data"].get<picojson::object>();
      auto auth_it = data.find("AUTHENTICATOR");
      auto token_it = data.find("TOKEN");
      bool is_mfa = auth_it != data.end() && auth_it->second.get<std::string>() == "USERNAME_PASSWORD_MFA";
      bool has_cached_token =
          token_it != data.end() && token_it->second.is<std::string>() && !token_it->second.get<std::string>().empty();
      if (is_mfa && !has_cached_token) ++interactive_count;
    }
  }
  REQUIRE(interactive_count == 1);

  // And All connections succeed using the cached MFA token
  REQUIRE_ODBC(ret1, dbc1);
  REQUIRE_ODBC(ret2, dbc2);

  SQLRETURN rdis1 = SQLDisconnect(dbc1.getHandle());
  REQUIRE_ODBC(rdis1, dbc1);
  SQLRETURN rdis2 = SQLDisconnect(dbc2.getHandle());
  REQUIRE_ODBC(rdis2, dbc2);
}

// =============================================================================
// Scenario: should show independent prompts when DISABLE_PARALLEL_USER_PROMPT
//           is false
// =============================================================================

TEST_CASE("should show independent prompts when DISABLE_PARALLEL_USER_PROMPT is false",
          "[parallel_user_prompt_locking]") {
#ifdef _WIN32
  SKIP("WireMock tests not yet validated on Windows");
#endif
  SKIP_OLD_DRIVER("", "New-driver-only: prompt-lock serialization");
  EnvOverride browser_env("SF_TEST_BROWSER_OPENER", "noop");

  // Given clientStoreTemporaryCredential is enabled and DISABLE_PARALLEL_USER_PROMPT is false
  WiremockClient wm;
  // And Wiremock returns valid ssoUrl and proofKey for each authenticator-request
  wm.add_mapping_file("auth/external_browser_authenticator_request.json");
  wm.add_mapping_file("auth/external_browser_authenticator_request.json");

  // And Login endpoint returns success
  wm.add_mapping_file("auth/login_success_external_browser.json");
  wm.add_mapping_file("auth/login_success_external_browser.json");

  auto conn_str = eb_conn_str_with_locking_disabled(wm);
  SQLRETURN ret1{SQL_SUCCESS}, ret2{SQL_SUCCESS};
  auto env1 = Connection::initEnv();
  ConnectionHandleWrapper dbc1 = env1.createConnectionHandle();
  auto env2 = Connection::initEnv();
  ConnectionHandleWrapper dbc2 = env2.createConnectionHandle();

  // Each callback thread targets a specific authenticator-request by index to
  // avoid routing two callbacks to the same connection's loopback port.
  // Serialize WireMock admin API calls: find_requests is not thread-safe when
  // two callback threads share the same WiremockClient connection.

  // When Multiple connections attempt external browser login concurrently
  std::mutex wm_mutex;
  std::thread cb1_thread([&wm, &wm_mutex]() {
    for (int i = 0; i < 100; ++i) {
      int n;
      {
        std::lock_guard<std::mutex> lock(wm_mutex);
        n = (int)wm.find_requests("/session/authenticator-request.*").size();
      }
      if (n >= 1) {
        simulate_browser_callback_nth(wm, "nlock_token_1", 0, &wm_mutex);
        return;
      }
      std::this_thread::sleep_for(std::chrono::milliseconds(100));
    }
    throw std::runtime_error("Timed out waiting for authenticator-request #1");
  });
  std::thread cb2_thread([&wm, &wm_mutex]() {
    for (int i = 0; i < 100; ++i) {
      int n;
      {
        std::lock_guard<std::mutex> lock(wm_mutex);
        n = (int)wm.find_requests("/session/authenticator-request.*").size();
      }
      if (n >= 2) {
        simulate_browser_callback_nth(wm, "nlock_token_2", 1, &wm_mutex);
        return;
      }
      std::this_thread::sleep_for(std::chrono::milliseconds(100));
    }
    throw std::runtime_error("Timed out waiting for authenticator-request #2");
  });

  std::thread conn2_thread([&]() {
    ret2 = SQLDriverConnect(dbc2.getHandle(), nullptr, sqlchar(conn_str.c_str()), SQL_NTS, nullptr, 0, nullptr,
                            SQL_DRIVER_NOPROMPT);
  });
  ret1 = SQLDriverConnect(dbc1.getHandle(), nullptr, sqlchar(conn_str.c_str()), SQL_NTS, nullptr, 0, nullptr,
                          SQL_DRIVER_NOPROMPT);
  conn2_thread.join();
  cb1_thread.join();
  cb2_thread.join();

  // Then Each connection sends its own authenticator-request to the server
  auto authn_reqs = wm.find_requests("/session/authenticator-request.*");
  REQUIRE(authn_reqs.size() >= 2);

  // And All connections succeed independently
  REQUIRE_ODBC(ret1, dbc1);
  REQUIRE_ODBC(ret2, dbc2);

  SQLRETURN rdis1 = SQLDisconnect(dbc1.getHandle());
  REQUIRE_ODBC(rdis1, dbc1);
  SQLRETURN rdis2 = SQLDisconnect(dbc2.getHandle());
  REQUIRE_ODBC(rdis2, dbc2);
}

// =============================================================================
// Scenario: should show independent prompts when clientStoreTemporaryCredential
//           is false
// =============================================================================

TEST_CASE("should show independent prompts when clientStoreTemporaryCredential is false",
          "[parallel_user_prompt_locking]") {
#ifdef _WIN32
  SKIP("WireMock tests not yet validated on Windows");
#endif
  SKIP_OLD_DRIVER("", "New-driver-only: prompt-lock serialization");
  EnvOverride browser_env("SF_TEST_BROWSER_OPENER", "noop");

  // Given clientStoreTemporaryCredential is disabled and DISABLE_PARALLEL_USER_PROMPT is true
  WiremockClient wm;
  // And Wiremock returns valid ssoUrl and proofKey for each authenticator-request
  wm.add_mapping_file("auth/external_browser_authenticator_request.json");
  wm.add_mapping_file("auth/external_browser_authenticator_request.json");

  // And Login endpoint returns success
  wm.add_mapping_file("auth/login_success_external_browser.json");
  wm.add_mapping_file("auth/login_success_external_browser.json");

  auto conn_str = eb_conn_str_no_caching(wm);
  SQLRETURN ret1{SQL_SUCCESS}, ret2{SQL_SUCCESS};
  auto env1 = Connection::initEnv();
  ConnectionHandleWrapper dbc1 = env1.createConnectionHandle();
  auto env2 = Connection::initEnv();
  ConnectionHandleWrapper dbc2 = env2.createConnectionHandle();

  // Each callback thread targets a specific authenticator-request by index to
  // avoid routing two callbacks to the same connection's loopback port.
  // Serialize WireMock admin API calls: find_requests is not thread-safe when
  // two callback threads share the same WiremockClient connection.

  // When Multiple connections attempt external browser login concurrently
  std::mutex wm_mutex2;
  std::thread cb1_thread([&wm, &wm_mutex2]() {
    for (int i = 0; i < 100; ++i) {
      int n;
      {
        std::lock_guard<std::mutex> lock(wm_mutex2);
        n = (int)wm.find_requests("/session/authenticator-request.*").size();
      }
      if (n >= 1) {
        simulate_browser_callback_nth(wm, "nocache_token_1", 0, &wm_mutex2);
        return;
      }
      std::this_thread::sleep_for(std::chrono::milliseconds(100));
    }
    throw std::runtime_error("Timed out waiting for authenticator-request #1");
  });
  std::thread cb2_thread([&wm, &wm_mutex2]() {
    for (int i = 0; i < 100; ++i) {
      int n;
      {
        std::lock_guard<std::mutex> lock(wm_mutex2);
        n = (int)wm.find_requests("/session/authenticator-request.*").size();
      }
      if (n >= 2) {
        simulate_browser_callback_nth(wm, "nocache_token_2", 1, &wm_mutex2);
        return;
      }
      std::this_thread::sleep_for(std::chrono::milliseconds(100));
    }
    throw std::runtime_error("Timed out waiting for authenticator-request #2");
  });

  std::thread conn2_thread([&]() {
    ret2 = SQLDriverConnect(dbc2.getHandle(), nullptr, sqlchar(conn_str.c_str()), SQL_NTS, nullptr, 0, nullptr,
                            SQL_DRIVER_NOPROMPT);
  });
  ret1 = SQLDriverConnect(dbc1.getHandle(), nullptr, sqlchar(conn_str.c_str()), SQL_NTS, nullptr, 0, nullptr,
                          SQL_DRIVER_NOPROMPT);
  conn2_thread.join();
  cb1_thread.join();
  cb2_thread.join();

  // Then Each connection sends its own authenticator-request to the server
  auto authn_reqs = wm.find_requests("/session/authenticator-request.*");
  REQUIRE(authn_reqs.size() >= 2);

  // And All connections succeed independently
  REQUIRE_ODBC(ret1, dbc1);
  REQUIRE_ODBC(ret2, dbc2);

  SQLRETURN rdis1 = SQLDisconnect(dbc1.getHandle());
  REQUIRE_ODBC(rdis1, dbc1);
  SQLRETURN rdis2 = SQLDisconnect(dbc2.getHandle());
  REQUIRE_ODBC(rdis2, dbc2);
}
