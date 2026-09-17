cmake_minimum_required(VERSION 4.0)
project(odbc_tests)

enable_testing()

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

Include(FetchContent)

FetchContent_Declare(
  Catch2
  GIT_REPOSITORY https://github.com/catchorg/Catch2.git
  GIT_TAG        v3.8.1 # or a later release
)

FetchContent_MakeAvailable(Catch2)

list(APPEND CMAKE_MODULE_PATH ${Catch2_SOURCE_DIR}/extras)
include(CTest)
include(Catch)

FetchContent_Declare(
  picojson
  GIT_REPOSITORY https://github.com/kazuho/picojson.git
  GIT_TAG v1.3.0
)

FetchContent_MakeAvailable(picojson)

if (NOT DEFINED DRIVER_TYPE)
  message(WARNING "DRIVER_TYPE not defined defaulting to NEW")
  set(DRIVER_TYPE "NEW")
endif()

# Find required libraries
find_package(ODBC REQUIRED)
find_package(Threads REQUIRED)

message(STATUS "ODBC_INCLUDE_DIRS: ${ODBC_INCLUDE_DIRS}")
message(STATUS "ODBC_LIBRARIES: ${ODBC_LIBRARIES}")

add_subdirectory(common)
add_subdirectory(tools)

function(add_odbc_test name)
  add_executable(${name} ${ARGN})
  target_sources(${name} PRIVATE
      "${PROJECT_SOURCE_DIR}/common/src/schema_cleanup_listener.cpp")
  target_link_libraries(${name} PRIVATE Catch2::Catch2WithMain ODBC::ODBC common Threads::Threads)
  if(NOT WIN32)
    target_link_libraries(${name} PRIVATE ${CMAKE_DL_LIBS})
  endif()
  target_include_directories(${name} PRIVATE ${picojson_SOURCE_DIR} ${PROJECT_SOURCE_DIR}/../odbc/include)
  # Windows: Catch2 POST_BUILD and vcpkg z-applocal copy DLLs next to the
  # executable. Sharing CMAKE_CURRENT_BINARY_DIR (tests/e2e has ~200 binaries)
  # races under parallel Ninja ("being used by another process"). Isolate each
  # test's runtime dir so copies do not collide; ctest uses TARGET_FILE.
  if(WIN32)
    set(_odbc_test_outdir "${CMAKE_CURRENT_BINARY_DIR}/${name}")
    set_target_properties(${name} PROPERTIES
      RUNTIME_OUTPUT_DIRECTORY "${_odbc_test_outdir}"
      RUNTIME_OUTPUT_DIRECTORY_DEBUG "${_odbc_test_outdir}"
      RUNTIME_OUTPUT_DIRECTORY_RELEASE "${_odbc_test_outdir}"
      RUNTIME_OUTPUT_DIRECTORY_RELWITHDEBINFO "${_odbc_test_outdir}"
      RUNTIME_OUTPUT_DIRECTORY_MINSIZEREL "${_odbc_test_outdir}")
  endif()
  catch_discover_tests(${name} TEST_PREFIX "${name}: " ADD_TAGS_AS_LABELS)
endfunction()

add_subdirectory(tests)
