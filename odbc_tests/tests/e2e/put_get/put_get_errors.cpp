#include <sql.h>
#include <sqlext.h>

#include <filesystem>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"
#include "put_get_utils.hpp"

using namespace pg_utils;

TEST_CASE("should return error when putting nonexistent local file", "[put_get]") {
  // Given A stage is created
  Connection conn;
  const std::string stage = create_stage(conn, unique_stage_name("ODBCTST_PUT_ERR"));
  TempTestDir tmp("odbc_put_err_");
  const auto missing = tmp.path() / ("gone_" + random_hex() + ".csv");
  REQUIRE_FALSE(std::filesystem::exists(missing));

  // When PUT is executed with a path to a nonexistent local file
  std::string put_sql = "PUT 'file://" + as_file_uri(missing) + "' @" + stage;
  const auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar(put_sql.c_str()), SQL_NTS);

  // Then An error is raised indicating the local file does not exist
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasDiagMessage("does not exist"));
}

TEST_CASE("should return empty result set when getting nonexistent file from stage", "[put_get]") {
  // Given An empty stage is created
  Connection conn;
  const std::string stage = create_stage(conn, unique_stage_name("ODBCTST_GET_ERR"));
  TempTestDir download("odbc_get_err_");

  // When GET is executed for a file that does not exist in stage
  std::string get_sql =
      "GET @" + stage + "/nonexistent_" + random_hex() + ".csv 'file://" + as_file_uri(download.path()) + "/'";
  const auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar(get_sql.c_str()), SQL_NTS);

  // Then An empty result set is returned
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(SQLFetch(stmt.getHandle()) == SQL_NO_DATA);
}
