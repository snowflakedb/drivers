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

TEST_CASE("PUT with ? wildcard and LS", "[put_get][odbc]") {
  Connection conn;
  // Setup stage
  const std::string stage = pg_utils::create_stage(conn, "ODBCTST_WILDCARD_Q");
  const std::string base = "test_put_wildcard_question_mark";

  // Set up test environment
  fs::path tmp = fs::temp_directory_path() / (std::string("odbc_put_get_") + random_hex());
  fs::create_directories(tmp);

  // Create matching files
  for (int i = 1; i <= 5; ++i)
    write_text_file(tmp, base + "_" + std::to_string(i) + ".csv", "1,2,3\n");
  // Create non-matching files
  write_text_file(tmp, base + "_10.csv", "1,2,3\n");
  write_text_file(tmp, base + "_abc.csv", "1,2,3\n");

  {
    // Upload files
    const std::string pattern = as_file_uri(tmp) + "/" + base + "_?.csv";
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

    for (int i = 1; i <= 5; ++i) {
      CHECK(all.find(base + "_" + std::to_string(i) + ".csv.gz") != std::string::npos);
    }
    CHECK(all.find(base + ".csv.gz") == std::string::npos);
    CHECK(all.find(base + "_test.txt.gz") == std::string::npos);
    CHECK(all.find(base + "_10.csv.gz") == std::string::npos);
    CHECK(all.find(base + "_abc.csv.gz") == std::string::npos);
  }
}

TEST_CASE("PUT with * wildcard and LS", "[put_get][odbc]") {
  Connection conn;
  // Setup stage
  const std::string stage = pg_utils::create_stage(conn, "ODBCTST_WILDCARD_STAR");
  const std::string base = "test_put_wildcard_star";

  // Set up test environment
  fs::path tmp = fs::temp_directory_path() / (std::string("odbc_put_get_") + random_hex());
  fs::create_directories(tmp);

  // Create matching files
  for (int i = 1; i <= 5; ++i) {
    write_text_file(tmp,
                    base + "_" + std::to_string(i) + std::to_string(i) + std::to_string(i) + ".csv",
                    "1,2,3\n");
  }
  // Create non-matching files
  write_text_file(tmp, base + ".csv", "1,2,3\n");
  write_text_file(tmp, base + "_test.txt", "1,2,3\n");

  {
    // Upload files
    const std::string pattern = as_file_uri(tmp) + "/" + base + "_*.csv";
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

    for (int i = 1; i <= 5; ++i) {
      CHECK(all.find(base + "_" + std::to_string(i) + std::to_string(i) + std::to_string(i) +
                     ".csv.gz") != std::string::npos);
    }
    CHECK(all.find(base + ".csv.gz") == std::string::npos);
    CHECK(all.find(base + "_test.txt.gz") == std::string::npos);
  }
}

TEST_CASE("GET with PATTERN regexp filters files", "[put_get][odbc]") {
  Connection conn;
  // Setup stage
  const std::string stage = pg_utils::create_stage(conn, "ODBCTST_REGEXP_GET");
  const std::string base = "data";

  // Set up test environment
  fs::path tmp = fs::temp_directory_path() / (std::string("odbc_put_get_") + random_hex());
  fs::create_directories(tmp);

  // Create and upload test files that match the regexp pattern
  for (int i = 1; i <= 5; ++i) {
    fs::path p = write_text_file(tmp, base + "_" + std::to_string(i) + ".csv", "1,2,3\n");
    conn.execute("PUT 'file://" + as_file_uri(p) + "' @" + stage);
  }

  // Create files that should NOT match the regexp pattern and upload them
  write_text_file(tmp, base + "_10.csv", "1,2,3\n");
  write_text_file(tmp, base + "_abc.csv", "1,2,3\n");
  {
    fs::path p = tmp / (base + "_10.csv");
    conn.execute("PUT 'file://" + as_file_uri(p) + "' @" + stage);
  }
  {
    fs::path p = tmp / (base + "_abc.csv");
    conn.execute("PUT 'file://" + as_file_uri(p) + "' @" + stage);
  }

  fs::path download_dir = tmp / "download";
  fs::create_directories(download_dir);
  const std::string get_pattern = R"(.*/data_.\.csv\.gz)";

  {
    // Download the files from the stage using a pattern
    conn.execute("GET @" + stage + " 'file://" + as_file_uri(download_dir) + "/' PATTERN='" +
                 get_pattern + "'");
  }

  // Verify that the matching files were downloaded and the non-matching files were not downloaded
  for (int i = 1; i <= 5; ++i) {
    fs::path expected = download_dir / (base + "_" + std::to_string(i) + ".csv.gz");
    REQUIRE(fs::exists(expected));
  }
  REQUIRE(!fs::exists(download_dir / (base + "_10.csv.gz")));
  REQUIRE(!fs::exists(download_dir / (base + "_abc.csv.gz")));
}
