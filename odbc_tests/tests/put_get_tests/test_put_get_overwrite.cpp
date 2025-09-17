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

static std::pair<std::string, fs::path> original_test_file() {
  return {"test_data.csv", shared_test_data_dir() / "overwrite" / "original" / "test_data.csv"};
}

static std::pair<std::string, fs::path> updated_test_file() {
  return {"test_data.csv", shared_test_data_dir() / "overwrite" / "updated" / "test_data.csv"};
}

TEST_CASE("PUT overwrite true", "[put_get][odbc]") {
  Connection conn;
  const std::string stage = pg_utils::create_stage(conn, "ODBCTST_OVERWRITE_TRUE");
  auto [filename, original] = original_test_file();
  auto [_, updated] = updated_test_file();

  {
    // Upload file
    auto stmt = conn.execute_fetch("PUT 'file://" + as_file_uri(original) + "' @" + stage);
    std::string src = get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_IDX);
    std::string status = get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX);
    CHECK(src == filename);
    CHECK(status == "UPLOADED");
  }

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
    CHECK(c2 == "test");
    CHECK(c3 == "data");
  }
}

TEST_CASE("PUT overwrite false", "[put_get][odbc]") {
  Connection conn;
  const std::string stage = pg_utils::create_stage(conn, "ODBCTST_OVERWRITE_FALSE");
  auto [filename, original] = original_test_file();
  auto [_, updated] = updated_test_file();

  {
    // Upload file
    auto stmt = conn.execute_fetch("PUT 'file://" + as_file_uri(original) + "' @" + stage);
    std::string src = get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_IDX);
    std::string status = get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX);
    CHECK(src == filename);
    CHECK(status == "UPLOADED");
  }

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
    CHECK(c2 == "test");
    CHECK(c3 == "data");
  }
}
