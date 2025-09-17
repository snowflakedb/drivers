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
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "macros.hpp"
#include "test_setup.hpp"

std::string get_jwt_connection_string_without_private_key() {
  std::stringstream ss;
  ss << "DRIVER=" << get_driver_path() << ";";
  ss << "SERVER=localhost;";
  ss << "ACCOUNT=test_account;";
  ss << "UID=test_user;";
  ss << "DATABASE=test_database;";
  ss << "SCHEMA=test_schema;";
  ss << "WAREHOUSE=test_warehouse;";
  ss << "ROLE=test_role;";
  ss << "PORT=8090;";
  ss << "AUTHENTICATOR=SNOWFLAKE_JWT;";
  // Deliberately omit PRIV_KEY_FILE parameter
  return ss.str();
}

EnvironmentHandleWrapper setup_environment_integration() {
  EnvironmentHandleWrapper env;
  SQLRETURN ret =
      SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
  CHECK_ODBC(ret, env);
  return env;
}

ConnectionHandleWrapper get_connection_handle_integration(EnvironmentHandleWrapper& env) {
  return env.createConnectionHandle();
}

SQLRETURN attempt_connection_expect_error_integration(ConnectionHandleWrapper& dbc,
                                                      const std::string& connection_string) {
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), NULL, (SQLCHAR*)connection_string.c_str(),
                                   SQL_NTS, NULL, 0, NULL, SQL_DRIVER_NOPROMPT);
  REQUIRE(ret == SQL_ERROR);
  return ret;
}

void verify_connection_fails_with_missing_private_key_error(ConnectionHandleWrapper& dbc,
                                                            const std::string& connection_string) {
  attempt_connection_expect_error_integration(dbc, connection_string);

  auto records = get_diag_rec(dbc);
  REQUIRE(records.size() == 1);  // Expecting one error record
  CHECK(records[0].sqlState == "28000");
  using Catch::Matchers::ContainsSubstring;
  OLD_DRIVER_ONLY("BC#1") {
    CHECK(records[0].nativeError == 20032);
    CHECK_THAT(records[0].messageText, ContainsSubstring("Required setting 'PRIV_KEY_FILE'"));
  }

  NEW_DRIVER_ONLY("BC#1") {
    CHECK(records[0].nativeError == 0);
    CHECK_THAT(records[0].messageText,
               ContainsSubstring("Missing required parameter: private_key_file"));
  }
}

TEST_CASE("should fail JWT authentication when no private file provided", "[private_key_auth]") {
  // Given Authentication is set to JWT
  auto env = setup_environment_integration();
  auto dbc = get_connection_handle_integration(env);

  // When Trying to Connect with no private file provided
  std::string connection_string = get_jwt_connection_string_without_private_key();

  // Then There is error returned
  verify_connection_fails_with_missing_private_key_error(dbc, connection_string);
}
