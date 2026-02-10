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
  [[nodiscard]] std::string name() const;
  static std::string get_driver_path();

 private:
  std::map<std::string, std::string> parameters_;
  std::string name_;
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
// WindowsConfigInstallation - RAII ODBC configuration via Windows registry
// ============================================================================
//
// On Windows the ODBC Driver Manager reads its configuration from the registry:
//   Drivers  : HKEY_LOCAL_MACHINE\SOFTWARE\ODBC\ODBCINST.INI
//   User DSNs: HKEY_CURRENT_USER\SOFTWARE\ODBC\ODBC.INI
//
// The constructor writes the appropriate keys/values and the destructor removes
// them, providing RAII semantics identical to the Unix file-based approach.

#ifdef _WIN32

class WindowsConfigInstallation {
 public:
  // Factory methods
  static WindowsConfigInstallation install(const std::vector<DataSourceConfig>& data_sources);
  static WindowsConfigInstallation install_driver(const std::shared_ptr<DriverConfig>& driver_config);

  // Destructor - removes installed registry keys
  ~WindowsConfigInstallation();

  // Non-copyable
  WindowsConfigInstallation(const WindowsConfigInstallation&) = delete;
  WindowsConfigInstallation& operator=(const WindowsConfigInstallation&) = delete;

  // Movable
  WindowsConfigInstallation(WindowsConfigInstallation&& other) noexcept;
  WindowsConfigInstallation& operator=(WindowsConfigInstallation&& other) noexcept;

  // Accessors
  [[nodiscard]] std::string dsn_name(size_t index = 0) const;

 private:
  explicit WindowsConfigInstallation(const std::vector<DataSourceConfig>& data_sources,
                                     const std::set<std::shared_ptr<DriverConfig>>& driver_configs);

  void collect_driver_configs();
  void install_drivers_to_registry();
  void install_dsns_to_registry();
  void uninstall_drivers_from_registry();
  void uninstall_dsns_from_registry();

  std::vector<DataSourceConfig> data_sources_;
  std::set<std::shared_ptr<DriverConfig>> driver_configs_;
};

#endif  // _WIN32

// ============================================================================
// UnixConfigInstallation - RAII ODBC configuration via unixODBC ini files
// ============================================================================
//
// Creates a temporary directory with odbcinst.ini and odbc.ini, then sets
// ODBCSYSINI / ODBCINI environment variables so the unixODBC driver manager
// picks up the configuration. Everything is cleaned up on destruction.

#ifndef _WIN32

class UnixConfigInstallation {
 public:
  // Factory methods
  static UnixConfigInstallation install(const std::vector<DataSourceConfig>& data_sources);
  static UnixConfigInstallation install_driver(const std::shared_ptr<DriverConfig>& driver_config);

  // Destructor - removes temporary config directory
  ~UnixConfigInstallation();

  // Non-copyable
  UnixConfigInstallation(const UnixConfigInstallation&) = delete;
  UnixConfigInstallation& operator=(const UnixConfigInstallation&) = delete;

  // Movable
  UnixConfigInstallation(UnixConfigInstallation&& other) noexcept;
  UnixConfigInstallation& operator=(UnixConfigInstallation&& other) noexcept;

  // Accessors
  [[nodiscard]] const std::string& config_dir() const;
  [[nodiscard]] std::string dsn_name(size_t index = 0) const;

 private:
  explicit UnixConfigInstallation(const std::vector<DataSourceConfig>& data_sources,
                                  const std::set<std::shared_ptr<DriverConfig>>& driver_configs);

  static std::string create_temp_dir();
  void collect_driver_configs();
  void write_odbcinst_ini() const;
  void write_odbc_ini() const;

  std::string config_dir_;
  std::vector<DataSourceConfig> data_sources_;
  std::set<std::shared_ptr<DriverConfig>> driver_configs_;
  std::vector<EnvOverride> env_overrides_;
};

#endif  // !_WIN32

#endif  // ODBC_CONFIG_HPP
