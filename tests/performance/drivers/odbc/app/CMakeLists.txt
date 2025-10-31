cmake_minimum_required(VERSION 3.20)
project(odbc_perf_driver)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Find ODBC
find_library(ODBC_LIBRARY NAMES odbc REQUIRED)
find_path(ODBC_INCLUDE_DIR sql.h REQUIRED)

# Create executable with all source files
add_executable(odbc-perf-driver 
    main.cpp
    config.cpp
    connection.cpp
    execution.cpp
    results.cpp
)

# Link ODBC
target_include_directories(odbc-perf-driver PRIVATE ${ODBC_INCLUDE_DIR})
target_link_libraries(odbc-perf-driver ${ODBC_LIBRARY})

# Install
install(TARGETS odbc-perf-driver DESTINATION /usr/local/bin)
