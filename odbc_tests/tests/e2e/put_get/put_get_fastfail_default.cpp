#include <sql.h>
#include <sqlext.h>

#include <algorithm>
#include <filesystem>
#include <fstream>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "odbc_matchers.hpp"
#include "put_get_utils.hpp"
#include "test_setup.hpp"

#ifndef _WIN32
#include <unistd.h>
#endif

// ODBC defaults PUT_FASTFAIL/GET_FASTFAIL to false (collect-all), but PUT and
// GET diverge on what that means (legacy libsnowflakeclient parity,
// SNOW-3838438): PUT attempts every file then raises one aggregate error
// naming all failures; GET keeps going and reports each failure as an ERROR
// row instead. Setting PUT_FASTFAIL=true / GET_FASTFAIL=true (connection
// string, session, or per-statement) overrides the default to fail-fast —
// abort at the first failing file (SNOW-3857648).
//
// All tests fail a file via Unix permissions — the only failure mode that
// happens after the batch is already expanded to concrete files (a bad
// glob/missing dir aborts the whole PUT/GET regardless of fastfail, so it
// can't demonstrate collect-all).

namespace fs = std::filesystem;
using namespace pg_utils;

namespace {

// Permission-based failure injection only works for a non-root Unix caller:
// root bypasses owner-permission checks, and Windows doesn't honor POSIX
// mode bits the way `std::filesystem::permissions` expects.
bool can_run_permission_based_test() {
#ifdef _WIN32
  return false;
#else
  return geteuid() != 0;
#endif
}

// Strips every permission bit so any subsequent open() on `path` fails with
// EACCES, independent of the process umask.
void make_unreadable(const fs::path& path) { fs::permissions(path, fs::perms::none, fs::perm_options::replace); }

struct GetRow {
  std::string file;
  SQLINTEGER size = 0;
  std::string status;
  std::string encryption;
  std::string message;
};

std::vector<GetRow> fetch_get_rows(StatementHandleWrapper& stmt) {
  std::vector<GetRow> rows;
  while (true) {
    SQLRETURN ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) break;
    REQUIRE_ODBC(ret, stmt);
    rows.push_back(GetRow{
        get_data<SQL_C_CHAR>(stmt, GET_ROW_FILE_IDX),
        get_data<SQL_C_LONG>(stmt, GET_ROW_SIZE_IDX),
        get_data<SQL_C_CHAR>(stmt, GET_ROW_STATUS_IDX),
        get_data<SQL_C_CHAR>(stmt, GET_ROW_ENCRYPTION_IDX),
        get_data<SQL_C_CHAR>(stmt, GET_ROW_MESSAGE_IDX),
    });
  }
  return rows;
}

// The GET result's `file` column is server-reported and may or may not carry
// a stage-name prefix depending on how the GET target was addressed; match
// by substring on the known-unique local filename rather than assuming an
// exact value.
const GetRow& find_row_containing(const std::vector<GetRow>& rows, const std::string& needle) {
  auto it = std::find_if(rows.begin(), rows.end(),
                         [&](const GetRow& row) { return row.file.find(needle) != std::string::npos; });
  REQUIRE(it != rows.end());
  return *it;
}

// Writes good.csv + blocked.csv into `source_dir` and PUTs both to `stage`.
void upload_good_and_blocked_file(Connection& conn, const std::string& stage, TempTestDir& source_dir) {
  const fs::path good_file = write_text_file(source_dir.path(), "good.csv", "1,2,3\n");
  const fs::path blocked_file = write_text_file(source_dir.path(), "blocked.csv", "4,5,6\n");
  conn.execute("PUT 'file://" + as_file_uri(good_file) + "' @" + stage);
  conn.execute("PUT 'file://" + as_file_uri(blocked_file) + "' @" + stage);
}

// Pre-creates an unreadable `.part` placeholder at `download_dir / part_filename`
// so the GET's rename-into-place write fails for that file.
void block_part_download(const fs::path& download_dir, const std::string& part_filename) {
  const fs::path blocked_part = download_dir / part_filename;
  {
    std::ofstream placeholder(blocked_part, std::ios::binary);
    placeholder << "placeholder";
  }
  make_unreadable(blocked_part);
}

}  // namespace

TEST_CASE("should fail the statement after attempting every file in a PUT batch (collect-all default)", "[put_get]") {
  if (!can_run_permission_based_test()) {
    SKIP("Permission-based failure injection requires a non-root Unix user");
  }

  // Given a stage and four local files, two of which are unreadable
  Connection conn;
  const std::string stage = create_stage(conn, unique_stage_name("ODBCTST_FASTFAIL_PUT"));
  TempTestDir source_dir("odbc_fastfail_put_");
  write_text_file(source_dir.path(), "fastfail_1_ok.csv", "1,2,3\n");
  const fs::path blocked_file1 = write_text_file(source_dir.path(), "fastfail_2_blocked.csv", "4,5,6\n");
  const fs::path blocked_file2 = write_text_file(source_dir.path(), "fastfail_3_blocked.csv", "7,8,9\n");
  write_text_file(source_dir.path(), "fastfail_4_ok.csv", "10,11,12\n");
  make_unreadable(blocked_file1);
  make_unreadable(blocked_file2);

  // When all four files are PUT in a single batch via a glob pattern (default
  // PUT_FASTFAIL=false, collect-all)
  const std::string pattern = as_file_uri(source_dir.path()) + "/*.csv";
  auto stmt = conn.createStatement();
  const std::string put_sql = "PUT 'file://" + pattern + "' @" + stage;
  const SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)put_sql.c_str(), SQL_NTS);

  // Then the statement fails — every file was attempted first (legacy ODBC
  // parity, SNOW-3838438) — and the aggregate error names both blocked files
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() &&
                                          OdbcMatchers::HasDiagMessage("fastfail_2_blocked.csv") &&
                                          OdbcMatchers::HasDiagMessage("fastfail_3_blocked.csv"));
}

TEST_CASE("should return a mixed rowset when one file in a GET batch cannot be written locally", "[put_get]") {
  SKIP_OLD_DRIVER("BD#104",
                  "Old driver uses a different local-write staging scheme than the new core's `<file>.part` rename, so "
                  "pre-blocking the `.part` placeholder doesn't reproduce a write failure against it");
  if (!can_run_permission_based_test()) {
    SKIP("Permission-based failure injection requires a non-root Unix user");
  }

  // Given two files uploaded to a stage
  Connection conn;
  const std::string stage = create_stage(conn, unique_stage_name("ODBCTST_FASTFAIL_GET"));
  TempTestDir source_dir("odbc_fastfail_get_src_");
  upload_good_and_blocked_file(conn, stage, source_dir);

  // And a download directory where the write destination for one of the two
  // files is pre-blocked: the driver downloads into a sibling `<file>.part`
  // and renames it into place on success, so pre-creating an unwritable
  // `.part` file makes exactly that file's write fail.
  TempTestDir download_dir("odbc_fastfail_get_dst_");
  block_part_download(download_dir.path(), "blocked.csv.gz.part");

  // When both files are downloaded in a single GET batch
  auto stmt = conn.execute("GET @" + stage + " 'file://" + as_file_uri(download_dir.path()) + "/'");

  // Then the statement succeeds (no exception) and returns one row per
  // file: DOWNLOADED for the unblocked file, ERROR for the blocked one
  auto rows = fetch_get_rows(stmt);
  REQUIRE(rows.size() == 2);

  const auto& ok_row = find_row_containing(rows, "good.csv.gz");
  CHECK(ok_row.status == "DOWNLOADED");
  CHECK(ok_row.encryption == "DECRYPTED");
  CHECK(ok_row.message.empty());
  CHECK(fs::exists(download_dir.path() / "good.csv.gz"));

  const auto& error_row = find_row_containing(rows, "blocked.csv.gz");
  CHECK(error_row.status == "ERROR");
  CHECK(error_row.size == 0);
  CHECK(error_row.encryption == "DECRYPTED");
  // Substring match: the message carries the OS reason (e.g. "... : Permission
  // denied (os error 13)"), whose exact text is platform-dependent.
  CHECK(error_row.message.find("Failed to read or write file") != std::string::npos);
  CHECK_FALSE(fs::exists(download_dir.path() / "blocked.csv.gz"));
}

TEST_CASE("should fail fast instead of collecting errors when PUT_FASTFAIL override is set on a failing PUT batch",
          "[put_get]") {
  if (!can_run_permission_based_test()) {
    SKIP("Permission-based failure injection requires a non-root Unix user");
  }

  // Given one readable and one unreadable file. PUT_FASTFAIL=true overrides
  // the wrapper's collect-all default.
  const std::string conn_str = get_connection_string() + "PUT_FASTFAIL=true;";
  Connection conn(conn_str);
  const std::string stage = create_stage(conn, unique_stage_name("ODBCTST_FASTFAIL_PUT_FF"));
  TempTestDir source_dir("odbc_fastfail_put_ff_");
  write_text_file(source_dir.path(), "good.csv", "1,2,3\n");
  const fs::path blocked_file = write_text_file(source_dir.path(), "blocked.csv", "4,5,6\n");
  make_unreadable(blocked_file);

  // When both files are PUT in a single batch via a glob pattern
  const std::string pattern = as_file_uri(source_dir.path()) + "/*.csv";
  const std::string put_sql = "PUT 'file://" + pattern + "' @" + stage;
  auto stmt = conn.createStatement();
  const SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)put_sql.c_str(), SQL_NTS);

  // Then the statement fails fast on both drivers, each with its own
  // diagnostic: the new core surfaces the underlying file error, while legacy
  // libsnowflakeclient reports its own "fast fail enabled" abort.
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError());
  OLD_DRIVER_ONLY("BD#105") { REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::HasDiagMessage("Fast fail enabled")); }
  NEW_DRIVER_ONLY("BD#105") {
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::HasDiagMessage("Failed to read or write file"));

    // And the message is the bare per-file error, not the collect-all
    // aggregate: the aggregate's per-file entries are "{name}: {error}", so
    // a regression to collect-all would still contain "Failed to read or
    // write file" (as part of one entry) but would also say
    // "PUT failed for N file(s)" -- absent here.
    REQUIRE_THAT(OdbcResult(ret, stmt), !OdbcMatchers::HasDiagMessage("PUT failed for"));
  }
}

TEST_CASE("should fail fast instead of collecting errors when GET_FASTFAIL override is set on a failing GET batch",
          "[put_get]") {
  SKIP_OLD_DRIVER("BD#104",
                  "Old driver uses a different local-write staging scheme than the new core's `<file>.part` rename, so "
                  "pre-blocking the `.part` placeholder doesn't reproduce a write failure against it (same root cause "
                  "as the collect-all GET case above)");
  if (!can_run_permission_based_test()) {
    SKIP("Permission-based failure injection requires a non-root Unix user");
  }

  // Given the same partial-failure setup as the collect-all GET case above,
  // but GET_FASTFAIL=true forces fail-fast instead of collect-all.
  const std::string conn_str = get_connection_string() + "GET_FASTFAIL=true;";
  Connection conn(conn_str);
  const std::string stage = create_stage(conn, unique_stage_name("ODBCTST_FASTFAIL_GET_FF"));
  TempTestDir source_dir("odbc_fastfail_get_ff_src_");
  upload_good_and_blocked_file(conn, stage, source_dir);

  TempTestDir download_dir("odbc_fastfail_get_ff_dst_");
  block_part_download(download_dir.path(), "blocked.csv.gz.part");

  // When both files are downloaded in a single GET batch
  const std::string get_sql = "GET @" + stage + " 'file://" + as_file_uri(download_dir.path()) + "/'";
  auto stmt = conn.createStatement();
  const SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)get_sql.c_str(), SQL_NTS);

  // Then the statement fails fast (contrast with the collect-all case
  // above). File processing order within the batch isn't guaranteed, so
  // only the batch-level failure is asserted here.
  REQUIRE_THAT(OdbcResult(ret, stmt),
               OdbcMatchers::IsError() && OdbcMatchers::HasDiagMessage("Failed to read or write file"));
}
