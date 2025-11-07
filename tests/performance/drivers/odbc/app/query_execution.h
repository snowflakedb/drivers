#pragma once

#include <sql.h>
#include <sqlext.h>

#include <string>
#include <vector>

#include "types.h"

void execute_fetch_test(SQLHDBC dbc, const std::string& sql_command, int warmup_iterations,
                        int iterations, const std::string& test_name,
                        const std::string& driver_type_str, const std::string& driver_version_str,
                        const std::string& server_version, time_t now);
