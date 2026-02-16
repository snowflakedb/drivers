#include "query_execution.h"

#include <chrono>
#include <ctime>
#include <iostream>

#include "common.h"
#include "connection.h"
#include "results.h"

// Forward declarations for private helpers
void run_warmup(SQLHDBC dbc, const std::string& sql, int warmup_iterations, bool use_bulk_fetch);
std::vector<TestResult> run_test_iterations(SQLHDBC dbc, const std::string& sql, int iterations, bool use_bulk_fetch);
void validate_row_counts(const std::vector<TestResult>& results);
void print_statistics(const std::vector<TestResult>& results);
TestResult run_query(SQLHDBC dbc, const std::string& sql, int iteration, bool use_bulk_fetch);
std::pair<std::size_t, size_t> get_expected_row_count(const std::vector<TestResult>& results);
void check_row_count_match(std::size_t actual_count, std::size_t expected_count, size_t iteration);

void execute_fetch_test(SQLHDBC dbc, const std::string& sql_command, int warmup_iterations, int iterations,
                        const std::string& test_name, const std::string& driver_type_str,
                        const std::string& driver_version_str, time_t now) {
  std::cout << "\n=== Executing SELECT Test ===\n";
  std::cout << "Query: " << sql_command << "\n";

  // TODO SNOW-2876245: Bulk fetch not yet implemented. Decision needed on which fetch method to
  // use. Currently using row-by-row fetch.
  bool use_bulk_fetch = false;

  run_warmup(dbc, sql_command, warmup_iterations, use_bulk_fetch);
  auto results = run_test_iterations(dbc, sql_command, iterations, use_bulk_fetch);
  validate_row_counts(results);

  std::string filename = generate_results_filename(test_name, driver_type_str, now);
  write_csv_results(results, filename);

  print_statistics(results);
  finalize_test_execution(dbc, filename, driver_type_str, driver_version_str, now);
}

void run_warmup(SQLHDBC dbc, const std::string& sql, int warmup_iterations, bool use_bulk_fetch) {
  if (warmup_iterations == 0) {
    return;
  }

  for (int i = 1; i <= warmup_iterations; i++) {
    run_query(dbc, sql, i, use_bulk_fetch);
  }
}

std::vector<TestResult> run_test_iterations(SQLHDBC dbc, const std::string& sql, int iterations, bool use_bulk_fetch) {
  std::vector<TestResult> results;

  for (int i = 1; i <= iterations; i++) {
    auto result = run_query(dbc, sql, i, use_bulk_fetch);
    results.push_back(result);
  }

  return results;
}

void validate_row_counts(const std::vector<TestResult>& results) {
  if (results.empty()) {
    return;
  }

  auto [expected_count, start_idx] = get_expected_row_count(results);

  for (size_t i = start_idx; i < results.size(); i++) {
    check_row_count_match(results[i].row_count, expected_count, i);
  }

  std::cout << "✓ All " << results.size() << " iterations returned " << expected_count << " rows\n";
}

void print_statistics(const std::vector<TestResult>& results) {
  if (results.empty()) {
    return;
  }

  std::vector<double> query_times, fetch_times;
  for (const auto& r : results) {
    query_times.push_back(r.query_time_s);
    fetch_times.push_back(r.fetch_time_s);
  }

  std::cout << "\nSummary:\n";
  print_timing_stats("Query", query_times);
  print_timing_stats("Fetch", fetch_times);
}

// Private functions

std::pair<std::size_t, size_t> get_expected_row_count(const std::vector<TestResult>& results) {
  const char* expected_from_recording = std::getenv("EXPECTED_ROW_COUNT");
  std::size_t expected_count;
  size_t start_idx;

  if (expected_from_recording) {
    expected_count = std::stoull(expected_from_recording);
    std::cout << "Row count baseline: " << expected_count << " rows (from recording phase)\n";
    start_idx = 0;  // Validate all iterations including first
  } else {
    // Use first iteration as baseline
    expected_count = results[0].row_count;
    std::cout << "Row count baseline: " << expected_count << " rows (from first iteration)\n";
    start_idx = 1;  // Skip first iteration since it's the baseline
  }

  return {expected_count, start_idx};
}

void check_row_count_match(std::size_t actual_count, std::size_t expected_count, size_t iteration) {
  if (actual_count != expected_count) {
    std::cerr << "ERROR: Row count mismatch: iteration " << iteration << " returned " << actual_count
              << " rows, expected " << expected_count << " rows\n";
    exit(1);
  }
}

TestResult run_query(SQLHDBC dbc, const std::string& sql_command, int iteration, bool use_bulk_fetch) {
  TestResult result;
  result.iteration = iteration;

  // Create statement
  SQLHSTMT stmt;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt);
  check_odbc_error(ret, SQL_HANDLE_DBC, dbc, "SQLAllocHandle STMT");

  // Execute query
  auto query_start = std::chrono::high_resolution_clock::now();
  ret = SQLExecDirect(stmt, (SQLCHAR*)sql_command.c_str(), SQL_NTS);
  check_odbc_error(ret, SQL_HANDLE_STMT, stmt, "SQLExecDirect");
  auto query_end = std::chrono::high_resolution_clock::now();

  // Fetch all rows
  auto fetch_start = std::chrono::high_resolution_clock::now();
  std::size_t row_count = 0;

  if (use_bulk_fetch) {
    // Bulk fetch: Set bulk fetch size to 1024 rows (matches old implementation)
    // Note: Universal driver doesn't support SQL_ATTR_ROW_ARRAY_SIZE yet
    const std::size_t bulk_size = 1024;
    ret = SQLSetStmtAttr(stmt, SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)bulk_size, 0);
    check_odbc_error(ret, SQL_HANDLE_STMT, stmt, "SQLSetStmtAttr ROW_ARRAY_SIZE");

    // Fetch in bulk (1024 rows at a time)
    while ((ret = SQLFetch(stmt)) != SQL_NO_DATA) {
      check_odbc_error(ret, SQL_HANDLE_STMT, stmt, "SQLFetch");
      row_count += bulk_size;
    }
  } else {
    // Row-by-row fetch
    while ((ret = SQLFetch(stmt)) != SQL_NO_DATA) {
      check_odbc_error(ret, SQL_HANDLE_STMT, stmt, "SQLFetch");
      row_count++;
    }
  }

  auto fetch_end = std::chrono::high_resolution_clock::now();

  result.query_time_s = std::chrono::duration<double>(query_end - query_start).count();
  result.fetch_time_s = std::chrono::duration<double>(fetch_end - fetch_start).count();
  result.row_count = row_count;
  result.timestamp = std::time(nullptr);

  SQLFreeHandle(SQL_HANDLE_STMT, stmt);

  return result;
}
