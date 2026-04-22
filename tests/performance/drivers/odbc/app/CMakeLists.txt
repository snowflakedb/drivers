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
    perf_metrics.cpp
    put_execution.cpp
    query_execution.cpp
    results.cpp
)

# Link ODBC, pthreads (for ResourceMonitor background thread) and dl (for dlsym)
find_package(Threads REQUIRED)
target_include_directories(odbc-perf-driver PRIVATE ${ODBC_INCLUDE_DIR})
target_link_libraries(odbc-perf-driver ${ODBC_LIBRARY} Threads::Threads ${CMAKE_DL_LIBS})

# Install
install(TARGETS odbc-perf-driver DESTINATION /usr/local/bin)
