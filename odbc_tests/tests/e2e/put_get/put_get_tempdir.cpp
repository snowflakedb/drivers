#include <sql.h>
#include <sqlext.h>

#include <filesystem>
#include <fstream>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "get_data.hpp"
#include "odbc_matchers.hpp"
#include "put_get_utils.hpp"
#include "test_setup.hpp"

namespace fs = std::filesystem;
using namespace pg_utils;

static std::string connection_string_with_put_tempdir(const fs::path& dir) {
  return get_connection_string() + "PUT_TEMPDIR={" + dir.string() + "};";
}

static void write_compressible_csv(const fs::path& file) {
  std::ofstream out(file, std::ios::binary);
  for (int i = 0; i < 1024; ++i) {
    out << "aaaaaaaaaa,bbbbbbbbbb,cccccccccc\n";
  }
}

TEST_CASE("should create nested PUT_TEMPDIR and clean gzip tempfiles after PUT AUTO_COMPRESS", "[put_get]") {
  TempTestDir src_dir("odbc_put_tempdir_src_");
  TempTestDir temp_root("odbc_put_tempdir_dst_");
  const fs::path nested = temp_root.path() / "nested" / "gzip";
  const auto file = src_dir.path() / "compressible.csv";
  write_compressible_csv(file);

  // Given a nested PUT_TEMPDIR that does not exist yet
  REQUIRE_FALSE(fs::exists(nested));
  Connection conn(connection_string_with_put_tempdir(nested));
  const std::string stage = pg_utils::create_stage(conn, unique_stage_name("ODBCTST_TEMPDIR"));

  // When the file is uploaded with AUTO_COMPRESS
  auto put_stmt =
      conn.execute_fetch("PUT 'file://" + as_file_uri(file) + "' @" + stage + " AUTO_COMPRESS=TRUE OVERWRITE=TRUE");

  // Then the upload succeeds, the nested directory is created, and gzip tempfiles are removed
  CHECK(get_data<SQL_C_CHAR>(put_stmt, PUT_ROW_STATUS_IDX) == "UPLOADED");
  CHECK(get_data<SQL_C_CHAR>(put_stmt, PUT_ROW_TARGET_COMPRESSION_IDX) == "gzip");
  REQUIRE(fs::is_directory(nested));
  REQUIRE(fs::is_empty(nested));
}

TEST_CASE("should treat PUT_TEMPDIR shell metacharacters as a directory name", "[put_get]") {
  TempTestDir src_dir("odbc_put_tempdir_inj_src_");
  TempTestDir temp_root("odbc_put_tempdir_inj_dst_");
  const fs::path injection_dir = temp_root.path() / "dir;touch pwned";
  const fs::path sentinel = temp_root.path() / "pwned";
  const auto file = src_dir.path() / "compressible.csv";
  write_compressible_csv(file);

  // Given PUT_TEMPDIR containing shell metacharacters
  Connection conn(connection_string_with_put_tempdir(injection_dir));
  const std::string stage = pg_utils::create_stage(conn, unique_stage_name("ODBCTST_TEMPDIR_INJ"));

  // When the file is uploaded with AUTO_COMPRESS
  auto put_stmt =
      conn.execute_fetch("PUT 'file://" + as_file_uri(file) + "' @" + stage + " AUTO_COMPRESS=TRUE OVERWRITE=TRUE");

  // Then the path is used as a directory name and the sentinel file is not created
  CHECK(get_data<SQL_C_CHAR>(put_stmt, PUT_ROW_STATUS_IDX) == "UPLOADED");
  REQUIRE(fs::is_directory(injection_dir));
  REQUIRE_FALSE(fs::exists(sentinel));
}

TEST_CASE("should fail PUT AUTO_COMPRESS when PUT_TEMPDIR is an existing file", "[put_get]") {
  TempTestDir src_dir("odbc_put_tempdir_bad_src_");
  TempTestDir temp_root("odbc_put_tempdir_bad_dst_");
  const fs::path not_a_dir = temp_root.path() / "not-a-dir";
  {
    std::ofstream out(not_a_dir, std::ios::binary);
    out << "not a directory\n";
  }
  const auto file = src_dir.path() / "compressible.csv";
  write_compressible_csv(file);

  // Given PUT_TEMPDIR pointing at a regular file
  Connection conn(connection_string_with_put_tempdir(not_a_dir));
  const std::string stage = pg_utils::create_stage(conn, unique_stage_name("ODBCTST_TEMPDIR_BAD"));

  // When the file is uploaded with AUTO_COMPRESS
  auto stmt = conn.createStatement();
  std::string put_sql = "PUT 'file://" + as_file_uri(file) + "' @" + stage + " AUTO_COMPRESS=TRUE OVERWRITE=TRUE";
  const SQLRETURN ret = SQLExecDirect(stmt.getHandle(), reinterpret_cast<SQLCHAR*>(put_sql.data()), SQL_NTS);

  // Then the PUT fails because the compression temp directory cannot be created
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("HY000"));
}
