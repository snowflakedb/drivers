#ifndef COMPATIBILITY_HPP
#define COMPATIBILITY_HPP

#include <catch2/catch_test_macros.hpp>

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

#define SKIP_NEW_DRIVER_NOT_IMPLEMENTED()        \
  do {                                           \
    if (get_driver_type() == DRIVER_TYPE::NEW) { \
      SKIP("Not implemented for new driver");    \
    }                                            \
  } while (0)

#endif  // COMPATIBILITY_HPP
