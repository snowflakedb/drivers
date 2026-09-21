#include <picojson.h>
#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <sstream>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "EnvOverride.hpp"
#include "get_data.hpp"
#include "test_setup.hpp"

std::string optional_string_param(const picojson::object& params, const char* key) {
  auto it = params.find(key);
  if (it != params.end() && it->second.is<std::string>()) {
    return it->second.get<std::string>();
  }
  return {};
}

std::string require_wif_provider() {
  const auto params = get_test_parameters("testconnection");
  const auto provider = optional_string_param(params, "SNOWFLAKE_TEST_WIF_PROVIDER");
  if (provider.empty()) {
    SKIP("SNOWFLAKE_TEST_WIF_PROVIDER is not set in parameters.json");
  }
  return provider;
}

std::string wif_live_connection_string(const std::string& authenticator, const std::string& provider,
                                       const std::string& extra) {
  const auto params = get_test_parameters("testconnection");
  std::stringstream ss;
  configure_driver_string(ss);
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_HOST", "SERVER");
  const auto account = optional_string_param(params, "SNOWFLAKE_TEST_WIF_ACCOUNT");
  if (!account.empty()) {
    ss << "ACCOUNT=" << account << ";";
  } else {
    add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_ACCOUNT", "ACCOUNT");
  }
  const auto user = optional_string_param(params, "SNOWFLAKE_TEST_WIF_USER");
  if (!user.empty()) {
    ss << "UID=" << user << ";";
  } else {
    add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_USER", "UID");
  }
  add_param_optional<std::string>(ss, params, "SNOWFLAKE_TEST_PORT", "PORT");
  add_param_optional<std::string>(ss, params, "SNOWFLAKE_TEST_PROTOCOL", "PROTOCOL");
  ss << "AUTHENTICATOR=" << authenticator << ";";
  ss << "WORKLOAD_IDENTITY_PROVIDER=" << provider << ";";
  const auto entra = optional_string_param(params, "SNOWFLAKE_TEST_WIF_ENTRA_RESOURCE");
  if (!entra.empty()) {
    ss << "WORKLOAD_IDENTITY_ENTRA_RESOURCE=" << entra << ";";
  }
  ss << extra;
  return ss.str();
}

void require_simple_query(Connection& conn) {
  auto stmt = conn.execute_fetch("SELECT 1");
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 1);
}

TEST_CASE("should authenticate with cloud identity using WIF", "[workload_identity]") {
  // Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_PROVIDER is configured
  const auto provider = require_wif_provider();
  const auto conn_str = wif_live_connection_string("WORKLOAD_IDENTITY", provider, "");

  // When Trying to Connect
  Connection conn(conn_str);

  // Then Login is successful and a simple query can be executed
  require_simple_query(conn);
}

TEST_CASE("should authenticate with WIF using lowercase authenticator", "[workload_identity]") {
  // Given Authentication is set to workload_identity (lowercase) and a valid provider is configured
  const auto provider = require_wif_provider();
  const auto conn_str = wif_live_connection_string("workload_identity", provider, "");

  // When Trying to Connect
  Connection conn(conn_str);

  // Then Login is successful and a simple query can be executed
  require_simple_query(conn);
}

TEST_CASE("should authenticate with WIF using impersonation", "[workload_identity]") {
  // Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_IMPERSONATION_PATH is configured
  const auto provider = require_wif_provider();
  const auto params = get_test_parameters("testconnection");
  const auto path = optional_string_param(params, "SNOWFLAKE_TEST_WIF_IMPERSONATION_PATH");
  if (path.empty()) {
    SKIP("SNOWFLAKE_TEST_WIF_IMPERSONATION_PATH is not set in parameters.json");
  }
  const auto conn_str =
      wif_live_connection_string("WORKLOAD_IDENTITY", provider, "WORKLOAD_IDENTITY_IMPERSONATION_PATH=" + path + ";");

  // When Trying to Connect
  Connection conn(conn_str);

  // Then Login is successful and a simple query can be executed
  require_simple_query(conn);
}

TEST_CASE("should authenticate AWS WIF using pre-signed GetCallerIdentity by default", "[workload_identity]") {
  // Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_PROVIDER is AWS
  const auto provider = require_wif_provider();
  if (provider != "AWS" && provider != "aws") {
    SKIP("SNOWFLAKE_TEST_WIF_PROVIDER is not AWS");
  }
  // And SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN is not set
  EnvOverride unset_outbound("SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN");
  const auto conn_str = wif_live_connection_string("WORKLOAD_IDENTITY", provider, "");

  // When Trying to Connect
  Connection conn(conn_str);

  // Then Login is successful and a simple query can be executed
  require_simple_query(conn);
}

TEST_CASE("should authenticate AWS WIF using GetWebIdentityToken when opted in", "[workload_identity]") {
  // Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_PROVIDER is AWS
  const auto provider = require_wif_provider();
  if (provider != "AWS" && provider != "aws") {
    SKIP("SNOWFLAKE_TEST_WIF_PROVIDER is not AWS");
  }
  // And SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN is set to true
  EnvOverride outbound("SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN", "true");
  const auto conn_str = wif_live_connection_string("WORKLOAD_IDENTITY", provider, "");

  // When Trying to Connect
  Connection conn(conn_str);

  // Then Login is successful and a simple query can be executed
  require_simple_query(conn);
}

TEST_CASE("should authenticate AWS WIF using GetWebIdentityToken when workload_identity_aws_use_outbound_token is true",
          "[workload_identity]") {
  // Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_PROVIDER is AWS
  const auto provider = require_wif_provider();
  if (provider != "AWS" && provider != "aws") {
    SKIP("SNOWFLAKE_TEST_WIF_PROVIDER is not AWS");
  }
  // And SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN is not set
  EnvOverride unset_outbound("SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN");
  // And workload_identity_aws_use_outbound_token is set to true
  const auto conn_str =
      wif_live_connection_string("WORKLOAD_IDENTITY", provider, "WORKLOAD_IDENTITY_AWS_USE_OUTBOUND_TOKEN=true;");

  // When Trying to Connect
  Connection conn(conn_str);

  // Then Login is successful and a simple query can be executed
  require_simple_query(conn);
}
