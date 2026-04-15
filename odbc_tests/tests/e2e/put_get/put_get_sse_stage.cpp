#include <filesystem>
#include <fstream>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "get_data.hpp"
#include "put_get_utils.hpp"
#include "utils.hpp"

namespace fs = std::filesystem;
using namespace pg_utils;

static std::string create_sse_stage(Connection& conn, const std::string& stage_name) {
  conn.execute("CREATE TEMPORARY STAGE IF NOT EXISTS " + stage_name + " ENCRYPTION = (TYPE = 'SNOWFLAKE_SSE')");
  return stage_name;
}

TEST_CASE("should put and get file on SSE stage", "[put_get]") {
  // Given Stage with server-side encryption (SNOWFLAKE_SSE)
  Connection conn;
  const std::string stage = create_sse_stage(conn, unique_stage_name("ODBCTST_SSE_PUT_GET"));

  TempTestDir upload_dir("odbc_sse_upload_");
  fs::path test_file = write_text_file(upload_dir.path(), "sse_test.txt", "hello sse\n");

  // When File is uploaded using PUT command
  std::string put_sql = "PUT 'file://" + as_file_uri(test_file) + "' @" + stage + " AUTO_COMPRESS=FALSE OVERWRITE=TRUE";
  auto put_stmt = conn.execute_fetch(put_sql);

  // Then File should be uploaded successfully
  CHECK(get_data<SQL_C_CHAR>(put_stmt, PUT_ROW_STATUS_IDX) == "UPLOADED");

  // When File is downloaded using GET command
  TempTestDir download_dir("odbc_sse_get_");
  std::string get_sql = "GET @" + stage + "/sse_test.txt 'file://" + as_file_uri(download_dir.path()) + "/'";
  auto get_stmt = conn.execute_fetch(get_sql);

  // Then File should be downloaded
  CHECK(get_data<SQL_C_CHAR>(get_stmt, GET_ROW_STATUS_IDX) == "DOWNLOADED");

  // And Have correct content
  fs::path downloaded = download_dir.path() / "sse_test.txt";
  REQUIRE(fs::exists(downloaded));
  std::ifstream ifs(downloaded);
  std::string content((std::istreambuf_iterator<char>(ifs)), std::istreambuf_iterator<char>());
  CHECK(content == "hello sse\n");
}

TEST_CASE("should put and get file on SSE stage with DIRECTORY enabled", "[put_get]") {
  // Given Stage with server-side encryption and DIRECTORY enabled
  Connection conn;
  const std::string stage = unique_stage_name("ODBCTST_SSE_DIR");
  conn.execute("CREATE TEMPORARY STAGE IF NOT EXISTS " + stage +
               " ENCRYPTION = (TYPE = 'SNOWFLAKE_SSE') DIRECTORY = (ENABLE = TRUE)");

  TempTestDir upload_dir("odbc_sse_dir_");
  fs::path test_file = write_text_file(upload_dir.path(), "test.txt", "Initial contents\n");

  // When File is uploaded using PUT command
  std::string put_sql = "PUT 'file://" + as_file_uri(test_file) + "' @" + stage + " AUTO_COMPRESS=FALSE OVERWRITE=TRUE";
  auto put_stmt = conn.execute_fetch(put_sql);

  // Then File should be uploaded successfully
  CHECK(get_data<SQL_C_CHAR>(put_stmt, PUT_ROW_STATUS_IDX) == "UPLOADED");

  // When File is downloaded using GET command
  TempTestDir download_dir("odbc_sse_dir_get_");
  std::string get_sql = "GET @" + stage + "/test.txt 'file://" + as_file_uri(download_dir.path()) + "/'";
  auto get_stmt = conn.execute_fetch(get_sql);

  // Then File should be downloaded
  CHECK(get_data<SQL_C_CHAR>(get_stmt, GET_ROW_STATUS_IDX) == "DOWNLOADED");

  // And Have correct content
  fs::path downloaded = download_dir.path() / "test.txt";
  REQUIRE(fs::exists(downloaded));
  std::ifstream ifs(downloaded);
  std::string content((std::istreambuf_iterator<char>(ifs)), std::istreambuf_iterator<char>());
  CHECK(content == "Initial contents\n");
}
