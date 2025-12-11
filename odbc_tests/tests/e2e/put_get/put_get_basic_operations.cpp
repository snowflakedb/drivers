#include <algorithm>
#include <filesystem>
#include <fstream>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "get_data.hpp"
#include "put_get_utils.hpp"
#include "utils.hpp"

using namespace pg_utils;
namespace fs = std::filesystem;

static std::string to_lower_copy(const std::string& s) {
  std::string out = s;
  std::transform(out.begin(), out.end(), out.begin(), [](unsigned char c) { return std::tolower(c); });
  return out;
}

static std::pair<std::string, fs::path> basic_test_file() {
  return {"test_data.csv", test_utils::shared_test_data_dir() / "basic" / "test_data.csv"};
}

TEST_CASE("should select data from file uploaded to stage", "[put_get]") {
  // Given File is uploaded to stage
  Connection conn;
  const std::string stage = pg_utils::create_stage(conn, unique_stage_name("ODBCTST_BASIC_OPS"));
  auto [filename, file] = basic_test_file();

  std::string put_sql = "PUT 'file://" + as_file_uri(file) + "' @" + stage;
  conn.execute(put_sql);

  // When File data is queried using Select command
  std::string select_sql = "SELECT $1, $2, $3 FROM @" + stage;
  auto stmt = conn.execute_fetch(select_sql);

  // Then File data should be correctly returned
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "1");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "2");
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "3");
}

TEST_CASE("should list file uploaded to stage", "[put_get]") {
  // Given File is uploaded to stage
  Connection conn;
  const std::string stage = pg_utils::create_stage(conn, unique_stage_name("ODBCTST_BASIC_OPS"));
  auto [filename, file] = basic_test_file();

  std::string put_sql = "PUT 'file://" + as_file_uri(file) + "' @" + stage;
  conn.execute(put_sql);

  // When Stage content is listed using LS command
  std::string ls_sql = "LS @" + stage;
  auto stmt = conn.execute_fetch(ls_sql);

  // Then File should be listed with correct filename
  std::string name = get_data<SQL_C_CHAR>(stmt, LS_ROW_NAME_IDX);
  std::string expected = to_lower_copy(stage) + "/" + filename + ".gz";
  CHECK(name == expected);
}

TEST_CASE("should get file uploaded to stage", "[put_get]") {
  // Given File is uploaded to stage
  Connection conn;
  const std::string stage = pg_utils::create_stage(conn, unique_stage_name("ODBCTST_BASIC_OPS"));
  auto [filename, file] = basic_test_file();

  std::string put_sql = "PUT 'file://" + as_file_uri(file) + "' @" + stage;
  conn.execute(put_sql);

  // When File is downloaded using GET command
  TempTestDir download_dir("odbc_put_get_");

  std::string get_sql = "GET @" + stage + "/" + filename + " 'file://" + as_file_uri(download_dir.path()) + "/'";
  auto stmt = conn.execute_fetch(get_sql);

  // Then File should be downloaded
  CHECK(get_data<SQL_C_CHAR>(stmt, GET_ROW_FILE_IDX) == filename + ".gz");

  fs::path gz = download_dir.path() / (filename + ".gz");
  REQUIRE(fs::exists(gz));

  // And Have correct content
  std::string decompressed = decompress_gzip_file(gz);
  std::ifstream ifs(file);
  std::string original_content((std::istreambuf_iterator<char>(ifs)), std::istreambuf_iterator<char>());
  CHECK(decompressed == original_content);
}

TEST_CASE("should return correct rowset for PUT", "[put_get]") {
  // Given Snowflake client is logged in
  Connection conn;
  const std::string stage = pg_utils::create_stage(conn, unique_stage_name("ODBCTST_BASIC_ROWSET"));
  auto [filename, file] = basic_test_file();

  // When File is uploaded to stage
  std::string put_sql = "PUT 'file://" + as_file_uri(file) + "' @" + stage;
  auto stmt = conn.execute_fetch(put_sql);

  // Then Rowset for PUT command should be correct
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_IDX) == filename);
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_IDX) == filename + ".gz");
  CHECK(get_data<SQL_C_LONG>(stmt, PUT_ROW_SOURCE_SIZE_IDX) == 6);

  CHECK(get_data<SQL_C_LONG>(stmt, PUT_ROW_TARGET_SIZE_IDX) == 32);

  compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_COMPRESSION_IDX), "NONE");
  compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_COMPRESSION_IDX), "GZIP");
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX) == "UPLOADED");

  OLD_DRIVER_ONLY("BD#3") { CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_MESSAGE_IDX) == "ENCRYPTED"); }
  NEW_DRIVER_ONLY("BD#3") { CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_MESSAGE_IDX) == ""); }
}

TEST_CASE("should return correct rowset for GET", "[put_get]") {
  // Given File is uploaded to stage
  Connection conn;
  const std::string stage = pg_utils::create_stage(conn, unique_stage_name("ODBCTST_ROWSET"));
  auto [filename, file] = basic_test_file();

  std::string put_sql = "PUT 'file://" + as_file_uri(file) + "' @" + stage;
  conn.execute(put_sql);

  // When File is downloaded using GET command
  TempTestDir download_dir("odbc_put_get_");

  std::string get_sql = "GET @" + stage + "/" + filename + " 'file://" + as_file_uri(download_dir.path()) + "/'";
  auto stmt = conn.execute_fetch(get_sql);

  // Then Rowset for GET command should be correct
  CHECK(get_data<SQL_C_CHAR>(stmt, GET_ROW_FILE_IDX) == filename + ".gz");

  OLD_DRIVER_ONLY("BD#4") { CHECK(get_data<SQL_C_LONG>(stmt, GET_ROW_SIZE_IDX) == 32); }
  NEW_DRIVER_ONLY("BD#4") { CHECK(get_data<SQL_C_LONG>(stmt, GET_ROW_SIZE_IDX) == 26); }

  CHECK(get_data<SQL_C_CHAR>(stmt, GET_ROW_STATUS_IDX) == "DOWNLOADED");

  OLD_DRIVER_ONLY("BD#3") { CHECK(get_data<SQL_C_CHAR>(stmt, GET_ROW_MESSAGE_IDX) == "DECRYPTED"); }
  NEW_DRIVER_ONLY("BD#3") { CHECK(get_data<SQL_C_CHAR>(stmt, GET_ROW_MESSAGE_IDX) == ""); }
}

TEST_CASE("should return correct column metadata for PUT", "[put_get]") {
  SKIP("SNOW-2391324: Metadata check not implemented in new driver (SQLDescribeCol returns IM001)");
  // Given Snowflake client is logged in
  // When File is uploaded to stage
  // Then Column metadata for PUT command should be correct
}

TEST_CASE("should return correct column metadata for GET", "[put_get]") {
  SKIP("SNOW-2391324: Metadata check not implemented in new driver (SQLDescribeCol returns IM001)");
  // Given File is uploaded to stage
  // When File is downloaded using GET command
  // Then Column metadata for GET command should be correct
}
