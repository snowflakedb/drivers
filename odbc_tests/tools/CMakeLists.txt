add_executable(schema_tool schema_tool.cpp)
target_link_libraries(schema_tool PRIVATE common ODBC::ODBC Threads::Threads)
if(NOT WIN32)
  target_link_libraries(schema_tool PRIVATE ${CMAKE_DL_LIBS})
endif()

# Force flat output directory so the binary is at cmake-build/tools/schema_tool
# regardless of single-config (Makefile) vs multi-config (MSVC) generators.
set_target_properties(schema_tool PROPERTIES
    RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/tools"
    RUNTIME_OUTPUT_DIRECTORY_DEBUG "${CMAKE_BINARY_DIR}/tools"
    RUNTIME_OUTPUT_DIRECTORY_RELEASE "${CMAKE_BINARY_DIR}/tools")
