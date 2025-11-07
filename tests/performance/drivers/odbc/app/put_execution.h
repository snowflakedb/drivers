#pragma once

#include <sql.h>
#include <sqlext.h>

#include <string>
#include <vector>

#include "types.h"

struct PutGetResult {
  int iteration;
  time_t timestamp;
  double query_time_s;
};

void execute_put_get_test(SQLHDBC dbc, const std::string& sql_command, int warmup_iterations,
                          int iterations, const std::string& test_name,
                          const std::string& driver_type_str, const std::string& driver_version_str,
                          const std::string& server_version, time_t now);
