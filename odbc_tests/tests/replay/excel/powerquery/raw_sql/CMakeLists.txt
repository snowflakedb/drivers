cmake_minimum_required(VERSION 4.0)

add_odbc_test(replay_excel_powerquery_raw_sql_select_1 select_1/test.cpp)
add_odbc_test(replay_excel_powerquery_raw_sql_all_datatypes all_datatypes/test.cpp)
add_odbc_test(replay_excel_powerquery_raw_sql_large_varchar large_varchar/test.cpp)
add_odbc_test(replay_excel_powerquery_raw_sql_no_data no_data/test.cpp)
if(MSVC)
  # The single ~4k-line TEST_CASE frame plus the driver's SQLExecDirect path exceeds
  # the default 1 MB thread stack.
  target_link_options(replay_excel_powerquery_raw_sql_no_data PRIVATE "/STACK:8388608")
endif()
add_odbc_test(replay_excel_powerquery_raw_sql_unknown_connection_param unknown_connection_param/test.cpp)
