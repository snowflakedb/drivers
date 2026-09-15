#pragma once

#include <sql.h>

#include <string>

void execute_binding_test(SQLHDBC dbc, const std::string& sql_command, int warmup_iterations, int iterations,
                          const std::string& test_name, const std::string& driver_type_str,
                          const std::string& driver_version_str, time_t now);
