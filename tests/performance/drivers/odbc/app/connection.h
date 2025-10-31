#pragma once

#include <sql.h>
#include <sqlext.h>

#include <string>
#include <vector>

void check_odbc_error(SQLRETURN ret, SQLSMALLINT handle_type, SQLHANDLE handle,
                      const std::string& context);
std::string get_connection_string();
SQLHENV create_environment();
SQLHDBC create_connection(SQLHENV env);
std::string get_driver_version(SQLHDBC dbc);
std::string get_server_version(SQLHDBC dbc);
void execute_setup_queries(SQLHDBC dbc, const std::vector<std::string>& setup_queries);
