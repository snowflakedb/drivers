cmake_minimum_required(VERSION 3.20)
project(odbc_perf_driver)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

include(FetchContent)
FetchContent_Declare(
  json
  URL https://github.com/nlohmann/json/releases/download/v3.11.3/json.tar.xz
)
FetchContent_MakeAvailable(json)

# Find ODBC
find_library(ODBC_LIBRARY NAMES odbc REQUIRED)
find_path(ODBC_INCLUDE_DIR sql.h REQUIRED)

# Create executable with all source files
add_executable(odbc-perf-driver
    main.cpp
    binding_execution.cpp
    config.cpp
    connection.cpp
    concurrent_execution.cpp
    perf_metrics.cpp
    put_execution.cpp
    query_execution.cpp
    results.cpp
)

# Link ODBC, pthreads (for ResourceMonitor background thread) and dl (for dlsym)
find_package(Threads REQUIRED)
target_include_directories(odbc-perf-driver PRIVATE ${ODBC_INCLUDE_DIR})
target_link_libraries(odbc-perf-driver ${ODBC_LIBRARY} Threads::Threads ${CMAKE_DL_LIBS} nlohmann_json::nlohmann_json)

# Install
install(TARGETS odbc-perf-driver DESTINATION /usr/local/bin)
