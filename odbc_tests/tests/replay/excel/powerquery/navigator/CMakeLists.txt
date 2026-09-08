cmake_minimum_required(VERSION 4.0)

add_odbc_test(replay_excel_powerquery_navigator_navigate_and_load navigate_and_load/test.cpp)
if(MSVC)
  # Single TEST_CASE replay (~26k lines, Debug MSVC) exceeds the default 1 MB thread stack.
  target_link_options(replay_excel_powerquery_navigator_navigate_and_load PRIVATE "/STACK:8388608")
endif()
add_odbc_test(replay_excel_powerquery_navigator_query_folding query_folding/test.cpp)
