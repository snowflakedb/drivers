#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <filesystem>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "put_get_utils.hpp"

namespace fs = std::filesystem;
using namespace pg_utils;

TEST_CASE("should upload file when source path contains dotdot segments", "[put_get]") {
  // Given A source file exists in a temporary directory
  TempTestDir temp_dir("odbc_put_dotdot_");
  fs::path sub_dir = temp_dir.path() / "sub";
  fs::create_directory(sub_dir);
  const fs::path source_file = write_text_file(temp_dir.path(), "dotdot_data.csv", "a,b,c\n");

  // When PUT command is executed with a source path containing dotdot segments
  fs::path dotdot_path = sub_dir / ".." / "dotdot_data.csv";  // absolute, un-normalized
  Connection conn;
  const std::string stage = create_stage(conn, unique_stage_name("ODBCTST_DOTDOT"));
  const std::string put_sql =
      "PUT 'file://" + as_file_uri(dotdot_path) + "' @" + stage + " AUTO_COMPRESS=FALSE OVERWRITE=TRUE";

  auto stmt = conn.execute_fetch(put_sql);

  // Then File is uploaded successfully with correct target name
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX) == "UPLOADED");
  // The `..` is resolved by canonicalization; stage object name is the canonical basename.
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_IDX) == "dotdot_data.csv");
}

TEST_CASE("should upload file when source path is relative to working directory", "[put_get]") {
  // Create under CWD so fs::relative stays same-root on Windows (system temp can live
  // on another drive, which makes lexically_relative return an empty path).
  const fs::path cwd = fs::current_path();
  const fs::path work_dir = cwd / ("odbc_put_relative_" + random_hex(4));
  fs::create_directories(work_dir);
  struct WorkDirCleanup {
    fs::path path;
    ~WorkDirCleanup() {
      // A destructor must not throw, so use the non-throwing overload and check
      // the result. Surface a cleanup failure via Catch2's WARN (same pattern as
      // HomeTempDir) without failing the test.
      std::error_code ec;
      fs::remove_all(path, ec);
      if (ec) {
        WARN("WorkDirCleanup failed for " << path.string() << ": " << ec.message());
      }
    }
  } cleanup{work_dir};

  // Given A source file exists in a temporary directory
  const fs::path source_file = write_text_file(work_dir, "relative_data.csv", "a,b,c\n");

  // When PUT command is executed with a path relative to the process working directory
  const fs::path relative_path = fs::relative(source_file, cwd);
  REQUIRE(!relative_path.empty());
  REQUIRE(!relative_path.is_absolute());
  Connection conn;
  const std::string stage = create_stage(conn, unique_stage_name("ODBCTST_RELATIVE"));
  const std::string put_sql =
      "PUT 'file://" + as_file_uri(relative_path) + "' @" + stage + " AUTO_COMPRESS=FALSE OVERWRITE=TRUE";

  auto stmt = conn.execute_fetch(put_sql);

  // Then File is uploaded successfully with correct target name
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX) == "UPLOADED");
  CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_IDX) == "relative_data.csv");
}

TEST_CASE("should upload file at symlinked source path", "[put_get]") {
  // Symlinks are a Unix feature; on Windows this scenario is not applicable.
  UNIX_ONLY {
    // Given A source file and a symlink pointing to it exist in a temporary directory
    TempTestDir temp_dir("odbc_put_symlink_");
    const fs::path real_file = write_text_file(temp_dir.path(), "real.csv", "a,b,c\n");
    const fs::path link_file = temp_dir.path() / "link.csv";
    fs::create_symlink(real_file, link_file);

    // When PUT command is executed with the symlink as source path
    Connection conn;
    const std::string stage = create_stage(conn, unique_stage_name("ODBCTST_SYMLINK"));
    const std::string put_sql =
        "PUT 'file://" + as_file_uri(link_file) + "' @" + stage + " AUTO_COMPRESS=FALSE OVERWRITE=TRUE";

    auto stmt = conn.execute_fetch(put_sql);

    // Then File is uploaded successfully
    CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX) == "UPLOADED");
    // New driver resolves the symlink via dunce::canonicalize; old driver (libsnowflakeclient)
    // does not canonicalize and preserves the symlink's own name.
    NEW_DRIVER_ONLY("BD#131") { CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_IDX) == "real.csv"); }
    OLD_DRIVER_ONLY("BD#131") { CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_TARGET_IDX) == "link.csv"); }
  }
}

TEST_CASE("should upload file when source path starts with tilde", "[put_get]") {
  const fs::path home = user_home_dir();
  if (home.empty()) {
    SKIP("Home directory is not set in the environment");
  }

  // Given A source file exists in a subdirectory under the home directory
  HomeTempDir home_subdir(home, "odbc_put_tilde_");
  write_text_file(home_subdir.path(), "tilde_data.csv", "a,b,c\n");

  // When PUT command is executed with a leading ~ in the source path
  Connection conn;
  const std::string stage = create_stage(conn, unique_stage_name("ODBCTST_TILDE"));
  const std::string put_sql =
      "PUT 'file://~/" + home_subdir.name() + "/tilde_data.csv' @" + stage + " AUTO_COMPRESS=FALSE OVERWRITE=TRUE";

  NEW_DRIVER_ONLY("BD#82") {
    auto stmt = conn.execute_fetch(put_sql);
    // Then File is uploaded successfully
    CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX) == "UPLOADED");
  }
  OLD_DRIVER_ONLY("BD#82") {
    // Legacy libsnowflakeclient treats `~` literally; PUT fails to match any file.
    auto stmt = conn.createStatement();
    const SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar(put_sql.c_str()), SQL_NTS);
    REQUIRE_ODBC_ERROR(ret, stmt);
  }
}
