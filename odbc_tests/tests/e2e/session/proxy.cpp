// End-to-end proxy support tests using WireMock as a forward HTTP proxy.
//
// The driver is pointed at an unresolvable host so that only proxy-routed
// requests can succeed. WireMock runs in `--enable-browser-proxying` mode,
// matches login by URL path, and serves the canned `login_success_any`
// mapping. A successful connect proves the request transited the proxy;
// an SQLDriverConnect error proves direct DNS resolution was attempted.

#include <sql.h>
#include <sqlext.h>

#include <cstdlib>
#include <sstream>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "WiremockClient.hpp"
#include "compatibility.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

namespace {

// Hostname that is guaranteed not to resolve. RFC 6761 reserves `.invalid`.
constexpr const char* UNRESOLVABLE_HOST = "nonexistent.invalid";

std::string driver_path_or_skip() {
  const char* p = std::getenv("DRIVER_PATH");
  if (p == nullptr || p[0] == '\0') {
    throw std::runtime_error("DRIVER_PATH not set — cannot locate ODBC driver library");
  }
  return p;
}

// Build a connection string targeting an unresolvable host.  Override or
// extend `extra` to add proxy DSN keys.  No SSL — wiremock serves plain HTTP
// because reqwest forwards HTTP requests as `GET http://host/path` to the
// proxy, which wiremock can match by path.
std::string build_unresolvable_connection_string(const std::string& extra) {
  std::ostringstream ss;
  ss << "DRIVER={" << driver_path_or_skip() << "};";
  ss << "SERVER=" << UNRESOLVABLE_HOST << ";";
  ss << "PORT=8090;";
  ss << "ACCOUNT=testaccount;";
  ss << "UID=testuser;";
  ss << "PWD=testpass;";
  ss << "SSL=off;";
  ss << "DisableOCSPCheck=true;";
  ss << extra;
  return ss.str();
}

}  // namespace

TEST_CASE("PROXY URL routes login through forward proxy", "[session][proxy]") {
#ifdef _WIN32
  SKIP("WireMock tests not yet validated on Windows");
#endif
  // Given a forward-proxy WireMock returning a canned login response
  WiremockClient wm(WiremockClient::Mode::ForwardProxy);
  wm.add_mapping_file("auth/login_success_any.json");

  // And an ODBC connection string with PROXY=http://localhost:<wm_port>
  std::ostringstream extra;
  extra << "PROXY=http://localhost:" << wm.port() << ";";
  auto conn_str = build_unresolvable_connection_string(extra.str());

  // When SQLDriverConnect is invoked
  auto env = Connection::initEnv();
  auto dbc = env.createConnectionHandle();
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), nullptr, sqlchar(conn_str.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_NOPROMPT);

  // Then the connect succeeds — proving the request transited the proxy
  // (without proxy routing, DNS would fail on `.invalid`).
  REQUIRE_ODBC(ret, dbc);

  // And the proxy received exactly one login request
  CHECK(wm.get_request_count("POST", "/session/v1/login-request") == 1);
}

TEST_CASE("Empty PROXY value with AllowEmptyProxy explicitly disables proxy", "[session][proxy]") {
#ifdef _WIN32
  SKIP("WireMock tests not yet validated on Windows");
#endif
  // Given a forward-proxy WireMock that *would* answer if reached
  WiremockClient wm(WiremockClient::Mode::ForwardProxy);
  wm.add_mapping_file("auth/login_success_any.json");

  // And HTTP_PROXY env var pointing at it (would otherwise be picked up
  // when use_proxy_env=true), but PROXY is set to empty with the legacy
  // AllowEmptyProxy=true knob → proxy explicitly disabled.
  std::ostringstream extra;
  extra << "PROXY=;ALLOWEMPTYPROXY=true;USE_PROXY_ENV=true;";
  auto conn_str = build_unresolvable_connection_string(extra.str());

  // When SQLDriverConnect is invoked, the explicit-disable forces direct
  // DNS resolution. The unresolvable host then fails the connect.
  auto env = Connection::initEnv();
  auto dbc = env.createConnectionHandle();
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), nullptr, sqlchar(conn_str.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_NOPROMPT);

  // Then the connect fails (direct DNS failure on `.invalid`)
  CHECK_THAT(OdbcResult(ret, dbc), OdbcMatchers::IsError());
  // And the proxy received nothing
  CHECK(wm.get_request_count("POST", "/session/v1/login-request") == 0);
}

TEST_CASE("USE_PROXY_ENV defaults to false: HTTP_PROXY env is ignored", "[session][proxy]") {
#ifdef _WIN32
  SKIP("WireMock tests not yet validated on Windows");
#endif
  // Given HTTP_PROXY env var pointing at a forward-proxy WireMock
  WiremockClient wm(WiremockClient::Mode::ForwardProxy);
  wm.add_mapping_file("auth/login_success_any.json");

  std::string http_proxy_url = "http://localhost:" + std::to_string(wm.port());
#ifdef _WIN32
  _putenv_s("HTTP_PROXY", http_proxy_url.c_str());
#else
  setenv("HTTP_PROXY", http_proxy_url.c_str(), 1);
#endif

  // And no explicit PROXY conn-string param + no USE_PROXY_ENV opt-in
  auto conn_str = build_unresolvable_connection_string("");

  // When SQLDriverConnect is invoked
  auto env = Connection::initEnv();
  auto dbc = env.createConnectionHandle();
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), nullptr, sqlchar(conn_str.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_NOPROMPT);

#ifdef _WIN32
  _putenv_s("HTTP_PROXY", "");
#else
  unsetenv("HTTP_PROXY");
#endif

  // Then env var is not consulted by default; direct DNS fails
  CHECK_THAT(OdbcResult(ret, dbc), OdbcMatchers::IsError());
  CHECK(wm.get_request_count("POST", "/session/v1/login-request") == 0);
}

TEST_CASE("USE_PROXY_ENV=true picks up HTTP_PROXY env var", "[session][proxy]") {
#ifdef _WIN32
  SKIP("WireMock tests not yet validated on Windows");
#endif
  // Given HTTP_PROXY env var pointing at a forward-proxy WireMock
  WiremockClient wm(WiremockClient::Mode::ForwardProxy);
  wm.add_mapping_file("auth/login_success_any.json");

  std::string http_proxy_url = "http://localhost:" + std::to_string(wm.port());
#ifdef _WIN32
  _putenv_s("HTTP_PROXY", http_proxy_url.c_str());
#else
  setenv("HTTP_PROXY", http_proxy_url.c_str(), 1);
#endif

  // And USE_PROXY_ENV=true on the connection string
  auto conn_str = build_unresolvable_connection_string("USE_PROXY_ENV=true;");

  // When SQLDriverConnect is invoked
  auto env = Connection::initEnv();
  auto dbc = env.createConnectionHandle();
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), nullptr, sqlchar(conn_str.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_NOPROMPT);

#ifdef _WIN32
  _putenv_s("HTTP_PROXY", "");
#else
  unsetenv("HTTP_PROXY");
#endif

  // Then env var routes the login through the proxy
  REQUIRE_ODBC(ret, dbc);
  CHECK(wm.get_request_count("POST", "/session/v1/login-request") == 1);
}
