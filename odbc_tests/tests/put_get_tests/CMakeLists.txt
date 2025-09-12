cmake_minimum_required(VERSION 4.0)

add_odbc_test(put_get_tests
  test_put_get_simple.cpp
  test_put_get_overwrite.cpp
  test_put_get_wildcards.cpp
  test_put_get_auto_compress.cpp
)


