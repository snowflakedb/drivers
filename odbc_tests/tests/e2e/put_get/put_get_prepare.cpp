#include <sql.h>
#include <sqlext.h>

#include <filesystem>
#include <string>
#include <utility>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "get_data.hpp"
#include "odbc_matchers.hpp"
#include "put_get_utils.hpp"
#include "utils.hpp"

// PUT and GET are client-side file-transfer commands the server cannot
// describe: routing them through the prepared-statement path (SQLPrepare, which
// issues a describeOnly request) had the server reject them with error 000007
// ("statement not preparable"), so every SQLPrepare+SQLExecute upload/download
// failed. Legacy ODBC 3.x never described PUT/GET, so these worked there. These
// tests exercise the SQLPrepare+SQLExecute path and pass on both drivers.

using namespace pg_utils;
namespace fs = std::filesystem;

namespace {

std::pair<std::string, fs::path> basic_test_file() {
  return {"test_data.csv", test_utils::shared_test_data_dir() / "basic" / "test_data.csv"};
}

}  // namespace

TEST_CASE("should upload file to stage via SQLPrepare + SQLExecute", "[put_get][prepare]") {
  // Given Snowflake client is logged in and a stage exists
  Connection conn;
  const std::string stage = pg_utils::create_stage(conn, unique_stage_name("ODBCTST_PREP_PUT"));
  auto [filename, file] = basic_test_file();

  // When a PUT is prepared and executed
  std::string put_sql = "PUT 'file://" + as_file_uri(file) + "' @" + stage;
  auto stmt = conn.createStatement();
  REQUIRE_ODBC(SQLPrepare(stmt.getHandle(), (SQLCHAR*)put_sql.c_str(), SQL_NTS), stmt);
  REQUIRE_ODBC(SQLExecute(stmt.getHandle()), stmt);
  REQUIRE_ODBC(SQLFetch(stmt.getHandle()), stmt);

  // Then the PUT rowset reports the upload succeeded
  SQLSMALLINT num_cols = 0;
  REQUIRE_ODBC(SQLNumResultCols(stmt.getHandle(), &num_cols), stmt);
  CHECK(num_cols == PUT_ROW_NUM_COLS);

  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_IDX) == expected_put_source(file));
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_IDX) == filename + ".gz");
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX) == "UPLOADED");
}

TEST_CASE("should download file from stage via SQLPrepare + SQLExecute", "[put_get][prepare]") {
  // Given a file already uploaded to the stage
  Connection conn;
  const std::string stage = pg_utils::create_stage(conn, unique_stage_name("ODBCTST_PREP_GET"));
  auto [filename, file] = basic_test_file();

  std::string put_sql = "PUT 'file://" + as_file_uri(file) + "' @" + stage;
  conn.execute(put_sql);

  // When a GET is prepared and executed
  TempTestDir download_dir("odbc_put_get_prepare_");
  std::string get_sql = "GET @" + stage + "/" + filename + " 'file://" + as_file_uri(download_dir.path()) + "/'";
  auto stmt = conn.createStatement();
  REQUIRE_ODBC(SQLPrepare(stmt.getHandle(), (SQLCHAR*)get_sql.c_str(), SQL_NTS), stmt);
  REQUIRE_ODBC(SQLExecute(stmt.getHandle()), stmt);
  REQUIRE_ODBC(SQLFetch(stmt.getHandle()), stmt);

  // Then the GET rowset reports the download succeeded and the file lands locally
  SQLSMALLINT num_cols = 0;
  REQUIRE_ODBC(SQLNumResultCols(stmt.getHandle(), &num_cols), stmt);
  CHECK(num_cols == GET_ROW_NUM_COLS);

  CHECK(get_data<SQL_C_CHAR>(stmt, GET_ROW_FILE_IDX) == filename + ".gz");
  CHECK(get_data<SQL_C_CHAR>(stmt, GET_ROW_STATUS_IDX) == "DOWNLOADED");

  fs::path gz = download_dir.path() / (filename + ".gz");
  CHECK(fs::exists(gz));
}
