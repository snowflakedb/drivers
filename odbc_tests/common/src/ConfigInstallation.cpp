#include <filesystem>
#include <fstream>
#include <iostream>
#include <random>
#include <stdexcept>

#include "ODBCConfig.hpp"
#include "compatibility.hpp"

// ============================================================================
// ConfigInstallation
// ============================================================================

ConfigInstallation ConfigInstallation::install(const std::vector<DataSourceConfig>& data_sources) {
  return ConfigInstallation(data_sources);
}

ConfigInstallation::ConfigInstallation(const std::vector<DataSourceConfig>& data_sources)
    : data_sources_(data_sources) {
  // TODO: Windows - Use registry
  config_dir_ = create_temp_dir();
  write_odbcinst_ini();
  write_odbc_ini();
  env_overrides_.emplace_back("ODBCSYSINI", config_dir_);
  env_overrides_.emplace_back("ODBCINI", (std::filesystem::path(config_dir_) / "odbc.ini").string());
}

ConfigInstallation::~ConfigInstallation() {
  // TODO: Windows - Delete/restore registry keys
  if (!config_dir_.empty()) {
    std::error_code ec;
    std::filesystem::remove_all(config_dir_, ec);
    if (ec) {
      std::cerr << "Warning: Failed to remove temporary config directory '" << config_dir_ << "': " << ec.message()
                << std::endl;
    }
  }
}

ConfigInstallation::ConfigInstallation(ConfigInstallation&& other) noexcept
    : config_dir_(std::move(other.config_dir_)),
      data_sources_(std::move(other.data_sources_)),
      env_overrides_(std::move(other.env_overrides_)) {
  other.config_dir_.clear();
}

ConfigInstallation& ConfigInstallation::operator=(ConfigInstallation&& other) noexcept {
  if (this != &other) {
    if (!config_dir_.empty()) {
      std::error_code ec;
      std::filesystem::remove_all(config_dir_, ec);
      if (ec) {
        std::cerr << "Warning: Failed to remove temporary config directory '" << config_dir_ << "': " << ec.message()
                  << std::endl;
      }
    }
    config_dir_ = std::move(other.config_dir_);
    data_sources_ = std::move(other.data_sources_);
    env_overrides_ = std::move(other.env_overrides_);
    other.config_dir_.clear();
  }
  return *this;
}

const std::string& ConfigInstallation::config_dir() const { return config_dir_; }

std::string ConfigInstallation::dsn_name(size_t index) const {
  if (index >= data_sources_.size()) {
    throw std::out_of_range("Data source index out of range");
  }
  return data_sources_[index].name();
}

std::string ConfigInstallation::create_temp_dir() {
  std::random_device rd;
  std::mt19937 gen(rd());
  std::uniform_int_distribution<> dis(0, 999999);

  // Include process ID to avoid race conditions when multiple test processes run in parallel
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

void ConfigInstallation::write_odbcinst_ini() const {
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

  // Collect all unique driver configs
  std::map<std::string, std::shared_ptr<DriverConfig>> drivers;
  for (const auto& ds : data_sources_) {
    if (auto dc = ds.driver_config()) {
      drivers[dc.value()->name()] = dc.value();
    }
  }

  // Write each driver config
  for (const auto& [name, config] : drivers) {
    file << "[" << name << "]\n";
    for (const auto& [key, value] : config->parameters()) {
      file << key << "=" << value << "\n";
    }
    file << "\n";

    if (!file.good()) {
      throw std::runtime_error("Failed to write driver config for '" + name + "' to odbcinst.ini");
    }
  }
}

void ConfigInstallation::write_odbc_ini() const {
  const std::string file_path = (std::filesystem::path(config_dir_) / "odbc.ini").string();
  std::ofstream file(file_path);

  if (!file.is_open()) {
    throw std::runtime_error("Failed to open odbc.ini for writing: " + file_path);
  }

  // Write data sources section
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

  // Write each data source entry
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
