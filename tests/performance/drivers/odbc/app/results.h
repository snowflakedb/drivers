#pragma once

#include <sql.h>
#include <sqlext.h>

#include <ctime>
#include <string>
#include <vector>

#include "resource_monitor.h"
#include "types.h"

// Forward declaration for PutGetResult
struct PutGetResult;

void write_csv_results(const std::vector<TestResult>& results, const std::string& filename);
void write_csv_results_put_get(const std::vector<PutGetResult>& results, const std::string& filename);
void write_memory_timeline(const std::vector<MemorySample>& samples, const std::string& test_name,
                           const std::string& driver_type, time_t timestamp);

std::string generate_results_filename(const std::string& test_name, const std::string& driver_type, time_t timestamp);
std::string generate_metadata_filename(const std::string& driver_type);
void write_run_metadata_json(const std::string& driver_type, const std::string& driver_version,
                             const std::string& server_version, time_t timestamp, const std::string& filename);
void finalize_test_execution(SQLHDBC dbc, const std::string& results_file, const std::string& driver_type,
                             const std::string& driver_version, time_t timestamp);
