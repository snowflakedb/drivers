#include <algorithm>
#include <filesystem>
#include <fstream>
#include <map>
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

static fs::path compression_tests_dir() { return test_utils::shared_test_data_dir() / "compression"; }

static std::pair<std::string, fs::path> test_file(const std::string& compression_type) {
  static const std::map<std::string, std::string> compression_map = {
      {"GZIP", "test_data.csv.gz"},         {"BZIP2", "test_data.csv.bz2"},
      {"BROTLI", "test_data.csv.br"},       {"ZSTD", "test_data.csv.zst"},
      {"DEFLATE", "test_data.csv.deflate"}, {"RAW_DEFLATE", "test_data.csv.raw_deflate"},
      {"LZMA", "test_data.csv.xz"},         {"NONE", "test_data.csv"}};

  auto it = compression_map.find(compression_type);
  if (it == compression_map.end()) {
    FAIL("Unsupported compression type: " << compression_type);
    return {"", ""};
  }

  return {it->second, compression_tests_dir() / it->second};
}

TEST_CASE("should auto-detect standard compression types when SOURCE_COMPRESSION set to AUTO_DETECT", "[put_get]") {
  Connection conn;
  // Given Snowflake client is logged in
  const std::string stage = create_stage(conn, unique_stage_name("ODBCTST_SC_AUTO"));

  // And File with standard type (GZIP, BZIP2, BROTLI, ZSTD, DEFLATE)
  const std::vector<std::string> types = {"GZIP", "BZIP2", "BROTLI", "ZSTD", "DEFLATE"};

  for (const auto& comp : types) {
    auto [filename, file] = test_file(comp);

    // When File is uploaded with SOURCE_COMPRESSION set to AUTO_DETECT
    auto stmt =
        conn.execute_fetch("PUT 'file://" + as_file_uri(file) + "' @" + stage + " SOURCE_COMPRESSION=AUTO_DETECT");

    // Then Target compression has correct type and all PUT results are correct
    CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_IDX) == expected_put_source(file));

    if (comp == "DEFLATE") {
      CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_IDX) == filename);
      compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_COMPRESSION_IDX), "DEFLATE");
      compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_COMPRESSION_IDX), "DEFLATE");
    } else {
      CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_IDX) == filename);
      compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_COMPRESSION_IDX), comp);
      compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_COMPRESSION_IDX), comp);
    }
    CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX) == std::string("UPLOADED"));
  }
}

TEST_CASE("should upload compressed files with SOURCE_COMPRESSION set to explicit types", "[put_get]") {
  Connection conn;
  // Given Snowflake client is logged in
  const std::string stage = create_stage(conn, unique_stage_name("ODBCTST_SC_EXPLICIT"));

  // And File with standard type (GZIP, BZIP2, BROTLI, ZSTD, DEFLATE, RAW_DEFLATE)
  const std::vector<std::string> types = {"GZIP", "BZIP2", "BROTLI", "ZSTD", "DEFLATE", "RAW_DEFLATE"};

  for (const auto& comp : types) {
    auto [filename, file] = test_file(comp);

    // When File is uploaded with SOURCE_COMPRESSION set to explicit type
    std::string put_sql = "PUT 'file://" + as_file_uri(file) + "' @" + stage + " SOURCE_COMPRESSION=" + comp;
    auto stmt = conn.execute_fetch(put_sql);

    // Then Target compression has correct type and all PUT results are correct
    CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_IDX) == expected_put_source(file));
    CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_IDX) == filename);
    compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_COMPRESSION_IDX), comp);
    compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_COMPRESSION_IDX), comp);
    CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX) == std::string("UPLOADED"));
  }
}

TEST_CASE("should not compress file when SOURCE_COMPRESSION set to AUTO_DETECT and AUTO_COMPRESS set to FALSE",
          "[put_get]") {
  Connection conn;
  // Given Snowflake client is logged in
  const std::string stage = create_stage(conn, unique_stage_name("ODBCTST_SC_AUTO_NO_AC"));

  // And Uncompressed file
  auto [filename, file] = test_file("NONE");

  // When File is uploaded with SOURCE_COMPRESSION set to AUTO_DETECT and AUTO_COMPRESS set to FALSE
  auto stmt = conn.execute_fetch("PUT 'file://" + as_file_uri(file) + "' @" + stage +
                                 " SOURCE_COMPRESSION=AUTO_DETECT AUTO_COMPRESS=FALSE");

  // Then File is not compressed
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_IDX) == expected_put_source(file));
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_IDX) == filename);
  compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_COMPRESSION_IDX), "NONE");
  compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_COMPRESSION_IDX), "NONE");
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX) == std::string("UPLOADED"));
}

TEST_CASE("should not compress file when SOURCE_COMPRESSION set to NONE and AUTO_COMPRESS set to FALSE", "[put_get]") {
  Connection conn;
  // Given Snowflake client is logged in
  const std::string stage = create_stage(conn, unique_stage_name("ODBCTST_SC_NONE_NO_AC"));

  // And Uncompressed file
  auto [filename, file] = test_file("NONE");

  // When File is uploaded with SOURCE_COMPRESSION set to NONE and AUTO_COMPRESS set to FALSE
  auto stmt = conn.execute_fetch("PUT 'file://" + as_file_uri(file) + "' @" + stage +
                                 " SOURCE_COMPRESSION=NONE AUTO_COMPRESS=FALSE");

  // Then File is not compressed
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_IDX) == expected_put_source(file));
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_IDX) == filename);
  compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_COMPRESSION_IDX), "NONE");
  compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_COMPRESSION_IDX), "NONE");
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX) == std::string("UPLOADED"));
}

TEST_CASE("should compress uncompressed file when SOURCE_COMPRESSION set to AUTO_DETECT and AUTO_COMPRESS set to TRUE",
          "[put_get]") {
  Connection conn;
  // Given Snowflake client is logged in
  const std::string stage = create_stage(conn, unique_stage_name("ODBCTST_SC_AUTO_AC"));

  // And Uncompressed file
  auto [filename, file] = test_file("NONE");

  // When File is uploaded with SOURCE_COMPRESSION set to AUTO_DETECT and AUTO_COMPRESS set to TRUE
  auto stmt = conn.execute_fetch("PUT 'file://" + as_file_uri(file) + "' @" + stage +
                                 " SOURCE_COMPRESSION=AUTO_DETECT AUTO_COMPRESS=TRUE");

  // Then Target compression has GZIP type and all PUT results are correct
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_IDX) == expected_put_source(file));
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_IDX) == filename + ".gz");
  compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_COMPRESSION_IDX), "NONE");
  compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_COMPRESSION_IDX), "GZIP");
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX) == std::string("UPLOADED"));
}

TEST_CASE("should compress uncompressed file when SOURCE_COMPRESSION set to NONE and AUTO_COMPRESS set to TRUE",
          "[put_get]") {
  Connection conn;
  // Given Snowflake client is logged in
  const std::string stage = create_stage(conn, unique_stage_name("ODBCTST_SC_NONE_AC"));

  // And Uncompressed file
  auto [filename, file] = test_file("NONE");

  // When File is uploaded with SOURCE_COMPRESSION set to NONE and AUTO_COMPRESS set to TRUE
  auto stmt = conn.execute_fetch("PUT 'file://" + as_file_uri(file) + "' @" + stage +
                                 " SOURCE_COMPRESSION=NONE AUTO_COMPRESS=TRUE");

  // Then Target compression has GZIP type and all PUT results are correct
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_IDX) == expected_put_source(file));
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_IDX) == filename + ".gz");
  compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_COMPRESSION_IDX), "NONE");
  compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_COMPRESSION_IDX), "GZIP");
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX) == std::string("UPLOADED"));
}

TEST_CASE("should silently upload file with unsupported compression type as uncompressed", "[put_get]") {
  Connection conn;
  // Given Snowflake client is logged in
  const std::string stage = create_stage(conn, unique_stage_name("ODBCTST_SC_UNSUPPORTED"));

  // And File compressed with unsupported format
  auto [filename, file] = test_file("LZMA");

  // When File is uploaded with SOURCE_COMPRESSION set to AUTO_DETECT
  auto put_stmt =
      conn.execute_fetch("PUT 'file://" + as_file_uri(file) + "' @" + stage + " SOURCE_COMPRESSION=AUTO_DETECT");

  // Then Upload succeeds and the file is treated as uncompressed source
  CHECK(get_data<SQL_C_CHAR>(put_stmt, PUT_ROW_SOURCE_IDX) == expected_put_source(file));
  CHECK(get_data<SQL_C_CHAR>(put_stmt, PUT_ROW_TARGET_IDX) == filename + ".gz");
  compare_compression_type(get_data<SQL_C_CHAR>(put_stmt, PUT_ROW_SOURCE_COMPRESSION_IDX), "NONE");
  compare_compression_type(get_data<SQL_C_CHAR>(put_stmt, PUT_ROW_TARGET_COMPRESSION_IDX), "GZIP");
  CHECK(get_data<SQL_C_CHAR>(put_stmt, PUT_ROW_STATUS_IDX) == std::string("UPLOADED"));

  // And Downloaded payload decompresses to the original file bytes
  TempTestDir download_dir("odbc_put_get_unsupported_");
  conn.execute("GET @" + stage + "/" + filename + " 'file://" + as_file_uri(download_dir.path()) + "/'");
  const fs::path downloaded_gz = download_dir.path() / (filename + ".gz");
  REQUIRE(fs::exists(downloaded_gz));

  const std::string decompressed = decompress_gzip_file(downloaded_gz);
  std::ifstream original_ifs(file, std::ios::binary);
  const std::string original_bytes((std::istreambuf_iterator(original_ifs)), std::istreambuf_iterator<char>());
  CHECK(decompressed == original_bytes);
}
