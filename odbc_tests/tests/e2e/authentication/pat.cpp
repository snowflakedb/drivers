#include <picojson.h>
#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <iostream>
#include <random>
#include <sstream>

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_all.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_matchers.hpp"
#include "require.hpp"
#include "test_setup.hpp"

using namespace Catch::Matchers;

EnvironmentHandleWrapper setup_pat_environment() {
  EnvironmentHandleWrapper env;
  SQLRETURN ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
  REQUIRE_ODBC(ret, env);
  return env;
}

ConnectionHandleWrapper get_pat_connection_handle(EnvironmentHandleWrapper& env) {
  return env.createConnectionHandle();
}

std::string get_pat_as_password_connection_string(const std::string& pat_secret) {
  auto params = get_test_parameters("testconnection");
  std::stringstream ss;
  configure_driver_string(ss);
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_HOST", "SERVER");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_ACCOUNT", "ACCOUNT");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_USER", "UID");
  ss << "PWD=" << pat_secret << ";";
  return ss.str();
}

std::string get_pat_as_token_connection_string(const std::string& pat_secret) {
  auto params = get_test_parameters("testconnection");
  std::stringstream ss;
  configure_driver_string(ss);
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_HOST", "SERVER");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_ACCOUNT", "ACCOUNT");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_USER", "UID");
  ss << "AUTHENTICATOR=PROGRAMMATIC_ACCESS_TOKEN;";
  ss << "TOKEN=" << pat_secret << ";";
  return ss.str();
}

void attempt_pat_connection(ConnectionHandleWrapper& dbc, const std::string& connection_string) {
  SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), NULL, (SQLCHAR*)connection_string.c_str(), SQL_NTS, NULL, 0, NULL,
                                   SQL_DRIVER_NOPROMPT);
  REQUIRE_ODBC(ret, dbc);
}

void verify_pat_simple_query_execution(ConnectionHandleWrapper& dbc) {
  StatementHandleWrapper stmt = dbc.createStatementHandle();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 1", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  SQLINTEGER result = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &result, sizeof(result), NULL);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(result == 1);
}

TEST_CASE("should authenticate using PAT as password", "[pat]") {
  // Given Authentication is set to password and valid PAT token is provided
  auto params = get_test_parameters("testconnection");
  std::string pat_secret = params.at("SNOWFLAKE_TEST_PAT").get<std::string>();
  // Build the connection string before allocating the environment handle: configure_driver_string()
  // installs the driver alias and sets ODBCSYSINI/ODBCINI, which the driver manager only honours if
  // they are in place before SQLAllocHandle(ENV).
  std::string connection_string = get_pat_as_password_connection_string(pat_secret);

  auto env = setup_pat_environment();
  auto dbc = get_pat_connection_handle(env);

  // When Trying to Connect
  attempt_pat_connection(dbc, connection_string);

  // Then Login is successful and simple query can be executed
  verify_pat_simple_query_execution(dbc);

  SQLRETURN disconnect_ret = SQLDisconnect(dbc.getHandle());
  REQUIRE((disconnect_ret == SQL_SUCCESS || disconnect_ret == SQL_SUCCESS_WITH_INFO));
}

TEST_CASE("should authenticate using PAT as token", "[pat]") {
  // Given Authentication is set to Programmatic Access Token and valid PAT token is provided
  auto params = get_test_parameters("testconnection");
  std::string pat_secret = params.at("SNOWFLAKE_TEST_PAT").get<std::string>();
  // Build the connection string before allocating the environment handle so the driver alias and
  // ODBCSYSINI/ODBCINI installed by configure_driver_string() are visible to the driver manager.
  std::string connection_string = get_pat_as_token_connection_string(pat_secret);

  auto env = setup_pat_environment();
  auto dbc = get_pat_connection_handle(env);

  // When Trying to Connect
  attempt_pat_connection(dbc, connection_string);

  // Then Login is successful and simple query can be executed
  verify_pat_simple_query_execution(dbc);

  SQLRETURN disconnect_ret = SQLDisconnect(dbc.getHandle());
  REQUIRE((disconnect_ret == SQL_SUCCESS || disconnect_ret == SQL_SUCCESS_WITH_INFO));
}

TEST_CASE("should authenticate using PAT as token with lowercase authenticator", "[pat]") {
  // Given Authentication is set to lowercase programmatic_access_token and valid PAT token is provided
  auto params = get_test_parameters("testconnection");
  std::string pat_secret = params.at("SNOWFLAKE_TEST_PAT").get<std::string>();

  std::string connection_string = get_pat_as_token_connection_string(pat_secret);
  const std::string upper_auth = "AUTHENTICATOR=PROGRAMMATIC_ACCESS_TOKEN;";
  const std::string lower_auth = "AUTHENTICATOR=programmatic_access_token;";
  auto auth_pos = connection_string.find(upper_auth);
  if (auth_pos != std::string::npos) {
    connection_string.replace(auth_pos, upper_auth.size(), lower_auth);
  }

  auto env = setup_pat_environment();
  auto dbc = get_pat_connection_handle(env);

  // When Trying to Connect
  attempt_pat_connection(dbc, connection_string);

  // Then Login is successful and simple query can be executed
  verify_pat_simple_query_execution(dbc);

  SQLRETURN disconnect_ret = SQLDisconnect(dbc.getHandle());
  REQUIRE((disconnect_ret == SQL_SUCCESS || disconnect_ret == SQL_SUCCESS_WITH_INFO));
}

TEST_CASE("should fail PAT authentication when invalid token provided", "[pat]") {
  // Given Authentication is set to Programmatic Access Token and invalid PAT token is provided
  std::string connection_string = get_pat_as_token_connection_string("invalid_token_12345");

  // When Trying to Connect
  auto records = require_connection_failed(connection_string);

  // Then There is error returned
  REQUIRE(records.size() == 1);
  CHECK(records[0].sqlState == "28000");
  CHECK(records[0].nativeError == 394400);
  CHECK_THAT(records[0].messageText, ContainsSubstring("Programmatic access token is invalid"));
}

TEST_CASE("should handle ALTER USER PAT result set: new driver returns token, old driver returns cursor state error",
          "[pat]") {
  // Given ALTER USER ADD PROGRAMMATIC ACCESS TOKEN is executed
  Connection conn;
  auto params = get_test_parameters("testconnection");
  std::string user = params.at("SNOWFLAKE_TEST_USER").get<std::string>();
  std::string role = params.at("SNOWFLAKE_TEST_ROLE").get<std::string>();

  std::random_device rd;
  std::mt19937 gen(rd());
  std::uniform_int_distribution<uint32_t> dis;
  std::string token_name = "UD_ODBC_BD7_" + std::to_string(dis(gen));

  struct PatCleanup {
    Connection& conn;
    const std::string& user;
    const std::string& token_name;
    ~PatCleanup() {
      try {
        conn.try_execute("ALTER USER IF EXISTS " + user + " REMOVE PROGRAMMATIC ACCESS TOKEN " + token_name);
      } catch (const std::exception& e) {
        std::cerr << "PAT cleanup failed: " << e.what() << std::endl;
      } catch (...) {
        std::cerr << "PAT cleanup failed with unknown exception" << std::endl;
      }
    }
  } cleanup{conn, user, token_name};

  auto stmt = conn.execute("ALTER USER IF EXISTS " + user + " ADD PROGRAMMATIC ACCESS TOKEN " + token_name +
                           " ROLE_RESTRICTION = " + role);

  // When SQLFetch is called on the ALTER USER result
  SQLRETURN ret = SQLFetch(stmt.getHandle());

  // Then The old driver returns invalid cursor state, the new driver returns the token
  OLD_DRIVER_ONLY("BD#7") {
    CHECK(ret == SQL_ERROR);
    auto diag = get_diag_rec(stmt);
    REQUIRE(diag.size() == 1);
    CHECK(diag[0].sqlState == "24000");
    CHECK(diag[0].nativeError == 10510);
    CHECK_THAT(diag[0].messageText, ContainsSubstring("Invalid cursor state"));
  }

  NEW_DRIVER_ONLY("BD#7") {
    REQUIRE(ret == SQL_SUCCESS);

    SQLCHAR name_buf[256];
    SQLLEN name_len = SQL_NULL_DATA;
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, name_buf, sizeof(name_buf), &name_len);
    REQUIRE_ODBC(ret, stmt);
    REQUIRE(name_len != SQL_NULL_DATA);
    CHECK(std::string(reinterpret_cast<char*>(name_buf), name_len) == token_name);

    SQLCHAR secret_buf[1024];
    SQLLEN secret_len = SQL_NULL_DATA;
    ret = SQLGetData(stmt.getHandle(), 2, SQL_C_CHAR, secret_buf, sizeof(secret_buf), &secret_len);
    REQUIRE_ODBC(ret, stmt);
    REQUIRE(secret_len != SQL_NULL_DATA);
    CHECK(secret_len > 0);
  }
}
