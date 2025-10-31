#pragma once

#include <sql.h>
#include <sqlext.h>

#include <string>
#include <vector>

#include "types.h"

TestResult run_query(SQLHDBC dbc, const std::string& sql, int iteration, bool use_bulk_fetch);
void run_warmup(SQLHDBC dbc, const std::string& sql, int warmup_iterations, bool use_bulk_fetch);
std::vector<TestResult> run_test_iterations(SQLHDBC dbc, const std::string& sql, int iterations,
                                            bool use_bulk_fetch);
void print_statistics(const std::vector<TestResult>& results);
