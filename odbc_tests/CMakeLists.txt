cmake_minimum_required(VERSION 4.0)
project(odbc_tests)

# Windows ARM64: Catch2 POST_BUILD (CatchAddTests.cmake) and vcpkg z-applocal
# copy DLLs into the same per-directory output folder. Parallel links then
# fail with "The process cannot access the file because it is being used by
# another process" (Driver CI build_odbc_tests_cpp). Serialize Ninja links;
# compiles can still run in parallel. Must be set before any targets.
if(WIN32 AND CMAKE_GENERATOR STREQUAL "Ninja")
  if(CMAKE_SYSTEM_PROCESSOR MATCHES "ARM64|aarch64|ARM64EC")
    set_property(GLOBAL PROPERTY JOB_POOLS odbc_win_arm64_link=1)
    set(CMAKE_JOB_POOL_LINK odbc_win_arm64_link)
  endif()
endif()

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
  catch_discover_tests(${name} TEST_PREFIX "${name}: " ADD_TAGS_AS_LABELS)
endfunction()

add_subdirectory(tests)
