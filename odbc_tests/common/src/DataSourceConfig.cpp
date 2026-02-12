#include "ODBCConfig.hpp"

#include <picojson.h>

#include <fstream>
#include <random>
#include <stdexcept>

// ============================================================================
// DataSourceConfig
// ============================================================================

DataSourceConfig DataSourceConfig::Snowflake(const std::string& connection_name) {
  const auto params = load_parameters(connection_name);

  // Generate unique DSN name for test isolation (parallel execution safety)
  std::random_device rd;
  std::mt19937 gen(rd());
  std::uniform_int_distribution<> dis(0, 999999);
  const std::string unique_suffix = std::to_string(dis(gen));

  DataSourceConfig config;
  config.name_ = "Snowflake_" + unique_suffix;
  config.driver_config_ = DriverConfig::Default();
  config.parameters_["Description"] = "Snowflake Test DSN";
  config.parameters_["Driver"] = config.driver_config_.value()->name();
  config.parameters_["Locale"] = "en-US";
  config.parameters_["SERVER"] = get_string(params, "SNOWFLAKE_TEST_HOST",
                                             get_string(params, "SNOWFLAKE_TEST_ACCOUNT", "") +
                                                 ".snowflakecomputing.com");
  config.parameters_["PORT"] = "443";
  config.parameters_["SSL"] = "on";
  config.parameters_["UID"] = get_string(params, "SNOWFLAKE_TEST_USER", "");
  config.parameters_["PWD"] = get_string(params, "SNOWFLAKE_TEST_PASSWORD", "");
  config.parameters_["ACCOUNT"] = get_string(params, "SNOWFLAKE_TEST_ACCOUNT", "");

  // Optional parameters
  if (auto db = get_string(params, "SNOWFLAKE_TEST_DATABASE", ""); !db.empty()) {
    config.parameters_["DATABASE"] = db;
  }
  if (auto schema = get_string(params, "SNOWFLAKE_TEST_SCHEMA", ""); !schema.empty()) {
    config.parameters_["SCHEMA"] = schema;
  }
  if (auto warehouse = get_string(params, "SNOWFLAKE_TEST_WAREHOUSE", ""); !warehouse.empty()) {
    config.parameters_["WAREHOUSE"] = warehouse;
  }
  if (auto role = get_string(params, "SNOWFLAKE_TEST_ROLE", ""); !role.empty()) {
    config.parameters_["ROLE"] = role;
  }
  config.parameters_["TRACING"] = "0";

  return config;
}

DataSourceConfig DataSourceConfig::SnowflakeNoAuth(const std::string& connection_name) {
  return Snowflake(connection_name).remove("UID").remove("PWD");
}

DataSourceConfig& DataSourceConfig::set(const std::string& key, const std::string& value) {
  parameters_[key] = value;
  return *this;
}

DataSourceConfig& DataSourceConfig::remove(const std::string& key) {
  parameters_.erase(key);
  return *this;
}

DataSourceConfig& DataSourceConfig::driver_config(
    const std::optional<std::shared_ptr<DriverConfig>>& dc) {
  driver_config_ = dc;
  return *this;
}

DataSourceConfig& DataSourceConfig::name(const std::string& name) {
  name_ = name;
  return *this;
}

const std::string& DataSourceConfig::name() const {
  return name_;
}

const std::map<std::string, std::string>& DataSourceConfig::parameters() const {
  return parameters_;
}

std::optional<std::shared_ptr<DriverConfig>> DataSourceConfig::driver_config() const {
  return driver_config_;
}

ConfigInstallation DataSourceConfig::install() {
  return ConfigInstallation::install({*this});
}

picojson::object DataSourceConfig::load_parameters(const std::string& connection_name) {
  const char* parameter_path_env = std::getenv("PARAMETER_PATH");
  if (parameter_path_env == nullptr) {
    throw std::runtime_error("PARAMETER_PATH environment variable not set");
  }

  std::ifstream params_file(parameter_path_env);
  if (!params_file.good()) {
    throw std::runtime_error("Cannot open parameters file: " + std::string(parameter_path_env));
  }

  picojson::value connections;
  if (const std::string err = picojson::parse(connections, params_file); !err.empty()) {
    throw std::runtime_error("Failed to parse parameters: " + err);
  }

  if (!connections.is<picojson::object>() || !connections.contains(connection_name)) {
    throw std::runtime_error("Connection '" + connection_name + "' not found in parameters");
  }

  const picojson::value& params = connections.get<picojson::object>().at(connection_name);
  if (!params.is<picojson::object>()) {
    throw std::runtime_error("Connection '" + connection_name + "' is not an object");
  }

  return params.get<picojson::object>();
}

std::string DataSourceConfig::get_string(const picojson::object& obj, const std::string& key,
                                          const std::string& default_value) {
  if (const auto it = obj.find(key); it != obj.end() && it->second.is<std::string>()) {
    return it->second.get<std::string>();
  }
  return default_value;
}
