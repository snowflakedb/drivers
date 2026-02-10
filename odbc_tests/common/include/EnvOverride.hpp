#ifndef ENV_OVERRIDE_HPP
#define ENV_OVERRIDE_HPP

#include <cstdlib>
#include <optional>
#include <string>

#ifdef _WIN32
#include <stdlib.h>  // _putenv_s
#endif

// RAII class for temporarily overriding environment variables.
// Supports both Unix (setenv/unsetenv) and Windows (_putenv_s).
class EnvOverride {
 public:
  // Sets the environment variable to the new value, saving the original.
  EnvOverride(const std::string& name, const std::string& value) : name_(name) {
    // Save original value
    if (const char* original = std::getenv(name.c_str()); original != nullptr) {
      original_value_ = std::string(original);
    }
    // Set new value
    set_env(name_, value);
  }

  // Unsets the environment variable, saving the original.
  explicit EnvOverride(const std::string& name) : name_(name) {
    // Save original value
    if (const char* original = std::getenv(name.c_str()); original != nullptr) {
      original_value_ = std::string(original);
    }
    // Unset the variable
    unset_env(name_);
  }

  ~EnvOverride() {
    if (!name_.empty()) {
      if (original_value_.has_value()) {
        set_env(name_, *original_value_);
      } else {
        unset_env(name_);
      }
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
          set_env(name_, *original_value_);
        } else {
          unset_env(name_);
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
  static void set_env(const std::string& name, const std::string& value) {
#ifdef _WIN32
    _putenv_s(name.c_str(), value.c_str());
#else
    setenv(name.c_str(), value.c_str(), 1);
#endif
  }

  static void unset_env(const std::string& name) {
#ifdef _WIN32
    _putenv_s(name.c_str(), "");
#else
    unsetenv(name.c_str());
#endif
  }

  std::string name_;
  std::optional<std::string> original_value_;
};

#endif  // ENV_OVERRIDE_HPP
