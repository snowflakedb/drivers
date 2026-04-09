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

TEST_CASE("should put file to SSE stage", "[put_get]") {
  // Given Stage with server-side encryption (SNOWFLAKE_SSE)
  Connection conn;
  const std::string stage = create_sse_stage(conn, unique_stage_name("ODBCTST_SSE_PUT"));

  TempTestDir temp_dir("odbc_sse_put_");
  fs::path test_file = write_text_file(temp_dir.path(), "sse_test.txt", "hello sse\n");

  // When File is uploaded using PUT command
  std::string put_sql = "PUT 'file://" + as_file_uri(test_file) + "' @" + stage + " AUTO_COMPRESS=FALSE OVERWRITE=TRUE";
  auto stmt = conn.execute_fetch(put_sql);

  // Then File should be uploaded successfully
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX) == "UPLOADED");
}

TEST_CASE("should get file from SSE stage", "[put_get]") {
  // Given File is uploaded to stage with server-side encryption (SNOWFLAKE_SSE)
  Connection conn;
  const std::string stage = create_sse_stage(conn, unique_stage_name("ODBCTST_SSE_GET"));

  TempTestDir upload_dir("odbc_sse_upload_");
  fs::path test_file = write_text_file(upload_dir.path(), "sse_get.txt", "get sse\n");

  auto put_stmt = conn.execute_fetch("PUT 'file://" + as_file_uri(test_file) + "' @" + stage +
                                     " AUTO_COMPRESS=FALSE OVERWRITE=TRUE");
  REQUIRE(get_data<SQL_C_CHAR>(put_stmt, PUT_ROW_STATUS_IDX) == "UPLOADED");

  // When File is downloaded using GET command
  TempTestDir download_dir("odbc_sse_get_");
  std::string get_sql = "GET @" + stage + "/sse_get.txt 'file://" + as_file_uri(download_dir.path()) + "/'";
  auto get_stmt = conn.execute_fetch(get_sql);

  // Then File should be downloaded
  CHECK(get_data<SQL_C_CHAR>(get_stmt, GET_ROW_STATUS_IDX) == "DOWNLOADED");

  // And Have correct content
  fs::path downloaded = download_dir.path() / "sse_get.txt";
  REQUIRE(fs::exists(downloaded));
  std::ifstream ifs(downloaded);
  std::string content((std::istreambuf_iterator<char>(ifs)), std::istreambuf_iterator<char>());
  CHECK(content == "get sse\n");
}

TEST_CASE("should put file to SSE stage with DIRECTORY enabled", "[put_get]") {
  // Given Stage with server-side encryption and DIRECTORY enabled
  Connection conn;
  const std::string stage = unique_stage_name("ODBCTST_SSE_DIR");
  conn.execute("CREATE TEMPORARY STAGE IF NOT EXISTS " + stage +
               " ENCRYPTION = (TYPE = 'SNOWFLAKE_SSE') DIRECTORY = (ENABLE = TRUE)");

  TempTestDir temp_dir("odbc_sse_dir_");
  fs::path test_file = write_text_file(temp_dir.path(), "test.txt", "Initial contents\n");

  // When File is uploaded using PUT command
  std::string put_sql = "PUT 'file://" + as_file_uri(test_file) + "' @" + stage + " AUTO_COMPRESS=FALSE OVERWRITE=TRUE";
  auto stmt = conn.execute_fetch(put_sql);

  // Then File should be uploaded successfully
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX) == "UPLOADED");
}
