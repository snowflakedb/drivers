cmake_minimum_required(VERSION 4.0)

option(BUILD_SETUP_TOOLS "Build one-time DB setup utilities (not for CI)" OFF)
if(BUILD_SETUP_TOOLS)
  add_subdirectory(setup)
endif()

add_subdirectory(e2e)
add_subdirectory(integration)
add_subdirectory(basic_tests)
add_subdirectory(bindings_tests)
add_subdirectory(datatype_tests)
add_subdirectory(odbc-api)
