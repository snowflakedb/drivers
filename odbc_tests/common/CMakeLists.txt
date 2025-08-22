cmake_minimum_required(VERSION 4.0)

add_library(common STATIC
    src/test_setup.cpp
    include/test_setup.hpp
    include/macros.hpp
    include/Connection.hpp
    include/require.hpp
    include/get_data.hpp
    include/MetaOfSqlCTypes.hpp
)
target_include_directories(common PUBLIC ${picojson_SOURCE_DIR} ${CMAKE_CURRENT_SOURCE_DIR}/include)
target_link_libraries(common PUBLIC Catch2::Catch2WithMain ODBC::ODBC)
