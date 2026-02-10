#ifdef _WIN32

#include <windows.h>

#include <iostream>
#include <stdexcept>
#include <string>

#include "ODBCConfig.hpp"

// ============================================================================
// Registry helpers
// ============================================================================

namespace {

constexpr const char* ODBCINST_INI_PATH = "SOFTWARE\\ODBC\\ODBCINST.INI";
constexpr const char* ODBC_INI_PATH = "SOFTWARE\\ODBC\\ODBC.INI";

// Set a REG_SZ value under the given subkey. Creates the key if it does not exist.
void reg_set_string(HKEY root, const std::string& subkey, const std::string& value_name, const std::string& data) {
  HKEY hkey = nullptr;
  LONG rc =
      RegCreateKeyExA(root, subkey.c_str(), 0, nullptr, REG_OPTION_NON_VOLATILE, KEY_WRITE, nullptr, &hkey, nullptr);
  if (rc != ERROR_SUCCESS) {
    throw std::runtime_error("Failed to create/open registry key '" + subkey + "' (error " + std::to_string(rc) + ")");
  }

  rc = RegSetValueExA(hkey, value_name.c_str(), 0, REG_SZ, reinterpret_cast<const BYTE*>(data.c_str()),
                      static_cast<DWORD>(data.size() + 1));
  RegCloseKey(hkey);

  if (rc != ERROR_SUCCESS) {
    throw std::runtime_error("Failed to set registry value '" + value_name + "' under '" + subkey + "' (error " +
                             std::to_string(rc) + ")");
  }
}

// Delete a single value from a registry key. Silently ignores missing keys.
void reg_delete_value(HKEY root, const std::string& subkey, const std::string& value_name) {
  HKEY hkey = nullptr;
  LONG rc = RegOpenKeyExA(root, subkey.c_str(), 0, KEY_SET_VALUE, &hkey);
  if (rc == ERROR_FILE_NOT_FOUND) {
    return;
  }
  if (rc != ERROR_SUCCESS) {
    std::cerr << "Warning: Failed to open registry key '" << subkey << "' for value deletion (error " << rc << ")"
              << std::endl;
    return;
  }

  RegDeleteValueA(hkey, value_name.c_str());
  RegCloseKey(hkey);
}

// Delete a registry key and all its values. The key must have no subkeys.
// Silently ignores missing keys.
void reg_delete_key(HKEY root, const std::string& subkey) {
  LONG rc = RegDeleteKeyA(root, subkey.c_str());
  if (rc != ERROR_SUCCESS && rc != ERROR_FILE_NOT_FOUND) {
    std::cerr << "Warning: Failed to delete registry key '" << subkey << "' (error " << rc << ")" << std::endl;
  }
}

}  // anonymous namespace

// ============================================================================
// WindowsConfigInstallation
// ============================================================================

WindowsConfigInstallation WindowsConfigInstallation::install(const std::vector<DataSourceConfig>& data_sources) {
  return WindowsConfigInstallation(data_sources, {});
}

WindowsConfigInstallation WindowsConfigInstallation::install_driver(
    const std::shared_ptr<DriverConfig>& driver_config) {
  return WindowsConfigInstallation({}, {driver_config});
}

WindowsConfigInstallation::WindowsConfigInstallation(const std::vector<DataSourceConfig>& data_sources,
                                                     const std::set<std::shared_ptr<DriverConfig>>& driver_configs)
    : data_sources_(data_sources), driver_configs_(driver_configs) {
  collect_driver_configs();
  install_drivers_to_registry();
  install_dsns_to_registry();
}

WindowsConfigInstallation::~WindowsConfigInstallation() {
  // Only clean up if this object was not moved-from.
  if (!driver_configs_.empty() || !data_sources_.empty()) {
    uninstall_dsns_from_registry();
    uninstall_drivers_from_registry();
  }
}

WindowsConfigInstallation::WindowsConfigInstallation(WindowsConfigInstallation&& other) noexcept
    : data_sources_(std::move(other.data_sources_)), driver_configs_(std::move(other.driver_configs_)) {
  other.data_sources_.clear();
  other.driver_configs_.clear();
}

WindowsConfigInstallation& WindowsConfigInstallation::operator=(WindowsConfigInstallation&& other) noexcept {
  if (this != &other) {
    // Clean up our current registry entries before taking ownership of new ones.
    if (!driver_configs_.empty() || !data_sources_.empty()) {
      uninstall_dsns_from_registry();
      uninstall_drivers_from_registry();
    }

    data_sources_ = std::move(other.data_sources_);
    driver_configs_ = std::move(other.driver_configs_);

    other.data_sources_.clear();
    other.driver_configs_.clear();
  }
  return *this;
}

std::string WindowsConfigInstallation::dsn_name(size_t index) const {
  if (index >= data_sources_.size()) {
    throw std::out_of_range("Data source index out of range");
  }
  return data_sources_[index].name();
}

void WindowsConfigInstallation::collect_driver_configs() {
  for (const auto& ds : data_sources_) {
    if (auto dc = ds.driver_config()) {
      driver_configs_.insert(dc.value());
    }
  }

  // Check for name conflicts
  for (const auto& dc : driver_configs_) {
    auto same_name =
        std::count_if(driver_configs_.begin(), driver_configs_.end(),
                      [&dc](const std::shared_ptr<DriverConfig>& other) { return other->name() == dc->name(); });
    if (same_name > 1) {
      throw std::runtime_error("Driver config name '" + dc->name() + "' is not unique");
    }
  }
}

void WindowsConfigInstallation::install_drivers_to_registry() {
  for (const auto& dc : driver_configs_) {
    const std::string name = dc->name();

    // Add the driver to the "ODBC Drivers" list
    const std::string drivers_list_key = std::string(ODBCINST_INI_PATH) + "\\ODBC Drivers";
    reg_set_string(HKEY_LOCAL_MACHINE, drivers_list_key, name, "Installed");

    // Create the driver's own key with all its parameters
    const std::string driver_key = std::string(ODBCINST_INI_PATH) + "\\" + name;
    for (const auto& [key, value] : dc->parameters()) {
      reg_set_string(HKEY_LOCAL_MACHINE, driver_key, key, value);
    }
  }
}

void WindowsConfigInstallation::install_dsns_to_registry() {
  if (data_sources_.empty()) {
    return;
  }

  for (const auto& ds : data_sources_) {
    // Register in the "ODBC Data Sources" list
    if (auto dc = ds.driver_config()) {
      const std::string dsn_list_key = std::string(ODBC_INI_PATH) + "\\ODBC Data Sources";
      reg_set_string(HKEY_CURRENT_USER, dsn_list_key, ds.name(), dc.value()->name());
    }

    // Create the DSN's own key with all its parameters
    const std::string dsn_key = std::string(ODBC_INI_PATH) + "\\" + ds.name();
    for (const auto& [key, value] : ds.parameters()) {
      if (!value.empty()) {
        reg_set_string(HKEY_CURRENT_USER, dsn_key, key, value);
      }
    }
  }
}

void WindowsConfigInstallation::uninstall_drivers_from_registry() {
  for (const auto& dc : driver_configs_) {
    const std::string name = dc->name();

    // Remove from the "ODBC Drivers" list
    const std::string drivers_list_key = std::string(ODBCINST_INI_PATH) + "\\ODBC Drivers";
    reg_delete_value(HKEY_LOCAL_MACHINE, drivers_list_key, name);

    // Delete the driver's own key
    const std::string driver_key = std::string(ODBCINST_INI_PATH) + "\\" + name;
    reg_delete_key(HKEY_LOCAL_MACHINE, driver_key);
  }
}

void WindowsConfigInstallation::uninstall_dsns_from_registry() {
  for (const auto& ds : data_sources_) {
    // Remove from the "ODBC Data Sources" list
    const std::string dsn_list_key = std::string(ODBC_INI_PATH) + "\\ODBC Data Sources";
    reg_delete_value(HKEY_CURRENT_USER, dsn_list_key, ds.name());

    // Delete the DSN's own key
    const std::string dsn_key = std::string(ODBC_INI_PATH) + "\\" + ds.name();
    reg_delete_key(HKEY_CURRENT_USER, dsn_key);
  }
}

#endif  // _WIN32
