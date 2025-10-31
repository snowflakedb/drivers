#pragma once

#include <string>
#include <vector>

#include "types.h"

void write_csv_results(const std::vector<TestResult>& results, const std::string& filename);
void write_run_metadata_json(const std::string& driver_type, const std::string& driver_version,
                             const std::string& server_version, time_t timestamp,
                             const std::string& filename);
