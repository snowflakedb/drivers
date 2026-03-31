#ifdef _WIN32

#include <stdexcept>
#include <string>

#include "ODBCConfig.hpp"

static const std::string ODBC_INI_REG_KEY = "SOFTWARE\\ODBC\\ODBC.INI";

static void reg_set_sz(HKEY root, const std::string& subkey, const std::string& name, const std::string& value) {
  HKEY hkey;
  LONG res =
      RegCreateKeyExA(root, subkey.c_str(), 0, nullptr, REG_OPTION_NON_VOLATILE, KEY_WRITE, nullptr, &hkey, nullptr);
  if (res != ERROR_SUCCESS) {
    std::string msg = "Failed to create registry key '" + subkey + "' (error " + std::to_string(res) + ")";
    if (res == ERROR_ACCESS_DENIED) {
      msg += ". ODBC driver registration on Windows requires administrator privileges.";
    }
    throw std::runtime_error(msg);
  }
  res = RegSetValueExA(hkey, name.c_str(), 0, REG_SZ, reinterpret_cast<const BYTE*>(value.c_str()),
                       static_cast<DWORD>(value.size() + 1));
  RegCloseKey(hkey);
  if (res != ERROR_SUCCESS) {
    throw std::runtime_error("Failed to set registry value '" + name + "' in key '" + subkey + "'");
  }
}

static void cleanup_registry_drivers(const std::set<std::shared_ptr<DriverConfig>>& driver_configs) {
  for (const auto& dc : driver_configs) {
    const std::string name = dc->name();
    RegDeleteKeyA(HKEY_CURRENT_USER, (ODBC_INI_REG_KEY + "\\" + name).c_str());
    HKEY hkey;
    if (RegOpenKeyExA(HKEY_CURRENT_USER, (ODBC_INI_REG_KEY + "\\ODBC Data Sources").c_str(), 0, KEY_SET_VALUE, &hkey) ==
        ERROR_SUCCESS) {
      RegDeleteValueA(hkey, name.c_str());
      RegCloseKey(hkey);
    }
  }
}

static void cleanup_registry_dsns(const std::vector<DataSourceConfig>& data_sources) {
  for (const auto& ds : data_sources) {
    const std::string dsn_name = ds.name();
    RegDeleteKeyA(HKEY_CURRENT_USER, (ODBC_INI_REG_KEY + "\\" + dsn_name).c_str());
    HKEY hkey;
    if (RegOpenKeyExA(HKEY_CURRENT_USER, (ODBC_INI_REG_KEY + "\\ODBC Data Sources").c_str(), 0, KEY_SET_VALUE, &hkey) ==
        ERROR_SUCCESS) {
      RegDeleteValueA(hkey, dsn_name.c_str());
      RegCloseKey(hkey);
    }
  }
}

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
    : BaseConfigInstallation(data_sources, driver_configs) {
  // The Microsoft ODBC DM only checks HKLM for ODBCINST.INI (driver registration),
  // which requires admin. Instead, create a User DSN in HKCU that references the
  // driver DLL path directly — the DM loads the DLL from the DSN's "Driver" value.
  // Tests must use DSN= instead of DRIVER={} in their connection strings on Windows.
  for (const auto& dc : driver_configs_) {
    const std::string name = dc->name();
    reg_set_sz(HKEY_CURRENT_USER, ODBC_INI_REG_KEY + "\\ODBC Data Sources", name, name);
    const std::string dsn_key = ODBC_INI_REG_KEY + "\\" + name;
    auto it = dc->parameters().find("Driver");
    if (it != dc->parameters().end()) {
      reg_set_sz(HKEY_CURRENT_USER, dsn_key, "Driver", it->second);
    }
  }
  for (const auto& ds : data_sources_) {
    const std::string dsn_name = ds.name();
    if (auto dc = ds.driver_config()) {
      reg_set_sz(HKEY_CURRENT_USER, ODBC_INI_REG_KEY + "\\ODBC Data Sources", dsn_name, dc.value()->name());
    }
    const std::string dsn_key = ODBC_INI_REG_KEY + "\\" + dsn_name;
    for (const auto& [k, v] : ds.parameters()) {
      if (!v.empty()) {
        reg_set_sz(HKEY_CURRENT_USER, dsn_key, k, v);
      }
    }
  }
}

WindowsConfigInstallation::~WindowsConfigInstallation() {
  cleanup_registry_dsns(data_sources_);
  cleanup_registry_drivers(driver_configs_);
}

WindowsConfigInstallation::WindowsConfigInstallation(WindowsConfigInstallation&& other) noexcept
    : BaseConfigInstallation(std::move(other)) {}

WindowsConfigInstallation& WindowsConfigInstallation::operator=(WindowsConfigInstallation&& other) noexcept {
  if (this != &other) {
    cleanup_registry_dsns(data_sources_);
    cleanup_registry_drivers(driver_configs_);
    BaseConfigInstallation::operator=(std::move(other));
  }
  return *this;
}

#endif  // _WIN32
