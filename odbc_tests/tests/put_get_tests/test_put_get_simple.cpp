#include <algorithm>
#include <cctype>
#include <filesystem>
#include <fstream>
#include <random>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "get_data.hpp"
#include "put_get_utils.hpp"
#include "utils.hpp"

using namespace pg_utils;
namespace fs = std::filesystem;

static std::string to_lower_copy(const std::string& s) {
  std::string out = s;
  std::transform(out.begin(), out.end(), out.begin(),
                 [](unsigned char c) { return std::tolower(c); });
  return out;
}

static std::pair<std::string, fs::path> basic_test_file() {
  return {"test_data.csv", test_utils::shared_test_data_dir() / "basic" / "test_data.csv"};
}

TEST_CASE("PUT then SELECT from stage", "[put_get][odbc]") {
  Connection conn;
  const std::string stage = pg_utils::create_stage(conn, "ODBCTST_PUT_SELECT");

  // Use shared test data
  auto [filename, file] = basic_test_file();

  // Setup stage and upload file
  std::string put_sql = "PUT 'file://" + as_file_uri(file) + "' @" + stage;
  conn.execute(put_sql);

  {
    // Query the uploaded file data
    std::string select_sql = "SELECT $1, $2, $3 FROM @" + stage;
    auto stmt = conn.execute_fetch(select_sql);

    // Verify the data matches what we uploaded
    CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "1");
    CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "2");
    CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "3");
  }
}

TEST_CASE("PUT then LS shows gz file", "[put_get][odbc]") {
  Connection conn;
  const std::string stage = pg_utils::create_stage(conn, "ODBCTST_PUT_LS");
  auto [filename, file] = basic_test_file();

  // Upload file
  std::string put_sql = "PUT 'file://" + as_file_uri(file) + "' @" + stage;
  conn.execute(put_sql);

  // Verify file was uploaded with LS command
  {
    std::string ls_sql = "LS @" + stage;
    auto stmt = conn.execute_fetch(ls_sql);
    std::string name = get_data<SQL_C_CHAR>(stmt, LS_ROW_NAME_IDX);
    std::string expected = to_lower_copy(stage) + "/" + filename + ".gz";
    CHECK(name == expected);
  }
}

TEST_CASE("GET downloads file to directory", "[put_get][odbc]") {
  Connection conn;
  const std::string stage = pg_utils::create_stage(conn, "ODBCTST_GET");
  auto [filename, file] = basic_test_file();

  // PUT file
  std::string put_sql = "PUT 'file://" + as_file_uri(file) + "' @" + stage;
  conn.execute(put_sql);

  // GET into download dir
  fs::path download_dir = fs::temp_directory_path() / (std::string("odbc_put_get_") + random_hex());
  fs::create_directories(download_dir);
  {
    std::string get_sql =
        "GET @" + stage + "/" + filename + " 'file://" + as_file_uri(download_dir) + "/'";
    auto stmt = conn.execute_fetch(get_sql);
    CHECK(get_data<SQL_C_CHAR>(stmt, GET_ROW_FILE_IDX) == filename + ".gz");
  }

  // Verify the downloaded file exists and content matches
  fs::path gz = download_dir / (filename + ".gz");
  REQUIRE(fs::exists(gz));

  // Decompress and verify content
  std::string decompressed = decompress_gzip_file(gz);
  std::ifstream ifs(file);
  std::string original_content((std::istreambuf_iterator<char>(ifs)),
                               std::istreambuf_iterator<char>());
  CHECK(decompressed == original_content);
}

// BREAKING CHANGE: Compression type is now returned in uppercase
TEST_CASE("PUT then GET returns expected rowset metadata", "[put_get][odbc]") {
  Connection conn;
  const std::string stage = pg_utils::create_stage(conn, "ODBCTST_PUT_ROWSET");
  auto [filename, file] = basic_test_file();

  {
    // Upload file
    auto stmt = conn.execute_fetch("PUT 'file://" + as_file_uri(file) + "' @" + stage);

    // Assert PUT result fields (file, gz target, sizes, compression, status, message)
    CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_IDX) == filename);
    CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_IDX) == filename + ".gz");
    CHECK(get_data<SQL_C_LONG>(stmt, PUT_ROW_SOURCE_SIZE_IDX) == 6);
    CHECK(get_data<SQL_C_LONG>(stmt, PUT_ROW_TARGET_SIZE_IDX) == 32);
    compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_COMPRESSION_IDX), "NONE");
    compare_compression_type(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_COMPRESSION_IDX), "GZIP");
    CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX) == "UPLOADED");

    OLD_DRIVER_ONLY("BD#3: Encryption field is no longer included in the result") {
      CHECK(get_data<SQL_C_CHAR>(stmt, 8) == "ENCRYPTED");
      CHECK(get_data<SQL_C_CHAR>(stmt, 9) == "");
    }

    NEW_DRIVER_ONLY("BD#3: Encryption field is no longer included in the result") {
      CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_MESSAGE_IDX) == "");  // 8
    }
  }

  // Create directory for download
  fs::path download_dir = fs::temp_directory_path() / (std::string("odbc_put_get_") + random_hex());
  fs::create_directories(download_dir);
  {
    // Download file
    auto stmt = conn.execute_fetch("GET @" + stage + "/" + filename + " 'file://" +
                                   as_file_uri(download_dir) + "/'");

    // Assert GET result fields (file, size, status, message)
    CHECK(get_data<SQL_C_CHAR>(stmt, GET_ROW_FILE_IDX) == filename + ".gz");

    OLD_DRIVER_ONLY("BD#4: GET rowset metadata contains file size after decryption") {
      CHECK(get_data<SQL_C_LONG>(stmt, GET_ROW_SIZE_IDX) == 32);
    }

    NEW_DRIVER_ONLY("BD#4: GET rowset metadata contains file size after decryption") {
      CHECK(get_data<SQL_C_LONG>(stmt, GET_ROW_SIZE_IDX) == 26);
    }

    CHECK(get_data<SQL_C_CHAR>(stmt, GET_ROW_STATUS_IDX) == "DOWNLOADED");

    OLD_DRIVER_ONLY("BD#3: Encryption field is no longer included in the result") {
      CHECK(get_data<SQL_C_CHAR>(stmt, 4) == "DECRYPTED");
      CHECK(get_data<SQL_C_CHAR>(stmt, 5) == "");
    }

    NEW_DRIVER_ONLY("BD#3: Encryption field is no longer included in the result") {
      CHECK(get_data<SQL_C_CHAR>(stmt, GET_ROW_MESSAGE_IDX) == "");  // 4
    }
  }
}
