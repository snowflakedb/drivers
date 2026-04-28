#include <picojson.h>
#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <fstream>
#include <iostream>
#include <sstream>

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_all.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "ODBCConfig.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_matchers.hpp"
#include "put_get_utils.hpp"
#include "sf_odbc.h"
#include "test_setup.hpp"
#include "utils.hpp"

using pg_utils::TempTestDir;

std::string get_jwt_connection_string_without_private_key() {
  std::stringstream ss;
  configure_driver_string(ss);
  ss << "SERVER=localhost;";
  ss << "ACCOUNT=test_account;";
  ss << "UID=test_user;";
  ss << "DATABASE=test_database;";
  ss << "SCHEMA=test_schema;";
  ss << "WAREHOUSE=test_warehouse;";
  ss << "ROLE=test_role;";
  ss << "PORT=8090;";
  ss << "AUTHENTICATOR=SNOWFLAKE_JWT;";
  return ss.str();
}

EnvironmentHandleWrapper setup_environment_integration() {
  ensure_driver_installed();
  EnvironmentHandleWrapper env;
  SQLRETURN ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
  REQUIRE_ODBC(ret, env);
  return env;
}

ConnectionHandleWrapper get_connection_handle_integration(EnvironmentHandleWrapper& env) {
  return env.createConnectionHandle();
}

SQLRETURN attempt_connection_expect_error_integration(ConnectionHandleWrapper& dbc,
                                                      const std::string& connection_string) {
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), NULL, (SQLCHAR*)connection_string.c_str(), SQL_NTS, NULL, 0, NULL,
                                   SQL_DRIVER_NOPROMPT);
  // connection failure is expected as the test is not E2E test
  REQUIRE(ret == SQL_ERROR);

  // however driver/environment setup error is unwanted
  auto records = get_diag_rec(dbc);
  using Catch::Matchers::ContainsSubstring;
  for (const auto& record : records) {
    CHECK_THAT(record.messageText, !ContainsSubstring("Can't open lib"));
    CHECK_THAT(record.messageText, !ContainsSubstring("Data source name not found and no default driver specified"));
  }
  return ret;
}

void verify_connection_fails_with_missing_private_key_error(ConnectionHandleWrapper& dbc,
                                                            const std::string& connection_string) {
  attempt_connection_expect_error_integration(dbc, connection_string);

  auto records = get_diag_rec(dbc);
  REQUIRE(records.size() == 1);
  using Catch::Matchers::ContainsSubstring;
  OLD_DRIVER_ONLY("BD#1") {
    CHECK(records[0].sqlState == "28000");
    CHECK(records[0].nativeError == 20032);
    CHECK_THAT(records[0].messageText, ContainsSubstring("Required setting 'PRIV_KEY_FILE'"));
  }

  NEW_DRIVER_ONLY("BD#1") {
    CHECK(records[0].sqlState == "01S00");
    CHECK(records[0].nativeError == 0);
    CHECK_THAT(records[0].messageText,
               ContainsSubstring("Missing required parameter: 'private_key' or 'private_key_file'"));
  }
}

// ============================================================================
// Integration test: missing parameter error
// ============================================================================

TEST_CASE("should fail JWT authentication when no private file provided", "[private_key_auth]") {
  // Given Authentication is set to JWT
  std::string connection_string = get_jwt_connection_string_without_private_key();

  // When Trying to Connect with no private file provided
  auto env = setup_environment_integration();
  auto dbc = get_connection_handle_integration(env);

  // Then There is error returned
  verify_connection_fails_with_missing_private_key_error(dbc, connection_string);
}
