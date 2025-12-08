#include <sql.h>
#include <sqlext.h>

#include <chrono>
#include <cstdlib>
#include <fstream>
#include <iostream>
#include <sstream>
#include <string>
#include <vector>

namespace {
struct ConnectionParams {
    std::string account;
    std::string user;
    std::string password;
    std::string database;
    std::string schema;
    std::string warehouse;
    std::string role;
};

bool loadParameters(const std::string& path, ConnectionParams& params) {
    std::ifstream file(path);
    if (!file.is_open()) {
        std::cerr << "ERROR|message=Unable to open parameters file: " << path << "\n";
        return false;
    }

    std::string line;
    bool inTestConnection = false;
    auto extractValue = [](const std::string& src) -> std::string {
        size_t start = src.find(": \"");
        if (start == std::string::npos) {
            return "";
        }
        start += 3;
        size_t end = src.find("\"", start);
        if (end == std::string::npos) {
            return "";
        }
        return src.substr(start, end - start);
    };

    while (std::getline(file, line)) {
        if (line.find("\"testconnection\"") != std::string::npos) {
            inTestConnection = true;
            continue;
        }
        if (!inTestConnection) {
            continue;
        }
        if (line.find('}') != std::string::npos && line.find("\"SNOWFLAKE") == std::string::npos) {
            break;
        }
        if (line.find("\"SNOWFLAKE_TEST_ACCOUNT\"") != std::string::npos) {
            params.account = extractValue(line);
        } else if (line.find("\"SNOWFLAKE_TEST_USER\"") != std::string::npos) {
            params.user = extractValue(line);
        } else if (line.find("\"SNOWFLAKE_TEST_PASSWORD\"") != std::string::npos) {
            params.password = extractValue(line);
        } else if (line.find("\"SNOWFLAKE_TEST_DATABASE\"") != std::string::npos) {
            params.database = extractValue(line);
        } else if (line.find("\"SNOWFLAKE_TEST_SCHEMA\"") != std::string::npos) {
            params.schema = extractValue(line);
        } else if (line.find("\"SNOWFLAKE_TEST_WAREHOUSE\"") != std::string::npos) {
            params.warehouse = extractValue(line);
        } else if (line.find("\"SNOWFLAKE_TEST_ROLE\"") != std::string::npos) {
            params.role = extractValue(line);
        }
    }
    return !params.account.empty() && !params.user.empty() && !params.password.empty();
}

void logDiagnostics(SQLSMALLINT handleType, SQLHANDLE handle, const std::string& scenario) {
    SQLCHAR sqlState[6] = {0};
    SQLINTEGER nativeError = 0;
    SQLCHAR message[SQL_MAX_MESSAGE_LENGTH] = {0};
    SQLSMALLINT textLength = 0;
    SQLSMALLINT rec = 1;

    while (SQLGetDiagRec(handleType, handle, rec, sqlState, &nativeError, message,
                         sizeof(message), &textLength) == SQL_SUCCESS) {
        std::cout << "DIAG|scenario=" << scenario << "|rec=" << rec << "|state=" << sqlState
                  << "|native=" << nativeError << "|message=" << message << "\n";
        ++rec;
    }
}

struct Scenario {
    std::string name;
    std::string sql;
    bool expectError;
};

std::vector<Scenario> buildScenarios(const std::string& tempTable) {
    return {
        // SQL compilation errors (42xxx)
        {"missing_table", "SELECT * FROM DOES_NOT_EXIST_TABLE", true},
        {"invalid_column", "SELECT missing_col FROM " + tempTable, true},
        {"syntax_error", "SELECT FROM " + tempTable, true},
        {"ambiguous_column", "SELECT id FROM " + tempTable + " a, " + tempTable + " b", true},
        {"invalid_function", "SELECT NONEXISTENT_FUNC(id) FROM " + tempTable, true},
        
        // Numeric errors (22xxx)
        {"division_by_zero", "SELECT 1/0 FROM " + tempTable, true},
        {"numeric_overflow", "SELECT 9999999999999999999999999999999999999999::NUMBER(10,0)", true},
        {"invalid_cast", "SELECT 'not_a_number'::NUMBER", true},
        {"invalid_date", "SELECT '2023-13-45'::DATE", true},
        
        // Transaction errors
        {"invalid_isolation", "SET TRANSACTION ISOLATION LEVEL INVALID", true},
        
        // Successful queries for baseline
        {"successful_query", "SELECT COUNT(*) FROM " + tempTable, false},
    };
}

}  // namespace

int main(int argc, char** argv) {
    if (argc < 2) {
        std::cerr << "Usage: " << argv[0] << " <path_to_parameters.json>\n";
        return EXIT_FAILURE;
    }
    const char* driverPath = std::getenv("DRIVER_PATH");
    if (!driverPath) {
        std::cerr << "ERROR|message=DRIVER_PATH not set\n";
        return EXIT_FAILURE;
    }

    ConnectionParams params;
    if (!loadParameters(argv[1], params)) {
        std::cerr << "ERROR|message=Failed to load connection parameters\n";
        return EXIT_FAILURE;
    }

    SQLHENV env = nullptr;
    SQLHDBC dbc = nullptr;
    SQLHSTMT stmt = nullptr;

    if (SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env) != SQL_SUCCESS) {
        std::cerr << "ERROR|message=Failed to allocate environment handle\n";
        return EXIT_FAILURE;
    }

    SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, reinterpret_cast<SQLPOINTER>(SQL_OV_ODBC3), 0);
    if (SQLAllocHandle(SQL_HANDLE_DBC, env, &dbc) != SQL_SUCCESS) {
        std::cerr << "ERROR|message=Failed to allocate connection handle\n";
        SQLFreeHandle(SQL_HANDLE_ENV, env);
        return EXIT_FAILURE;
    }

    std::stringstream connStr;
    connStr << "DRIVER=" << driverPath << ";"
            << "SERVER=" << params.account << ".snowflakecomputing.com;"
            << "ACCOUNT=" << params.account << ";"
            << "UID=" << params.user << ";"
            << "PWD=" << params.password << ";"
            << "DATABASE=" << params.database << ";"
            << "SCHEMA=" << params.schema << ";"
            << "WAREHOUSE=" << params.warehouse << ";"
            << "ROLE=" << params.role;

    SQLCHAR outConn[1024];
    SQLSMALLINT outConnLen = 0;
    std::cerr << "DEBUG|message=Attempting connection with: " << connStr.str() << "\n";
    SQLRETURN rc = SQLDriverConnect(dbc, nullptr,
                                    (SQLCHAR*)connStr.str().c_str(),
                                    SQL_NTS,
                                    outConn,
                                    sizeof(outConn),
                                    &outConnLen,
                                    SQL_DRIVER_NOPROMPT);
    std::cerr << "DEBUG|message=SQLDriverConnect returned: " << rc << "\n";
    if (!SQL_SUCCEEDED(rc)) {
        std::cerr << "ERROR|message=SQLDriverConnect failed\n";
        logDiagnostics(SQL_HANDLE_DBC, dbc, "connect");
        logDiagnostics(SQL_HANDLE_ENV, env, "connect_env");
        SQLFreeHandle(SQL_HANDLE_DBC, dbc);
        SQLFreeHandle(SQL_HANDLE_ENV, env);
        return EXIT_FAILURE;
    }

    SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt);

    const std::string tempTable = "TEMP_DIAG_TABLE";
    const std::string create_sql =
        "CREATE OR REPLACE TEMP TABLE " + tempTable + " (id INT, val VARCHAR)";
    if (!SQL_SUCCEEDED(SQLExecDirect(stmt, (SQLCHAR*)create_sql.c_str(), SQL_NTS))) {
        std::cerr << "ERROR|message=Failed to create temp table\n";
        logDiagnostics(SQL_HANDLE_STMT, stmt, "create_table");
        SQLFreeHandle(SQL_HANDLE_STMT, stmt);
        SQLDisconnect(dbc);
        SQLFreeHandle(SQL_HANDLE_DBC, dbc);
        SQLFreeHandle(SQL_HANDLE_ENV, env);
        return EXIT_FAILURE;
    }

    const std::string insert_sql =
        "INSERT INTO " + tempTable + " VALUES (1,'a'),(2,'b'),(3,'c')";
    if (!SQL_SUCCEEDED(SQLExecDirect(stmt, (SQLCHAR*)insert_sql.c_str(), SQL_NTS))) {
        std::cerr << "ERROR|message=Failed to seed temp table\n";
        logDiagnostics(SQL_HANDLE_STMT, stmt, "seed_table");
        SQLFreeHandle(SQL_HANDLE_STMT, stmt);
        SQLDisconnect(dbc);
        SQLFreeHandle(SQL_HANDLE_DBC, dbc);
        SQLFreeHandle(SQL_HANDLE_ENV, env);
        return EXIT_FAILURE;
    }

    auto scenarios = buildScenarios(tempTable);
    for (const auto& scenario : scenarios) {
        auto start = std::chrono::steady_clock::now();
        SQLRETURN result =
            SQLExecDirect(stmt, (SQLCHAR*)scenario.sql.c_str(), (SQLINTEGER)scenario.sql.size());
        bool success = SQL_SUCCEEDED(result);
        long long duration = std::chrono::duration_cast<std::chrono::milliseconds>(
                                 std::chrono::steady_clock::now() - start)
                                 .count();

        std::cout << "SCENARIO|name=" << scenario.name << "|success=" << (success ? "1" : "0")
                  << "|duration_ms=" << duration << "\n";

        if (scenario.expectError) {
            if (success) {
                std::cout << "ERROR|message=Scenario expected error but succeeded|scenario="
                          << scenario.name << "\n";
            } else {
                logDiagnostics(SQL_HANDLE_STMT, stmt, scenario.name);
            }
        } else if (!success) {
            logDiagnostics(SQL_HANDLE_STMT, stmt, scenario.name);
        }
        SQLCloseCursor(stmt);
    }

    SQLFreeHandle(SQL_HANDLE_STMT, stmt);
    SQLDisconnect(dbc);
    SQLFreeHandle(SQL_HANDLE_DBC, dbc);
    SQLFreeHandle(SQL_HANDLE_ENV, env);
    return EXIT_SUCCESS;
}

