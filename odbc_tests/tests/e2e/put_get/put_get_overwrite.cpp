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
#include "utils.hpp"

namespace fs = std::filesystem;
using namespace pg_utils;

static std::pair<std::string, fs::path> original_test_file() {
  return {"test_data.csv", test_utils::shared_test_data_dir() / "overwrite" / "original" / "test_data.csv"};
}

static std::pair<std::string, fs::path> updated_test_file() {
  return {"test_data.csv", test_utils::shared_test_data_dir() / "overwrite" / "updated" / "test_data.csv"};
}

TEST_CASE("should overwrite file when OVERWRITE is set to true", "[put_get]") {
  Connection conn;
  const std::string stage = pg_utils::create_stage(conn, unique_stage_name("ODBCTST_OVERWRITE"));
  auto [filename, original] = original_test_file();
  auto [_, updated] = updated_test_file();

  // Given File is uploaded to stage
  auto stmt_initial = conn.execute_fetch("PUT 'file://" + as_file_uri(original) + "' @" + stage);
  std::string src = get_data<SQL_C_CHAR>(stmt_initial, PUT_ROW_SOURCE_IDX);
  std::string status = get_data<SQL_C_CHAR>(stmt_initial, PUT_ROW_STATUS_IDX);
  CHECK(src == filename);
  CHECK(status == "UPLOADED");

  // When Updated file is uploaded with OVERWRITE set to true
  auto stmt_update = conn.execute_fetch("PUT 'file://" + as_file_uri(updated) + "' @" + stage + " OVERWRITE=TRUE");
  std::string src_update = get_data<SQL_C_CHAR>(stmt_update, PUT_ROW_SOURCE_IDX);
  std::string status_update = get_data<SQL_C_CHAR>(stmt_update, PUT_ROW_STATUS_IDX);

  // Then UPLOADED status is returned
  CHECK(src_update == filename);
  CHECK(status_update == "UPLOADED");

  // And File was overwritten
  auto stmt_check = conn.execute_fetch("SELECT $1, $2, $3 FROM @" + stage);
  std::string c1 = get_data<SQL_C_CHAR>(stmt_check, 1);
  std::string c2 = get_data<SQL_C_CHAR>(stmt_check, 2);
  std::string c3 = get_data<SQL_C_CHAR>(stmt_check, 3);
  CHECK(c1 == "updated");
  CHECK(c2 == "test");
  CHECK(c3 == "data");
}

TEST_CASE("should not overwrite file when OVERWRITE is set to false", "[put_get]") {
  Connection conn;
  const std::string stage = pg_utils::create_stage(conn, unique_stage_name("ODBCTST_OVERWRITE"));
  auto [filename, original] = original_test_file();
  auto [_, updated] = updated_test_file();

  // Given File is uploaded to stage
  auto stmt_initial = conn.execute_fetch("PUT 'file://" + as_file_uri(original) + "' @" + stage);
  std::string src = get_data<SQL_C_CHAR>(stmt_initial, PUT_ROW_SOURCE_IDX);
  std::string status = get_data<SQL_C_CHAR>(stmt_initial, PUT_ROW_STATUS_IDX);
  CHECK(src == filename);
  CHECK(status == "UPLOADED");

  // When Updated file is uploaded with OVERWRITE set to false
  auto stmt_update = conn.execute_fetch("PUT 'file://" + as_file_uri(updated) + "' @" + stage + " OVERWRITE=FALSE");
  std::string src_update = get_data<SQL_C_CHAR>(stmt_update, PUT_ROW_SOURCE_IDX);
  std::string status_update = get_data<SQL_C_CHAR>(stmt_update, PUT_ROW_STATUS_IDX);

  // Then SKIPPED status is returned
  CHECK(src_update == filename);
  CHECK(status_update == "SKIPPED");

  // And File was not overwritten
  auto stmt_check = conn.execute_fetch("SELECT $1, $2, $3 FROM @" + stage);
  std::string c1 = get_data<SQL_C_CHAR>(stmt_check, 1);
  std::string c2 = get_data<SQL_C_CHAR>(stmt_check, 2);
  std::string c3 = get_data<SQL_C_CHAR>(stmt_check, 3);
  CHECK(c1 == "original");
  CHECK(c2 == "test");
  CHECK(c3 == "data");
}
