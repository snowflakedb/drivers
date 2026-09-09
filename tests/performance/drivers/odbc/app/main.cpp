#include <sys/wait.h>
#include <unistd.h>

#include <chrono>
#include <ctime>
#include <functional>
#include <iomanip>
#include <iostream>
#include <map>
#include <stdexcept>
#include <string>
#include <vector>

#include "common.h"
#include "concurrent_execution.h"
#include "config.h"
#include "connection.h"
#include "put_execution.h"
#include "query_execution.h"
#include "results.h"
#include "types.h"

using TestExecutor = std::function<void(SQLHDBC, const std::string&, int, int, const std::string&, const std::string&,
                                        const std::string&, time_t)>;

const std::map<TestType, TestExecutor> TEST_EXECUTORS = {
    {TestType::Select, execute_fetch_test},
    {TestType::PutGet, execute_put_get_test},
};

static void run_cold_start_child(const std::string& sql_command) {
  auto t0 = std::chrono::steady_clock::now();
  SQLHENV env = create_environment();
  SQLHDBC dbc = create_connection(env);
  auto t1 = std::chrono::steady_clock::now();

  SQLHSTMT stmt = SQL_NULL_HSTMT;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt);
  check_odbc_error(ret, SQL_HANDLE_DBC, dbc, "SQLAllocHandle STMT");
  ret = SQLExecDirect(stmt, (SQLCHAR*)sql_command.c_str(), SQL_NTS);
  check_odbc_error(ret, SQL_HANDLE_STMT, stmt, "SQLExecDirect");
  ret = SQLFetch(stmt);
  check_odbc_error(ret, SQL_HANDLE_STMT, stmt, "SQLFetch");
  SQLINTEGER value = 0;
  ret = SQLGetData(stmt, 1, SQL_C_SLONG, &value, sizeof(value), nullptr);
  check_odbc_error(ret, SQL_HANDLE_STMT, stmt, "SQLGetData");
  if (value != 1) {
    throw std::runtime_error("Expected 1, got " + std::to_string(value));
  }
  SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  auto t2 = std::chrono::steady_clock::now();

  SQLDisconnect(dbc);
  SQLFreeHandle(SQL_HANDLE_DBC, dbc);
  SQLFreeHandle(SQL_HANDLE_ENV, env);

  double connect_s = std::chrono::duration<double>(t1 - t0).count();
  double select1_s = std::chrono::duration<double>(t2 - t1).count();
  double e2e_s = std::chrono::duration<double>(t2 - t0).count();
  struct rusage usage;
  getrusage(RUSAGE_SELF, &usage);
  auto now_ms =
      std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::system_clock::now().time_since_epoch())
          .count();
  std::cout << now_ms << "," << std::fixed << std::setprecision(6) << e2e_s << "," << connect_s << "," << select1_s
            << "," << cpu_seconds(usage) << "," << std::setprecision(1) << get_peak_rss_mb() << "\n";
}

static std::string run_cold_start_once() {
  int fds[2];
  if (pipe(fds) != 0) {
    throw std::runtime_error("pipe failed");
  }
  pid_t pid = fork();
  if (pid < 0) {
    throw std::runtime_error("fork failed");
  }
  if (pid == 0) {
    close(fds[0]);
    if (dup2(fds[1], STDOUT_FILENO) < 0) {
      _exit(127);
    }
    close(fds[1]);
    setenv("COLD_START_CHILD", "1", 1);
    char exe[4096];
    ssize_t n = readlink("/proc/self/exe", exe, sizeof(exe) - 1);
    if (n <= 0) {
      _exit(127);
    }
    exe[n] = '\0';
    execl(exe, exe, static_cast<char*>(nullptr));
    _exit(127);
  }
  close(fds[1]);
  std::string out;
  char buf[4096];
  ssize_t n;
  while ((n = read(fds[0], buf, sizeof(buf))) > 0) {
    out.append(buf, static_cast<size_t>(n));
  }
  close(fds[0]);
  int status = 0;
  waitpid(pid, &status, 0);
  if (!WIFEXITED(status) || WEXITSTATUS(status) != 0) {
    throw std::runtime_error("cold-start child failed");
  }
  while (!out.empty() && (out.back() == '\n' || out.back() == '\r')) {
    out.pop_back();
  }
  auto pos = out.find_last_of('\n');
  return pos == std::string::npos ? out : out.substr(pos + 1);
}

static void run_cold_start_parent(const std::string& test_name, const std::string& driver_type, int iterations) {
  std::cout << "\n=== Cold-Start Test (" << iterations << " iterations) ===\n";
  std::vector<std::string> rows;
  for (int i = 0; i < iterations; ++i) {
    std::string label = "iter " + std::to_string(i + 1) + "/" + std::to_string(iterations);
    std::string row = run_cold_start_once();
    std::cout << "  [" << label << "] " << row << "\n";
    rows.push_back(row);
  }
  time_t now = time(nullptr);
  std::string filename = generate_results_filename(test_name, driver_type, now);
  write_csv_results_cold_start(rows, filename);
  write_run_metadata_json(driver_type, "UNKNOWN", "N/A", now, generate_metadata_filename(driver_type));
  std::cout << "\n✓ Complete → " << filename << "\n";
}

int main() {
  std::cout.setf(std::ios::unitbuf);
  std::cerr.setf(std::ios::unitbuf);

  std::string test_name = get_env_required("TEST_NAME");
  std::string sql_command = get_env_required("SQL_COMMAND");
  TestType test_type = get_test_type();
  int iterations = get_env_int("PERF_ITERATIONS", 1);
  int warmup_iterations = get_env_int("PERF_WARMUP_ITERATIONS", 0);

  if (test_type == TestType::ColdStart) {
    try {
      if (std::getenv("COLD_START_CHILD")) {
        run_cold_start_child(sql_command);
      } else {
        run_cold_start_parent(test_name, get_driver_type(), iterations);
      }
      return 0;
    } catch (const std::exception& e) {
      std::cerr << "ERROR: " << e.what() << "\n";
      return 1;
    }
  }

  auto setup_queries = parse_setup_queries();

  SQLHENV env = SQL_NULL_HENV;
  SQLHDBC dbc = SQL_NULL_HDBC;
  try {
    env = create_environment();
    std::string driver_type_str = get_driver_type();
    time_t now = time(nullptr);

    dbc = create_connection(env);
    std::string driver_version_str = get_driver_version(dbc);
    execute_setup_queries(dbc, setup_queries);

    if (test_type == TestType::Concurrent) {
      int worker_count = get_env_int("WORKER_COUNT", 1);
      execute_concurrent_test(env, dbc, sql_command, warmup_iterations, iterations, worker_count, setup_queries,
                              test_name, driver_type_str, driver_version_str, now);
    } else {
      auto executor_it = TEST_EXECUTORS.find(test_type);
      if (executor_it == TEST_EXECUTORS.end()) {
        std::cerr << "ERROR: Unknown test type: " << test_type_to_string(test_type) << "\n";
        std::cerr << "Supported types: select, put_get, concurrent\n";
        SQLDisconnect(dbc);
        SQLFreeHandle(SQL_HANDLE_DBC, dbc);
        SQLFreeHandle(SQL_HANDLE_ENV, env);
        return 1;
      }
      executor_it->second(dbc, sql_command, warmup_iterations, iterations, test_name, driver_type_str,
                          driver_version_str, now);
    }

    SQLDisconnect(dbc);
    SQLFreeHandle(SQL_HANDLE_DBC, dbc);
    SQLFreeHandle(SQL_HANDLE_ENV, env);
    return 0;
  } catch (const std::exception&) {
    if (dbc != SQL_NULL_HDBC) {
      SQLDisconnect(dbc);
      SQLFreeHandle(SQL_HANDLE_DBC, dbc);
    }
    if (env != SQL_NULL_HENV) {
      SQLFreeHandle(SQL_HANDLE_ENV, env);
    }
    return 1;
  }
}
