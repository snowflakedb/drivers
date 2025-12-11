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
  return {"test_data.csv.gz", test_utils::shared_test_data_dir() / "compression" / "test_data.csv.gz"};
}

TEST_CASE("should compress the file before uploading to stage when AUTO_COMPRESS set to true", "[put_get]") {
  Connection conn;
  // Given Snowflake client is logged in
  const std::string stage = pg_utils::create_stage(conn, unique_stage_name("ODBCTST_COMPRESS"));
  auto [filename, file] = uncompressed_test_file();
  auto [compressed, file_gz] = compressed_test_file();

  // When File is uploaded to stage with AUTO_COMPRESS set to true
  auto put_stmt = conn.execute_fetch("PUT 'file://" + as_file_uri(file) + "' @" + stage + " AUTO_COMPRESS=TRUE");
  std::string src = get_data<SQL_C_CHAR>(put_stmt, PUT_ROW_SOURCE_IDX);
  std::string tgt = get_data<SQL_C_CHAR>(put_stmt, PUT_ROW_TARGET_IDX);
  std::string status = get_data<SQL_C_CHAR>(put_stmt, PUT_ROW_STATUS_IDX);
  CHECK(src == filename);
  CHECK(tgt == compressed);
  CHECK(status == "UPLOADED");

  TempTestDir download_dir("odbc_put_get_");

  // Then Only compressed file should be downloaded
  auto get_stmt =
      conn.execute_fetch("GET @" + stage + "/" + filename + " 'file://" + as_file_uri(download_dir.path()) + "/'");
  std::string file_col = get_data<SQL_C_CHAR>(get_stmt, GET_ROW_FILE_IDX);
  std::string get_status = get_data<SQL_C_CHAR>(get_stmt, GET_ROW_STATUS_IDX);
  CHECK(file_col == compressed);
  CHECK(get_status == "DOWNLOADED");

  REQUIRE(fs::exists(download_dir.path() / compressed));
  REQUIRE(!fs::exists(download_dir.path() / filename));

  // And Have correct content
  std::ifstream dl(download_dir.path() / compressed, std::ios::binary);
  std::string downloaded_bytes((std::istreambuf_iterator<char>(dl)), std::istreambuf_iterator<char>());
  std::ifstream ref(file_gz, std::ios::binary);
  std::string reference_bytes((std::istreambuf_iterator<char>(ref)), std::istreambuf_iterator<char>());

  OLD_DRIVER_ONLY("BD#5") { CHECK(downloaded_bytes != reference_bytes); }
  NEW_DRIVER_ONLY("BD#5") { CHECK(downloaded_bytes == reference_bytes); }
}

TEST_CASE("should not compress the file before uploading to stage when AUTO_COMPRESS set to false", "[put_get]") {
  Connection conn;
  // Given Snowflake client is logged in
  const std::string stage = pg_utils::create_stage(conn, unique_stage_name("ODBCTST_COMPRESS"));
  auto [filename, file] = uncompressed_test_file();
  auto [compressed, file_gz] = compressed_test_file();

  // When File is uploaded to stage with AUTO_COMPRESS set to false
  auto put_stmt = conn.execute_fetch("PUT 'file://" + as_file_uri(file) + "' @" + stage + " AUTO_COMPRESS=FALSE");
  std::string src = get_data<SQL_C_CHAR>(put_stmt, PUT_ROW_SOURCE_IDX);
  std::string tgt = get_data<SQL_C_CHAR>(put_stmt, PUT_ROW_TARGET_IDX);
  std::string status = get_data<SQL_C_CHAR>(put_stmt, PUT_ROW_STATUS_IDX);
  CHECK(src == filename);
  CHECK(tgt == filename);
  CHECK(status == "UPLOADED");

  TempTestDir download_dir("odbc_put_get_");

  // Then Only uncompressed file should be downloaded
  auto get_stmt =
      conn.execute_fetch("GET @" + stage + "/" + filename + " 'file://" + as_file_uri(download_dir.path()) + "/'");
  std::string file_col = get_data<SQL_C_CHAR>(get_stmt, GET_ROW_FILE_IDX);
  std::string get_status = get_data<SQL_C_CHAR>(get_stmt, GET_ROW_STATUS_IDX);
  CHECK(file_col == filename);
  CHECK(get_status == "DOWNLOADED");

  REQUIRE(fs::exists(download_dir.path() / filename));
  REQUIRE(!fs::exists(download_dir.path() / compressed));

  // And Have correct content
  std::ifstream ifs2(download_dir.path() / filename);
  std::string downloaded_content((std::istreambuf_iterator<char>(ifs2)), std::istreambuf_iterator<char>());
  std::ifstream ifs_src(file);
  std::string original_content((std::istreambuf_iterator<char>(ifs_src)), std::istreambuf_iterator<char>());
  CHECK(downloaded_content == original_content);
}
