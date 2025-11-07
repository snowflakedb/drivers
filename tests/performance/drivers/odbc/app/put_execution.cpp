#include "put_execution.h"

#include <chrono>
#include <ctime>
#include <filesystem>
#include <iostream>
#include <regex>

#include "common.h"
#include "connection.h"
#include "results.h"

// Forward declarations for private helpers
void run_warmup_put_get(SQLHDBC dbc, const std::string& sql, int warmup_iterations);
std::vector<PutGetResult> run_test_iterations_put_get(SQLHDBC dbc, const std::string& sql,
                                                      int iterations);
void print_statistics_put_get(const std::vector<PutGetResult>& results);
PutGetResult run_put_get_query(SQLHDBC dbc, const std::string& sql, int iteration);
void create_get_target_directory(const std::string& sql_command);

void execute_put_get_test(SQLHDBC dbc, const std::string& sql_command, int warmup_iterations,
                          int iterations, const std::string& test_name,
                          const std::string& driver_type_str, const std::string& driver_version_str,
                          const std::string& server_version, time_t now) {
  std::cout << "\n=== Executing PUT_GET Test ===\n";
  std::cout << "Query: " << sql_command << "\n";

  run_warmup_put_get(dbc, sql_command, warmup_iterations);
  auto results = run_test_iterations_put_get(dbc, sql_command, iterations);

  std::string filename = generate_results_filename(test_name, driver_type_str, now);
  write_csv_results_put_get(results, filename);

  print_statistics_put_get(results);
  finalize_test_execution(filename, driver_type_str, driver_version_str, server_version, now);
}

void run_warmup_put_get(SQLHDBC dbc, const std::string& sql, int warmup_iterations) {
  if (warmup_iterations == 0) {
    return;
  }

  for (int i = 1; i <= warmup_iterations; i++) {
    run_put_get_query(dbc, sql, i);
  }
}

std::vector<PutGetResult> run_test_iterations_put_get(SQLHDBC dbc, const std::string& sql,
                                                      int iterations) {
  std::vector<PutGetResult> results;

  for (int i = 1; i <= iterations; i++) {
    auto result = run_put_get_query(dbc, sql, i);
    results.push_back(result);
  }

  return results;
}

void print_statistics_put_get(const std::vector<PutGetResult>& results) {
  if (results.empty()) {
    return;
  }

  std::vector<double> query_times;
  for (const auto& r : results) {
    query_times.push_back(r.query_time_s);
  }

  std::cout << "\nSummary:\n";
  print_timing_stats("Operation time", query_times);
}

PutGetResult run_put_get_query(SQLHDBC dbc, const std::string& sql_command, int iteration) {
  PutGetResult result;
  result.iteration = iteration;

  create_get_target_directory(sql_command);

  // Create statement
  SQLHSTMT stmt;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt);
  check_odbc_error(ret, SQL_HANDLE_DBC, dbc, "SQLAllocHandle STMT");

  // Execute PUT/GET command
  auto query_start = std::chrono::high_resolution_clock::now();
  ret = SQLExecDirect(stmt, (SQLCHAR*)sql_command.c_str(), SQL_NTS);
  check_odbc_error(ret, SQL_HANDLE_STMT, stmt, "SQLExecDirect");
  auto query_end = std::chrono::high_resolution_clock::now();

  result.query_time_s = std::chrono::duration<double>(query_end - query_start).count();
  result.timestamp = std::time(nullptr);

  SQLFreeHandle(SQL_HANDLE_STMT, stmt);

  return result;
}

/**
 * Prepare target directory for GET commands.
 *
 * For GET commands:
 * - Removes existing directory to ensure clean iteration
 * - Creates fresh directory structure
 */
void create_get_target_directory(const std::string& sql_command) {
  std::string sql_upper = sql_command;
  std::transform(sql_upper.begin(), sql_upper.end(), sql_upper.begin(), ::toupper);

  if (sql_upper.find("GET") == 0 || sql_upper.find(" GET ") != std::string::npos) {
    std::regex file_regex(R"(file://([^\s]+))");
    std::smatch match;
    if (std::regex_search(sql_command, match, file_regex)) {
      std::string target_path = match[1].str();

      if (std::filesystem::exists(target_path)) {
        std::filesystem::remove_all(target_path);
      }
      std::filesystem::create_directories(target_path);
    }
  }
}
