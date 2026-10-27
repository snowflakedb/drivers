#include <sql.h>
#include <sqlext.h>

#include <cstddef>
#include <filesystem>
#include <fstream>
#include <sstream>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "EnvOverride.hpp"
#include "HandleWrapper.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"
#include "put_get_utils.hpp"
#include "test_setup.hpp"

namespace fs = std::filesystem;
using pg_utils::TempTestDir;

namespace {
size_t wide_len(const SQLWCHAR* s, size_t max_units) {
  size_t n = 0;
  while (n < max_units && s[n] != 0) {
    ++n;
  }
  return n;
}

std::string ascii_from_wide(const SQLWCHAR* s, size_t units) {
  std::string out;
  out.reserve(units);
  for (size_t i = 0; i < units; ++i) {
    out.push_back(static_cast<char>(s[i]));
  }
  return out;
}

std::string read_logs(const fs::path& dir) {
  std::ostringstream combined;
  for (const auto& entry : fs::directory_iterator(dir)) {
    if (!entry.is_regular_file() || entry.path().filename() == "sf.odbc.ini") {
      continue;
    }
    std::ifstream in(entry.path(), std::ios::binary);
    combined << in.rdbuf();
  }
  return combined.str();
}

// `state` has room for a UTF-32 write of the 6-character SQLSTATE. A UTF-16
// write leaves the unit past the NUL as the sentinel.
void require_utf16_diag(SQLSMALLINT handle_type, SQLHANDLE handle, const std::string& expected_state) {
  std::vector<SQLWCHAR> state(12, static_cast<SQLWCHAR>(0xFFFF));
  SQLINTEGER native = 0;
  SQLSMALLINT full_len = 0;
  SQLRETURN ret = SQLGetDiagRecW(handle_type, handle, 1, state.data(), &native, nullptr, 0, &full_len);
  REQUIRE(ret != SQL_ERROR);
  REQUIRE(full_len > 1);
  REQUIRE(state[6] == static_cast<SQLWCHAR>(0xFFFF));
  REQUIRE(wide_len(state.data(), state.size()) == expected_state.size());
  REQUIRE(ascii_from_wide(state.data(), expected_state.size()) == expected_state);

  std::vector<SQLWCHAR> msg(static_cast<size_t>(full_len) + 1 + 8, static_cast<SQLWCHAR>(0xFFFF));
  SQLSMALLINT msg_len = 0;
  const auto buf_len = static_cast<SQLSMALLINT>(full_len + 1);
  ret = SQLGetDiagRecW(handle_type, handle, 1, state.data(), &native, msg.data(), buf_len, &msg_len);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(state[6] == static_cast<SQLWCHAR>(0xFFFF));
  CHECK(ascii_from_wide(state.data(), expected_state.size()) == expected_state);
  CHECK(msg_len == full_len);
  CHECK(wide_len(msg.data(), msg.size()) == static_cast<size_t>(msg_len));
  CHECK(msg[static_cast<size_t>(full_len) + 1] == static_cast<SQLWCHAR>(0xFFFF));
}
}  // namespace

TEST_CASE("DriverManagerEncoding=UTF-32 on Windows: SQLGetDiagRecW fits the driver manager's 2-byte SQLWCHAR",
          "[e2e][session][encoding]") {
  SKIP_NON_WINDOWS("The Windows driver manager is the only one whose SQLWCHAR width cannot be UTF-32");

  // Given an sf.odbc.ini that sets UTF-32 and a log file, before the first environment exists
  TempTestDir ini_dir("odbc_dm_encoding_ini_");
  auto ini_path = ini_dir.path() / "sf.odbc.ini";
  {
    std::ofstream ini(ini_path, std::ios::trunc);
    REQUIRE(ini.good());
    ini << "DriverManagerEncoding=UTF-32\n";
    ini << "LogLevel=INFO\n";
    ini << "LogPath=" << ini_dir.path().string() << "\n";
    ini << "LogFile=driver_manager_encoding.log\n";
  }
  EnvOverride env_override("SF_ODBC_INI", ini_path.string());

  EnvironmentHandleWrapper env;
  SQLRETURN ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHDBC raw_dbc = SQL_NULL_HDBC;
  REQUIRE(SQLAllocHandle(SQL_HANDLE_DBC, env.getHandle(), &raw_dbc) == SQL_SUCCESS);
  ConnectedConnectionWrapper dbc(raw_dbc);
  const std::string conn_str = get_connection_string();
  ret =
      SQLDriverConnect(raw_dbc, nullptr, sqlchar(conn_str.c_str()), SQL_NTS, nullptr, 0, nullptr, SQL_DRIVER_NOPROMPT);
  const bool connected = ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO;

  // When SQLGetDiagRecW reads the diagnostic into a driver-manager-sized buffer
  if (connected) {
    StatementHandleWrapper stmt(raw_dbc, SQL_HANDLE_STMT);
    ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT * FROM dm_encoding_table_does_not_exist"), SQL_NTS);
    REQUIRE_EXPECTED_ERROR(ret, "42S02", stmt.getHandle(), SQL_HANDLE_STMT);
    // Then the SQLSTATE is 42S02 and the write fits a 2-byte SQLWCHAR
    require_utf16_diag(SQL_HANDLE_STMT, stmt.getHandle(), "42S02");
  } else {
    // Connect did not succeed, so its diagnostic is the wide-string read under test.
    REQUIRE(ret == SQL_ERROR);
    const std::string state = get_sqlstate(SQL_HANDLE_DBC, raw_dbc);
    REQUIRE(state.size() == 5);
    // Then that SQLSTATE is whole UTF-16 text in a 2-byte SQLWCHAR
    require_utf16_diag(SQL_HANDLE_DBC, raw_dbc, state);
  }

  // The new driver logs the substitution only when this ini was the one negotiated.
  if (get_driver_type() == DRIVER_TYPE::NEW) {
    const std::string log = read_logs(ini_dir.path());
    REQUIRE(log.find("ignored on Windows") != std::string::npos);
  }
}
