#include <algorithm>
#include <filesystem>
#include <fstream>
#include <random>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "get_data.hpp"
#include "put_get_utils.hpp"

namespace fs = std::filesystem;

using namespace pg_utils;

TEST_CASE("PUT+GET with AUTO_COMPRESS=TRUE", "[put_get][odbc]") {
  Connection conn;
  const std::string stage = pg_utils::create_stage(conn, "ODBCTST_COMPRESS_TRUE");
  const std::string filename = "test_put_get_compress_true.csv";
  const std::string compressed = filename + ".gz";

  // Create test file with CSV data
  fs::path tmp = fs::temp_directory_path() / (std::string("odbc_put_get_") + random_hex());
  fs::create_directories(tmp);
  fs::path file = write_text_file(tmp, filename, "1,2,3\n");

  // PUT with AUTO_COMPRESS=TRUE
  {
    // Upload file
    auto stmt = conn.execute_fetch("PUT 'file://" + as_file_uri(file) + "' @" + stage +
                                   " AUTO_COMPRESS=TRUE");
    std::string src = get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_IDX);
    std::string tgt = get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_IDX);
    std::string status = get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX);
    CHECK(src == filename);
    CHECK(tgt == compressed);
    CHECK(status == "UPLOADED");
  }

  // Create directory for download
  fs::path download_dir = tmp / "download";
  fs::create_directories(download_dir);
  {
    // Download file using GET
    auto stmt = conn.execute_fetch("GET @" + stage + "/" + filename + " 'file://" +
                                   as_file_uri(download_dir) + "/'");
    std::string file_col = get_data<SQL_C_CHAR>(stmt, GET_ROW_FILE_IDX);
    std::string status = get_data<SQL_C_CHAR>(stmt, GET_ROW_STATUS_IDX);
    CHECK(file_col == compressed);
    CHECK(status == "DOWNLOADED");
  }

  // Verify the downloaded file exists and content matches
  REQUIRE(fs::exists(download_dir / compressed));
  REQUIRE(!fs::exists(download_dir / filename));
  // Decompress and verify content
  std::string decompressed = decompress_gzip_file(download_dir / compressed);
  std::ifstream ifs(file);
  std::string original_content((std::istreambuf_iterator<char>(ifs)),
                               std::istreambuf_iterator<char>());
  CHECK(decompressed == original_content);
}

TEST_CASE("PUT+GET with AUTO_COMPRESS=FALSE", "[put_get][odbc]") {
  Connection conn;
  const std::string stage = pg_utils::create_stage(conn, "ODBCTST_COMPRESS_FALSE");
  const std::string filename = "test_put_get_compress_false.csv";
  const std::string compressed = filename + ".gz";

  // Create test file with CSV data
  fs::path tmp = fs::temp_directory_path() / (std::string("odbc_put_get_") + random_hex());
  fs::create_directories(tmp);
  fs::path file = write_text_file(tmp, filename, "1,2,3\n");

  // PUT with AUTO_COMPRESS=FALSE
  {
    // Upload file
    auto stmt = conn.execute_fetch("PUT 'file://" + as_file_uri(file) + "' @" + stage +
                                   " AUTO_COMPRESS=FALSE");
    std::string src = get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_IDX);
    std::string tgt = get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_IDX);
    std::string status = get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX);
    CHECK(src == filename);
    CHECK(tgt == filename);
    CHECK(status == "UPLOADED");
  }

  // Create directory for download
  fs::path download_dir = tmp / "download";
  fs::create_directories(download_dir);
  {
    // Download file using GET
    auto stmt = conn.execute_fetch("GET @" + stage + "/" + filename + " 'file://" +
                                   as_file_uri(download_dir) + "/'");
    std::string file_col = get_data<SQL_C_CHAR>(stmt, GET_ROW_FILE_IDX);
    std::string status = get_data<SQL_C_CHAR>(stmt, GET_ROW_STATUS_IDX);
    CHECK(file_col == filename);
    CHECK(status == "DOWNLOADED");
  }

  // Verify the downloaded file exists and content matches
  REQUIRE(fs::exists(download_dir / filename));
  REQUIRE(!fs::exists(download_dir / compressed));
  std::ifstream ifs2(download_dir / filename);
  std::string downloaded_content((std::istreambuf_iterator<char>(ifs2)),
                                 std::istreambuf_iterator<char>());
  std::ifstream ifs_src(file);
  std::string original_content((std::istreambuf_iterator<char>(ifs_src)),
                               std::istreambuf_iterator<char>());
  CHECK(downloaded_content == original_content);
}
