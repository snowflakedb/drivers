// External browser authentication E2E test.
//
// Requires the snowdrivers-test-external-browser-universal-driver Docker container
// (headless Chromium + /externalbrowser/provideBrowserCredentials.js). The driver
// opens Chromium via the real browser opener (SF_TEST_BROWSER_OPENER is NOT set to
// "noop" here, unlike the WireMock integration test), and the Node automation script
// drives the Okta IdP login over Chromium's remote-debugging port.
//
// This mirrors python/tests/e2e/authentication/test_external_browser.py.
//
// Run locally:
//   ./tests/auth/run_auth_browser.sh odbc

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <chrono>
#include <cstdlib>
#include <future>
#include <sstream>
#include <stdexcept>
#include <string>
#include <thread>

#include <catch2/catch_test_macros.hpp>

#include "HandleWrapper.hpp"
#include "compatibility.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"
#include "platform.hpp"
#include "require.hpp"
#include "test_setup.hpp"

// These Node.js scripts and the Chromium remote-debugging port are provided by the
// snowdrivers-test-external-browser-universal-driver Docker image (see tests/auth/). They
// do not exist outside that container, which is why this test is gated behind
// REQUIRE_BROWSER.
namespace {

constexpr const char* PROVIDE_CREDENTIALS_SCRIPT = "/externalbrowser/provideBrowserCredentials.js";
constexpr const char* CLEAN_BROWSER_SCRIPT = "/externalbrowser/cleanBrowserProcesses.js";
constexpr int CHROMIUM_DEBUG_PORT = 9222;

std::string get_external_browser_connection_string() {
  auto params = get_test_parameters("testconnection");
  std::stringstream ss;
  configure_driver_string(ss);
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_OKTA_HOST", "SERVER");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_OKTA_ACCOUNT", "ACCOUNT");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_OKTA_USER", "UID");
  ss << "AUTHENTICATOR=EXTERNALBROWSER;";
  ss << "ROLE=PUBLIC;";
  // Never reuse a cached id-token: always exercise the real browser flow.
  ss << "CLIENT_STORE_TEMPORARY_CREDENTIAL=false;";
  return ss.str();
}

// Kill any lingering Chromium processes left over from previous runs.
void clean_browser_processes() {
  std::string cmd = "node " + std::string(CLEAN_BROWSER_SCRIPT) + platform::null_redirect();
  std::system(cmd.c_str());
}

// Run the Node.js browser automation script that fills the IdP credentials.
void provide_browser_credentials(const std::string& scenario, const std::string& login, const std::string& password) {
  std::string cmd = "node " + std::string(PROVIDE_CREDENTIALS_SCRIPT) + " \"" + scenario + "\" \"" + login + "\" \"" +
                    password + "\"";
  int rc = std::system(cmd.c_str());
  if (rc != 0) {
    throw std::runtime_error("provideBrowserCredentials.js failed (rc=" + std::to_string(rc) + ")");
  }
}

// Block until Chromium's remote-debugging port is accepting connections. The driver's
// connect thread launches Chromium, so the automation script must wait for the port
// before it can attach.
bool wait_for_chromium(int timeout_ms = 60000, int poll_interval_ms = 1000) {
  auto deadline = std::chrono::steady_clock::now() + std::chrono::milliseconds(timeout_ms);
  while (std::chrono::steady_clock::now() < deadline) {
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
    addr.sin_port = htons(static_cast<uint16_t>(CHROMIUM_DEBUG_PORT));

    bool connected = ::connect(sock, reinterpret_cast<sockaddr*>(&addr), sizeof(addr)) == 0;
#ifdef _WIN32
    ::closesocket(sock);
#else
    ::close(sock);
#endif
    if (connected) {
      return true;
    }
    std::this_thread::sleep_for(std::chrono::milliseconds(poll_interval_ms));
  }
  return false;
}

void verify_simple_query_execution(ConnectionHandleWrapper& dbc) {
  StatementHandleWrapper stmt = dbc.createStatementHandle();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  SQLINTEGER result = 0;
  SQLLEN indicator = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &result, sizeof(result), &indicator);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(indicator != SQL_NULL_DATA);
  REQUIRE(result == 1);
}

}  // namespace

TEST_CASE("should authenticate with external browser via Okta IdP", "[external_browser_e2e]") {
  REQUIRE_BROWSER("External browser E2E needs the headless Chromium container");
  SKIP_OLD_DRIVER("", "New-driver-only: external browser E2E against headless Chromium");

  clean_browser_processes();

  // Given External browser authentication is configured with valid Okta user
  std::string connection_string = get_external_browser_connection_string();

  EnvironmentHandleWrapper env;
  SQLRETURN ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
  REQUIRE_ODBC(ret, env);
  ConnectionHandleWrapper dbc = env.createConnectionHandle();

  auto params = get_test_parameters("testconnection");
  std::string login = get_param_required<std::string>(params, "SNOWFLAKE_TEST_OKTA_USER");
  std::string password = get_param_required<std::string>(params, "SNOWFLAKE_TEST_OKTA_PASSWORD");

  // The connect thread drives SQLDriverConnect (which launches Chromium); the browser
  // thread waits for the Chromium debug port, then feeds the Okta credentials.
  // When Trying to Connect with headless browser providing valid credentials
  std::promise<SQLRETURN> connect_rc_promise;
  std::future<SQLRETURN> connect_rc_future = connect_rc_promise.get_future();

  std::thread connect_thread([&dbc, &connection_string, &connect_rc_promise]() {
    SQLRETURN rc = SQLDriverConnect(dbc.getHandle(), nullptr, sqlchar(connection_string.c_str()), SQL_NTS, nullptr, 0,
                                    nullptr, SQL_DRIVER_NOPROMPT);
    connect_rc_promise.set_value(rc);
  });

  // Capture any failure from the browser thread rather than letting an uncaught
  // exception call std::terminate; it is rethrown on the main thread after the join.
  std::exception_ptr browser_error;
  std::thread browser_thread([&login, &password, &browser_error]() {
    try {
      if (!wait_for_chromium()) {
        throw std::runtime_error("Chromium did not start on port 9222 within timeout");
      }
      provide_browser_credentials("success", login, password);
    } catch (...) {
      browser_error = std::current_exception();
    }
  });

  browser_thread.join();
  connect_thread.join();

  if (browser_error) {
    std::rethrow_exception(browser_error);
  }

  struct CleanupGuard {
    ConnectionHandleWrapper& dbc;
    bool connected = false;
    ~CleanupGuard() {
      if (connected) {
        SQLDisconnect(dbc.getHandle());
      }
      clean_browser_processes();
    }
  } cleanup{dbc};

  // Then Login is successful and simple query can be executed
  ret = connect_rc_future.get();
  REQUIRE_ODBC(ret, dbc);
  cleanup.connected = true;

  verify_simple_query_execution(dbc);

  ret = SQLDisconnect(dbc.getHandle());
  REQUIRE_ODBC(ret, dbc);
  cleanup.connected = false;
}
