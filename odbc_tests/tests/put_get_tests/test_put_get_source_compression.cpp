#include <filesystem>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "get_data.hpp"
#include "put_get_utils.hpp"

namespace fs = std::filesystem;
using namespace pg_utils;

static fs::path compression_tests_dir() { return shared_test_data_dir() / "compression"; }

static std::pair<std::string, fs::path> test_file(const std::string& compression_type) {
  if (compression_type == "GZIP") {
    return {"test_data.csv.gz", compression_tests_dir() / "test_data.csv.gz"};
  } else if (compression_type == "BZIP2") {
    return {"test_data.csv.bz2", compression_tests_dir() / "test_data.csv.bz2"};
  } else if (compression_type == "BROTLI") {
    return {"test_data.csv.br", compression_tests_dir() / "test_data.csv.br"};
  } else if (compression_type == "ZSTD") {
    return {"test_data.csv.zst", compression_tests_dir() / "test_data.csv.zst"};
  } else if (compression_type == "DEFLATE") {
    return {"test_data.csv.deflate", compression_tests_dir() / "test_data.csv.deflate"};
  } else if (compression_type == "RAW_DEFLATE") {
    return {"test_data.csv.raw_deflate", compression_tests_dir() / "test_data.csv.raw_deflate"};
  } else if (compression_type == "LZMA") {
    return {"test_data.csv.xz", compression_tests_dir() / "test_data.csv.xz"};
  } else if (compression_type == "NONE") {
    return {"test_data.csv", compression_tests_dir() / "test_data.csv"};
  }
  FAIL("Unsupported compression type: " << compression_type);
  return {"", ""};
}

TEST_CASE("PUT SOURCE_COMPRESSION=AUTO_DETECT standard types", "[put_get][odbc]") {
  Connection conn;
  const std::string stage = create_stage(conn, "ODBCTST_SC_AUTO_DETECT_STD");

  for (const std::string comp : {"GZIP", "BZIP2", "BROTLI", "ZSTD"}) {
    auto [filename, file] = test_file(comp);
    auto stmt = conn.execute_fetch("PUT 'file://" + as_file_uri(file) + "' @" + stage +
                                   " SOURCE_COMPRESSION=AUTO_DETECT");

    CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_IDX) == filename);
    CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_IDX) == filename);
    compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_COMPRESSION_IDX), comp);
    compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_COMPRESSION_IDX), comp);
    CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX) == std::string("UPLOADED"));
  }
}

TEST_CASE("PUT SOURCE_COMPRESSION=AUTO_DETECT with DEFLATE", "[put_get][odbc]") {
  Connection conn;
  const std::string stage = create_stage(conn, "ODBCTST_SC_AUTO_DETECT_DEFLATE");
  auto [filename, file] = test_file("DEFLATE");

  auto stmt = conn.execute_fetch("PUT 'file://" + as_file_uri(file) + "' @" + stage +
                                 " SOURCE_COMPRESSION=AUTO_DETECT");

  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_IDX) == filename);
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_IDX) == filename);
  compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_COMPRESSION_IDX), "DEFLATE");
  compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_COMPRESSION_IDX), "DEFLATE");
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX) == std::string("UPLOADED"));
}

TEST_CASE("PUT SOURCE_COMPRESSION=AUTO_DETECT NONE with AUTO_COMPRESS=FALSE", "[put_get][odbc]") {
  Connection conn;
  const std::string stage = create_stage(conn, "ODBCTST_SC_AUTO_DETECT_NONE_NO_AC");
  auto [filename, file] = test_file("NONE");

  auto stmt = conn.execute_fetch("PUT 'file://" + as_file_uri(file) + "' @" + stage +
                                 " SOURCE_COMPRESSION=AUTO_DETECT AUTO_COMPRESS=FALSE");

  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_IDX) == filename);
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_IDX) == filename);
  compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_COMPRESSION_IDX), "NONE");
  compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_COMPRESSION_IDX), "NONE");
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX) == std::string("UPLOADED"));
}

TEST_CASE("PUT SOURCE_COMPRESSION=AUTO_DETECT NONE with AUTO_COMPRESS=TRUE", "[put_get][odbc]") {
  Connection conn;
  const std::string stage = create_stage(conn, "ODBCTST_SC_AUTO_DETECT_NONE_AC");
  auto [filename, file] = test_file("NONE");

  auto stmt = conn.execute_fetch("PUT 'file://" + as_file_uri(file) + "' @" + stage +
                                 " SOURCE_COMPRESSION=AUTO_DETECT AUTO_COMPRESS=TRUE");

  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_IDX) == filename);
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_IDX) == filename + ".gz");
  compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_COMPRESSION_IDX), "NONE");
  compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_COMPRESSION_IDX), "GZIP");
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX) == std::string("UPLOADED"));
}

TEST_CASE("PUT SOURCE_COMPRESSION= explicit standard types", "[put_get][odbc]") {
  Connection conn;
  const std::string stage = create_stage(conn, "ODBCTST_SC_EXPLICIT_STD");

  for (const std::string comp : {"GZIP", "BZIP2", "ZSTD", "DEFLATE", "RAW_DEFLATE"}) {
    auto [filename, file] = test_file(comp);
    auto stmt = conn.execute_fetch("PUT 'file://" + as_file_uri(file) + "' @" + stage +
                                   " SOURCE_COMPRESSION=" + comp);

    CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_IDX) == filename);
    CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_IDX) == filename);
    compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_COMPRESSION_IDX), comp);
    compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_COMPRESSION_IDX), comp);
    CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX) == std::string("UPLOADED"));
  }
}

TEST_CASE("PUT SOURCE_COMPRESSION=BROTLI explicit", "[put_get][odbc]") {
  Connection conn;
  const std::string stage = create_stage(conn, "ODBCTST_SC_EXPLICIT_BROTLI");
  auto [filename, file] = test_file("BROTLI");

  auto stmt = conn.execute_fetch("PUT 'file://" + as_file_uri(file) + "' @" + stage +
                                 " SOURCE_COMPRESSION=BROTLI");

  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_IDX) == filename);
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_IDX) == filename);
  compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_COMPRESSION_IDX), "BROTLI");
  compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_COMPRESSION_IDX), "BROTLI");
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX) == std::string("UPLOADED"));
}

TEST_CASE("PUT SOURCE_COMPRESSION=NONE with AUTO_COMPRESS=FALSE explicit", "[put_get][odbc]") {
  Connection conn;
  const std::string stage = create_stage(conn, "ODBCTST_SC_EXPLICIT_NONE_NO_AC");
  auto [filename, file] = test_file("NONE");

  auto stmt = conn.execute_fetch("PUT 'file://" + as_file_uri(file) + "' @" + stage +
                                 " SOURCE_COMPRESSION=NONE AUTO_COMPRESS=FALSE");

  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_IDX) == filename);
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_IDX) == filename);
  compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_COMPRESSION_IDX), "NONE");
  compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_COMPRESSION_IDX), "NONE");
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX) == std::string("UPLOADED"));
}

TEST_CASE("PUT SOURCE_COMPRESSION=NONE with AUTO_COMPRESS=TRUE explicit", "[put_get][odbc]") {
  Connection conn;
  const std::string stage = create_stage(conn, "ODBCTST_SC_EXPLICIT_NONE_AC");
  auto [filename, file] = test_file("NONE");

  auto stmt = conn.execute_fetch("PUT 'file://" + as_file_uri(file) + "' @" + stage +
                                 " SOURCE_COMPRESSION=NONE AUTO_COMPRESS=TRUE");

  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_IDX) == filename);
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_IDX) == filename + ".gz");
  compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_COMPRESSION_IDX), "NONE");
  compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_COMPRESSION_IDX), "GZIP");
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX) == std::string("UPLOADED"));
}
