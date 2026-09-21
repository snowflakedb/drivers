#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <sstream>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_all.hpp>

#include "HandleWrapper.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_matchers.hpp"
#include "test_setup.hpp"

using Catch::Matchers::ContainsSubstring;

std::string wif_connection_string(const std::string& extra) {
  std::stringstream ss;
  configure_driver_string(ss);
  ss << "SERVER=localhost;";
  ss << "ACCOUNT=test_account;";
  ss << "UID=test_user;";
  ss << "AUTHENTICATOR=workload_identity;";
  ss << extra;
  return ss.str();
}

SQLRETURN connect_wif(const std::string& connection_string, ConnectionHandleWrapper& dbc) {
  return SQLDriverConnect(dbc.getHandle(), NULL, (SQLCHAR*)connection_string.c_str(), SQL_NTS, NULL, 0, NULL,
                          SQL_DRIVER_NOPROMPT);
}

EnvironmentHandleWrapper setup_wif_environment() {
  ensure_driver_installed();
  EnvironmentHandleWrapper env;
  SQLRETURN env_ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
  REQUIRE_ODBC(env_ret, env);
  return env;
}

std::vector<DiagRec> require_connect_error(ConnectionHandleWrapper& dbc, const std::string& connection_string) {
  SQLRETURN ret = connect_wif(connection_string, dbc);
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(dbc);
  REQUIRE(records.size() >= 1);
  for (const auto& record : records) {
    CHECK_THAT(record.messageText, !ContainsSubstring("Can't open lib"));
    CHECK_THAT(record.messageText, !ContainsSubstring("Data source name not found and no default driver specified"));
  }
  return records;
}

void require_3x_wif_connect_sqlstate(const DiagRec& rec) {
  CHECK((rec.sqlState == "08001" || rec.sqlState == "HY000" || rec.sqlState == "28000"));
}

void require_4x_wif_client_reject(const DiagRec& rec, const char* needle) {
  CHECK(rec.sqlState == "28000");
  CHECK(rec.nativeError == 0);
  CHECK_THAT(rec.messageText, ContainsSubstring(needle));
}

bool mentions_malformed_wif_token(const std::vector<DiagRec>& records) {
  for (const auto& record : records) {
    INFO("sqlState=" << record.sqlState << " nativeError=" << record.nativeError
                     << " messageText=" << record.messageText);
    const auto& text = record.messageText;
    if (text.find("jwt") != std::string::npos || text.find("JWT") != std::string::npos ||
        text.find("malformed") != std::string::npos || text.find("Malformed") != std::string::npos ||
        text.find("invalid token") != std::string::npos || text.find("attestation") != std::string::npos) {
      return true;
    }
  }
  return false;
}

TEST_CASE("should fail WORKLOAD_IDENTITY when provider is missing", "[workload_identity_auth]") {
  // Given Authentication is set to WORKLOAD_IDENTITY but WORKLOAD_IDENTITY_PROVIDER is absent
  auto env = setup_wif_environment();
  auto dbc = env.createConnectionHandle();
  const std::string connection_string = wif_connection_string("");

  // When Trying to Connect
  auto records = require_connect_error(dbc, connection_string);

  // Then Connection fails with a missing-parameter error citing workload_identity_provider
  OLD_DRIVER_ONLY("BD#1") {
    CHECK(records[0].sqlState == "28000");
    CHECK(records[0].nativeError == 20032);
    CHECK_THAT(records[0].messageText, ContainsSubstring("Required setting 'WORKLOAD_IDENTITY_PROVIDER'"));
  }

  NEW_DRIVER_ONLY("BD#1") {
    CHECK(records[0].sqlState == "28000");
    CHECK(records[0].nativeError == 0);
    CHECK_THAT(records[0].messageText, ContainsSubstring("workload_identity_provider"));
  }
}

TEST_CASE("should fail WORKLOAD_IDENTITY when provider is an invalid value", "[workload_identity_auth]") {
  // Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_PROVIDER is an invalid value
  auto env = setup_wif_environment();
  auto dbc = env.createConnectionHandle();
  const std::string connection_string = wif_connection_string("WORKLOAD_IDENTITY_PROVIDER=INVALID_CLOUD;");

  // When Trying to Connect
  auto records = require_connect_error(dbc, connection_string);

  // Then Connection fails with an invalid-parameter error citing workload_identity_provider
  OLD_DRIVER_ONLY("BD#155") { require_3x_wif_connect_sqlstate(records[0]); }

  NEW_DRIVER_ONLY("BD#155") { require_4x_wif_client_reject(records[0], "workload_identity_provider"); }
}

TEST_CASE("should fail OIDC WIF when token is missing", "[workload_identity_auth]") {
  // Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_PROVIDER is OIDC but token is absent
  auto env = setup_wif_environment();
  auto dbc = env.createConnectionHandle();
  const std::string connection_string = wif_connection_string("WORKLOAD_IDENTITY_PROVIDER=OIDC;");

  // When Trying to Connect
  auto records = require_connect_error(dbc, connection_string);

  // Then Connection fails with a missing-parameter error citing token
  OLD_DRIVER_ONLY("BD#155") { require_3x_wif_connect_sqlstate(records[0]); }

  NEW_DRIVER_ONLY("BD#155") { require_4x_wif_client_reject(records[0], "token"); }
}

TEST_CASE("should fail OIDC WIF when token is malformed", "[workload_identity_auth]") {
  // Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_PROVIDER is OIDC
  auto env = setup_wif_environment();
  auto dbc = env.createConnectionHandle();
  // And Token is set to a malformed value that is not a valid JWT
  const std::string connection_string = wif_connection_string("WORKLOAD_IDENTITY_PROVIDER=OIDC;TOKEN=not-a-valid-jwt;");

  // When Trying to Connect
  auto records = require_connect_error(dbc, connection_string);

  // Then Connection fails with an attestation error indicating a malformed token
  OLD_DRIVER_ONLY("BD#155") { require_3x_wif_connect_sqlstate(records[0]); }

  NEW_DRIVER_ONLY("BD#155") {
    CHECK(records[0].sqlState == "28000");
    CHECK(records[0].nativeError == 0);
    CHECK(mentions_malformed_wif_token(records));
  }
}
