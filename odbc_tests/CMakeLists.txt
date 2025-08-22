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

if (SKIP_NEW_TESTS STREQUAL "TRUE")
  set(CATCH_TAGS "~[new_odbc]")
else()
  set(CATCH_TAGS "")
endif()


# Find odbc library
find_package(ODBC REQUIRED)
message(STATUS "ODBC_INCLUDE_DIRS: ${ODBC_INCLUDE_DIRS}")
message(STATUS "ODBC_LIBRARIES: ${ODBC_LIBRARIES}")

add_subdirectory(common)

function(add_odbc_test name)
  add_executable(${name} ${ARGN})
  target_link_libraries(${name} PRIVATE Catch2::Catch2WithMain ODBC::ODBC common)
  target_include_directories(${name} PRIVATE ${picojson_SOURCE_DIR})
  add_test(NAME ${name} COMMAND ${name} ${CATCH_TAGS})
endfunction()

add_subdirectory(tests)
