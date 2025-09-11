#include <picojson.h>
#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <fstream>
#include <iostream>
#include <sstream>

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_all.hpp>

#include "HandleWrapper.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "macros.hpp"
#include "test_setup.hpp"

class PrivateKeyAuthTest {
 public:
  std::string get_private_key_path_for_auth(picojson::object& params) {
    auto private_key = read_private_key(params);
    const std::string path = "./rsa_key_auth.p8";
    std::ofstream file(path, std::ios::out | std::ios::trunc);
    REQUIRE(file.is_open());
    file << private_key;
    file.close();
    return path;
  }

  std::string get_jwt_connection_string_with_private_key() {
    auto params = get_test_parameters("testconnection");
    std::stringstream ss;
    read_default_params(ss, params);
    add_param_optional<std::string>(ss, params, "SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD",
                                    "PRIV_KEY_FILE_PWD");
    ss << "AUTHENTICATOR=SNOWFLAKE_JWT;";
    ss << "PRIV_KEY_FILE=" << get_private_key_path_for_auth(params) << ";";
    return ss.str();
  }

  std::string get_jwt_connection_string_without_private_key() {
    auto params = get_test_parameters("testconnection");
    std::stringstream ss;
    ss << "DRIVER=" << get_driver_path() << ";";
    add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_HOST", "SERVER");
    add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_ACCOUNT", "ACCOUNT");
    add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_USER", "UID");
    ss << "AUTHENTICATOR=SNOWFLAKE_JWT;";
    // Deliberately omit PRIV_KEY_FILE parameter
    return ss.str();
  }

  EnvironmentHandleWrapper setup_environment() {
    EnvironmentHandleWrapper env;
    SQLRETURN ret =
        SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
    CHECK_ODBC(ret, env);
    return env;
  }

  ConnectionHandleWrapper get_connection_handle(EnvironmentHandleWrapper& env) {
    return env.createConnectionHandle();
  }

  void attempt_connection(ConnectionHandleWrapper& dbc, const std::string& connection_string) {
    SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), NULL, (SQLCHAR*)connection_string.c_str(),
                                     SQL_NTS, NULL, 0, NULL, SQL_DRIVER_NOPROMPT);
    CHECK_ODBC(ret, dbc);
  }

  void verify_simple_query_execution(ConnectionHandleWrapper& dbc) {
    StatementHandleWrapper stmt = dbc.createStatementHandle();
    SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 1", SQL_NTS);
    CHECK_ODBC(ret, stmt);

    ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);

    SQLINTEGER result = 0;
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &result, sizeof(result), NULL);
    CHECK_ODBC(ret, stmt);
    REQUIRE(result == 1);
  }

  void verify_connection_fails_with_missing_private_key_error(
      ConnectionHandleWrapper& dbc, const std::string& connection_string) {
    SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), NULL, (SQLCHAR*)connection_string.c_str(),
                                     SQL_NTS, NULL, 0, NULL, SQL_DRIVER_NOPROMPT);
    REQUIRE(ret == SQL_ERROR);

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
};

TEST_CASE("should authenticate using private file with password", "[private_key_auth]") {
  PrivateKeyAuthTest test;

  // Given Authentication is set to JWT
  auto env = test.setup_environment();
  auto dbc = test.get_connection_handle(env);

  // And Private file with password is provided
  std::string connection_string = test.get_jwt_connection_string_with_private_key();

  // When Trying to Connect
  test.attempt_connection(dbc, connection_string);

  // Then Login is successful and simple query can be executed
  test.verify_simple_query_execution(dbc);

  SQLDisconnect(dbc.getHandle());
}

TEST_CASE("should fail JWT authentication when no private file provided", "[private_key_auth]") {
  PrivateKeyAuthTest test;

  // Given Authentication is set to JWT
  auto env = test.setup_environment();
  auto dbc = test.get_connection_handle(env);

  // When Trying to Connect with no private file provided
  std::string connection_string = test.get_jwt_connection_string_without_private_key();

  // Then There is error returned
  test.verify_connection_fails_with_missing_private_key_error(dbc, connection_string);
}
