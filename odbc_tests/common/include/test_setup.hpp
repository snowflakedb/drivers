#ifndef TEST_SETUP_HPP
#define TEST_SETUP_HPP

#include <picojson.h>

#include <fstream>
#include <iostream>
#include <set>
#include <sstream>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"

inline picojson::object get_test_parameters(const std::string& connection_name) {
  const char* parameter_path_env_value = std::getenv("PARAMETER_PATH");
  REQUIRE(parameter_path_env_value != nullptr);
  std::string parameter_path = std::string(parameter_path_env_value);
  INFO("Reading parameters from " << parameter_path);
  std::ifstream params_file(parameter_path);
  INFO("File exists: " << params_file.good());

  picojson::value connections;
  std::string err = picojson::parse(connections, params_file);
  if (!err.empty()) {
    FAIL("Failed to parse parameters: '" << err << "'");
  }

  REQUIRE(connections.is<picojson::object>());
  REQUIRE(connections.contains(connection_name));
  const picojson::value& params = connections.get<picojson::object>().at(connection_name);
  REQUIRE(params.is<picojson::object>());
  return params.get<picojson::object>();
}

template <typename T>
inline void add_param_required(std::stringstream& ss, const picojson::object& params, const std::string& cfg_param_name,
                               const std::string& conn_param_name) {
  auto it = params.find(cfg_param_name);
  if (it == params.end()) {
    FAIL("Required parameter '" + cfg_param_name + "' is missing in the test parameters.");
  }
  if (it->second.is<T>()) {
    ss << conn_param_name << "=" << it->second.get<T>() << ";";
  } else {
    FAIL("Parameter '" + cfg_param_name + "' is not of expected type.");
  }
}

template <typename T>
inline void add_param_optional(std::stringstream& ss, const picojson::object& params, const std::string& cfg_param_name,
                               const std::string& conn_param_name) {
  auto it = params.find(cfg_param_name);
  if (it != params.end()) {
    if (it->second.is<T>()) {
      ss << conn_param_name << "=" << it->second.get<T>() << ";";
    } else {
      WARN("Parameter '" + cfg_param_name + "' is not of expected type.");
    }
  }
}

inline std::string get_private_key_file_path(const picojson::object& params) {
  auto it = params.find("SNOWFLAKE_TEST_PRIVATE_KEY_FILE");
  if (it != params.end() && it->second.is<std::string>()) {
    return it->second.get<std::string>();
  }
  return "";
}

inline std::string read_private_key(const picojson::object& params) {
  auto it = params.find("SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS");
  if (it == params.end()) {
    FAIL(
        "Required parameter 'SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS' is missing in the test "
        "parameters.");
  }

  if (!it->second.is<picojson::array>()) {
    FAIL("Parameter 'SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS' is not of expected type.");
  }

  auto private_key_lines = it->second.get<picojson::array>();
  std::stringstream private_key_stream;
  for (const auto& line : private_key_lines) {
    private_key_stream << line.get<std::string>() << "\n";
  }
  return private_key_stream.str();
}

inline void configure_driver_string(std::stringstream& ss) {
  static std::shared_ptr<DriverConfig> driver_config = DriverConfig::Default();
  static ConfigInstallation config_installation = ConfigInstallation::install_driver(driver_config);
#ifdef _WIN32
  ss << "DSN=" << driver_config->name() << ";";
#else
  ss << "DRIVER={" << driver_config->name() << "};";
#endif
}

inline void read_default_params(std::stringstream& ss, const picojson::object& params,
                                const std::set<std::string>& skip_conn_keys = {}) {
  auto req = [&](const std::string& cfg, const std::string& conn) {
    if (!skip_conn_keys.count(conn)) add_param_required<std::string>(ss, params, cfg, conn);
  };
  auto opt = [&](const std::string& cfg, const std::string& conn) {
    if (!skip_conn_keys.count(conn)) add_param_optional<std::string>(ss, params, cfg, conn);
  };

  configure_driver_string(ss);
  req("SNOWFLAKE_TEST_HOST", "SERVER");
  req("SNOWFLAKE_TEST_ACCOUNT", "ACCOUNT");
  req("SNOWFLAKE_TEST_USER", "UID");
  if (!skip_conn_keys.count("WAREHOUSE")) {
    if (params.count("SNOWFLAKE_TEST_WAREHOUSE_ODBC")) {
      add_param_optional<std::string>(ss, params, "SNOWFLAKE_TEST_WAREHOUSE_ODBC", "WAREHOUSE");
    } else {
      add_param_optional<std::string>(ss, params, "SNOWFLAKE_TEST_WAREHOUSE", "WAREHOUSE");
    }
  }
  opt("SNOWFLAKE_TEST_ROLE", "ROLE");
  opt("SNOWFLAKE_TEST_SCHEMA", "SCHEMA");
  opt("SNOWFLAKE_TEST_DATABASE", "DATABASE");
  opt("SNOWFLAKE_TEST_PORT", "PORT");
  opt("SNOWFLAKE_TEST_PROTOCOL", "PROTOCOL");
}

inline std::string get_connection_string() {
  auto params = get_test_parameters("testconnection");
  std::stringstream ss;
  read_default_params(ss, params);
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_PASSWORD", "PWD");
  return ss.str();
}

#endif  // TEST_SETUP_HPP
