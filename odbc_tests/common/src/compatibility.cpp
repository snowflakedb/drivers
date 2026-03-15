#include "compatibility.hpp"

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
