#include "compatibility.hpp"

DRIVER_TYPE get_driver_type() {
#ifdef SNOWFLAKE_OLD_DRIVER
  return DRIVER_TYPE::OLD;
#else
  return DRIVER_TYPE::NEW;
#endif
}
