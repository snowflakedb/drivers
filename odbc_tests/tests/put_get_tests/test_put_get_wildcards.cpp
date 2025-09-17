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

static fs::path wildcard_tests_dir() { return shared_test_data_dir() / "wildcard"; }

TEST_CASE("PUT with ? wildcard and LS", "[put_get][odbc]") {
  Connection conn;
  // Setup stage
  const std::string stage = pg_utils::create_stage(conn, "ODBCTST_WILDCARD_Q");
  fs::path wildcard_dir = wildcard_tests_dir();

  {
    // Upload files
    const std::string pattern = as_file_uri(wildcard_dir) + "/pattern_?.csv";
    conn.execute("PUT 'file://" + pattern + "' @" + stage);
  }

  {
    // List stage contents
    auto stmt = conn.execute("LS @" + stage);

    // Collect LS rows text
    std::string all;
    while (true) {
      SQLRETURN ret = SQLFetch(stmt.getHandle());
      if (ret == SQL_NO_DATA) break;
      CHECK_ODBC(ret, stmt);
      all += get_data<SQL_C_CHAR>(stmt, LS_ROW_NAME_IDX) + "\n";
    }

    CHECK(all.find("pattern_1.csv.gz") != std::string::npos);
    CHECK(all.find("pattern_2.csv.gz") != std::string::npos);
    CHECK(all.find("pattern_10.csv.gz") == std::string::npos);
    CHECK(all.find("patternabc.csv.gz") == std::string::npos);
  }
}

TEST_CASE("PUT with * wildcard and LS", "[put_get][odbc]") {
  Connection conn;
  // Setup stage
  const std::string stage = pg_utils::create_stage(conn, "ODBCTST_WILDCARD_STAR");
  fs::path wildcard_dir = wildcard_tests_dir();

  {
    // Upload files
    const std::string pattern = as_file_uri(wildcard_dir) + "/pattern_*.csv";
    conn.execute("PUT 'file://" + pattern + "' @" + stage);
  }

  {
    // List stage contents
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
    CHECK(all.find("patternabc.csv.gz") == std::string::npos);
  }
}

TEST_CASE("GET with PATTERN regexp filters files", "[put_get][odbc]") {
  Connection conn;
  // Setup stage
  const std::string stage = pg_utils::create_stage(conn, "ODBCTST_REGEXP_GET");
  fs::path wildcard_dir = wildcard_tests_dir();
  for (const auto& name : {"pattern_1.csv", "pattern_2.csv", "pattern_10.csv", "patternabc.csv"}) {
    conn.execute("PUT 'file://" + as_file_uri(wildcard_dir / name) + "' @" + stage);
  }

  fs::path download_dir = fs::temp_directory_path() / (std::string("odbc_put_get_") + random_hex());
  fs::create_directories(download_dir);
  const std::string get_pattern = R"(.*/pattern_.\.csv\.gz)";

  {
    // Download the files from the stage using a pattern
    conn.execute("GET @" + stage + " 'file://" + as_file_uri(download_dir) + "/' PATTERN='" +
                 get_pattern + "'");
  }

  // Verify that the matching files were downloaded and the non-matching files were not downloaded
  CHECK(fs::exists(download_dir / "pattern_1.csv.gz"));
  CHECK(fs::exists(download_dir / "pattern_2.csv.gz"));
  CHECK(!fs::exists(download_dir / "pattern_10.csv.gz"));
  CHECK(!fs::exists(download_dir / "patternabc.csv.gz"));
}
