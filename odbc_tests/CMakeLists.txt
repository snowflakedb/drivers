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

if (DRIVER_TYPE STREQUAL "OLD")
  set(CATCH_TAGS "~[new_odbc]")
endif()

if (DRIVER_TYPE STREQUAL "NEW")
  set(CATCH_TAGS "")
endif()


# Find odbc library
# Use unixODBC by default (compatible with universal driver)
# For official Snowflake driver testing, use iODBC build
if(DEFINED ENV{USE_IODBC})
    set(ODBC_INCLUDE_DIRS "/opt/homebrew/opt/libiodbc/include")
    set(ODBC_LIBRARIES "/opt/homebrew/opt/libiodbc/lib/libiodbc.dylib")
    message(STATUS "Using iODBC for official Snowflake driver compatibility")
else()
    set(ODBC_INCLUDE_DIRS "/opt/homebrew/opt/unixodbc/include")
    set(ODBC_LIBRARIES "/opt/homebrew/opt/unixodbc/lib/libodbc.dylib")
    message(STATUS "Using unixODBC for universal driver compatibility")
endif()

message(STATUS "ODBC_INCLUDE_DIRS: ${ODBC_INCLUDE_DIRS}")
message(STATUS "ODBC_LIBRARIES: ${ODBC_LIBRARIES}")

# Create ODBC::ODBC target
add_library(ODBC::ODBC SHARED IMPORTED)
set_target_properties(ODBC::ODBC PROPERTIES
    IMPORTED_LOCATION "${ODBC_LIBRARIES}"
    INTERFACE_INCLUDE_DIRECTORIES "${ODBC_INCLUDE_DIRS}"
)

add_subdirectory(common)

function(add_odbc_test name)
  add_executable(${name} ${ARGN})
  target_link_libraries(${name} PRIVATE Catch2::Catch2WithMain ODBC::ODBC common)
  target_include_directories(${name} PRIVATE ${picojson_SOURCE_DIR})
  add_test(NAME ${name} COMMAND ${name} ${CATCH_TAGS})
endfunction()

add_subdirectory(tests)
add_subdirectory(demo)
add_subdirectory(perf)
