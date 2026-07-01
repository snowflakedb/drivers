#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstdlib>
#include <filesystem>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "put_get_utils.hpp"

namespace fs = std::filesystem;
using namespace pg_utils;

namespace {

// Resolve the user's home directory the same way the new driver does
// (`dirs::home_dir()`): `$HOME` on Unix, `%USERPROFILE%` on Windows. Returns an
// empty path when the variable is unset so the caller can skip rather than
// write to an unexpected location.
fs::path user_home_dir() {
#ifdef _WIN32
  const char* home = std::getenv("USERPROFILE");
#else
  const char* home = std::getenv("HOME");
#endif
  return (home != nullptr) ? fs::path(home) : fs::path();
}

// RAII wrapper for a uniquely-named subdirectory created directly under the
// user's home directory. The new driver expands a leading `~` to the real home
// directory, so the source file must physically live there; this guard keeps
// the test hermetic by removing the subtree on destruction. Mirrors
// `pg_utils::TempTestDir`, which only ever roots under the system temp dir.
class HomeTempDir {
 public:
  HomeTempDir(const fs::path& home, const std::string& prefix) : name_(prefix + random_hex(4)), path_(home / name_) {
    fs::create_directories(path_);
  }

  ~HomeTempDir() {
    // A destructor must not throw, so use the non-throwing overload and check
    // the result. Surface a cleanup failure via Catch2's WARN rather than bare
    // std::cerr (test-diagnostic-output), without failing the test.
    std::error_code ec;
    fs::remove_all(path_, ec);
    if (ec) {
      WARN("HomeTempDir cleanup failed for " << path_.string() << ": " << ec.message());
    }
  }

  HomeTempDir(const HomeTempDir&) = delete;
  HomeTempDir& operator=(const HomeTempDir&) = delete;

  [[nodiscard]] const fs::path& path() const { return path_; }
  [[nodiscard]] const std::string& name() const { return name_; }

 private:
  std::string name_;
  fs::path path_;
};

}  // namespace

TEST_CASE("should expand a leading ~ in the PUT source path to the home directory", "[put_get]") {
  // Given a connection, a temporary stage, and a CSV file staged in a uniquely
  // named subdirectory directly under the user's home directory
  const fs::path home = user_home_dir();
  if (home.empty()) {
    SKIP("Home directory is not set in the environment");
  }

  Connection conn;
  const std::string stage = create_stage(conn, unique_stage_name("ODBCTST_TILDE"));

  HomeTempDir home_subdir(home, "odbc_tilde_test_");
  const fs::path staged_file = write_text_file(home_subdir.path(), "tilde_data.csv", "a,b,c\n");

  // When PUT is executed with a leading `~/` in the source path
  const std::string put_query = "PUT 'file://~/" + home_subdir.name() + "/tilde_data.csv' @" + stage;
  auto stmt = conn.createStatement();
  const SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)put_query.c_str(), SQL_NTS);

  NEW_DRIVER_ONLY("BD#82") {
    // Then `~` expands to the home directory, the file is found and uploaded
    REQUIRE_ODBC(ret, stmt);
    REQUIRE_ODBC(SQLFetch(stmt.getHandle()), stmt);
    CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_STATUS_IDX) == "UPLOADED");
    CHECK(get_data<SQL_C_CHAR>(stmt, PUT_ROW_SOURCE_IDX) == expected_put_source(staged_file));
  }
  OLD_DRIVER_ONLY("BD#82") {
    // Then `~` is treated literally; no file matches the literal pattern and
    // the PUT fails rather than uploading anything
    REQUIRE_ODBC_ERROR(ret, stmt);
  }
}
