#pragma once

#include <sql.h>
#include <sqlext.h>

#include <ctime>
#include <string>
#include <vector>

void execute_concurrent_test(SQLHENV env, SQLHDBC setup_dbc, const std::string& sql_command, int warmup_iterations,
                             int iterations, int worker_count, const std::vector<std::string>& setup_queries,
                             const std::string& test_name, const std::string& driver_type_str,
                             const std::string& driver_version_str, time_t now);
