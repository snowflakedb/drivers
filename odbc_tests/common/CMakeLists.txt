cmake_minimum_required(VERSION 4.0)

find_package(ZLIB REQUIRED)
find_package(OpenSSL REQUIRED)

add_library(common STATIC
    src/test_setup.cpp
    src/compatibility.cpp
    src/get_diag_rec.cpp
    src/query_helpers.cpp
    src/ODBCConfig.cpp
    src/DataSourceConfig.cpp
    src/ConfigInstallation.cpp
    include/test_setup.hpp
    include/macros.hpp
    include/Connection.hpp
    include/require.hpp
    include/get_data.hpp
    include/MetaOfSqlCTypes.hpp
    include/get_diag_rec.hpp
    include/query_helpers.hpp
    include/compatibility.hpp
    include/ODBCConfig.hpp
    include/ODBCFixtures.hpp
    include/EnvOverride.hpp
)

target_include_directories(common PUBLIC ${picojson_SOURCE_DIR} ${CMAKE_CURRENT_SOURCE_DIR}/include)
target_link_libraries(common PUBLIC Catch2::Catch2WithMain ODBC::ODBC ZLIB::ZLIB OpenSSL::Crypto)
if (DRIVER_TYPE STREQUAL "OLD")
    target_compile_definitions(common PUBLIC SNOWFLAKE_OLD_DRIVER)
endif()
