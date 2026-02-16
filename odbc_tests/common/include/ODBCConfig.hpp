#ifndef ODBC_CONFIG_HPP
#define ODBC_CONFIG_HPP

#include <picojson.h>

#include <map>
#include <memory>
#include <optional>
#include <string>
#include <vector>

#include "EnvOverride.hpp"

// Forward declarations
class DriverConfig;
class DataSourceConfig;
class ConfigInstallation;

// ============================================================================
// DriverConfig - Manages ODBC driver configuration
// ============================================================================

class DriverConfig {
 public:
  // Factory method
  static std::shared_ptr<DriverConfig> Default();

  // Builder methods
  DriverConfig& set(const std::string& key, const std::string& value);
  DriverConfig& remove(const std::string& key);

  // Accessors
  [[nodiscard]] const std::map<std::string, std::string>& parameters() const;
  [[nodiscard]] static std::string name();
  static std::string get_driver_path();

 private:
  std::map<std::string, std::string> parameters_;
};

// ============================================================================
// DataSourceConfig - Manages ODBC data source configuration
// ============================================================================

class DataSourceConfig {
 public:
  // Factory methods
  static DataSourceConfig Snowflake(const std::string& connection_name = "testconnection");
  static DataSourceConfig SnowflakeNoAuth(const std::string& connection_name = "testconnection");

  // Builder methods
  DataSourceConfig& set(const std::string& key, const std::string& value);
  DataSourceConfig& remove(const std::string& key);
  DataSourceConfig& driver_config(const std::optional<std::shared_ptr<DriverConfig>>& dc);
  DataSourceConfig& name(const std::string& name);

  // Accessors
  [[nodiscard]] const std::string& name() const;
  [[nodiscard]] const std::map<std::string, std::string>& parameters() const;
  [[nodiscard]] std::optional<std::shared_ptr<DriverConfig>> driver_config() const;

  // Installation
  ConfigInstallation install();

 private:
  std::string name_;
  std::map<std::string, std::string> parameters_;
  std::optional<std::shared_ptr<DriverConfig>> driver_config_;

  // Helper methods
  static picojson::object load_parameters(const std::string& connection_name);
  static std::string get_string(const picojson::object& obj, const std::string& key,
                                const std::string& default_value = "");
};

// ============================================================================
// ConfigInstallation - RAII class for managing installed ODBC configuration
// ============================================================================

class ConfigInstallation {
 public:
  // Factory method
  static ConfigInstallation install(const std::vector<DataSourceConfig>& data_sources);

  // Destructor
  ~ConfigInstallation();

  // Non-copyable
  ConfigInstallation(const ConfigInstallation&) = delete;
  ConfigInstallation& operator=(const ConfigInstallation&) = delete;

  // Movable
  ConfigInstallation(ConfigInstallation&& other) noexcept;
  ConfigInstallation& operator=(ConfigInstallation&& other) noexcept;

  // Accessors
  [[nodiscard]] const std::string& config_dir() const;
  [[nodiscard]] std::string dsn_name(size_t index = 0) const;

 private:
  // Private constructor (use factory method)
  explicit ConfigInstallation(const std::vector<DataSourceConfig>& data_sources);

  // Helper methods
  static std::string create_temp_dir();
  void write_odbcinst_ini() const;
  void write_odbc_ini() const;

  // Members
  std::string config_dir_;
  std::vector<DataSourceConfig> data_sources_;
  std::vector<EnvOverride> env_overrides_;
};

#endif  // ODBC_CONFIG_HPP
