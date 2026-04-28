cmake_minimum_required(VERSION 4.0)

add_odbc_test(odbc_api_alloc_handle_tests alloc_handle_tests.cpp)
add_odbc_test(odbc_api_connect_tests connect_tests.cpp)
add_odbc_test(odbc_api_driver_connect_tests driver_connect_tests.cpp)
add_odbc_test(odbc_api_browse_connect_tests browse_connect_tests.cpp)
add_odbc_test(odbc_api_conn_attr_tests conn_attr_tests.cpp)
add_odbc_test(odbc_api_stmt_attr_tests stmt_attr_tests.cpp)

if(WIN32 AND DRIVER_TYPE STREQUAL "NEW")
  add_executable(odbc_api_config_dsn_tests config_dsn_tests.cpp)
  target_link_libraries(odbc_api_config_dsn_tests PRIVATE Catch2::Catch2WithMain)
  catch_discover_tests(odbc_api_config_dsn_tests TEST_PREFIX "odbc_api_config_dsn_tests: ")
endif()
