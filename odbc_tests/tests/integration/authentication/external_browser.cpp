#include <sql.h>
#include <sqlext.h>

#include <algorithm>
#include <chrono>
#include <string>
#include <thread>

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

static std::string get_external_browser_connection_string(const WiremockClient& wm) {
  std::ostringstream ss;
  configure_driver_string(ss);
  ss << "SERVER=localhost;";
  ss << "PORT=" << wm.port() << ";";
  ss << "ACCOUNT=testaccount;";
  ss << "UID=test_user;";
  ss << "AUTHENTICATOR=EXTERNALBROWSER;";
  ss << "SSL=off;";
  ss << "DisableOCSPCheck=true;";
  return ss.str();
}

/// Poll WireMock for the authenticator-request, extract the redirect port,
/// then send a fake token to sf_core's localhost callback listener.
static void simulate_browser_callback(const WiremockClient& wm, const std::string& token, int timeout_ms = 10000) {
  auto deadline = std::chrono::steady_clock::now() + std::chrono::milliseconds(timeout_ms);
  while (std::chrono::steady_clock::now() < deadline) {
    auto requests = wm.find_requests("/session/authenticator-request.*");
    if (!requests.empty()) {
      const auto& req_obj = requests[0].get<picojson::object>();
      auto body_it = req_obj.find("body");
      if (body_it == req_obj.end() || !body_it->second.is<std::string>()) {
        throw std::runtime_error("authenticator-request has no body string");
      }

      picojson::value body_json;
      std::string err = picojson::parse(body_json, body_it->second.get<std::string>());
      if (!err.empty()) {
        throw std::runtime_error("Failed to parse authenticator-request body: " + err);
      }

      auto port_str = body_json.get<picojson::object>()["data"]
                          .get<picojson::object>()["BROWSER_MODE_REDIRECT_PORT"]
                          .get<std::string>();
      int port = std::stoi(port_str);

#ifdef _WIN32
      WSADATA wsa_data;
      WSAStartup(MAKEWORD(2, 2), &wsa_data);
      SOCKET sock = ::socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
#else
      int sock = ::socket(AF_INET, SOCK_STREAM, 0);
#endif
      sockaddr_in addr{};
      addr.sin_family = AF_INET;
      addr.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
      addr.sin_port = htons(static_cast<uint16_t>(port));

      if (::connect(sock, reinterpret_cast<sockaddr*>(&addr), sizeof(addr)) < 0) {
        throw std::runtime_error("Failed to connect to callback listener on port " + port_str);
      }

      std::string http_request = "GET /?token=" + token + " HTTP/1.1\r\nHost: localhost\r\n\r\n";
#ifdef _WIN32
      ::send(sock, http_request.c_str(), static_cast<int>(http_request.size()), 0);
      char buf[4096];
      ::recv(sock, buf, sizeof(buf), 0);
      ::closesocket(sock);
#else
      ::send(sock, http_request.c_str(), http_request.size(), 0);
      char buf[4096];
      ::recv(sock, buf, sizeof(buf), 0);
      ::close(sock);
#endif
      return;
    }
    std::this_thread::sleep_for(std::chrono::milliseconds(200));
  }
  throw std::runtime_error("authenticator-request never arrived at WireMock");
}

// =============================================================================
// Happy Path
// =============================================================================

TEST_CASE("should login with external browser using simulated callback", "[external_browser_auth]") {
#ifdef _WIN32
  SKIP("WireMock tests not yet validated on Windows");
#endif
  SKIP_OLD_DRIVER("", "New-driver-only: external browser auth via WireMock");
  EnvOverride browser_env("SF_TEST_BROWSER_OPENER", "noop");

  // Given Wiremock returns valid ssoUrl and proofKey for authenticator-request
  WiremockClient wm;
  wm.add_mapping_file("auth/external_browser_authenticator_request.json");

  // And Login endpoint returns success
  wm.add_mapping_file("auth/login_success_external_browser.json");

  // When Trying to Connect with simulated browser callback delivering a token
  std::string token = "browser_sso_token_12345";
  auto conn_str = get_external_browser_connection_string(wm);

  std::thread callback_thread([&wm, &token]() { simulate_browser_callback(wm, token); });

  auto env = Connection::initEnv();
  ConnectionHandleWrapper dbc = env.createConnectionHandle();
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), nullptr, sqlchar(conn_str.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_NOPROMPT);

  callback_thread.join();

  // Then Login is successful
  REQUIRE_ODBC(ret, dbc);

  // And Login request contains EXTERNALBROWSER authenticator, token, proof key, and login name
  auto login_requests = wm.find_requests("/session/v1/login-request.*");
  REQUIRE(!login_requests.empty());
  const auto& req_obj = login_requests[0].get<picojson::object>();
  auto body_it = req_obj.find("body");
  REQUIRE(body_it != req_obj.end());

  picojson::value body_json;
  picojson::parse(body_json, body_it->second.get<std::string>());
  auto& data = body_json.get<picojson::object>()["data"].get<picojson::object>();
  CHECK(data["AUTHENTICATOR"].get<std::string>() == "EXTERNALBROWSER");
  CHECK(data["TOKEN"].get<std::string>() == token);
  CHECK(data["PROOF_KEY"].get<std::string>() == "mock_proof_key_abc123");
  CHECK(data["LOGIN_NAME"].get<std::string>() == "test_user");

  SQLDisconnect(dbc.getHandle());
}

// =============================================================================
// Error Handling - Authenticator Request Failures
// =============================================================================

TEST_CASE("should fail when authenticator-request returns forbidden", "[external_browser_auth]") {
#ifdef _WIN32
  SKIP("WireMock tests not yet validated on Windows");
#endif
  SKIP_OLD_DRIVER("", "New-driver-only: external browser auth via WireMock");
  EnvOverride browser_env("SF_TEST_BROWSER_OPENER", "noop");

  // Given Wiremock returns HTTP 403 for authenticator-request
  WiremockClient wm;
  wm.add_mapping_file("auth/external_browser_authenticator_request_forbidden.json");

  // When Trying to Connect
  auto conn_str = get_external_browser_connection_string(wm);
  auto env = Connection::initEnv();
  ConnectionHandleWrapper dbc = env.createConnectionHandle();
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), nullptr, sqlchar(conn_str.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_NOPROMPT);

  // Then Connection fails with authenticator error
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(dbc);
  REQUIRE(!records.empty());
  bool has_relevant_error = std::any_of(records.begin(), records.end(), [](const auto& r) {
    return r.messageText.find("403") != std::string::npos || r.messageText.find("Forbidden") != std::string::npos ||
           r.messageText.find("authenticator") != std::string::npos;
  });
  CHECK(has_relevant_error);
}

TEST_CASE("should fail when authenticator-request returns logical failure", "[external_browser_auth]") {
#ifdef _WIN32
  SKIP("WireMock tests not yet validated on Windows");
#endif
  SKIP_OLD_DRIVER("", "New-driver-only: external browser auth via WireMock");
  EnvOverride browser_env("SF_TEST_BROWSER_OPENER", "noop");

  // Given Wiremock returns success false for authenticator-request
  WiremockClient wm;
  wm.add_mapping_file("auth/external_browser_authenticator_request_logical_failure.json");

  // When Trying to Connect
  auto conn_str = get_external_browser_connection_string(wm);
  auto env = Connection::initEnv();
  ConnectionHandleWrapper dbc = env.createConnectionHandle();
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), nullptr, sqlchar(conn_str.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_NOPROMPT);

  // Then Connection fails with authenticator error
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(dbc);
  REQUIRE(!records.empty());
  bool has_relevant_error = std::any_of(records.begin(), records.end(), [](const auto& r) {
    return r.messageText.find("not enabled") != std::string::npos ||
           r.messageText.find("authenticator") != std::string::npos;
  });
  CHECK(has_relevant_error);
}

// =============================================================================
// Error Handling - Timeout (no browser callback)
// =============================================================================

TEST_CASE("should fail with timeout when no browser callback arrives", "[external_browser_auth]") {
#ifdef _WIN32
  SKIP("WireMock tests not yet validated on Windows");
#endif
  SKIP_OLD_DRIVER("", "New-driver-only: external browser auth via WireMock");
  EnvOverride browser_env("SF_TEST_BROWSER_OPENER", "noop");

  // Given Wiremock returns valid ssoUrl and proofKey for authenticator-request
  WiremockClient wm;
  wm.add_mapping_file("auth/external_browser_authenticator_request.json");

  // And Authentication timeout is set to 2 seconds
  auto conn_str = get_external_browser_connection_string(wm);
  conn_str += "AUTHENTICATION_TIMEOUT=2;";

  // When Trying to Connect without any browser callback
  auto env = Connection::initEnv();
  ConnectionHandleWrapper dbc = env.createConnectionHandle();
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), nullptr, sqlchar(conn_str.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_NOPROMPT);

  // Then Connection fails with timeout or browser error
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(dbc);
  REQUIRE(!records.empty());
  bool has_relevant_error = std::any_of(records.begin(), records.end(), [](const auto& r) {
    auto msg = r.messageText;
    std::transform(msg.begin(), msg.end(), msg.begin(), ::tolower);
    return msg.find("timeout") != std::string::npos || msg.find("browser") != std::string::npos;
  });
  CHECK(has_relevant_error);
}

// =============================================================================
// Error Handling - Login Failure After Successful Browser Flow
// =============================================================================

TEST_CASE("should fail when login request is rejected after browser callback", "[external_browser_auth]") {
#ifdef _WIN32
  SKIP("WireMock tests not yet validated on Windows");
#endif
  SKIP_OLD_DRIVER("", "New-driver-only: external browser auth via WireMock");
  EnvOverride browser_env("SF_TEST_BROWSER_OPENER", "noop");

  // Given Wiremock returns valid ssoUrl and proofKey for authenticator-request
  WiremockClient wm;
  wm.add_mapping_file("auth/external_browser_authenticator_request.json");

  // And Login endpoint returns failure
  wm.add_mapping_file("auth/login_failure_external_browser.json");

  // When Trying to Connect with simulated browser callback delivering a token
  std::string token = "browser_sso_token_rejected";
  auto conn_str = get_external_browser_connection_string(wm);

  std::thread callback_thread([&wm, &token]() { simulate_browser_callback(wm, token); });

  auto env = Connection::initEnv();
  ConnectionHandleWrapper dbc = env.createConnectionHandle();
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), nullptr, sqlchar(conn_str.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_NOPROMPT);

  callback_thread.join();

  // Then Connection fails with login error
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(dbc);
  REQUIRE(!records.empty());
  bool has_relevant_error = std::any_of(records.begin(), records.end(), [](const auto& r) {
    return r.messageText.find("Invalid credentials") != std::string::npos ||
           r.messageText.find("login") != std::string::npos || r.messageText.find("Login") != std::string::npos;
  });
  CHECK(has_relevant_error);
}

// The full external browser flow against a real headless-Chrome browser is exercised by the
// e2e test in tests/e2e/authentication/external_browser.cpp (run via tests/auth/run_auth_browser.sh odbc).
// The integration tests above only simulate the callback via raw sockets.
