
#define CHECK_ERROR(function, handle_type, handle)                                      \
  ret = function;                                                                       \
  if (ret != SQL_SUCCESS && ret != SQL_SUCCESS_WITH_INFO) {                             \
    SQLCHAR state[1024];                                                                \
    SQLCHAR message[1024];                                                              \
    SQLGetDiagRec(handle_type, handle, 1, state, NULL, message, sizeof(message), NULL); \
    printf("%s@%s:%d failed with error: %s\n", #function, __FILE__, __LINE__, message); \
    exit(1);                                                                            \
  }

#define ASSERT_SUCCESS(function)                                           \
  ret = function;                                                          \
  if (ret != SQL_SUCCESS && ret != SQL_SUCCESS_WITH_INFO) {                \
    printf("%s@%s:%d failed with error\n", #function, __FILE__, __LINE__); \
    exit(1);                                                               \
  }
