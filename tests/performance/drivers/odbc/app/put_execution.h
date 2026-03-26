#pragma once

#include <sql.h>
#include <sqlext.h>

#include <cstdint>
#include <ctime>
#include <string>
#include <vector>

#include "resource_monitor.h"
#include "types.h"

struct PutGetResult {
  int iteration;
  int64_t timestamp_ms;
  double query_time_s;
  double cpu_time_s;
  double peak_rss_mb;
};

void execute_put_get_test(SQLHDBC dbc, const std::string& sql_command, int warmup_iterations, int iterations,
                          const std::string& test_name, const std::string& driver_type_str,
                          const std::string& driver_version_str, time_t now);
