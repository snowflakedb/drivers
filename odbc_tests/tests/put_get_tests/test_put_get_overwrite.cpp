#include <algorithm>
#include <filesystem>
#include <fstream>
#include <random>
#include <string>
#include <tuple>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "get_data.hpp"
#include "put_get_utils.hpp"

namespace fs = std::filesystem;
using namespace pg_utils;

TEST_CASE("PUT overwrite true", "[put_get][odbc]") {
  Connection conn;
  const std::string stage = pg_utils::create_stage(conn, "ODBCTST_OVERWRITE_TRUE");
  const std::string filename = "test_overwrite_true.csv";

  // Create test file with CSV data
  fs::path tmp = fs::temp_directory_path() / (std::string("odbc_put_get_") + random_hex());
  fs::create_directories(tmp);

  fs::path original = write_text_file(tmp, filename, "original,data,1\n");
  {
    // Upload file
    auto stmt = conn.execute_fetch("PUT 'file://" + as_file_uri(original) + "' @" + stage);
    std::string src = get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_IDX);
    std::string status = get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX);
    CHECK(src == filename);
    CHECK(status == "UPLOADED");
  }

  fs::path updated = write_text_file(tmp, filename, "updated,data,2\n");
  {
    // Overwrite existing file
    auto stmt = conn.execute_fetch("PUT 'file://" + as_file_uri(updated) + "' @" + stage +
                                   " OVERWRITE=TRUE");
    std::string src = get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_IDX);
    std::string status = get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX);
    CHECK(src == filename);
    CHECK(status == "UPLOADED");
  }

  // Verify the data matches what we uploaded
  {
    auto stmt = conn.execute_fetch("SELECT $1, $2, $3 FROM @" + stage);
    std::string c1 = get_data<SQL_C_CHAR>(stmt, 1);
    std::string c2 = get_data<SQL_C_CHAR>(stmt, 2);
    std::string c3 = get_data<SQL_C_CHAR>(stmt, 3);
    CHECK(c1 == "updated");
    CHECK(c2 == "data");
    CHECK(c3 == "2");
  }
}

TEST_CASE("PUT overwrite false", "[put_get][odbc]") {
  Connection conn;
  const std::string stage = pg_utils::create_stage(conn, "ODBCTST_OVERWRITE_FALSE");
  const std::string filename = "test_overwrite_false.csv";

  // Create test file with CSV data
  fs::path tmp = fs::temp_directory_path() / (std::string("odbc_put_get_") + random_hex());
  fs::create_directories(tmp);

  fs::path original = write_text_file(tmp, filename, "original,data,1\n");
  {
    // Upload file
    auto stmt = conn.execute_fetch("PUT 'file://" + as_file_uri(original) + "' @" + stage);
    std::string src = get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_IDX);
    std::string status = get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX);
    CHECK(src == filename);
    CHECK(status == "UPLOADED");
  }

  fs::path updated = write_text_file(tmp, filename, "updated,data,2\n");
  {
    // Attempt overwrite with OVERWRITE=FALSE -> should be skipped
    auto stmt = conn.execute_fetch("PUT 'file://" + as_file_uri(updated) + "' @" + stage +
                                   " OVERWRITE=FALSE");
    std::string src = get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_IDX);
    std::string status = get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX);
    CHECK(src == filename);
    CHECK(status == "SKIPPED");
  }

  // Verify the data matches what we uploaded
  {
    auto stmt = conn.execute_fetch("SELECT $1, $2, $3 FROM @" + stage);
    std::string c1 = get_data<SQL_C_CHAR>(stmt, 1);
    std::string c2 = get_data<SQL_C_CHAR>(stmt, 2);
    std::string c3 = get_data<SQL_C_CHAR>(stmt, 3);
    CHECK(c1 == "original");
    CHECK(c2 == "data");
    CHECK(c3 == "1");
  }
}

TEST_CASE("PUT overwrite false multiple files mixed status", "[put_get][odbc]") {
  Connection conn;
  const std::string stage = pg_utils::create_stage(conn, "ODBCTST_OVERWRITE_MIXED");
  const std::string base = "test_overwrite_mixed";

  // Create test file with CSV data
  fs::path tmp = fs::temp_directory_path() / (std::string("odbc_put_get_") + random_hex());
  fs::create_directories(tmp);

  const std::string f1 = base + "_1.csv";
  const std::string f2 = base + "_2.csv";
  const std::string f3 = base + "_3.csv";
  write_text_file(tmp, f1, "file1,content,1\n");
  fs::path p2 = write_text_file(tmp, f2, "file2,content,2\n");
  write_text_file(tmp, f3, "file3,content,3\n");

  // Upload file2 first
  {
    auto stmt = conn.execute_fetch("PUT 'file://" + as_file_uri(p2) + "' @" + stage);
    std::string src = get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_IDX);
    std::string status = get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX);
    CHECK(src == f2);
    CHECK(status == "UPLOADED");
  }

  // Update file2 content locally
  write_text_file(tmp, f2, "file2,new_content,2\n");

  // Upload wildcard with OVERWRITE=FALSE
  {
    const std::string pattern = as_file_uri(tmp) + "/" + base + "_*.csv";
    auto stmt = conn.execute("PUT 'file://" + pattern + "' @" + stage + " OVERWRITE=FALSE");

    // Expect 3 rows
    std::vector<std::pair<std::string, std::string>> rows;
    for (int i = 0; i < 3; ++i) {
      SQLRETURN ret = SQLFetch(stmt.getHandle());
      CHECK_ODBC(ret, stmt);
      std::string src = get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_IDX);
      std::string status = get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX);
      rows.emplace_back(src, status);
    }
    std::sort(rows.begin(), rows.end());
    REQUIRE(rows.size() == 3);
    CHECK(rows[0].first == f1);
    CHECK(rows[0].second == "UPLOADED");
    CHECK(rows[1].first == f2);
    CHECK(rows[1].second == "SKIPPED");
    CHECK(rows[2].first == f3);
    CHECK(rows[2].second == "UPLOADED");
  }

  // Verify stage content unchanged for file2
  {
    auto stmt = conn.execute("SELECT $1, $2, $3 FROM @" + stage + " ORDER BY $1");
    // Expect 3 rows
    std::vector<std::tuple<std::string, std::string, std::string>> data;
    for (int i = 0; i < 3; ++i) {
      SQLRETURN ret = SQLFetch(stmt.getHandle());
      CHECK_ODBC(ret, stmt);
      data.emplace_back(get_data<SQL_C_CHAR>(stmt, 1), get_data<SQL_C_CHAR>(stmt, 2),
                        get_data<SQL_C_CHAR>(stmt, 3));
    }
    std::sort(data.begin(), data.end());
    CHECK(std::get<0>(data[0]) == "file1");
    CHECK(std::get<1>(data[0]) == "content");
    CHECK(std::get<2>(data[0]) == "1");
    CHECK(std::get<0>(data[1]) == "file2");
    CHECK(std::get<1>(data[1]) == "content");
    CHECK(std::get<2>(data[1]) == "2");
    CHECK(std::get<0>(data[2]) == "file3");
    CHECK(std::get<1>(data[2]) == "content");
    CHECK(std::get<2>(data[2]) == "3");
  }
}
