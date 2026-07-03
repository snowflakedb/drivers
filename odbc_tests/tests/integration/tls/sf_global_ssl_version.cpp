#include <sql.h>
#include <sqlext.h>

#include <sstream>
#include <string>

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_all.hpp>

#include "Connection.hpp"
#include "EnvOverride.hpp"
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

TEST_CASE("SF_GLOBAL_SSL_VERSION=TLSv1_3 connects to a TLS 1.3 server", "[tls]") {
  WiremockClient wm(WiremockClient::Mode::Server, WiremockClient::TlsVersion::Tls13);
  wm.add_mapping_file("auth/login_success_any.json");
  EnvOverride ssl_version("SF_GLOBAL_SSL_VERSION", "TLSv1_3");
  auto conn_str = get_wiremock_https_connection_string(wm);
  auto env = Connection::initEnv();
  auto dbc = env.createConnectionHandle();
  SQLRETURN ret = driver_connect(dbc, conn_str);
  REQUIRE_ODBC(ret, dbc);
}

TEST_CASE("SF_GLOBAL_SSL_VERSION=TLSv1_2 connects to a TLS 1.2 server", "[tls]") {
  WiremockClient wm(WiremockClient::Mode::Server, WiremockClient::TlsVersion::Tls12);
  wm.add_mapping_file("auth/login_success_any.json");
  EnvOverride ssl_version("SF_GLOBAL_SSL_VERSION", "TLSv1_2");
  auto conn_str = get_wiremock_https_connection_string(wm);
  auto env = Connection::initEnv();
  auto dbc = env.createConnectionHandle();
  SQLRETURN ret = driver_connect(dbc, conn_str);
  REQUIRE_ODBC(ret, dbc);
}

TEST_CASE("SF_GLOBAL_SSL_VERSION=TLSv1_3 fails to connect to a TLS 1.2-only server", "[tls]") {
  WiremockClient wm(WiremockClient::Mode::Server, WiremockClient::TlsVersion::Tls12);
  EnvOverride ssl_version("SF_GLOBAL_SSL_VERSION", "TLSv1_3");
  auto conn_str = get_wiremock_https_connection_string(wm);
  auto env = Connection::initEnv();
  auto dbc = env.createConnectionHandle();
  SQLRETURN ret = driver_connect(dbc, conn_str);
  REQUIRE(ret == SQL_ERROR);
}

TEST_CASE("SF_GLOBAL_SSL_VERSION unset does not restrict TLS version negotiation", "[tls]") {
  WiremockClient wm(WiremockClient::Mode::Server, WiremockClient::TlsVersion::Tls12);
  wm.add_mapping_file("auth/login_success_any.json");
  EnvOverride ssl_version("SF_GLOBAL_SSL_VERSION");
  auto conn_str = get_wiremock_https_connection_string(wm);
  auto env = Connection::initEnv();
  auto dbc = env.createConnectionHandle();
  SQLRETURN ret = driver_connect(dbc, conn_str);
  REQUIRE_ODBC(ret, dbc);
}

TEST_CASE("SF_GLOBAL_SSL_VERSION with unrecognized value fails with a configuration error", "[tls]") {
  SKIP_OLD_DRIVER("", "New driver only check");
  EnvOverride ssl_version("SF_GLOBAL_SSL_VERSION", "TLSv99");
  std::ostringstream ss;
  configure_driver_string(ss);
  ss << "SERVER=localhost;PORT=443;ACCOUNT=testaccount;UID=testuser;PWD=testpass;";
  ss << "SSL=on;VERIFY_CERTIFICATES=false;DisableOCSPCheck=true;";
  auto env = Connection::initEnv();
  auto dbc = env.createConnectionHandle();
  SQLRETURN ret = driver_connect(dbc, ss.str());
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(dbc);
  REQUIRE(!records.empty());
  CHECK_THAT(records[0].messageText, ContainsSubstring("SF_GLOBAL_SSL_VERSION"));
}

TEST_CASE("SF_GLOBAL_SSL_VERSION overrides the configured TLS version window", "[tls]") {
  SKIP_OLD_DRIVER("", "New driver only check");
  WiremockClient wm(WiremockClient::Mode::Server, WiremockClient::TlsVersion::Tls12);
  wm.add_mapping_file("auth/login_success_any.json");
  EnvOverride global_ssl_version("SF_GLOBAL_SSL_VERSION", "TLSv1_3");
  auto conn_str = get_wiremock_https_connection_string(wm, "MIN_TLS_VERSION=tls12;MAX_TLS_VERSION=tls12;");
  auto env = Connection::initEnv();
  auto dbc = env.createConnectionHandle();
  SQLRETURN ret = driver_connect(dbc, conn_str);
  REQUIRE(ret == SQL_ERROR);
}
