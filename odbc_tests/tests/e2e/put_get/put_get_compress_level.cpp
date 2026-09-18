#include <fstream>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "get_data.hpp"
#include "put_get_utils.hpp"
#include "test_setup.hpp"

using namespace pg_utils;

TEST_CASE("should compress PUT files smaller at gzip level 9 than at level 1", "[put_get]") {
  TempTestDir src_dir("odbc_put_compresslv_");
  const auto file = src_dir.path() / "compressible.csv";
  {
    std::ofstream out(file, std::ios::binary);
    for (int i = 0; i < 32 * 1024; ++i) {
      out << "aaaaaaaaaa,bbbbbbbbbb,cccccccccc\n";
    }
  }

  // Given a compressible local file
  auto put_target_size = [&](int level) -> SQLINTEGER {
    Connection conn(get_connection_string() + "PUT_COMPRESSLV=" + std::to_string(level) + ";");
    const std::string stage = pg_utils::create_stage(conn, unique_stage_name("ODBCTST_COMPLV"));
    // When the file is uploaded with AUTO_COMPRESS at the requested gzip level
    auto put_stmt =
        conn.execute_fetch("PUT 'file://" + as_file_uri(file) + "' @" + stage + " AUTO_COMPRESS=TRUE OVERWRITE=TRUE");
    CHECK(get_data<SQL_C_CHAR>(put_stmt, PUT_ROW_STATUS_IDX) == "UPLOADED");
    CHECK(get_data<SQL_C_CHAR>(put_stmt, PUT_ROW_TARGET_COMPRESSION_IDX) == "gzip");
    return get_data<SQL_C_LONG>(put_stmt, PUT_ROW_TARGET_SIZE_IDX);
  };

  const SQLINTEGER size_level_1 = put_target_size(1);
  const SQLINTEGER size_level_9 = put_target_size(9);
  INFO("gzip level 1 target size=" << size_level_1 << " level 9 target size=" << size_level_9);
  // Then the gzip level-9 object is smaller than the level-1 object
  REQUIRE(size_level_9 < size_level_1);
}
