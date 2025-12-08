cmake_minimum_required(VERSION 3.20)

project(ODBCAdvancedDemo)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Find ODBC
find_package(ODBC REQUIRED)

# Add the demo executable
add_executable(advanced_demo advanced_demo.cpp)
add_executable(error_harness error_harness.cpp)

# Link ODBC libraries
target_link_libraries(advanced_demo PRIVATE ODBC::ODBC)
target_link_libraries(error_harness PRIVATE ODBC::ODBC)

# Set output directory
set_target_properties(advanced_demo PROPERTIES
    RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/demo"
)
set_target_properties(error_harness PROPERTIES
    RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/demo"
)

message(STATUS "ODBC Advanced Demo configured")

