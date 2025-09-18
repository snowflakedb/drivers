#include <algorithm>
#include <filesystem>
#include <fstream>
#include <random>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "get_data.hpp"
#include "put_get_utils.hpp"
#include "utils.hpp"

namespace fs = std::filesystem;
using namespace pg_utils;

static std::pair<std::string, fs::path> uncompressed_test_file() {
  return {"test_data.csv", test_utils::shared_test_data_dir() / "compression" / "test_data.csv"};
}

static std::pair<std::string, fs::path> compressed_test_file() {
  return {"test_data.csv.gz",
          test_utils::shared_test_data_dir() / "compression" / "test_data.csv.gz"};
}

TEST_CASE("PUT+GET with AUTO_COMPRESS=TRUE", "[put_get][odbc]") {
  Connection conn;
  const std::string stage = pg_utils::create_stage(conn, "ODBCTST_COMPRESS_TRUE");
  auto [filename, file] = uncompressed_test_file();
  auto [compressed, file_gz] = compressed_test_file();

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
  fs::path download_dir = fs::temp_directory_path() / (std::string("odbc_put_get_") + random_hex());
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
  // Compare compressed bytes (compat layer differences handled by OLD/NEW guards)
  std::ifstream dl(download_dir / compressed, std::ios::binary);
  std::string downloaded_bytes((std::istreambuf_iterator<char>(dl)),
                               std::istreambuf_iterator<char>());
  std::ifstream ref(file_gz, std::ios::binary);
  std::string reference_bytes((std::istreambuf_iterator<char>(ref)),
                              std::istreambuf_iterator<char>());
  OLD_DRIVER_ONLY("BC#1") { CHECK(downloaded_bytes != reference_bytes); }
  NEW_DRIVER_ONLY("BC#1") { CHECK(downloaded_bytes == reference_bytes); }
}

TEST_CASE("PUT+GET with AUTO_COMPRESS=FALSE", "[put_get][odbc]") {
  Connection conn;
  const std::string stage = pg_utils::create_stage(conn, "ODBCTST_COMPRESS_FALSE");
  auto [filename, file] = uncompressed_test_file();
  auto [compressed, file_gz] = compressed_test_file();

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
  fs::path download_dir = fs::temp_directory_path() / (std::string("odbc_put_get_") + random_hex());
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
