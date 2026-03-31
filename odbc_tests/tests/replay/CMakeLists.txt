cmake_minimum_required(VERSION 4.0)

# Replay tests generated from ODBC trace logs.
# To generate a test file from a trace log, use odbc-trace-tool:
#
#   cargo run -p odbc_trace_tool -- generate \
#     --input path/to/trace.log \
#     --output odbc_tests/tests/replay/my_trace_test.cpp \
#     --test-name "My Trace"
#
# Then add the generated .cpp file to the list below.

add_odbc_test(replay_iodbctest_select_1 iodbctest/select_1.cpp)
