#ifndef ODBC_CONFIG_HPP
#define ODBC_CONFIG_HPP

#include <picojson.h>

#include <map>
#include <memory>
#include <optional>
#include <set>
#include <string>
#include <vector>

#include "EnvOverride.hpp"

// Forward declarations
class DriverConfig;
class DataSourceConfig;
class BaseConfigInstallation;

#ifdef _WIN32
class WindowsConfigInstallation;
using ConfigInstallation = WindowsConfigInstallation;
#else
class UnixConfigInstallation;
using ConfigInstallation = UnixConfigInstallation;
#endif

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
  [[nodiscard]] const std::string& name() const;
  static std::string get_driver_path();

 private:
  static std::string generate_random_name();

  std::string name_;
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
  [[nodiscard]] std::string connection_string() const;
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
// BaseConfigInstallation - Common RAII base for managing installed ODBC config
// ============================================================================

class BaseConfigInstallation {
 public:
  BaseConfigInstallation(const BaseConfigInstallation&) = delete;
  BaseConfigInstallation& operator=(const BaseConfigInstallation&) = delete;

  [[nodiscard]] const std::string& config_dir() const;
  [[nodiscard]] std::string dsn_name(size_t index = 0) const;
  [[nodiscard]] std::string connection_string(size_t index = 0) const;

 protected:
  explicit BaseConfigInstallation(const std::vector<DataSourceConfig>& data_sources,
                                  const std::set<std::shared_ptr<DriverConfig>>& driver_configs);
  ~BaseConfigInstallation() = default;

  BaseConfigInstallation(BaseConfigInstallation&& other) noexcept;
  BaseConfigInstallation& operator=(BaseConfigInstallation&& other) noexcept;

  void collect_driver_configs();

  std::string config_dir_;
  std::vector<DataSourceConfig> data_sources_;
  std::set<std::shared_ptr<DriverConfig>> driver_configs_;
  std::vector<EnvOverride> env_overrides_;
};

// ============================================================================
// WindowsConfigInstallation - Windows registry-based ODBC configuration
// ============================================================================

#ifdef _WIN32

class WindowsConfigInstallation : public BaseConfigInstallation {
 public:
  static WindowsConfigInstallation install(const std::vector<DataSourceConfig>& data_sources);
  static WindowsConfigInstallation install_driver(const std::shared_ptr<DriverConfig>& driver_config);

  ~WindowsConfigInstallation();

  WindowsConfigInstallation(const WindowsConfigInstallation&) = delete;
  WindowsConfigInstallation& operator=(const WindowsConfigInstallation&) = delete;

  WindowsConfigInstallation(WindowsConfigInstallation&& other) noexcept;
  WindowsConfigInstallation& operator=(WindowsConfigInstallation&& other) noexcept;

 private:
  explicit WindowsConfigInstallation(const std::vector<DataSourceConfig>& data_sources,
                                     const std::set<std::shared_ptr<DriverConfig>>& driver_configs);
};

#endif

// ============================================================================
// UnixConfigInstallation - Unix file-based ODBC configuration
// ============================================================================

#ifndef _WIN32

class UnixConfigInstallation : public BaseConfigInstallation {
 public:
  static UnixConfigInstallation install(const std::vector<DataSourceConfig>& data_sources);
  static UnixConfigInstallation install_driver(const std::shared_ptr<DriverConfig>& driver_config);

  ~UnixConfigInstallation();

  UnixConfigInstallation(const UnixConfigInstallation&) = delete;
  UnixConfigInstallation& operator=(const UnixConfigInstallation&) = delete;

  UnixConfigInstallation(UnixConfigInstallation&& other) noexcept;
  UnixConfigInstallation& operator=(UnixConfigInstallation&& other) noexcept;

 private:
  explicit UnixConfigInstallation(const std::vector<DataSourceConfig>& data_sources,
                                  const std::set<std::shared_ptr<DriverConfig>>& driver_configs);
  static std::string create_temp_dir();
  void write_odbcinst_ini() const;
  void write_odbc_ini() const;
};

#endif

#endif  // ODBC_CONFIG_HPP
