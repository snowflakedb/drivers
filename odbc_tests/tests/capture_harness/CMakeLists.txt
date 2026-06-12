if(BUILD_CAPTURE_HARNESS)
  if(NOT EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/capture.cpp")
    message(FATAL_ERROR
      "BUILD_CAPTURE_HARNESS=ON but capture.cpp is missing; "
      "run odbc_trace_tool/capture_reference.sh first")
  endif()
  add_odbc_test(capture_harness capture.cpp)
endif()
