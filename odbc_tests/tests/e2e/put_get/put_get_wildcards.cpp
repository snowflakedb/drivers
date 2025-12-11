#include <algorithm>
#include <filesystem>
#include <fstream>
#include <random>
#include <set>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "get_data.hpp"
#include "put_get_utils.hpp"
#include "utils.hpp"

namespace fs = std::filesystem;
using namespace pg_utils;

static fs::path wildcard_tests_dir() { return test_utils::shared_test_data_dir() / "wildcard"; }

// Copies test files to a TempTestDir and populates the directory
static void populate_test_files(const TempTestDir& temp_dir, const std::vector<std::string>& filenames) {
  fs::path source_dir = wildcard_tests_dir();
  for (const auto& name : filenames) {
    fs::path dest = temp_dir.path() / name;
    // Read source file content
    std::ifstream src(source_dir / name, std::ios::binary);
    std::string content((std::istreambuf_iterator<char>(src)), std::istreambuf_iterator<char>());
    src.close();
    // Write to destination and sync
    std::ofstream dst(dest, std::ios::binary | std::ios::trunc);
    dst << content;
    dst.flush();
    dst.close();
    // Verify file is readable and has content
    REQUIRE(fs::exists(dest));
    REQUIRE(fs::file_size(dest) > 0);
  }
}

TEST_CASE("should upload files that match wildcard question mark pattern", "[put_get]") {
  Connection conn;
  const std::string stage = pg_utils::create_stage(conn, unique_stage_name("ODBCTST_WILDCARD_Q"));
  TempTestDir wildcard_dir("odbc_wildcard_");
  populate_test_files(wildcard_dir, {"pattern_1.csv", "pattern_2.csv", "pattern_10.csv", "patternabc.csv"});

  // Given Files matching wildcard pattern
  REQUIRE(fs::exists(wildcard_dir.path() / "pattern_1.csv"));
  REQUIRE(fs::exists(wildcard_dir.path() / "pattern_2.csv"));

  // And Files not matching wildcard pattern
  REQUIRE(fs::exists(wildcard_dir.path() / "pattern_10.csv"));
  REQUIRE(fs::exists(wildcard_dir.path() / "patternabc.csv"));

  // When Files are uploaded using command with question mark wildcard
  const std::string pattern = as_file_uri(wildcard_dir.path()) + "/pattern_?.csv";
  conn.execute("PUT 'file://" + pattern + "' @" + stage);

  // Then Files matching wildcard pattern are uploaded
  auto stmt = conn.execute("LS @" + stage);

  std::string all;
  while (true) {
    SQLRETURN ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) break;
    CHECK_ODBC(ret, stmt);
    all += get_data<SQL_C_CHAR>(stmt, LS_ROW_NAME_IDX) + "\n";
  }

  CHECK(all.find("pattern_1.csv.gz") != std::string::npos);
  CHECK(all.find("pattern_2.csv.gz") != std::string::npos);

  // And Files not matching wildcard pattern are not uploaded
  CHECK(all.find("pattern_10.csv.gz") == std::string::npos);
  CHECK(all.find("patternabc.csv.gz") == std::string::npos);
}

TEST_CASE("should upload files that match wildcard star pattern", "[put_get]") {
  Connection conn;
  const std::string stage = pg_utils::create_stage(conn, unique_stage_name("ODBCTST_WILDCARD_STAR"));

  // Given Files matching wildcard pattern
  TempTestDir wildcard_dir("odbc_wildcard_");
  populate_test_files(wildcard_dir, {"pattern_1.csv", "pattern_2.csv", "pattern_10.csv", "patternabc.csv"});

  REQUIRE(fs::exists(wildcard_dir.path() / "pattern_1.csv"));
  REQUIRE(fs::exists(wildcard_dir.path() / "pattern_2.csv"));

  // And Files not matching wildcard pattern
  REQUIRE(fs::exists(wildcard_dir.path() / "pattern_10.csv"));
  REQUIRE(fs::exists(wildcard_dir.path() / "patternabc.csv"));

  // When Files are uploaded using command with star wildcard
  const std::string pattern = as_file_uri(wildcard_dir.path()) + "/pattern_*.csv";
  conn.execute("PUT 'file://" + pattern + "' @" + stage);

  // Then Files matching wildcard pattern are uploaded
  auto stmt = conn.execute("LS @" + stage);

  std::string all;
  while (true) {
    SQLRETURN ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) break;
    CHECK_ODBC(ret, stmt);
    all += get_data<SQL_C_CHAR>(stmt, LS_ROW_NAME_IDX) + "\n";
  }

  CHECK(all.find("pattern_1.csv.gz") != std::string::npos);
  CHECK(all.find("pattern_2.csv.gz") != std::string::npos);
  CHECK(all.find("pattern_10.csv.gz") != std::string::npos);

  // And Files not matching wildcard pattern are not uploaded
  CHECK(all.find("patternabc.csv.gz") == std::string::npos);
}

TEST_CASE("should download files that are matching wildcard pattern", "[put_get]") {
  Connection conn;
  const std::string stage = pg_utils::create_stage(conn, unique_stage_name("ODBCTST_REGEXP_GET"));

  // Given Files matching wildcard pattern are uploaded
  TempTestDir wildcard_dir("odbc_wildcard_");
  populate_test_files(wildcard_dir, {"pattern_1.csv", "pattern_2.csv", "pattern_10.csv", "patternabc.csv"});

  for (const auto& name : {"pattern_1.csv", "pattern_2.csv"}) {
    conn.execute("PUT 'file://" + as_file_uri(wildcard_dir.path() / name) + "' @" + stage);
  }

  // And Files not matching wildcard pattern are uploaded
  for (const auto& name : {"pattern_10.csv", "patternabc.csv"}) {
    conn.execute("PUT 'file://" + as_file_uri(wildcard_dir.path() / name) + "' @" + stage);
  }

  TempTestDir download_dir("odbc_put_get_");
  const std::string get_pattern = R"(.*/pattern_.\.csv\.gz)";

  // When Files are downloaded using command with wildcard
  conn.execute("GET @" + stage + " 'file://" + as_file_uri(download_dir.path()) + "/' PATTERN='" + get_pattern + "'");

  // Then Files matching wildcard pattern are downloaded
  std::set<std::string> downloaded_files;
  for (const auto& entry : fs::directory_iterator(download_dir.path())) {
    downloaded_files.insert(entry.path().filename().string());
  }

  CHECK(downloaded_files.count("pattern_1.csv.gz"));
  CHECK(downloaded_files.count("pattern_2.csv.gz"));

  // And Files not matching wildcard pattern are not downloaded
  CHECK(downloaded_files.size() == 2);
  CHECK(downloaded_files.find("pattern_10.csv.gz") == downloaded_files.end());
  CHECK(downloaded_files.find("patternabc.csv.gz") == downloaded_files.end());
}
