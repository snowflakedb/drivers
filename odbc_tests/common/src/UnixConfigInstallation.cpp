#ifndef _WIN32

#include <filesystem>
#include <fstream>
#include <iostream>
#include <random>
#include <stdexcept>

#include "ODBCConfig.hpp"
#include "compatibility.hpp"

// ============================================================================
// UnixConfigInstallation
// ============================================================================

UnixConfigInstallation UnixConfigInstallation::install(const std::vector<DataSourceConfig>& data_sources) {
  return UnixConfigInstallation(data_sources, {});
}

UnixConfigInstallation UnixConfigInstallation::install_driver(const std::shared_ptr<DriverConfig>& driver_config) {
  return UnixConfigInstallation({}, {driver_config});
}

UnixConfigInstallation::UnixConfigInstallation(const std::vector<DataSourceConfig>& data_sources,
                                               const std::set<std::shared_ptr<DriverConfig>>& driver_configs)
    : BaseConfigInstallation(data_sources, driver_configs) {
  config_dir_ = create_temp_dir();
  write_odbcinst_ini();
  write_odbc_ini();
  env_overrides_.emplace_back("ODBCSYSINI", config_dir_);
  env_overrides_.emplace_back("ODBCINI", (std::filesystem::path(config_dir_) / "odbc.ini").string());
}

UnixConfigInstallation::~UnixConfigInstallation() {
  if (!config_dir_.empty()) {
    std::error_code ec;
    std::filesystem::remove_all(config_dir_, ec);
    if (ec) {
      std::cerr << "Warning: Failed to remove temporary config directory '" << config_dir_ << "': " << ec.message()
                << std::endl;
    }
  }
}

UnixConfigInstallation::UnixConfigInstallation(UnixConfigInstallation&& other) noexcept
    : BaseConfigInstallation(std::move(other)) {}

UnixConfigInstallation& UnixConfigInstallation::operator=(UnixConfigInstallation&& other) noexcept {
  if (this != &other) {
    if (!config_dir_.empty()) {
      std::error_code ec;
      std::filesystem::remove_all(config_dir_, ec);
      if (ec) {
        std::cerr << "Warning: Failed to remove temporary config directory '" << config_dir_ << "': " << ec.message()
                  << std::endl;
      }
    }
    BaseConfigInstallation::operator=(std::move(other));
  }
  return *this;
}

std::string UnixConfigInstallation::create_temp_dir() {
  std::random_device rd;
  std::mt19937 gen(rd());
  std::uniform_int_distribution<> dis(0, 999999);

  const int pid = GET_PROCESS_ID();
  const std::string dir_name = "odbc_dsn_" + std::to_string(pid) + "_" + std::to_string(dis(gen));
  const std::filesystem::path full_path = std::filesystem::current_path() / dir_name;

  std::error_code ec;
  if (!std::filesystem::create_directories(full_path, ec) && ec) {
    throw std::runtime_error("Failed to create temporary config directory '" + full_path.string() +
                             "': " + ec.message());
  }
  return full_path.string();
}

void UnixConfigInstallation::write_odbcinst_ini() const {
  const std::string file_path = (std::filesystem::path(config_dir_) / "odbcinst.ini").string();
  std::ofstream file(file_path);

  if (!file.is_open()) {
    throw std::runtime_error("Failed to open odbcinst.ini for writing: " + file_path);
  }

  file << "[ODBC]\n";
  file << "Trace=no\n";
  file << "TraceFile=" << (std::filesystem::path(config_dir_) / "odbc_trace.log").string() << "\n";
  file << "\n";

  if (!file.good()) {
    throw std::runtime_error("Failed to write ODBC section to odbcinst.ini");
  }

  for (const auto& dc : driver_configs_) {
    const std::string name = dc->name();
    file << "[" << name << "]\n";
    for (const auto& [key, value] : dc->parameters()) {
      file << key << "=" << value << "\n";
    }
    file << "\n";

    if (!file.good()) {
      throw std::runtime_error("Failed to write driver config for '" + name + "' to odbcinst.ini");
    }
  }
}

void UnixConfigInstallation::write_odbc_ini() const {
  const std::string file_path = (std::filesystem::path(config_dir_) / "odbc.ini").string();
  std::ofstream file(file_path);

  if (!file.is_open()) {
    throw std::runtime_error("Failed to open odbc.ini for writing: " + file_path);
  }

  if (data_sources_.empty()) {
    file << "";
    return;
  }

  file << "[ODBC Data Sources]\n";
  for (const auto& ds : data_sources_) {
    if (auto dc = ds.driver_config()) {
      file << ds.name() << "=" << dc.value()->name() << "\n";
    }
  }
  file << "\n";

  if (!file.good()) {
    throw std::runtime_error("Failed to write data sources section to odbc.ini");
  }

  for (const auto& ds : data_sources_) {
    file << "[" << ds.name() << "]\n";
    for (const auto& [key, value] : ds.parameters()) {
      if (!value.empty()) {
        file << key << "=" << value << "\n";
      }
    }
    file << "\n";

    if (!file.good()) {
      throw std::runtime_error("Failed to write data source config for '" + ds.name() + "' to odbc.ini");
    }
  }
}

#endif  // _WIN32
