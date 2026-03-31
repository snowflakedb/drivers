#include <algorithm>
#include <stdexcept>

#include "ODBCConfig.hpp"

// ============================================================================
// BaseConfigInstallation
// ============================================================================

BaseConfigInstallation::BaseConfigInstallation(const std::vector<DataSourceConfig>& data_sources,
                                               const std::set<std::shared_ptr<DriverConfig>>& driver_configs)
    : data_sources_(data_sources), driver_configs_(driver_configs) {
  collect_driver_configs();
}

BaseConfigInstallation::BaseConfigInstallation(BaseConfigInstallation&& other) noexcept
    : config_dir_(std::move(other.config_dir_)),
      data_sources_(std::move(other.data_sources_)),
      driver_configs_(std::move(other.driver_configs_)),
      env_overrides_(std::move(other.env_overrides_)) {
  other.config_dir_.clear();
}

BaseConfigInstallation& BaseConfigInstallation::operator=(BaseConfigInstallation&& other) noexcept {
  config_dir_ = std::move(other.config_dir_);
  data_sources_ = std::move(other.data_sources_);
  driver_configs_ = std::move(other.driver_configs_);
  env_overrides_ = std::move(other.env_overrides_);
  other.config_dir_.clear();
  return *this;
}

const std::string& BaseConfigInstallation::config_dir() const { return config_dir_; }

std::string BaseConfigInstallation::dsn_name(size_t index) const {
  if (index >= data_sources_.size()) {
    throw std::out_of_range("Data source index out of range");
  }
  return data_sources_[index].name();
}

std::string BaseConfigInstallation::connection_string(size_t index) const {
  if (index >= data_sources_.size()) {
    throw std::out_of_range("Data source index out of range");
  }
  return data_sources_[index].connection_string();
}

void BaseConfigInstallation::collect_driver_configs() {
  for (const auto& ds : data_sources_) {
    if (auto dc = ds.driver_config()) {
      driver_configs_.insert(dc.value());
    }
  }

  for (const auto& dc : driver_configs_) {
    auto same_name =
        std::count_if(driver_configs_.begin(), driver_configs_.end(),
                      [&dc](const std::shared_ptr<DriverConfig>& other) { return other->name() == dc->name(); });
    if (same_name > 1) {
      throw std::runtime_error("Driver config name '" + dc->name() + "' is not unique");
    }
  }
}
