#include <sql.h>
#include <sqlext.h>

#include <filesystem>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "put_get_utils.hpp"

namespace fs = std::filesystem;
using namespace pg_utils;

// SNOW-3704966 (BD#92): GET into a missing local directory. The universal driver
// creates the directory tree before writing (like Python/JDBC); legacy ODBC fails.
TEST_CASE("should create the local destination directory when it is missing on GET", "[put_get]") {
  // Given a CSV file uploaded to a temporary stage
  Connection conn;
  const std::string stage = create_stage(conn, unique_stage_name("ODBCTST_GET_MKDIR"));

  TempTestDir download_root("odbc_get_mkdir_");
  const fs::path source_file = write_text_file(download_root.path(), "mkdir_data.csv", "a,b,c\n");

  const std::string put_sql = "PUT 'file://" + as_file_uri(source_file) + "' @" + stage;
  conn.execute(put_sql);

  // When GET targets a nested subdirectory that does not exist yet
  const fs::path missing_dir = download_root.path() / "nested" / "missing";
  REQUIRE_FALSE(fs::exists(missing_dir));

  const std::string get_sql = "GET @" + stage + "/mkdir_data.csv 'file://" + as_file_uri(missing_dir) + "/'";
  auto stmt = conn.createStatement();
  const SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)get_sql.c_str(), SQL_NTS);

  NEW_DRIVER_ONLY("BD#92") {
    // Then the destination directory is created and the file is downloaded into it
    REQUIRE_ODBC(ret, stmt);
    REQUIRE_ODBC(SQLFetch(stmt.getHandle()), stmt);
    CHECK(get_data<SQL_C_CHAR>(stmt, GET_ROW_FILE_IDX) == "mkdir_data.csv.gz");
    CHECK(get_data<SQL_C_CHAR>(stmt, GET_ROW_STATUS_IDX) == "DOWNLOADED");
    CHECK(fs::exists(missing_dir / "mkdir_data.csv.gz"));
  }
  OLD_DRIVER_ONLY("BD#92") {
    // Then the legacy driver fails because it does not create the missing directory
    REQUIRE_ODBC_ERROR(ret, stmt);
  }
}
