add_executable(perf_runner perf_runner.cpp)
target_link_libraries(perf_runner PRIVATE ODBC::ODBC common)
target_include_directories(perf_runner PRIVATE ${picojson_SOURCE_DIR})

