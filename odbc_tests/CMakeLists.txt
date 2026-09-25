cmake_minimum_required(VERSION 4.0)
project(odbc_tests)

# Windows ARM64: each Ninja link edge is
#   cmake -E vs_link_exe (lld-link + mt.exe) && vcpkg z-applocal && CatchAddTests
# #2094 isolated RUNTIME_OUTPUT_DIRECTORY; #2168 isolated Catch2 *_tests.cmake.
# Parallel=2 still flakes after both: mt.exe / z-applocal / CatchAddTests POSTs
# share the same edge, and two edges run at once ("being used by another process").
# Serialize links only. Compiles stay at cmake --build --parallel $NPROC.
# Must be set before any targets. Ninja job pools only.
if(WIN32 AND CMAKE_GENERATOR STREQUAL "Ninja")
  if(CMAKE_SYSTEM_PROCESSOR MATCHES "ARM64|aarch64|ARM64EC")
    set_property(GLOBAL PROPERTY JOB_POOLS odbc_win_arm64_link=1)
    set(CMAKE_JOB_POOL_LINK odbc_win_arm64_link)
  endif()
endif()

# Catch2 POST_BUILD runs each test exe during that link edge. PRE_TEST delays
# discovery until ctest (run_tests_windows.ps1 / odbc_tests job). Linux/mac
# keep the default POST_BUILD.
if(WIN32)
  set(CMAKE_CATCH_DISCOVER_TESTS_DISCOVERY_MODE PRE_TEST)
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
  # Windows: vcpkg z-applocal copies DLLs next to the exe. Isolate each test's
  # output dir so those copies (and import libs / PDBs) do not share tests/e2e/.
  # Discovery is PRE_TEST (see CMAKE_CATCH_DISCOVER_TESTS_DISCOVERY_MODE above);
  # WORKING_DIRECTORY must still be the per-target dir so ctest finds the DLLs.
  if(WIN32)
    set(_odbc_test_outdir "${CMAKE_CURRENT_BINARY_DIR}/${name}")
    set_target_properties(${name} PROPERTIES
      RUNTIME_OUTPUT_DIRECTORY "${_odbc_test_outdir}"
      RUNTIME_OUTPUT_DIRECTORY_DEBUG "${_odbc_test_outdir}"
      RUNTIME_OUTPUT_DIRECTORY_RELEASE "${_odbc_test_outdir}"
      RUNTIME_OUTPUT_DIRECTORY_RELWITHDEBINFO "${_odbc_test_outdir}"
      RUNTIME_OUTPUT_DIRECTORY_MINSIZEREL "${_odbc_test_outdir}"
      ARCHIVE_OUTPUT_DIRECTORY "${_odbc_test_outdir}"
      ARCHIVE_OUTPUT_DIRECTORY_DEBUG "${_odbc_test_outdir}"
      ARCHIVE_OUTPUT_DIRECTORY_RELEASE "${_odbc_test_outdir}"
      ARCHIVE_OUTPUT_DIRECTORY_RELWITHDEBINFO "${_odbc_test_outdir}"
      ARCHIVE_OUTPUT_DIRECTORY_MINSIZEREL "${_odbc_test_outdir}"
      PDB_OUTPUT_DIRECTORY "${_odbc_test_outdir}"
      PDB_OUTPUT_DIRECTORY_DEBUG "${_odbc_test_outdir}"
      PDB_OUTPUT_DIRECTORY_RELEASE "${_odbc_test_outdir}"
      PDB_OUTPUT_DIRECTORY_RELWITHDEBINFO "${_odbc_test_outdir}"
      PDB_OUTPUT_DIRECTORY_MINSIZEREL "${_odbc_test_outdir}")
    file(MAKE_DIRECTORY "${_odbc_test_outdir}")
    catch_discover_tests(${name} TEST_PREFIX "${name}: " ADD_TAGS_AS_LABELS
      WORKING_DIRECTORY "${_odbc_test_outdir}")
  else()
    catch_discover_tests(${name} TEST_PREFIX "${name}: " ADD_TAGS_AS_LABELS)
  endif()
endfunction()

add_subdirectory(tests)
