#ifndef TEST_SETUP_HPP
#define TEST_SETUP_HPP

#include <picojson.h>

#include <fstream>
#include <iostream>
#include <sstream>

#include <catch2/catch_test_macros.hpp>

inline std::string get_driver_path() {
  // Prefer a driver name if provided/installed via ODBCINSTINI, otherwise fall back to path
  const char* driver_name_env_value = std::getenv("DRIVER_NAME");
  if (driver_name_env_value != nullptr && driver_name_env_value[0] != '\0') {
    std::string driver_name = std::string(driver_name_env_value);
    INFO("Driver name: " << driver_name);
    // If ODBCINSTINI is not set, warn the user; still return braced name
    const char* odbcinstini_env_value = std::getenv("ODBCINSTINI");
    if (odbcinstini_env_value == nullptr || odbcinstini_env_value[0] == '\0') {
      WARN(std::string(
               "You are using DRIVER_NAME variable to set the driver implementation, while "
               "ODBCINSTINI is not set.\nPlease make sure ODBCINSTINI points to configuration "
               "file for ODBC drivers.")
               .c_str());
    }
    // Return braced name so the Driver Manager resolves installed driver entry
    return "{" + driver_name + "}";
  }

  // Fallback: DRIVER_PATH from environment variable
  const char* driver_path_env_value = std::getenv("DRIVER_PATH");
  REQUIRE(driver_path_env_value != nullptr);
  std::string driver_path = std::string(driver_path_env_value);
  INFO("Driver path: " << driver_path);
  return driver_path;
}

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
inline void add_param_required(std::stringstream& ss, const picojson::object& params,
                               const std::string& cfg_param_name,
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
inline void add_param_optional(std::stringstream& ss, const picojson::object& params,
                               const std::string& cfg_param_name,
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

inline void read_default_params(std::stringstream& ss, const picojson::object& params) {
  ss << "DRIVER=" << get_driver_path() << ";";
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_HOST", "SERVER");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_ACCOUNT", "ACCOUNT");
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_USER", "UID");
  add_param_optional<std::string>(ss, params, "SNOWFLAKE_TEST_WAREHOUSE", "WAREHOUSE");
  add_param_optional<std::string>(ss, params, "SNOWFLAKE_TEST_ROLE", "ROLE");
  add_param_optional<std::string>(ss, params, "SNOWFLAKE_TEST_SCHEMA", "SCHEMA");
  add_param_optional<std::string>(ss, params, "SNOWFLAKE_TEST_DATABASE", "DATABASE");
  add_param_optional<std::string>(ss, params, "SNOWFLAKE_TEST_PORT", "PORT");
  add_param_optional<std::string>(ss, params, "SNOWFLAKE_TEST_PROTOCOL", "PROTOCOL");
}

inline std::string get_connection_string() {
  auto params = get_test_parameters("testconnection");
  std::stringstream ss;
  read_default_params(ss, params);
  add_param_required<std::string>(ss, params, "SNOWFLAKE_TEST_PASSWORD", "PWD");
  return ss.str();
}

#endif  // TEST_SETUP_HPP
