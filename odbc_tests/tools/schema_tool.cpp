// Standalone utility for managing temporary test schemas via ODBC.
//
// Subcommands:
//   create                  Create a new TEMP_TEST_SCHEMA_<random>, print name to stdout.
//   drop <name>             Drop a specific schema (validates name format).
//   cleanup [--age-days N]  Drop orphaned TEMP_TEST_SCHEMA_* schemas older than N days (default 2).
//
// Uses DataSourceConfig::Snowflake() from the common library for connection
// configuration, and raw ODBC API calls to avoid any Catch2 dependency at runtime.
// Links against the common static library (Catch2's main() is not pulled in
// because this translation unit already defines main()).

#include <sql.h>
#include <sqlext.h>

#include <iostream>
#include <optional>
#include <random>
#include <regex>
#include <string>
#include <vector>

#include "ODBCConfig.hpp"
#include "odbc_cast.hpp"

static constexpr auto TAG = "schema_tool";
static const std::regex SCHEMA_NAME_RE("^TEMP_TEST_SCHEMA_[0-9]+$");

struct OdbcConnection {
  SQLHENV env = SQL_NULL_HENV;
  SQLHDBC dbc = SQL_NULL_HDBC;

  OdbcConnection(const OdbcConnection&) = delete;
  OdbcConnection& operator=(const OdbcConnection&) = delete;

  OdbcConnection() = default;

  ~OdbcConnection() {
    if (dbc != SQL_NULL_HDBC) {
      SQLDisconnect(dbc);
      SQLFreeHandle(SQL_HANDLE_DBC, dbc);
    }
    if (env != SQL_NULL_HENV) {
      SQLFreeHandle(SQL_HANDLE_ENV, env);
    }
  }

  bool connect(const std::string& conn_str) {
    if (!SQL_SUCCEEDED(SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env))) {
      std::cerr << TAG << ": SQLAllocHandle(ENV) failed\n";
      return false;
    }
    SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);

    if (!SQL_SUCCEEDED(SQLAllocHandle(SQL_HANDLE_DBC, env, &dbc))) {
      std::cerr << TAG << ": SQLAllocHandle(DBC) failed\n";
      return false;
    }

    const SQLRETURN ret =
        SQLDriverConnect(dbc, nullptr, sqlchar(conn_str.c_str()), SQL_NTS, nullptr, 0, nullptr, SQL_DRIVER_NOPROMPT);
    if (!SQL_SUCCEEDED(ret)) {
      std::cerr << TAG << ": SQLDriverConnect failed\n";
      return false;
    }
    return true;
  }

  [[nodiscard]] bool exec(const std::string& sql) const {
    SQLHSTMT stmt = SQL_NULL_HSTMT;
    if (!SQL_SUCCEEDED(SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt))) {
      std::cerr << TAG << ": SQLAllocHandle(STMT) failed for: " << sql << "\n";
      return false;
    }
    const SQLRETURN ret = SQLExecDirect(stmt, sqlchar(sql.c_str()), SQL_NTS);
    const bool ok = SQL_SUCCEEDED(ret);
    if (!ok) {
      std::cerr << TAG << ": SQLExecDirect failed for: " << sql << "\n";
    }
    SQLFreeHandle(SQL_HANDLE_STMT, stmt);
    return ok;
  }

  [[nodiscard]] std::vector<std::string> query_column(const std::string& sql) const {
    std::vector<std::string> results;
    SQLHSTMT stmt = SQL_NULL_HSTMT;
    if (!SQL_SUCCEEDED(SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt))) {
      std::cerr << TAG << ": SQLAllocHandle(STMT) failed for: " << sql << "\n";
      return results;
    }
    if (!SQL_SUCCEEDED(SQLExecDirect(stmt, sqlchar(sql.c_str()), SQL_NTS))) {
      std::cerr << TAG << ": SQLExecDirect failed for: " << sql << "\n";
      SQLFreeHandle(SQL_HANDLE_STMT, stmt);
      return results;
    }
    char buf[512];
    while (SQL_SUCCEEDED(SQLFetch(stmt))) {
      SQLLEN indicator = 0;
      if (SQL_SUCCEEDED(SQLGetData(stmt, 1, SQL_C_CHAR, buf, sizeof(buf), &indicator))) {
        if (indicator != SQL_NULL_DATA) {
          results.emplace_back(buf);
        }
      }
    }
    SQLFreeHandle(SQL_HANDLE_STMT, stmt);
    return results;
  }
};

// Opens a connection using DataSourceConfig::Snowflake().  The returned
// ConfigInstallation keeps odbc.ini/odbcinst.ini alive for the connection's
// lifetime — callers must hold onto it until they are done with `conn`.
static std::optional<ConfigInstallation> open_connection(OdbcConnection& conn) {
  try {
    auto dsn_config = DataSourceConfig::Snowflake();
    auto installation = dsn_config.install();
    if (!conn.connect(dsn_config.connection_string())) {
      return std::nullopt;
    }
    return installation;
  } catch (const std::exception& e) {
    std::cerr << TAG << ": " << e.what() << "\n";
    return std::nullopt;
  }
}

static std::string generate_schema_name() {
  std::random_device rd;
  std::mt19937_64 gen(rd());
  return "TEMP_TEST_SCHEMA_" + std::to_string(gen());
}

static int cmd_create() {
  const std::string name = generate_schema_name();

  OdbcConnection conn;
  if (const auto installation = open_connection(conn); !installation) {
    return 1;
  }
  if (!conn.exec("CREATE SCHEMA IF NOT EXISTS " + name)) {
    return 1;
  }
  std::cerr << TAG << ": created schema " << name << "\n";
  std::cout << name;
  return 0;
}

static int cmd_drop(const std::string& name) {
  if (!std::regex_match(name, SCHEMA_NAME_RE)) {
    std::cerr << TAG << ": refusing to drop schema with unexpected name: " << name << "\n";
    return 1;
  }

  OdbcConnection conn;
  auto installation = open_connection(conn);
  if (!installation) {
    return 1;
  }
  if (!conn.exec("DROP SCHEMA IF EXISTS " + name + " CASCADE")) {
    return 1;
  }
  std::cerr << TAG << ": dropped schema " << name << "\n";
  return 0;
}

static int cmd_cleanup(const int age_days) {
  OdbcConnection conn;
  auto installation = open_connection(conn);
  if (!installation) {
    return 1;
  }

  std::string query =
      "SELECT SCHEMA_NAME FROM INFORMATION_SCHEMA.SCHEMATA "
      "WHERE SCHEMA_NAME LIKE 'TEMP_TEST_SCHEMA_%' "
      "AND CREATED < DATEADD(day, -" +
      std::to_string(age_days) +
      ", CURRENT_TIMESTAMP()) "
      "ORDER BY CREATED";

  std::cerr << TAG << ": looking for TEMP_TEST_SCHEMA_% older than " << age_days << " days\n";

  auto schemas = conn.query_column(query);
  if (schemas.empty()) {
    std::cerr << TAG << ": no orphaned schemas found\n";
    return 0;
  }

  int count = 0;
  for (const auto& schema : schemas) {
    if (!std::regex_match(schema, SCHEMA_NAME_RE)) {
      std::cerr << TAG << ": skipping unexpected schema name: " << schema << "\n";
      continue;
    }
    std::cerr << TAG << ": dropping " << schema << "\n";
    if (!conn.exec("DROP SCHEMA IF EXISTS " + schema + " CASCADE")) {
      std::cerr << TAG << ": failed to drop schema: " << schema << "\n";
    }
    ++count;
  }

  std::cerr << TAG << ": dropped " << count << " orphaned schema(s)\n";
  return 0;
}

static void usage() {
  std::cerr << "Usage:\n"
            << "  schema_tool create\n"
            << "  schema_tool drop <SCHEMA_NAME>\n"
            << "  schema_tool cleanup [--age-days N]\n";
}

int main(const int argc, char* argv[]) {
  if (argc < 2) {
    usage();
    return 1;
  }

  const std::string cmd = argv[1];

  if (cmd == "create") {
    return cmd_create();
  }

  if (cmd == "drop") {
    if (argc < 3) {
      std::cerr << TAG << ": 'drop' requires a schema name argument\n";
      usage();
      return 1;
    }
    return cmd_drop(argv[2]);
  }

  if (cmd == "cleanup") {
    int age_days = 2;
    for (int i = 2; i < argc - 1; ++i) {
      if (std::strcmp(argv[i], "--age-days") == 0) {
        try {
          age_days = std::stoi(argv[i + 1]);
          if (age_days < 0) {
            age_days = 2;
          }
        } catch (...) {
          std::cerr << TAG << ": invalid --age-days value, using default 2\n";
          age_days = 2;
        }
        break;
      }
    }
    return cmd_cleanup(age_days);
  }

  std::cerr << TAG << ": unknown command '" << cmd << "'\n";
  usage();
  return 1;
}
