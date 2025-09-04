
enum class DRIVER_TYPE {
  NEW = 0,
  OLD = 1,
};

extern DRIVER_TYPE get_driver_type();

#define NEW_DRIVER_ONLY(x) if (get_driver_type() == DRIVER_TYPE::NEW)

#define OLD_DRIVER_ONLY(x) if (get_driver_type() == DRIVER_TYPE::OLD)
