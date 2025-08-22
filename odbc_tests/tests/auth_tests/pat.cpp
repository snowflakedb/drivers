#include <iomanip>
#include <random>
#include <sstream>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "require.hpp"

class Pat {
 private:
  std::string token_name;
  std::string token_secret;

 public:
  Pat() { acquire(); }

  ~Pat() { cleanup(); }

  const std::string& getTokenName() const { return token_name; }
  const std::string& getTokenSecret() const { return token_secret; }

 private:
  void acquire() {
    std::random_device rd;
    std::mt19937 gen(rd());
    std::uniform_int_distribution<uint32_t> dis;
    uint32_t random_number = dis(gen);
    std::stringstream ss;
    ss << "pat_" << std::hex << std::setw(8) << std::setfill('0') << random_number;
    token_name = ss.str();

    // Create connection and get user/role
    Connection conn;
    auto params = get_test_parameters("testconnection");
    std::string user = params.at("SNOWFLAKE_TEST_USER").get<std::string>();
    std::string role = params.at("SNOWFLAKE_TEST_ROLE").get<std::string>();

    // Execute ALTER USER command to create PAT
    std::stringstream create_sql;
    create_sql << "ALTER USER IF EXISTS " << user << " ADD PROGRAMMATIC ACCESS TOKEN " << token_name
               << " ROLE_RESTRICTION = " << role;

    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)create_sql.str().c_str(), SQL_NTS);
    CHECK_ODBC(ret, stmt);

    // Fetch the result to get token name and secret
    ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);

    // Get token name (first column)
    SQLCHAR token_name_buffer[256];
    SQLLEN token_name_length;
    ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, token_name_buffer, sizeof(token_name_buffer),
                     &token_name_length);
    CHECK_ODBC(ret, stmt);
    token_name = std::string((char*)token_name_buffer, token_name_length);

    // Get token secret (second column)
    SQLCHAR token_secret_buffer[1024];
    SQLLEN token_secret_length;
    ret = SQLGetData(stmt.getHandle(), 2, SQL_C_CHAR, token_secret_buffer,
                     sizeof(token_secret_buffer), &token_secret_length);
    CHECK_ODBC(ret, stmt);
    token_secret = std::string((char*)token_secret_buffer, token_secret_length);
  }

  void cleanup() {
    try {
      Connection conn;
      auto params = get_test_parameters("testconnection");
      std::string user = params.at("SNOWFLAKE_TEST_USER").get<std::string>();

      std::stringstream cleanup_sql;
      cleanup_sql << "ALTER USER IF EXISTS " << user << " REMOVE PROGRAMMATIC ACCESS TOKEN "
                  << token_name;

      conn.execute(cleanup_sql.str());
    } catch (...) {
      // Ignore cleanup errors to avoid throwing in destructor
    }
  }
};

std::string get_pat_as_password_connection_string(const std::string& pat_secret) {
  auto params = get_test_parameters("testconnection");
  std::stringstream ss;
  read_default_params(ss, params);
  ss << "PWD=" << pat_secret << ";";
  return ss.str();
}

std::string get_pat_as_token_connection_string(const std::string& pat_secret) {
  auto params = get_test_parameters("testconnection");
  std::stringstream ss;
  read_default_params(ss, params);
  ss << "AUTHENTICATOR=PROGRAMMATIC_ACCESS_TOKEN;";
  ss << "TOKEN=" << pat_secret << ";";
  return ss.str();
}

// PAT Setup doesn't work with old ODBC driver.
TEST_CASE("PAT Authentication - As Password", "[pat_auth][new_odbc]") {
  INFO("Testing PAT authentication using token as password");

  Pat pat;
  std::string connection_string = get_pat_as_password_connection_string(pat.getTokenSecret());
  Connection conn(connection_string);

  // Test a simple query to verify connection works
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 1", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  SQLINTEGER result_value;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &result_value, sizeof(result_value), NULL);
  CHECK_ODBC(ret, stmt);

  REQUIRE(result_value == 1);
}

// PAT Setup doesn't work with old ODBC driver.
TEST_CASE("PAT Authentication - As Token", "[pat_auth][new_odbc]") {
  INFO("Testing PAT authentication using PROGRAMMATIC_ACCESS_TOKEN authenticator");

  Pat pat;
  std::string connection_string = get_pat_as_token_connection_string(pat.getTokenSecret());
  Connection conn(connection_string);

  // Test a simple query to verify connection works
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 1", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  SQLINTEGER result_value;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &result_value, sizeof(result_value), NULL);
  CHECK_ODBC(ret, stmt);

  REQUIRE(result_value == 1);
}

TEST_CASE("PAT Authentication - Invalid Token", "[pat_auth]") {
  INFO("Testing PAT authentication with invalid token");

  // Use an obviously invalid token
  std::string connection_string = get_pat_as_password_connection_string("invalid_token_12345");
  require_connection_failed(connection_string);
}

TEST_CASE("PAT Authentication - Missing Token with PROGRAMMATIC_ACCESS_TOKEN", "[pat_auth]") {
  INFO("Testing PAT authentication with PROGRAMMATIC_ACCESS_TOKEN authenticator but no token");

  // Create connection string with PROGRAMMATIC_ACCESS_TOKEN but no TOKEN parameter
  auto params = get_test_parameters("testconnection");
  std::stringstream ss;
  ss << "DRIVER=" << get_driver_path() << ";";
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_HOST", "SERVER");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_ACCOUNT", "ACCOUNT");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_USER", "UID");
  ss << "AUTHENTICATOR=PROGRAMMATIC_ACCESS_TOKEN;";

  std::string connection_string = ss.str();
  require_connection_failed(connection_string);
}
