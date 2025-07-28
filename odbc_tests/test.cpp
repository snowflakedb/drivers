#include <catch2/catch_test_macros.hpp>
#include <sql.h>
#include <sqltypes.h>
#include <sqlext.h>
#include <sstream>
#include <iostream>
#include <fstream>
#include <picojson.h>

std::string get_driver_path() {
    // DRIVER_PATH from environment variable
    const char* driver_path_env_value = std::getenv("DRIVER_PATH");
    REQUIRE(driver_path_env_value != nullptr);
    std::string driver_path = std::string(driver_path_env_value);
    std::cerr << "Driver path: " << driver_path << std::endl;
    return driver_path;
}

TEST_CASE("Test ODBC connection", "[odbc]") {
    SECTION("Test connection") {
        SQLHENV env;
        SQLHDBC dbc;
        SQLHSTMT stmt;

        SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, NULL, &env);
        REQUIRE(ret == SQL_SUCCESS);

        ret = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
        REQUIRE(ret == SQL_SUCCESS);

        ret = SQLAllocHandle(SQL_HANDLE_DBC, env, &dbc);
        if (ret != SQL_SUCCESS) {
            SQLINTEGER nativeError;
            SQLCHAR state[1024];
            SQLCHAR message[1024];
            SQLGetDiagRec(SQL_HANDLE_DBC, dbc, 1, state, &nativeError, message, sizeof(message), NULL);
            std::cerr << "Error: " << message << std::endl;
        }
        REQUIRE(ret == SQL_SUCCESS);

        // Get parameter path from environment variable
        const char* parameter_path_env_value = std::getenv("PARAMETER_PATH");
        REQUIRE(parameter_path_env_value != nullptr);
        std::string parameter_path = std::string(parameter_path_env_value);
        std::cerr << "Parameter path: " << parameter_path << std::endl;
        // Read parameters from parameters.json
        std::ifstream params_file(parameter_path);
        picojson::value params;
        std::cerr << "Reading parameters from " << parameter_path << std::endl;
        std::cerr << "File exists: " << std::ifstream(parameter_path).good() << std::endl;
        std::string err = picojson::parse(params, params_file);
        if (!err.empty()) {
            throw std::runtime_error("Failed to parse parameters.json: " + err);
        }
        std::cerr << "Parsed parameters: " << params.to_str() << std::endl;
        std::stringstream ss;
        ss << "DRIVER=" << get_driver_path() << ";";
        ss << "SERVER=" << params.get("testconnection").get("SNOWFLAKE_TEST_HOST").get<std::string>() << ";";
        ss << "ACCOUNT=" << params.get("testconnection").get("SNOWFLAKE_TEST_ACCOUNT").get<std::string>() << ";";
        ss << "UID=" << params.get("testconnection").get("SNOWFLAKE_TEST_USER").get<std::string>() << ";";
        ss << "PWD=" << params.get("testconnection").get("SNOWFLAKE_TEST_PASSWORD").get<std::string>() << ";";
        
        // Add optional parameters if specified
        if (params.get("testconnection").contains("SNOWFLAKE_TEST_PORT")) {
            double port_val = params.get("testconnection").get("SNOWFLAKE_TEST_PORT").get<double>();
            ss << "PORT=" << static_cast<int>(port_val) << ";";
        }
        if (params.get("testconnection").contains("SNOWFLAKE_TEST_PROTOCOL")) {
            ss << "PROTOCOL=" << params.get("testconnection").get("SNOWFLAKE_TEST_PROTOCOL").get<std::string>() << ";";
        }
        if (params.get("testconnection").contains("SNOWFLAKE_TEST_DATABASE")) {
            ss << "DATABASE=" << params.get("testconnection").get("SNOWFLAKE_TEST_DATABASE").get<std::string>() << ";";
        }

        std::cerr << "Connection string: " << ss.str() << std::endl;

        ret = SQLDriverConnect(dbc, NULL, (SQLCHAR*)ss.str().c_str(), SQL_NTS, NULL, 0, NULL, SQL_DRIVER_NOPROMPT);
        // Check for error code
        // if (ret != SQL_SUCCESS) {
        SQLINTEGER nativeError;
        SQLCHAR state[1024];
        SQLCHAR message[1024];
        SQLGetDiagRec(SQL_HANDLE_DBC, dbc, 1, state, &nativeError, message, sizeof(message), NULL);
        std::cerr << "Driver connect: " << "Status: " << ret << " Error: " << message << " State: " << state << std::endl;
        REQUIRE(((ret == (SQLRETURN)SQL_SUCCESS_WITH_INFO) || (ret == (SQLRETURN)SQL_SUCCESS)));

        ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt);
        REQUIRE(ret == SQL_SUCCESS);

        ret = SQLExecDirect(stmt, (SQLCHAR*)"SELECT 1", SQL_NTS);
        SQLGetDiagRec(SQL_HANDLE_STMT, stmt, 1, state, &nativeError, message, sizeof(message), NULL);
        std::cerr << "Exec direct: " << "Status: " << ret << " Error: " << message << " State: " << state << std::endl;
        REQUIRE(((ret == SQL_SUCCESS_WITH_INFO) || (ret == SQL_SUCCESS)));

        SQLSMALLINT num_cols;
        ret = SQLNumResultCols(stmt, &num_cols);
        SQLGetDiagRec(SQL_HANDLE_STMT, stmt, 1, state, &nativeError, message, sizeof(message), NULL);
        std::cerr << "Exec direct: " << "Status: " << ret << " Error: " << message << " State: " << state << std::endl;
        REQUIRE(ret == SQL_SUCCESS);
        REQUIRE(num_cols == 1);

        // Get the result
        ret = SQLFetch(stmt);
        SQLGetDiagRec(SQL_HANDLE_STMT, stmt, 1, state, &nativeError, message, sizeof(message), NULL);
        std::cerr << "SQLFetch: " << "Status: " << ret << " Error: " << message << " State: " << state << std::endl;
        REQUIRE(ret == SQL_SUCCESS);

        // Test if the result is 1
        SQLINTEGER result = 0;
        ret = SQLGetData(stmt, 1, SQL_C_LONG, &result, sizeof(result), NULL);
        SQLGetDiagRec(SQL_HANDLE_STMT, stmt, 1, state, &nativeError, message, sizeof(message), NULL);
        std::cerr << "SQLGetData: " << "Status: " << ret << " Error: " << message << " State: " << state << std::endl;
        REQUIRE(result == 1);

        SQLFreeHandle(SQL_HANDLE_STMT, stmt);
        SQLDisconnect(dbc);
        SQLFreeHandle(SQL_HANDLE_DBC, dbc);
        SQLFreeHandle(SQL_HANDLE_ENV, env);
    }
}
