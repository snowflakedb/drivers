
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
