#pragma once

#include <sql.h>
#include <sqlext.h>

#include <cstddef>
#include <ctime>
#include <string>
#include <vector>

#include "resource_monitor.h"
#include "types.h"

enum class BindMode { Char, Default };

BindMode resolve_bind_mode();
const char* bind_mode_label(BindMode mode);

QueryFetchResult run_query_fetch(SQLHDBC dbc, const std::string& sql_command, BindMode bind_mode,
                                 class CoreInstrumentation* perf = nullptr, bool collect_cpu = true);

void execute_fetch_test(SQLHDBC dbc, const std::string& sql_command, int warmup_iterations, int iterations,
                        const std::string& test_name, const std::string& driver_type_str,
                        const std::string& driver_version_str, time_t now);
