#include "ODBCConfig.hpp"

// ============================================================================
// DriverConfig
// ============================================================================

std::shared_ptr<DriverConfig> DriverConfig::Default() {
  auto config = std::make_shared<DriverConfig>();
  config->parameters_["APILevel"] = "1";
  config->parameters_["ConnectFunctions"] = "YYY";
  config->parameters_["Description"] = "Snowflake ODBC Driver";
  config->parameters_["Driver"] = get_driver_path();
  config->parameters_["DriverODBCVer"] = "03.52";
  config->parameters_["SQLLevel"] = "1";
  config->parameters_["UsageCount"] = "1";
  return config;
}

DriverConfig& DriverConfig::set(const std::string& key, const std::string& value) {
  parameters_[key] = value;
  return *this;
}

DriverConfig& DriverConfig::remove(const std::string& key) {
  parameters_.erase(key);
  return *this;
}

const std::map<std::string, std::string>& DriverConfig::parameters() const {
  return parameters_;
}

std::string DriverConfig::name() {
  return "SnowflakeDriver";
}

std::string DriverConfig::get_driver_path() {
  if (const char* driver_path_env = std::getenv("DRIVER_PATH");
      driver_path_env != nullptr && driver_path_env[0] != '\0') {
    return driver_path_env;
  }
  return "/usr/lib/snowflake/odbc/lib/libSnowflake.so";
}
