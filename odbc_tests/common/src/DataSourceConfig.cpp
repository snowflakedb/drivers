#include <picojson.h>

#include <filesystem>
#include <fstream>
#include <random>
#include <sstream>
#include <stdexcept>

#ifdef _WIN32
#include <process.h>
#else
#include <unistd.h>
#endif

#include "ODBCConfig.hpp"
#include "utils.hpp"

static int current_pid() {
#ifdef _WIN32
  return _getpid();
#else
  return getpid();
#endif
}

// ============================================================================
// DataSourceConfig
// ============================================================================

static std::string read_private_key_pem(const picojson::object& params) {
  const auto it = params.find("SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS");
  if (it == params.end() || !it->second.is<picojson::array>()) {
    throw std::runtime_error("SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS not found or not an array");
  }
  std::stringstream ss;
  for (const auto& line : it->second.get<picojson::array>()) {
    ss << line.get<std::string>() << "\n";
  }
  return ss.str();
}

static std::string get_or_create_private_key_file(const picojson::object& params) {
  if (const auto it = params.find("SNOWFLAKE_TEST_PRIVATE_KEY_FILE");
      it != params.end() && it->second.is<std::string>() && !it->second.get<std::string>().empty()) {
    return it->second.get<std::string>();
  }
  static const std::string shared_path = (std::filesystem::temp_directory_path() / "odbc_test_rsa_key.p8").string();

  std::error_code ec;
  if (std::filesystem::exists(shared_path, ec) && !ec && std::filesystem::file_size(shared_path, ec) > 0 && !ec) {
    return shared_path;
  }

  std::string pem = read_private_key_pem(params);
  std::string staging = shared_path + "." + std::to_string(current_pid());
  std::ofstream f(staging, std::ios::out | std::ios::trunc);
  if (!f.is_open()) {
    throw std::runtime_error("Cannot create temp key file: " + staging);
  }
  f << pem;
  f.close();

  std::filesystem::rename(staging, shared_path, ec);
  if (ec) {
    std::filesystem::remove(staging, ec);
    if (!std::filesystem::exists(shared_path, ec) || ec || std::filesystem::file_size(shared_path, ec) == 0 || ec) {
      throw std::runtime_error("Failed to create shared key file: " + shared_path);
    }
  }
  return shared_path;
}

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
#ifdef _WIN32
  config.parameters_["Driver"] = DriverConfig::get_driver_path();
#else
  config.parameters_["Driver"] = config.driver_config_.value()->name();
#endif
  config.parameters_["Locale"] = "en-US";
  config.parameters_["SERVER"] = get_string(
      params, "SNOWFLAKE_TEST_HOST", get_string(params, "SNOWFLAKE_TEST_ACCOUNT", "") + ".snowflakecomputing.com");
  config.parameters_["PORT"] = "443";
  config.parameters_["SSL"] = "on";
  config.parameters_["UID"] = get_string(params, "SNOWFLAKE_TEST_USER", "");
  config.parameters_["ACCOUNT"] = get_string(params, "SNOWFLAKE_TEST_ACCOUNT", "");
  config.parameters_["AUTHENTICATOR"] = "SNOWFLAKE_JWT";
#ifdef SNOWFLAKE_OLD_DRIVER
  config.parameters_["PRIV_KEY_FILE"] = get_or_create_private_key_file(params);
  if (auto pwd = get_string(params, "SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD", ""); !pwd.empty()) {
    config.parameters_["PRIV_KEY_FILE_PWD"] = pwd;
  }
#else
  config.parameters_["PRIV_KEY_BASE64"] = test_utils::base64_encode(read_private_key_pem(params));
  if (auto pwd = get_string(params, "SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD", ""); !pwd.empty()) {
    config.parameters_["PRIV_KEY_PWD"] = pwd;
  }
#endif

  // Optional parameters
  if (auto db = get_string(params, "SNOWFLAKE_TEST_DATABASE", ""); !db.empty()) {
    config.parameters_["DATABASE"] = db;
  }
  if (auto schema = get_string(params, "SNOWFLAKE_TEST_SCHEMA", ""); !schema.empty()) {
    config.parameters_["SCHEMA"] = schema;
  }
  auto warehouse = get_string(params, "SNOWFLAKE_TEST_WAREHOUSE_ODBC", "");
  if (warehouse.empty()) {
    warehouse = get_string(params, "SNOWFLAKE_TEST_WAREHOUSE", "");
  }
  if (!warehouse.empty()) {
    config.parameters_["WAREHOUSE"] = warehouse;
  }
  if (auto role = get_string(params, "SNOWFLAKE_TEST_ROLE", ""); !role.empty()) {
    config.parameters_["ROLE"] = role;
  }
  config.parameters_["TRACING"] = "0";

  return config;
}

DataSourceConfig DataSourceConfig::SnowflakeNoAuth(const std::string& connection_name) {
  return Snowflake(connection_name)
      .remove("UID")
      .remove("PWD")
      .remove("AUTHENTICATOR")
      .remove("PRIV_KEY_BASE64")
      .remove("PRIV_KEY_FILE")
      .remove("PRIV_KEY_PWD")
      .remove("PRIV_KEY_FILE_PWD");
}

DataSourceConfig& DataSourceConfig::set(const std::string& key, const std::string& value) {
  parameters_[key] = value;
  return *this;
}

DataSourceConfig& DataSourceConfig::remove(const std::string& key) {
  parameters_.erase(key);
  return *this;
}

DataSourceConfig& DataSourceConfig::driver_config(const std::optional<std::shared_ptr<DriverConfig>>& dc) {
  driver_config_ = dc;
  return *this;
}

DataSourceConfig& DataSourceConfig::name(const std::string& name) {
  name_ = name;
  return *this;
}

const std::string& DataSourceConfig::name() const { return name_; }

const std::map<std::string, std::string>& DataSourceConfig::parameters() const { return parameters_; }

std::string DataSourceConfig::connection_string() const {
  std::stringstream ss;
  if (driver_config_.has_value()) {
#ifdef _WIN32
    ss << "DSN=" << driver_config_.value()->name() << ";";
#else
    ss << "DRIVER={" << driver_config_.value()->get_driver_path() << "};";
#endif
  }
  for (const auto& [key, value] : parameters_) {
    if (key == "Driver") {
      continue;
    }
    ss << key << "=" << value << ";";
  }
  return ss.str();
}

std::optional<std::shared_ptr<DriverConfig>> DataSourceConfig::driver_config() const { return driver_config_; }

ConfigInstallation DataSourceConfig::install() { return ConfigInstallation::install({*this}); }

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
