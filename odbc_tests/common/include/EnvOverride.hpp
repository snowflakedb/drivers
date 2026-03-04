#ifndef ENV_OVERRIDE_HPP
#define ENV_OVERRIDE_HPP

#include <cstdlib>
#include <optional>
#include <string>

#include <catch2/catch_test_macros.hpp>

#ifdef _WIN32
#include <stdlib.h>
inline void portable_setenv(const char* name, const char* value) { _putenv_s(name, value); }
inline void portable_unsetenv(const char* name) { _putenv_s(name, ""); }
#else
inline void portable_setenv(const char* name, const char* value) { setenv(name, value, 1); }
inline void portable_unsetenv(const char* name) { unsetenv(name); }
#endif

// RAII class for temporarily overriding environment variables
class EnvOverride {
 public:
  // Sets the environment variable to the new value, saving the original.
  EnvOverride(const std::string& name, const std::string& value) : name_(name) {
    // Save original value
    if (const char* original = std::getenv(name.c_str()); original != nullptr) {
      original_value_ = std::string(original);
    }
    // Set new value
    portable_setenv(name.c_str(), value.c_str());
  }

  // Unsets the environment variable, saving the original.
  explicit EnvOverride(const std::string& name) : name_(name) {
    // Save original value
    if (const char* original = std::getenv(name.c_str()); original != nullptr) {
      original_value_ = std::string(original);
    }
    // Unset the variable
    portable_unsetenv(name.c_str());
  }

  ~EnvOverride() {
    if (original_value_.has_value()) {
      // Restore original value
      portable_setenv(name_.c_str(), original_value_->c_str());
    } else {
      // Variable was not set originally, unset it
      portable_unsetenv(name_.c_str());
    }
  }

  // Non-copyable
  EnvOverride(const EnvOverride&) = delete;
  EnvOverride& operator=(const EnvOverride&) = delete;

  // Movable
  EnvOverride(EnvOverride&& other) noexcept
      : name_(std::move(other.name_)), original_value_(std::move(other.original_value_)) {
    other.name_.clear();  // Mark as moved-from
  }

  EnvOverride& operator=(EnvOverride&& other) noexcept {
    if (this != &other) {
      // Restore our original value before taking on new responsibility
      if (!name_.empty()) {
        if (original_value_.has_value()) {
          portable_setenv(name_.c_str(), original_value_->c_str());
        } else {
          portable_unsetenv(name_.c_str());
        }
      }
      name_ = std::move(other.name_);
      original_value_ = std::move(other.original_value_);
      other.name_.clear();
    }
    return *this;
  }

  // Get the variable name
  [[nodiscard]] const std::string& name() const { return name_; }

  // Get the original value (if it was set)
  [[nodiscard]] const std::optional<std::string>& original_value() const { return original_value_; }

 private:
  std::string name_;
  std::optional<std::string> original_value_;
};

#endif  // ENV_OVERRIDE_HPP
