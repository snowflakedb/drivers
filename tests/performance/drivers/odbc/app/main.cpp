#include <ctime>
#include <filesystem>
#include <iostream>
#include <sstream>
#include <string>

#include "config.h"
#include "connection.h"
#include "execution.h"
#include "results.h"
#include "types.h"

int main() {
  std::cout.setf(std::ios::unitbuf);
  std::cerr.setf(std::ios::unitbuf);

  std::string test_name = get_env_required("TEST_NAME");
  std::string sql_command = get_env_required("SQL_COMMAND");
  int iterations = get_env_int("PERF_ITERATIONS", 1);
  int warmup_iterations = get_env_int("PERF_WARMUP_ITERATIONS", 0);

  auto params = parse_parameters_json();

  auto setup_queries = parse_setup_queries();

  SQLHENV env = create_environment();

  SQLHDBC dbc = create_connection(env);

  std::string driver_version_str = get_driver_version(dbc);
  std::string server_version = get_server_version(dbc);

  execute_setup_queries(dbc, setup_queries);

  // Bulk fetching configuration
  // Note: Bulk fetch (SQL_ATTR_ROW_ARRAY_SIZE) is used in Reference driver's old performance tests
  // where it fetches 1024 rows at a time.
  // Universal driver currently doesn't support SQL_ATTR_ROW_ARRAY_SIZE.
  // For now, row-by-row fetching is used for both drivers to ensure compatibility.
  bool use_bulk_fetch = false;  // Set to true to enable bulk fetching (1024 rows/fetch)

  std::cout << "\n=== Executing Test Query ===\n";

  run_warmup(dbc, sql_command, warmup_iterations, use_bulk_fetch);
  auto results = run_test_iterations(dbc, sql_command, iterations, use_bulk_fetch);

  // Save results (OS-agnostic path construction)
  std::string driver_type_str = get_driver_type();
  time_t now = time(nullptr);

  std::filesystem::path results_dir = std::filesystem::path("/results");
  std::stringstream filename_ss;
  filename_ss << test_name << "_odbc_" << driver_type_str << "_" << now << ".csv";
  std::string filename = (results_dir / filename_ss.str()).string();

  write_csv_results(results, filename);

  // Write run metadata (only once per run)
  std::stringstream metadata_filename_ss;
  metadata_filename_ss << "run_metadata_odbc_" << driver_type_str << ".json";
  std::string metadata_filename = (results_dir / metadata_filename_ss.str()).string();
  write_run_metadata_json(driver_type_str, driver_version_str, server_version, now,
                          metadata_filename);

  // Print statistics
  print_statistics(results);
  std::cout << "\n✓ Complete → " << filename << "\n";

  // Cleanup
  SQLDisconnect(dbc);
  SQLFreeHandle(SQL_HANDLE_DBC, dbc);
  SQLFreeHandle(SQL_HANDLE_ENV, env);

  return 0;
}
