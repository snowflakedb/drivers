#ifndef OAUTH_AUTH_HELPERS_HPP
#define OAUTH_AUTH_HELPERS_HPP

#include <picojson.h>
#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#ifdef _WIN32
#include <winsock2.h>
#else
#include <sys/socket.h>
#include <unistd.h>

#include <arpa/inet.h>
#include <netinet/in.h>
#endif

#include <cctype>
#include <chrono>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <future>
#include <sstream>
#include <stdexcept>
#include <string>
#include <thread>

#include <catch2/catch_test_macros.hpp>

#include "HandleWrapper.hpp"
#include "platform.hpp"

// Shared OAuth / headless-browser E2E helpers for the ODBC wrapper, mirroring
// python/tests/e2e/authentication/auth_helpers.py. All of this only runs inside
// the snowdrivers-test-external-browser-universal-driver container (Chromium
// debug port + /externalbrowser scripts), so callers gate on REQUIRE_BROWSER.

namespace oauth_auth {

constexpr const char* PROVIDE_CREDENTIALS_SCRIPT = "/externalbrowser/provideBrowserCredentials.js";
constexpr const char* CLEAN_BROWSER_SCRIPT = "/externalbrowser/cleanBrowserProcesses.js";
constexpr int CHROMIUM_DEBUG_PORT = 9222;

// Wrap a value in single quotes for a POSIX shell, escaping embedded quotes, so
// IdP credentials reach curl without word-splitting or injection.
inline std::string shell_single_quote(const std::string& value) {
  std::string out = "'";
  for (char ch : value) {
    out += (ch == '\'') ? "'\\''" : std::string(1, ch);
  }
  out += "'";
  return out;
}

// Mint a fresh OAuth access token via the IdP's Resource Owner Password grant,
// using curl (the same HTTP tool the WireMock harness shells out to).
inline std::string retrieve_oauth_access_token(const std::string& token_url, const std::string& client_id,
                                               const std::string& client_secret, const std::string& user,
                                               const std::string& password, const std::string& role) {
  std::string lower_role = role;
  for (auto& ch : lower_role) {
    ch = static_cast<char>(std::tolower(static_cast<unsigned char>(ch)));
  }

  std::stringstream cmd;
  cmd << "curl -s -X POST " << shell_single_quote(token_url)
      << " -H \"Content-Type: application/x-www-form-urlencoded;charset=UTF-8\"" << " -u "
      << shell_single_quote(client_id + ":" + client_secret) << " --data-urlencode "
      << shell_single_quote("username=" + user) << " --data-urlencode " << shell_single_quote("password=" + password)
      << " --data-urlencode \"grant_type=password\"" << " --data-urlencode "
      << shell_single_quote("scope=session:role:" + lower_role);

  std::string response = platform::exec_command(cmd.str());

  picojson::value json;
  std::string err = picojson::parse(json, response);
  if (!err.empty() || !json.is<picojson::object>()) {
    FAIL("Failed to parse OAuth token response: " << err << " | body: " << response);
  }
  const auto& obj = json.get<picojson::object>();
  auto it = obj.find("access_token");
  if (it == obj.end() || !it->second.is<std::string>()) {
    FAIL("OAuth token response missing 'access_token': " << response);
  }
  return it->second.get<std::string>();
}

// Kill any lingering Chromium processes left over from previous runs.
inline void clean_browser_processes() {
  std::system(("node " + std::string(CLEAN_BROWSER_SCRIPT) + platform::null_redirect()).c_str());
}

// Run the Node.js browser automation script that fills the IdP credentials.
// Single-quote each arg so a credential containing $, backtick, or backslash
// reaches the script verbatim instead of being expanded by /bin/sh -c.
inline void provide_browser_credentials(const std::string& scenario, const std::string& login,
                                        const std::string& password) {
  std::string cmd = "node " + std::string(PROVIDE_CREDENTIALS_SCRIPT) + " " + shell_single_quote(scenario) + " " +
                    shell_single_quote(login) + " " + shell_single_quote(password);
  int rc = std::system(cmd.c_str());
  if (rc != 0) {
    throw std::runtime_error("provideBrowserCredentials.js failed (rc=" + std::to_string(rc) + ")");
  }
}

// Block until Chromium's remote-debugging port accepts connections. The driver's
// connect thread launches Chromium, so the automation must wait for the port.
inline bool wait_for_chromium(int timeout_ms = 60000, int poll_interval_ms = 1000) {
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

// Drive SQLDriverConnect and the browser automation concurrently. The connect
// thread spawns Chromium via the OS browser opener; the browser thread waits for
// the debug port and feeds the Okta credentials. Mirrors
// connect_with_browser_automation() in auth_helpers.py: the connect leg is
// authoritative, so its return code is reported even when the browser leg fails,
// and a hung connect is bounded so the test fails instead of blocking forever.
//
// Returns the SQLDriverConnect return code; the connection handle is left in
// whatever state the connect attempt produced so callers can assert success or
// failure themselves.
inline SQLRETURN connect_with_browser_automation(ConnectionHandleWrapper& dbc, const std::string& connection_string,
                                                 const std::string& scenario, const std::string& login,
                                                 const std::string& password) {
  std::promise<SQLRETURN> connect_rc_promise;
  std::future<SQLRETURN> connect_rc_future = connect_rc_promise.get_future();

  std::thread connect_thread([&dbc, &connection_string, &connect_rc_promise]() {
    SQLRETURN rc = SQLDriverConnect(dbc.getHandle(), nullptr, (SQLCHAR*)connection_string.c_str(), SQL_NTS, nullptr, 0,
                                    nullptr, SQL_DRIVER_NOPROMPT);
    connect_rc_promise.set_value(rc);
  });

  std::exception_ptr browser_error;
  std::thread browser_thread([&scenario, &login, &password, &browser_error]() {
    try {
      if (!wait_for_chromium()) {
        throw std::runtime_error("Chromium did not start on port 9222 within timeout");
      }
      provide_browser_credentials(scenario, login, password);
    } catch (...) {
      browser_error = std::current_exception();
    }
  });

  browser_thread.join();

  // The connect leg is authoritative. Bound the wait so a hung SQLDriverConnect
  // fails the run instead of blocking forever. We cannot safely return here: the
  // connect thread still references the local promise, `dbc`'s handle, and
  // `connection_string`, so unwinding would let it use-after-free as those die.
  // A clean process death is preferable to heap corruption bleeding into later
  // tests, so abort rather than detach + throw.
  if (connect_rc_future.wait_for(std::chrono::seconds(90)) != std::future_status::ready) {
    std::string browser_detail = "none";
    if (browser_error) {
      try {
        std::rethrow_exception(browser_error);
      } catch (const std::exception& e) {
        browser_detail = e.what();
      } catch (...) {
        browser_detail = "unknown browser-leg error";
      }
    }
    std::fprintf(stderr,
                 "FATAL: SQLDriverConnect did not finish within 90s; aborting to avoid "
                 "use-after-free on the still-running connect thread. Browser leg: %s\n",
                 browser_detail.c_str());
    std::abort();
  }

  connect_thread.join();
  return connect_rc_future.get();
}

}  // namespace oauth_auth

#endif  // OAUTH_AUTH_HELPERS_HPP
