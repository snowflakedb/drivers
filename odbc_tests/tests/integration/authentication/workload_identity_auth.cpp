#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <sstream>
#include <string>

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_all.hpp>

#include "HandleWrapper.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_matchers.hpp"
#include "test_setup.hpp"

using Catch::Matchers::ContainsSubstring;

// No live server/cloud metadata service required: WORKLOAD_IDENTITY_PROVIDER
// is validated client-side before any network call is made, on both drivers.
std::string get_wif_connection_string_without_provider() {
  std::stringstream ss;
  configure_driver_string(ss);
  ss << "SERVER=localhost;";
  ss << "ACCOUNT=test_account;";
  ss << "UID=test_user;";
  ss << "AUTHENTICATOR=workload_identity;";
  return ss.str();
}

// ============================================================================
// Integration test: WORKLOAD_IDENTITY_PROVIDER is required for WIF auth
// ============================================================================
//
// Old driver: Tests/AuthenticationTests/WIFLatestTest/WIFLatestTest.cpp
// ("shouldn't authenticate using wif without explicit provider") — verified
// directly against that source: SQLSTATE 28000, message "Required setting
// 'WORKLOAD_IDENTITY_PROVIDER' is not present in the connection settings".
//
// New driver: sf_core::config::connection_config::validate_settings reports
// ValidationCode::MissingRequired for "workload_identity_provider"
// (validate_wif_missing_provider_reports_issue unit test), which the
// connection layer surfaces as SQLSTATE 01S00.
TEST_CASE("should fail workload identity authentication when provider is missing", "[workload_identity_auth]") {
  // Given a connection string with Authenticator=workload_identity but no
  // WORKLOAD_IDENTITY_PROVIDER
  // unixODBC caches odbcinst.ini at env-alloc; install the driver first.
  ensure_driver_installed();
  auto env = EnvironmentHandleWrapper();
  SQLRETURN env_ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
  REQUIRE_ODBC(env_ret, env);

  auto dbc = env.createConnectionHandle();
  const std::string connection_string = get_wif_connection_string_without_provider();

  // When Trying to Connect
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), NULL, (SQLCHAR*)connection_string.c_str(), SQL_NTS, NULL, 0, NULL,
                                   SQL_DRIVER_NOPROMPT);

  // Then Connection fails client-side, before any network call
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(dbc);
  REQUIRE(records.size() >= 1);

  OLD_DRIVER_ONLY("BD#1") {
    CHECK(records[0].sqlState == "28000");
    CHECK_THAT(records[0].messageText, ContainsSubstring("Required setting 'WORKLOAD_IDENTITY_PROVIDER'"));
  }

  NEW_DRIVER_ONLY("BD#1") {
    CHECK(records[0].sqlState == "01S00");
    CHECK_THAT(records[0].messageText, ContainsSubstring("workload_identity_provider"));
  }
}
