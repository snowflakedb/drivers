#include <sql.h>
#include <sqlext.h>

#include <filesystem>
#include <fstream>
#include <iostream>
#include <sstream>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "HandleWrapper.hpp"
#include "test_setup.hpp"
#include "utils.hpp"

static std::string strip_comments_and_trim(const std::string& raw) {
  std::istringstream stream(raw);
  std::string line;
  std::string result;

  while (std::getline(stream, line)) {
    size_t start = line.find_first_not_of(" \t\r");
    if (start == std::string::npos) continue;
    if (line.size() >= start + 2 && line[start] == '-' && line[start + 1] == '-') continue;
    if (!result.empty()) result += ' ';
    result += line.substr(start);
  }

  while (!result.empty() && (result.back() == ' ' || result.back() == '\t' || result.back() == '\r')) {
    result.pop_back();
  }
  return result;
}

static std::vector<std::string> parse_sql_statements(const std::string& sql) {
  std::vector<std::string> statements;
  std::string current;
  bool in_single_quote = false;

  for (size_t i = 0; i < sql.size(); ++i) {
    char c = sql[i];

    if (c == '\'' && (i == 0 || sql[i - 1] != '\\')) {
      in_single_quote = !in_single_quote;
    }

    if (c == ';' && !in_single_quote) {
      std::string cleaned = strip_comments_and_trim(current);
      if (!cleaned.empty()) {
        statements.push_back(cleaned);
      }
      current.clear();
    } else {
      current += c;
    }
  }

  std::string trailing = strip_comments_and_trim(current);
  if (!trailing.empty()) {
    statements.push_back(trailing);
  }

  return statements;
}

static std::string get_odbc_diagnostics(SQLSMALLINT handle_type, SQLHANDLE handle) {
  std::string diag;
  SQLCHAR state[8];
  SQLINTEGER native_error;
  SQLCHAR message[1024];
  SQLSMALLINT msg_len;

  for (SQLSMALLINT rec = 1;; ++rec) {
    SQLRETURN dr = SQLGetDiagRec(handle_type, handle, rec, state, &native_error, message, sizeof(message), &msg_len);
    if (dr == SQL_NO_DATA) break;
    if (!diag.empty()) diag += " | ";
    diag += "SQLSTATE=" + std::string(reinterpret_cast<char*>(state)) + " NativeError=" + std::to_string(native_error) +
            " " + std::string(reinterpret_cast<char*>(message), msg_len);
  }
  return diag;
}

TEST_CASE("Setup readonly metadata test database", "[setup]") {
  const auto sql_path = test_utils::repo_root() / "scripts" / "odbc" / "setup_readonly_metadata_db.sql";

  REQUIRE(std::filesystem::exists(sql_path));

  std::ifstream file(sql_path);
  REQUIRE(file.good());

  std::stringstream buffer;
  buffer << file.rdbuf();
  const std::string sql_contents = buffer.str();

  auto statements = parse_sql_statements(sql_contents);
  REQUIRE(!statements.empty());

  // Build the connection string first — this triggers driver registration
  // and sets ODBCSYSINI before any ODBC handles are allocated.
  std::string conn_str = get_connection_string();

  EnvironmentHandleWrapper env;
  SQLRETURN ret = SQLSetEnvAttr(env.getHandle(), SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
  if (!SQL_SUCCEEDED(ret)) {
    FAIL("SQLSetEnvAttr failed: " << get_odbc_diagnostics(SQL_HANDLE_ENV, env.getHandle()));
  }
  ConnectionHandleWrapper dbc = env.createConnectionHandle();
  ret = SQLDriverConnect(dbc.getHandle(), nullptr, (SQLCHAR*)conn_str.c_str(), SQL_NTS, nullptr, 0, nullptr,
                         SQL_DRIVER_NOPROMPT);
  if (!SQL_SUCCEEDED(ret)) {
    FAIL("SQLDriverConnect failed: " << get_odbc_diagnostics(SQL_HANDLE_DBC, dbc.getHandle()));
  }

  int success_count = 0;
  int total = static_cast<int>(statements.size());

  for (int i = 0; i < total; ++i) {
    const auto& stmt_sql = statements[i];

    StatementHandleWrapper stmt = dbc.createStatementHandle();
    ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)stmt_sql.c_str(), SQL_NTS);

    if (!SQL_SUCCEEDED(ret)) {
      std::string diag = get_odbc_diagnostics(SQL_HANDLE_STMT, stmt.getHandle());
      FAIL("Statement " << (i + 1) << "/" << total << " failed: " << stmt_sql << "\n  " << diag);
    }

    success_count++;
    std::cout << "[OK] (" << (i + 1) << "/" << total << ") " << stmt_sql << "\n";
  }

  SQLDisconnect(dbc.getHandle());
  std::cout << "\n=== Setup complete: " << success_count << "/" << total << " statements executed successfully ===\n";
}
