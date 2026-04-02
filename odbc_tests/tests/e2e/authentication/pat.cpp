#include <picojson.h>
#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <iomanip>
#include <optional>
#include <random>
#include <sstream>

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_all.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "odbc_matchers.hpp"
#include "require.hpp"
#include "test_setup.hpp"

using namespace Catch::Matchers;

struct PatResult {
  std::string token_name;
  std::string token_secret;
  SQLRETURN fetch_ret;
  std::vector<DiagRec> diag_records;
};

class PatSetup {
 private:
  std::string token_name;
  std::string user;
  std::string role;
  Connection connection;

 public:
  PatSetup() : connection(Connection()) {}

  ~PatSetup() { cleanup(); }

  PatResult acquire() {
    PatResult result;

    std::random_device rd;
    std::mt19937 gen(rd());
    std::uniform_int_distribution<uint32_t> dis;
    uint32_t random_number = dis(gen);
    std::stringstream ss;
    ss << "pat_" << std::hex << std::setw(8) << std::setfill('0') << random_number;
    token_name = ss.str();
    result.token_name = token_name;

    auto params = get_test_parameters("testconnection");
    user = params.at("SNOWFLAKE_TEST_USER").get<std::string>();
    role = params.at("SNOWFLAKE_TEST_ROLE").get<std::string>();

    std::stringstream create_sql;
    create_sql << "ALTER USER IF EXISTS " << user << " ADD PROGRAMMATIC ACCESS TOKEN " << token_name
               << " ROLE_RESTRICTION = " << role;

    auto stmt = connection.execute(create_sql.str());

    result.fetch_ret = SQLFetch(stmt.getHandle());

    if (result.fetch_ret != SQL_SUCCESS) {
      result.diag_records = get_diag_rec(stmt);
      return result;
    }

    SQLCHAR token_name_buffer[256];
    SQLLEN token_name_length;
    SQLRETURN ret =
        SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, token_name_buffer, sizeof(token_name_buffer), &token_name_length);
    REQUIRE_ODBC(ret, stmt);
    result.token_name = std::string((char*)token_name_buffer, token_name_length);
    token_name = result.token_name;

    SQLCHAR token_secret_buffer[1024];
    SQLLEN token_secret_length;
    ret = SQLGetData(stmt.getHandle(), 2, SQL_C_CHAR, token_secret_buffer, sizeof(token_secret_buffer),
                     &token_secret_length);
    REQUIRE_ODBC(ret, stmt);
    result.token_secret = std::string((char*)token_secret_buffer, token_secret_length);

    return result;
  }

 private:
  void cleanup() {
    connection.execute("ALTER USER IF EXISTS " + user + " REMOVE PROGRAMMATIC ACCESS TOKEN " + token_name);
  }
};

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
  PatSetup pat_setup;
  PatResult pat = pat_setup.acquire();

  // BD#7: Old driver returns invalid cursor state (10510) when fetching PAT token from ALTER USER command
  OLD_DRIVER_ONLY("BD#7") {
    CHECK(pat.fetch_ret == SQL_ERROR);
    REQUIRE(pat.diag_records.size() == 1);
    CHECK(pat.diag_records[0].sqlState == "24000");
    CHECK(pat.diag_records[0].nativeError == 10510);
    CHECK_THAT(pat.diag_records[0].messageText, ContainsSubstring("Invalid cursor state"));
  }

  NEW_DRIVER_ONLY("BD#7") {
    REQUIRE(pat.fetch_ret == SQL_SUCCESS);

    auto env = setup_pat_environment();
    auto dbc = get_pat_connection_handle(env);
    std::string connection_string = get_pat_as_password_connection_string(pat.token_secret);

    // When Trying to Connect
    attempt_pat_connection(dbc, connection_string);

    // Then Login is successful and simple query can be executed
    verify_pat_simple_query_execution(dbc);

    SQLDisconnect(dbc.getHandle());
  }
}

TEST_CASE("should authenticate using PAT as token", "[pat]") {
  // Given Authentication is set to Programmatic Access Token and valid PAT token is provided
  PatSetup pat_setup;
  PatResult pat = pat_setup.acquire();

  // BD#7: Old driver returns invalid cursor state (10510) when fetching PAT token from ALTER USER command
  OLD_DRIVER_ONLY("BD#7") {
    CHECK(pat.fetch_ret == SQL_ERROR);
    REQUIRE(pat.diag_records.size() == 1);
    CHECK(pat.diag_records[0].sqlState == "24000");
    CHECK(pat.diag_records[0].nativeError == 10510);
    CHECK_THAT(pat.diag_records[0].messageText, ContainsSubstring("Invalid cursor state"));
  }

  NEW_DRIVER_ONLY("BD#7") {
    REQUIRE(pat.fetch_ret == SQL_SUCCESS);

    auto env = setup_pat_environment();
    auto dbc = get_pat_connection_handle(env);
    std::string connection_string = get_pat_as_token_connection_string(pat.token_secret);

    // When Trying to Connect
    attempt_pat_connection(dbc, connection_string);

    // Then Login is successful and simple query can be executed
    verify_pat_simple_query_execution(dbc);

    SQLDisconnect(dbc.getHandle());
  }
}

TEST_CASE("should authenticate using PAT as token with lowercase authenticator", "[pat]") {
  // Given Authentication is set to lowercase programmatic_access_token and valid PAT token is provided
  PatSetup pat_setup;
  PatResult pat = pat_setup.acquire();

  OLD_DRIVER_ONLY("BD#7") {
    CHECK(pat.fetch_ret == SQL_ERROR);
    REQUIRE(pat.diag_records.size() == 1);
    CHECK(pat.diag_records[0].sqlState == "24000");
    CHECK(pat.diag_records[0].nativeError == 10510);
    CHECK_THAT(pat.diag_records[0].messageText, ContainsSubstring("Invalid cursor state"));
  }

  NEW_DRIVER_ONLY("BD#7") {
    REQUIRE(pat.fetch_ret == SQL_SUCCESS);

    std::string connection_string = get_pat_as_token_connection_string(pat.token_secret);
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

    SQLDisconnect(dbc.getHandle());
  }
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
