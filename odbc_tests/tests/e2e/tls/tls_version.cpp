#include <sql.h>
#include <sqlext.h>

#include <sstream>
#include <string>

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_all.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"
#include "test_setup.hpp"

using Catch::Matchers::ContainsSubstring;

namespace {

SQLRETURN driver_connect(ConnectionHandleWrapper& dbc, const std::string& conn_str) {
  return SQLDriverConnect(dbc.getHandle(), NULL, sqlchar(conn_str.c_str()), SQL_NTS, NULL, 0, NULL,
                          SQL_DRIVER_NOPROMPT);
}

}  // namespace

// Scenario: should negotiate TLS when the server offers a version inside the window
TEST_CASE("should negotiate TLS when the server offers a version inside the window", "[tls]") {
  SKIP_OLD_DRIVER("", "New driver only. Old driver does not support MIN_TLS_VERSION and MAX_TLS_VERSION");
  // Given a TLS server that offers only TLS 1.3
  WiremockClient wm(WiremockClient::Mode::Server, WiremockClient::TlsVersion::Tls13);
  wm.add_mapping_file("auth/login_success_any.json");
  // And a client configured with min_tls_version tls12 and max_tls_version tls13
  auto conn_str = get_wiremock_https_connection_string(wm, "MIN_TLS_VERSION=tls12;MAX_TLS_VERSION=tls13;");
  // When a request is sent to the server
  auto env = Connection::initEnv();
  auto dbc = env.createConnectionHandle();
  SQLRETURN ret = driver_connect(dbc, conn_str);
  // Then the handshake succeeds
  REQUIRE_ODBC(ret, dbc);
}

// Scenario: should fail the handshake when the server only offers a version below the minimum
TEST_CASE("should fail the handshake when the server only offers a version below the minimum", "[tls]") {
  SKIP_OLD_DRIVER("", "New driver only. Old driver does not support MIN_TLS_VERSION and MAX_TLS_VERSION");
  // Given a TLS server that offers only TLS 1.2
  WiremockClient wm(WiremockClient::Mode::Server, WiremockClient::TlsVersion::Tls12);
  // And a client configured with min_tls_version tls13
  auto conn_str = get_wiremock_https_connection_string(wm, "MIN_TLS_VERSION=tls13;");
  // When a request is sent to the server
  auto env = Connection::initEnv();
  auto dbc = env.createConnectionHandle();
  SQLRETURN ret = driver_connect(dbc, conn_str);
  // Then the handshake fails
  REQUIRE(ret == SQL_ERROR);
}

TEST_CASE("should reject the configuration when the minimum exceeds the maximum", "[tls]") {
  SKIP_OLD_DRIVER("", "New driver only. Old driver does not support MIN_TLS_VERSION and MAX_TLS_VERSION");
  // Given settings with min_tls_version tls13 and max_tls_version tls12
  std::ostringstream ss;
  configure_driver_string(ss);
  ss << "SERVER=localhost;PORT=443;ACCOUNT=testaccount;UID=testuser;PWD=testpass;";
  ss << "SSL=on;VERIFY_CERTIFICATES=false;DisableOCSPCheck=true;";
  ss << "MIN_TLS_VERSION=tls13;MAX_TLS_VERSION=tls12;";
  auto conn_str = ss.str();
  // When the TLS configuration is built from settings
  auto env = Connection::initEnv();
  auto dbc = env.createConnectionHandle();
  SQLRETURN ret = driver_connect(dbc, conn_str);
  // Then a configuration error is returned
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(dbc);
  REQUIRE(!records.empty());
  CHECK_THAT(records[0].messageText, ContainsSubstring("max_tls_version"));
}
