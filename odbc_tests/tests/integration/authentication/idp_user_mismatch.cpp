#include <sql.h>
#include <sqlext.h>

#include <string>
#include <thread>

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_all.hpp>

#include "Connection.hpp"
#include "EnvOverride.hpp"
#include "HandleWrapper.hpp"
#include "WiremockClient.hpp"
#include "compatibility.hpp"
#include "external_browser_test_helpers.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"

using Catch::Matchers::ContainsSubstring;

// =============================================================================
// BD#140: IDP-user mismatch
// =============================================================================

/// GS rejects an external-browser login whose IdP session belongs to a user
/// other than the one named on the connection, answering with code 390191 and
/// a message naming the mismatch. This pins what the 4.x driver makes of that
/// response; the 3.x message quoted in BD#140 comes from a real IdP round trip,
/// which the old driver cannot reach here - suppressing the browser launch
/// relies on SF_TEST_BROWSER_OPENER, which only sf_core reads.
TEST_CASE("should reject external browser login when the IdP user differs from the connection user",
          "[external_browser_auth][idp_user_mismatch]") {
  SKIP_OLD_DRIVER("BD#140", "Old driver has no SF_TEST_BROWSER_OPENER override, so it cannot run the mocked flow");
  EnvOverride browser_env("SF_TEST_BROWSER_OPENER", "noop");

  // Given Wiremock returns valid ssoUrl and proofKey for authenticator-request
  WiremockClient wm;
  wm.add_mapping_file("auth/external_browser_authenticator_request.json");

  // And the login endpoint answers with the USERNAMES_MISMATCH rejection
  wm.add_mapping_file("auth/login_failure_external_browser_usernames_mismatch.json");

  // When connecting as a user the IdP session does not belong to
  auto conn_str = external_browser_test::get_external_browser_connection_string(wm, "connection_user");
  std::string token = "browser_sso_token_mismatched_user";

  std::thread callback_thread([&wm, &token]() { external_browser_test::simulate_browser_callback(wm, token); });

  auto env = Connection::initEnv();
  ConnectionHandleWrapper dbc = env.createConnectionHandle();
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), nullptr, sqlchar(conn_str.c_str()), SQL_NTS, nullptr, 0, nullptr,
                                   SQL_DRIVER_NOPROMPT);

  callback_thread.join();

  // Then the connection is rejected under the GS code for the mismatch
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(dbc);
  REQUIRE(!records.empty());
  CHECK(records[0].sqlState == "28000");
  CHECK(records[0].nativeError == 390191);

  // And the first line carries the login context, the sentence naming the
  // mismatch and the GS code together, so all three reach the application
  // whether or not the error trace is appended below them.
  const std::string& message = records[0].messageText;
  std::string headline = message.substr(0, message.find('\n'));
  CHECK_THAT(headline, ContainsSubstring("Failed to login"));
  CHECK_THAT(headline, ContainsSubstring("Login error:"));
  CHECK_THAT(headline, ContainsSubstring("differs from the user currently logged in at the IDP"));
  CHECK_THAT(headline, ContainsSubstring("390191"));
}
