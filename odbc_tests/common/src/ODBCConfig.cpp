#include "ODBCConfig.hpp"

#include <random>

// ============================================================================
// DriverConfig
// ============================================================================

std::shared_ptr<DriverConfig> DriverConfig::Default() {
  auto config = std::make_shared<DriverConfig>();
  config->name_ = generate_random_name();
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

const std::map<std::string, std::string>& DriverConfig::parameters() const { return parameters_; }

const std::string& DriverConfig::name() const { return name_; }

std::string DriverConfig::generate_random_name() {
  static constexpr char chars[] = "abcdefghijklmnopqrstuvwxyz0123456789";
  static constexpr size_t suffix_len = 8;

  std::random_device rd;
  std::mt19937 gen(rd());
  std::uniform_int_distribution<size_t> dist(0, sizeof(chars) - 2);

  std::string suffix(suffix_len, '\0');
  for (size_t i = 0; i < suffix_len; ++i) {
    suffix[i] = chars[dist(gen)];
  }
  return "SnowflakeDriver_" + suffix;
}

std::string DriverConfig::get_driver_path() {
  if (const char* driver_path_env = std::getenv("DRIVER_PATH");
      driver_path_env != nullptr && driver_path_env[0] != '\0') {
    return driver_path_env;
  }
  throw std::runtime_error("DRIVER_PATH environment variable not set");
}
