#include "compatibility.hpp"

#include <algorithm>
#include <cctype>
#include <cstdlib>
#include <cstring>
#include <string>

DRIVER_TYPE get_driver_type() {
#ifdef SNOWFLAKE_OLD_DRIVER
  return DRIVER_TYPE::OLD;
#else
  return DRIVER_TYPE::NEW;
#endif
}

PLATFORM get_platform() {
#if defined(_WIN32)
  return PLATFORM::PLATFORM_WINDOWS;
#elif defined(__linux__)
  return PLATFORM::PLATFORM_LINUX;
#elif defined(__APPLE__)
  return PLATFORM::PLATFORM_MACOS;
#else
  return PLATFORM::PLATFORM_UNKNOWN;
#endif
}

ARCH get_arch() {
#if defined(__aarch64__) || defined(_M_ARM64)
  return ARCH::ARCH_AARCH64;
#elif defined(__x86_64__) || defined(_M_X64) || defined(__amd64__)
  return ARCH::ARCH_X86_64;
#else
  return ARCH::ARCH_UNKNOWN;
#endif
}

bool is_iodbc_test_suite() {
#ifdef _WIN32
  return false;
#else
  const char* raw = std::getenv("SF_RUNNING_IODBC_TEST_SUITE");
  if (raw == nullptr) {
    return false;
  }
  std::string value(raw);
  std::transform(value.begin(), value.end(), value.begin(),
                 [](unsigned char c) { return static_cast<char>(std::tolower(c)); });
  if (value.empty() || value == "0" || value == "false" || value == "no" || value == "off") {
    return false;
  }
  return true;
#endif
}
