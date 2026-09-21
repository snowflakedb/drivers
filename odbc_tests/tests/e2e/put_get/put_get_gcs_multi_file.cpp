#include <sql.h>
#include <sqlext.h>

#include <algorithm>
#include <cctype>
#include <filesystem>
#include <fstream>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "get_data.hpp"
#include "odbc_matchers.hpp"
#include "put_get_utils.hpp"

using namespace pg_utils;

namespace {

bool is_gcp_account(Connection& conn) {
  auto stmt = conn.execute_fetch("SELECT CURRENT_REGION()");
  std::string region = get_data<SQL_C_CHAR>(stmt, 1);
  std::transform(region.begin(), region.end(), region.begin(), [](unsigned char c) { return std::toupper(c); });
  return region.find("GCP") != std::string::npos;
}

std::string read_text(const std::filesystem::path& path) {
  std::ifstream ifs(path);
  return {std::istreambuf_iterator<char>(ifs), std::istreambuf_iterator<char>()};
}

}  // namespace

TEST_CASE("should get multiple files from stage in single command", "[put_get]") {
  Connection conn;
  if (!is_gcp_account(conn)) {
    SKIP("account CURRENT_REGION() is not GCP; multi-file GET presigned-URL path is GCS-only");
  }

  const std::string stage = create_stage(conn, unique_stage_name("ODBCTST_GCS_MULTI"));
  TempTestDir upload("odbc_gcs_up_");
  const auto alpha = write_text_file(upload.path(), "alpha.txt", "contents of alpha\n");
  const auto beta = write_text_file(upload.path(), "beta.txt", "contents of beta\n");

  // Given Two files are uploaded to stage
  conn.execute("PUT 'file://" + as_file_uri(alpha) + "' @" + stage + " AUTO_COMPRESS=FALSE OVERWRITE=TRUE");
  conn.execute("PUT 'file://" + as_file_uri(beta) + "' @" + stage + " AUTO_COMPRESS=FALSE OVERWRITE=TRUE");

  // When All files are downloaded from stage using GET command
  TempTestDir download("odbc_gcs_down_");
  auto stmt = conn.execute_fetch("GET @" + stage + " 'file://" + as_file_uri(download.path()) + "/'");

  // Then All files should be downloaded
  int downloaded = 0;
  SQLRETURN fetch_ret = SQL_SUCCESS;
  do {
    REQUIRE(get_data<SQL_C_CHAR>(stmt, GET_ROW_STATUS_IDX) == "DOWNLOADED");
    ++downloaded;
    fetch_ret = SQLFetch(stmt.getHandle());
  } while (fetch_ret == SQL_SUCCESS);
  REQUIRE(fetch_ret == SQL_NO_DATA);
  REQUIRE(downloaded == 2);

  // And Each file should have correct content
  REQUIRE(read_text(download.path() / "alpha.txt") == "contents of alpha\n");
  REQUIRE(read_text(download.path() / "beta.txt") == "contents of beta\n");
}
