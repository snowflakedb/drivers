#include <sql.h>
#include <sqlext.h>

#include <filesystem>
#include <string>
#include <utility>
#include <vector>

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_all.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"
#include "put_get_utils.hpp"
#include "test_setup.hpp"

using Catch::Matchers::ContainsSubstring;
using pg_utils::TempTestDir;

namespace {

std::string joined_text_for_state(const std::vector<DiagRec>& records, const std::string& sql_state) {
  std::string text;
  for (const auto& rec : records) {
    if (rec.sqlState == sql_state) {
      text += rec.messageText;
      text += '\n';
    }
  }
  return text;
}

}  // namespace

TEST_CASE("should warn on deprecated connection-string logging keys only in the new driver",
          "[logging][connection][BD#154]") {
  // Given a live connection string plus leftover 3.x logging keys. The new driver
  //   ignores both key and value; LogLevel=OFF leaves the old driver's file logger
  //   off during reference runs.
  TempTestDir tmp("odbc_deprecated_dsn_");
  const std::string conn_str =
      get_connection_string() + "LogLevel=OFF;LogPath=" + tmp.path().string() +
      ";LogFileSize=10;LogFileCount=2;CURLVerboseMode=false;EnablePidLogFileNames=false;CLIENT_CONFIG_FILE=" +
      (tmp.path() / "sf.json").string() + ";";

  // When SQLDriverConnect runs with those keys
  auto env = Connection::initEnv();
  auto dbc = env.createConnectionHandle();
  const SQLRETURN ret = SQLDriverConnect(dbc.getHandle(), nullptr, sqlchar(conn_str.c_str()), SQL_NTS, nullptr, 0,
                                         nullptr, SQL_DRIVER_NOPROMPT);
  ConnectedConnectionWrapper connected(dbc.getHandle());
  dbc.release();
  const auto connect_result = OdbcResult(ret, SQL_HANDLE_DBC, connected.getHandle());
  const auto records = get_diag_rec(SQL_HANDLE_DBC, connected.getHandle());

  NEW_DRIVER_ONLY("BD#154") {
    REQUIRE_THAT(connect_result, OdbcMatchers::IsSuccessWithInfo());
    // iODBC's DM drops the driver's SQLDriverConnect SUCCESS_WITH_INFO record
    // (BD#61); the 01000 text is asserted on unixODBC / Windows.
    NON_IODBC {
      REQUIRE_THAT(connect_result, OdbcMatchers::HasSqlState("01000"));
      const std::string warnings = joined_text_for_state(records, "01000");
      const std::vector<std::pair<const char*, const char*>> expected_guidance = {
          {"LOGLEVEL", "Set LogLevel in sf.odbc.ini"},
          {"LOGPATH", "Set LogPath in sf.odbc.ini"},
          {"LOGFILESIZE", "Set LogMaxSize in sf.odbc.ini"},
          {"LOGFILECOUNT", "Set LogMaxCount in sf.odbc.ini"},
          {"CURLVERBOSEMODE", "Set LogLevel=DEBUG in sf.odbc.ini"},
          {"ENABLEPIDLOGFILENAMES", "Set LogFile in sf.odbc.ini"},
          {"CLIENT_CONFIG_FILE", "Configure driver logging in sf.odbc.ini"},
      };
      for (const auto& [key, guidance] : expected_guidance) {
        CHECK_THAT(warnings, ContainsSubstring(std::string("Parameter '") + key +
                                               "' is deprecated and has no effect. " + guidance));
      }
    }
  }
  OLD_DRIVER_ONLY("BD#154") {
    // The old driver accepts the six logging keys, but not CLIENT_CONFIG_FILE: it reads that key
    // and still reports it as invalid. The count in the text pins the other six as accepted.
    REQUIRE_THAT(connect_result, OdbcMatchers::IsSuccessWithInfo());
    NON_IODBC {
      REQUIRE_THAT(connect_result, OdbcMatchers::HasSqlState("01S00"));
      CHECK_THAT(joined_text_for_state(records, "01S00"),
                 ContainsSubstring("1 invalid keys are found in the connection string: CLIENT_CONFIG_FILE"));
    }
    CHECK_THAT(joined_text_for_state(records, "01000"), !ContainsSubstring("is deprecated"));
  }

  // Then the session is usable
  auto stmt = StatementHandleWrapper(connected.getHandle(), SQL_HANDLE_STMT);
  std::string query = "SELECT 1";
  const SQLRETURN exec = SQLExecDirect(stmt.getHandle(), sqlchar(query.c_str()), SQL_NTS);
  REQUIRE_ODBC(exec, stmt);
  const SQLRETURN fetched = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(fetched, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "1");
}
