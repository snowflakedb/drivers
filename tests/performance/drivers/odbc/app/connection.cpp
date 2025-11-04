#include "connection.h"

#include <chrono>
#include <filesystem>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <sstream>

#include "config.h"

void check_odbc_error(SQLRETURN ret, SQLSMALLINT handle_type, SQLHANDLE handle,
                      const std::string& context) {
  if (ret != SQL_SUCCESS && ret != SQL_SUCCESS_WITH_INFO) {
    std::cerr << "ERROR: " << context << " failed with return code " << ret << "\n";

    // Attempt to retrieve diagnostic records
    SQLCHAR sql_state[6] = {0};
    SQLCHAR error_msg[SQL_MAX_MESSAGE_LENGTH] = {0};
    SQLINTEGER native_error = 0;
    SQLSMALLINT msg_len = 0;

    SQLSMALLINT rec_num = 1;
    while (true) {
      SQLRETURN diag_ret = SQLGetDiagRec(handle_type, handle, rec_num, sql_state, &native_error,
                                         error_msg, sizeof(error_msg), &msg_len);

      if (diag_ret == SQL_SUCCESS || diag_ret == SQL_SUCCESS_WITH_INFO) {
        std::cerr << "[Diagnostic Record " << rec_num << "]\n";
        std::cerr << "  SQLSTATE: " << sql_state << "\n";
        std::cerr << "  Native Error: " << native_error << "\n";
        std::cerr << "  Message: " << (msg_len > 0 ? (const char*)error_msg : "(empty)") << "\n";
        rec_num++;
      } else if (diag_ret == SQL_NO_DATA) {
        if (rec_num == 1) {
          std::cerr << "(No diagnostic records available - driver bug)\n";
        }
        break;
      } else {
        std::cerr << "SQLGetDiagRec failed with return code: " << diag_ret << "\n";
        break;
      }
    }

    exit(1);
  }
}

std::string write_private_key_to_file(const std::string& private_key) {
  // Write private key to a temporary file (OS-agnostic)
  std::filesystem::path temp_dir = std::filesystem::temp_directory_path();
  std::filesystem::path key_file_path = temp_dir / "perf_test_private_key.p8";
  std::ofstream key_file(key_file_path, std::ios::out | std::ios::trunc);

  if (!key_file.is_open()) {
    std::cerr << "ERROR: Failed to create temporary private key file: " << key_file_path << "\n";
    exit(1);
  }

  key_file << private_key;
  key_file.close();

  return key_file_path.string();
}

std::string get_connection_string() {
  std::string driver_path = get_driver_path();

  std::cout << "Using driver: " << driver_path << "\n";

  // Parse connection parameters from PARAMETERS_JSON
  auto params = parse_parameters_json();

  // Build connection string
  std::stringstream ss;
  ss << "DRIVER=" << driver_path << ";";

  // Required parameters
  std::string account = params["account"];
  std::string host = params["host"];
  std::string user = params["user"];
  std::string private_key = params["private_key"];

  if (account.empty() || user.empty() || private_key.empty()) {
    std::cerr << "ERROR: Missing required connection parameters in PARAMETERS_JSON\n";
    std::cerr << "Required: account, user, private_key\n";
    std::cerr << "Found: account=" << (account.empty() ? "MISSING" : "OK")
              << ", user=" << (user.empty() ? "MISSING" : "OK")
              << ", private_key=" << (private_key.empty() ? "MISSING" : "OK") << "\n";
    exit(1);
  }

  ss << "SERVER=" << host << ";";
  ss << "ACCOUNT=" << account << ";";
  ss << "UID=" << user << ";";

  // Use key-pair authentication
  // ODBC driver requires private key to be in a file
  std::string key_file_path = write_private_key_to_file(private_key);
  ss << "AUTHENTICATOR=SNOWFLAKE_JWT;";
  ss << "PRIV_KEY_FILE=" << key_file_path << ";";

  // Optional parameters
  if (!params["database"].empty()) ss << "DATABASE=" << params["database"] << ";";
  if (!params["schema"].empty()) ss << "SCHEMA=" << params["schema"] << ";";
  if (!params["warehouse"].empty()) ss << "WAREHOUSE=" << params["warehouse"] << ";";
  if (!params["role"].empty()) ss << "ROLE=" << params["role"] << ";";

  return ss.str();
}

SQLHENV create_environment() {
  SQLHENV env;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
  check_odbc_error(ret, SQL_HANDLE_ENV, env, "SQLAllocHandle ENV");

  ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
  check_odbc_error(ret, SQL_HANDLE_ENV, env, "SQLSetEnvAttr");

  return env;
}

SQLHDBC create_connection(SQLHENV env) {
  SQLHDBC dbc;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env, &dbc);
  check_odbc_error(ret, SQL_HANDLE_ENV, env, "SQLAllocHandle DBC");

  std::string conn_string = get_connection_string();
  ret = SQLDriverConnect(dbc, NULL, (SQLCHAR*)conn_string.c_str(), SQL_NTS, NULL, 0, NULL,
                         SQL_DRIVER_NOPROMPT);
  check_odbc_error(ret, SQL_HANDLE_DBC, dbc, "SQLDriverConnect");

  return dbc;
}

std::string get_driver_version(SQLHDBC dbc) {
  char driver_version[256] = {0};
  SQLSMALLINT version_len = 0;
  SQLRETURN ret =
      SQLGetInfo(dbc, SQL_DRIVER_VER, driver_version, sizeof(driver_version), &version_len);

  if (ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO) {
    return std::string(driver_version);
  }
  return "UNKNOWN";
}

std::string get_server_version(SQLHDBC dbc) {
  std::string server_version = "UNKNOWN";
  SQLHSTMT version_stmt;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc, &version_stmt);

  if (ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO) {
    ret = SQLExecDirect(version_stmt, (SQLCHAR*)"SELECT CURRENT_VERSION() AS VERSION", SQL_NTS);
    if (ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO) {
      ret = SQLFetch(version_stmt);
      if (ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO) {
        char version_buffer[256] = {0};
        SQLLEN indicator = 0;
        ret = SQLGetData(version_stmt, 1, SQL_C_CHAR, version_buffer, sizeof(version_buffer),
                         &indicator);
        if (ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO) {
          server_version = std::string(version_buffer);
        }
      }
    }
    SQLFreeHandle(SQL_HANDLE_STMT, version_stmt);
  }

  return server_version;
}

void execute_setup_queries(SQLHDBC dbc, const std::vector<std::string>& setup_queries) {
  if (setup_queries.empty()) {
    return;
  }

  std::cout << "\n=== Executing Setup Queries (" << setup_queries.size() << " queries) ===\n";
  for (size_t i = 0; i < setup_queries.size(); i++) {
    std::cout << "  Setup query " << (i + 1) << ": " << setup_queries[i] << "\n";

    SQLHSTMT stmt;
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt);
    check_odbc_error(ret, SQL_HANDLE_DBC, dbc, "SQLAllocHandle STMT");

    ret = SQLExecDirect(stmt, (SQLCHAR*)setup_queries[i].c_str(), SQL_NTS);
    check_odbc_error(ret, SQL_HANDLE_STMT, stmt, "Setup query execution");

    SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  }

  std::cout << "✓ Setup queries completed\n";
}
