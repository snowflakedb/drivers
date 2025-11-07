#include <ctime>
#include <filesystem>
#include <functional>
#include <iostream>
#include <map>
#include <sstream>
#include <string>

#include "config.h"
#include "connection.h"
#include "put_execution.h"
#include "query_execution.h"
#include "results.h"
#include "types.h"

using TestExecutor =
    std::function<void(SQLHDBC, const std::string&, int, int, const std::string&,
                       const std::string&, const std::string&, const std::string&, time_t)>;

const std::map<TestType, TestExecutor> TEST_EXECUTORS = {
    {TestType::Select, execute_fetch_test},
    {TestType::PutGet, execute_put_get_test},
};

int main() {
  std::cout.setf(std::ios::unitbuf);
  std::cerr.setf(std::ios::unitbuf);

  std::string test_name = get_env_required("TEST_NAME");
  std::string sql_command = get_env_required("SQL_COMMAND");
  TestType test_type = get_test_type();
  int iterations = get_env_int("PERF_ITERATIONS", 1);
  int warmup_iterations = get_env_int("PERF_WARMUP_ITERATIONS", 0);

  auto params = parse_parameters_json();
  auto setup_queries = parse_setup_queries();

  SQLHENV env = create_environment();
  SQLHDBC dbc = create_connection(env);

  std::string driver_version_str = get_driver_version(dbc);
  std::string server_version = get_server_version(dbc);

  execute_setup_queries(dbc, setup_queries);

  std::string driver_type_str = get_driver_type();
  time_t now = time(nullptr);

  // Use appropriate test executor
  auto executor_it = TEST_EXECUTORS.find(test_type);
  if (executor_it != TEST_EXECUTORS.end()) {
    executor_it->second(dbc, sql_command, warmup_iterations, iterations, test_name, driver_type_str,
                        driver_version_str, server_version, now);
  } else {
    std::cerr << "ERROR: Unknown test type: " << test_type_to_string(test_type) << "\n";
    std::cerr << "Supported types: select, put_get\n";
    SQLDisconnect(dbc);
    SQLFreeHandle(SQL_HANDLE_DBC, dbc);
    SQLFreeHandle(SQL_HANDLE_ENV, env);
    return 1;
  }

  // Cleanup
  SQLDisconnect(dbc);
  SQLFreeHandle(SQL_HANDLE_DBC, dbc);
  SQLFreeHandle(SQL_HANDLE_ENV, env);

  return 0;
}
