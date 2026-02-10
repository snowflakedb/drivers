#ifndef COMPATIBILITY_HPP
#define COMPATIBILITY_HPP

#include <catch2/catch_test_macros.hpp>

#include "compatibility.hpp"

// Cross-platform process ID
#ifdef _WIN32
#include <process.h>
#define GET_PROCESS_ID() _getpid()
#else
#include <unistd.h>
#define GET_PROCESS_ID() getpid()
#endif

enum class DRIVER_TYPE {
  NEW = 0,
  OLD = 1,
};

extern DRIVER_TYPE get_driver_type();

#define NEW_DRIVER_ONLY(x) if (get_driver_type() == DRIVER_TYPE::NEW)

#define OLD_DRIVER_ONLY(x) if (get_driver_type() == DRIVER_TYPE::OLD)

#define SKIP_OLD_DRIVER(bd, message)                            \
  if (get_driver_type() == DRIVER_TYPE::OLD) {                  \
    SKIP("Skipping for old driver: " << bd << ": " << message); \
  }

#define SKIP_NEW_DRIVER(bd, message)                            \
  if (get_driver_type() == DRIVER_TYPE::NEW) {                  \
    SKIP("Skipping for new driver: " << bd << ": " << message); \
  }

// Skip test if running against the new Universal Driver
#define SKIP_NEW_DRIVER_NOT_IMPLEMENTED()                          \
  do {                                                             \
    if (get_driver_type() == DRIVER_TYPE::NEW) {                   \
      SKIP("Feature not yet implemented in new Universal Driver"); \
    }                                                              \
  } while (0)

// Skip test on Windows. TODO: Re-enable these tests on Windows once the underlying issues are resolved.
#ifdef _WIN32
#define SKIP_ON_WINDOWS(message)              \
  do {                                        \
    SKIP("Skipping on Windows: " << message); \
  } while (0)
#else
#define SKIP_ON_WINDOWS(message) \
  do {                           \
  } while (0)
#endif

#endif  // COMPATIBILITY_HPP
