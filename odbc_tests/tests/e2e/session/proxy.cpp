// End-to-end proxy support tests using WireMock as a forward HTTP proxy.
//
// The driver is pointed at an unresolvable host so that only proxy-routed
// requests can succeed. WireMock runs in `--enable-browser-proxying` mode,
// matches login by URL path, and serves the canned `login_success_any`
// mapping. A successful connect proves the request transited the proxy;
// an SQLDriverConnect error proves direct DNS resolution was attempted.

#include <sql.h>
#include <sqlext.h>

#include <sstream>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "EnvOverride.hpp"
#include "HandleWrapper.hpp"
#include "WiremockClient.hpp"
#include "compatibility.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"
#include "test_setup.hpp"

namespace {

// Hostname that is guaranteed not to resolve. RFC 6761 reserves `.invalid`.
constexpr const char* UNRESOLVABLE_HOST = "nonexistent.invalid";

// Build a connection string targeting an unresolvable host.  Override or
// extend `extra` to add proxy DSN keys.  No SSL — wiremock serves plain HTTP
// because reqwest forwards HTTP requests as `GET http://host/path` to the
// proxy, which wiremock can match by path.
std::string build_unresolvable_connection_string(const std::string& extra) {
  std::ostringstream ss;
  // Use the registered driver alias (via odbcinst.ini), not DRIVER_PATH
  // directly — iODBC passes brace-delimited absolute paths to dlopen verbatim.
  configure_driver_string(ss);
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

TEST_CASE("should route login through forward proxy via PROXY URL", "[session][proxy]") {
#ifdef _WIN32
  SKIP("WireMock tests not yet validated on Windows");
#endif
  // Given a forward-proxy WireMock serving a canned login response
  WiremockClient wm(WiremockClient::Mode::ForwardProxy);
  wm.add_mapping_file("auth/login_success_any.json");

  std::ostringstream extra;
  extra << "PROXY=http://localhost:" << wm.port() << ";";
  auto conn_str = build_unresolvable_connection_string(extra.str());

  // When SQLDriverConnect is invoked with PROXY pointing at the proxy
  auto env = Connection::initEnv();
  auto dbc = env.createConnectionHandle();
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), nullptr, sqlchar(conn_str.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_NOPROMPT);

  // Then the connect succeeds and the proxy received exactly one login request
  REQUIRE_ODBC(ret, dbc);
  CHECK(wm.get_request_count("POST", "/session/v1/login-request") == 1);
}

TEST_CASE("should disable proxy when PROXY is empty and AllowEmptyProxy is true", "[session][proxy]") {
#ifdef _WIN32
  SKIP("WireMock tests not yet validated on Windows");
#endif
  // Given a forward-proxy WireMock serving a canned login response
  WiremockClient wm(WiremockClient::Mode::ForwardProxy);
  wm.add_mapping_file("auth/login_success_any.json");

  std::ostringstream extra;
  extra << "PROXY=;ALLOWEMPTYPROXY=true;USE_PROXY_ENV=true;";
  auto conn_str = build_unresolvable_connection_string(extra.str());

  // When SQLDriverConnect is invoked with empty PROXY and AllowEmptyProxy=true
  auto env = Connection::initEnv();
  auto dbc = env.createConnectionHandle();
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), nullptr, sqlchar(conn_str.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_NOPROMPT);

  // Then the connect fails and the proxy received no requests
  CHECK_THAT(OdbcResult(ret, dbc), OdbcMatchers::IsError());
  CHECK(wm.get_request_count("POST", "/session/v1/login-request") == 0);
}

TEST_CASE("should ignore HTTP_PROXY env var when USE_PROXY_ENV is not set", "[session][proxy]") {
#ifdef _WIN32
  SKIP("WireMock tests not yet validated on Windows");
#endif
  // Given HTTP_PROXY env var points at a forward-proxy WireMock
  WiremockClient wm(WiremockClient::Mode::ForwardProxy);
  wm.add_mapping_file("auth/login_success_any.json");

  std::string http_proxy_url = "http://localhost:" + std::to_string(wm.port());
  EnvOverride http_proxy_env("HTTP_PROXY", http_proxy_url);

  auto conn_str = build_unresolvable_connection_string("");

  // When SQLDriverConnect is invoked without USE_PROXY_ENV
  auto env = Connection::initEnv();
  auto dbc = env.createConnectionHandle();
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), nullptr, sqlchar(conn_str.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_NOPROMPT);

  // Then the connect fails and the proxy received no requests
  CHECK_THAT(OdbcResult(ret, dbc), OdbcMatchers::IsError());
  CHECK(wm.get_request_count("POST", "/session/v1/login-request") == 0);
}

TEST_CASE("should pick up HTTP_PROXY env var when USE_PROXY_ENV is true", "[session][proxy]") {
#ifdef _WIN32
  SKIP("WireMock tests not yet validated on Windows");
#endif
  SKIP_OLD_DRIVER("SNOW-2314158", "USE_PROXY_ENV is new in universal driver; old driver ignores it");
  // Given HTTP_PROXY env var points at a forward-proxy WireMock
  WiremockClient wm(WiremockClient::Mode::ForwardProxy);
  wm.add_mapping_file("auth/login_success_any.json");

  std::string http_proxy_url = "http://localhost:" + std::to_string(wm.port());
  EnvOverride http_proxy_env("HTTP_PROXY", http_proxy_url);

  auto conn_str = build_unresolvable_connection_string("USE_PROXY_ENV=true;");

  // When SQLDriverConnect is invoked with USE_PROXY_ENV=true
  auto env = Connection::initEnv();
  auto dbc = env.createConnectionHandle();
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), nullptr, sqlchar(conn_str.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_NOPROMPT);

  // Then the connect succeeds and the proxy received exactly one login request
  REQUIRE_ODBC(ret, dbc);
  CHECK(wm.get_request_count("POST", "/session/v1/login-request") == 1);
}
