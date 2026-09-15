cmake_minimum_required(VERSION 4.0)

add_odbc_test(replay_excel_powerquery_navigator_navigate_and_load navigate_and_load/test.cpp)
add_odbc_test(replay_excel_powerquery_navigator_query_folding query_folding/test.cpp)

if(MSVC)
  # These single-TEST_CASE replays (~26k and ~22k lines, Debug MSVC) allocate their frame on
  # entry, so they exceed the default 1 MB thread stack before the body runs — including
  # before a SKIP macro at the top of the body can take effect.
  target_link_options(replay_excel_powerquery_navigator_navigate_and_load PRIVATE "/STACK:8388608")
  target_link_options(replay_excel_powerquery_navigator_query_folding PRIVATE "/STACK:8388608")
endif()
