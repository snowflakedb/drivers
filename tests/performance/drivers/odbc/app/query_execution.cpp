#include "query_execution.h"

#include <algorithm>
#include <chrono>
#include <cstdlib>
#include <ctime>
#include <iostream>
#include <utility>
#include <vector>

#include "common.h"
#include "config.h"
#include "connection.h"
#include "perf_metrics.h"
#include "results.h"

static constexpr std::size_t BULK_SIZE = 1024;
static constexpr std::size_t CHAR_COL_BUF_LEN = 1024;

// Forward declarations for private helpers
void run_warmup(SQLHDBC dbc, const std::string& sql, int warmup_iterations, CoreInstrumentation& perf,
                BindMode bind_mode);
std::vector<TestResult> run_test_iterations(SQLHDBC dbc, const std::string& sql, int iterations,
                                            CoreInstrumentation& perf, BindMode bind_mode);
void validate_row_counts(const std::vector<TestResult>& results);
void print_statistics(const std::vector<TestResult>& results);
TestResult run_query(SQLHDBC dbc, const std::string& sql, int iteration, CoreInstrumentation& perf, BindMode bind_mode);
std::pair<std::size_t, std::size_t> get_expected_row_count(const std::vector<TestResult>& results);
void assert_nonzero_row_count(std::size_t count);
void check_row_count_match(std::size_t actual_count, std::size_t expected_count, std::size_t iteration);
void set_bulk_fetch_attrs(SQLHSTMT stmt, SQLULEN* rows_fetched);
void bind_columns_for_bulk_fetch(SQLHSTMT stmt, SQLSMALLINT column_count, SQLSMALLINT c_type,
                                 std::vector<std::vector<char>>& bufs, std::vector<std::vector<SQLLEN>>& indicators);

BindMode resolve_bind_mode() {
  std::string bind_mode = get_env_optional("BIND_MODE", "char");
  if (bind_mode == "char") {
    return BindMode::Char;
  }
  if (bind_mode == "default") {
    return BindMode::Default;
  }
  std::cerr << "ERROR: Invalid BIND_MODE '" << bind_mode << "'. Supported: char, default\n";
  exit(1);
}

const char* bind_mode_label(BindMode mode) { return mode == BindMode::Default ? "default" : "char"; }

void execute_fetch_test(SQLHDBC dbc, const std::string& sql_command, int warmup_iterations, int iterations,
                        const std::string& test_name, const std::string& driver_type_str,
                        const std::string& driver_version_str, time_t now) {
  BindMode bind_mode = resolve_bind_mode();
  std::cout << "\n=== Executing SELECT Test (bulk fetch, " << BULK_SIZE << " rows/batch"
            << ", bind=" << bind_mode_label(bind_mode) << ") ===\n";
  std::cout << "Query: " << sql_command << "\n";

  CoreInstrumentation perf;
  if (perf.available()) {
    std::cout << "Perf metrics: enabled (sf_core perf_timing symbols found)\n";
  }

  run_warmup(dbc, sql_command, warmup_iterations, perf, bind_mode);

  ResourceMonitor monitor(std::chrono::milliseconds(100));
  monitor.start();

  auto results = run_test_iterations(dbc, sql_command, iterations, perf, bind_mode);

  auto memory_timeline = monitor.stop();

  validate_row_counts(results);

  std::string filename = generate_results_filename(test_name, driver_type_str, now);
  write_csv_results(results, filename, perf.available());
  write_memory_timeline(memory_timeline, test_name, driver_type_str, now);

  print_statistics(results);
  std::cout << "  Memory timeline: " << memory_timeline.size() << " samples collected\n";
  finalize_test_execution(dbc, filename, driver_type_str, driver_version_str, now);
}

void run_warmup(SQLHDBC dbc, const std::string& sql, int warmup_iterations, CoreInstrumentation& perf,
                BindMode bind_mode) {
  if (warmup_iterations == 0) {
    return;
  }

  for (int i = 1; i <= warmup_iterations; i++) {
    run_query(dbc, sql, i, perf, bind_mode);
  }
}

std::vector<TestResult> run_test_iterations(SQLHDBC dbc, const std::string& sql, int iterations,
                                            CoreInstrumentation& perf, BindMode bind_mode) {
  std::vector<TestResult> results;

  for (int i = 1; i <= iterations; i++) {
    auto result = run_query(dbc, sql, i, perf, bind_mode);
    results.push_back(result);
  }

  return results;
}

void validate_row_counts(const std::vector<TestResult>& results) {
  if (results.empty()) {
    return;
  }

  auto [expected_count, start_idx] = get_expected_row_count(results);

  for (std::size_t i = start_idx; i < results.size(); i++) {
    check_row_count_match(results[i].fetch.row_count, expected_count, i);
  }

  std::cout << "✓ All " << results.size() << " iterations returned " << expected_count << " rows\n";
}

void print_statistics(const std::vector<TestResult>& results) {
  if (results.empty()) {
    return;
  }

  std::vector<double> query_times, fetch_times;
  for (const auto& r : results) {
    query_times.push_back(r.fetch.query_time_s);
    fetch_times.push_back(r.fetch.fetch_time_s);
  }

  std::cout << "\nSummary:\n";
  print_timing_stats("Query", query_times);
  print_timing_stats("Fetch", fetch_times);
}

// Private functions

std::pair<std::size_t, std::size_t> get_expected_row_count(const std::vector<TestResult>& results) {
  const char* expected_from_recording = std::getenv("EXPECTED_ROW_COUNT");
  std::size_t expected_count;
  std::size_t start_idx;

  if (expected_from_recording) {
    expected_count = std::stoull(expected_from_recording);
    std::cout << "Row count baseline: " << expected_count << " rows (from recording phase)\n";
    assert_nonzero_row_count(expected_count);
    start_idx = 0;
  } else {
    expected_count = results[0].fetch.row_count;
    std::cout << "Row count baseline: " << expected_count << " rows (from first iteration)\n";
    assert_nonzero_row_count(expected_count);
    start_idx = 1;
  }

  return {expected_count, start_idx};
}

void assert_nonzero_row_count(std::size_t count) {
  if (count == 0) {
    std::cerr << "ERROR: Row count baseline is 0 — this likely indicates a silent query failure "
              << "(e.g. async execution timeout). Refusing to use 0 as baseline.\n";
    exit(1);
  }
}

void check_row_count_match(std::size_t actual_count, std::size_t expected_count, std::size_t iteration) {
  if (actual_count != expected_count) {
    std::cerr << "ERROR: Row count mismatch: iteration " << iteration << " returned " << actual_count
              << " rows, expected " << expected_count << " rows\n";
    exit(1);
  }
}

void set_bulk_fetch_attrs(SQLHSTMT stmt, SQLULEN* rows_fetched) {
  SQLRETURN ret = SQLSetStmtAttr(stmt, SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)BULK_SIZE, 0);
  check_odbc_error(ret, SQL_HANDLE_STMT, stmt, "SQLSetStmtAttr ROW_ARRAY_SIZE");

  ret = SQLSetStmtAttr(stmt, SQL_ATTR_ROWS_FETCHED_PTR, rows_fetched, 0);
  check_odbc_error(ret, SQL_HANDLE_STMT, stmt, "SQLSetStmtAttr ROWS_FETCHED_PTR");
}

void bind_columns_for_bulk_fetch(SQLHSTMT stmt, SQLSMALLINT column_count, SQLSMALLINT c_type,
                                 std::vector<std::vector<char>>& bufs, std::vector<std::vector<SQLLEN>>& indicators) {
  bufs.resize(column_count);
  indicators.resize(column_count);

  for (SQLSMALLINT i = 0; i < column_count; i++) {
    indicators[i].resize(BULK_SIZE, 0);
    bufs[i].resize(BULK_SIZE * CHAR_COL_BUF_LEN, 0);

    SQLRETURN ret = SQLBindCol(stmt, static_cast<SQLUSMALLINT>(i + 1), c_type, bufs[i].data(), CHAR_COL_BUF_LEN,
                               indicators[i].data());
    check_odbc_error(ret, SQL_HANDLE_STMT, stmt, "SQLBindCol");
  }
}

TestResult run_query(SQLHDBC dbc, const std::string& sql_command, int iteration, CoreInstrumentation& perf,
                     BindMode bind_mode) {
  TestResult result;
  result.iteration = iteration;
  result.fetch = run_query_fetch(dbc, sql_command, bind_mode, &perf);
  result.peak_rss_mb = get_peak_rss_mb();
  result.timestamp_ms =
      std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::system_clock::now().time_since_epoch())
          .count();

  return result;
}

QueryFetchResult run_query_fetch(SQLHDBC dbc, const std::string& sql_command, BindMode bind_mode,
                                 CoreInstrumentation* perf, bool collect_cpu) {
  QueryFetchResult result;

  SQLHSTMT stmt = SQL_NULL_HSTMT;
  SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt);
  check_odbc_error(ret, SQL_HANDLE_DBC, dbc, "SQLAllocHandle STMT");
  struct FreeStmt {
    SQLHSTMT handle;
    ~FreeStmt() {
      if (handle != SQL_NULL_HSTMT) {
        SQLFreeHandle(SQL_HANDLE_STMT, handle);
      }
    }
  } free_stmt{stmt};

  auto query_start = std::chrono::high_resolution_clock::now();
  ret = SQLExecDirect(stmt, (SQLCHAR*)sql_command.c_str(), SQL_NTS);
  check_odbc_error(ret, SQL_HANDLE_STMT, stmt, "SQLExecDirect");
  auto query_end = std::chrono::high_resolution_clock::now();

  SQLSMALLINT column_count = 0;
  ret = SQLNumResultCols(stmt, &column_count);
  check_odbc_error(ret, SQL_HANDLE_STMT, stmt, "SQLNumResultCols");

  SQLULEN rows_fetched = 0;
  std::vector<std::vector<char>> col_bufs;
  std::vector<std::vector<SQLLEN>> indicators;
  const SQLSMALLINT c_type = (bind_mode == BindMode::Default) ? SQL_C_DEFAULT : SQL_C_CHAR;

  // CHAR baselines keep historical order (bind, then bulk attrs) so BenchDash series stay comparable.
  if (bind_mode == BindMode::Default) {
    set_bulk_fetch_attrs(stmt, &rows_fetched);
    bind_columns_for_bulk_fetch(stmt, column_count, c_type, col_bufs, indicators);
  } else {
    bind_columns_for_bulk_fetch(stmt, column_count, c_type, col_bufs, indicators);
    set_bulk_fetch_attrs(stmt, &rows_fetched);
  }

  struct rusage usage_before;
  if (collect_cpu) {
    getrusage(RUSAGE_SELF, &usage_before);
  }

  if (perf) {
    perf->reset();
  }

  auto fetch_start = std::chrono::high_resolution_clock::now();
  std::size_t row_count = 0;

  while ((ret = SQLFetch(stmt)) != SQL_NO_DATA) {
    check_odbc_error(ret, SQL_HANDLE_STMT, stmt, "SQLFetch");
    row_count += rows_fetched;
  }

  auto fetch_end = std::chrono::high_resolution_clock::now();

  CoreInstrumentationData core_metrics;
  if (perf) {
    core_metrics = perf->collect();
  }

  result.query_time_s = std::chrono::duration<double>(query_end - query_start).count();
  result.fetch_time_s = std::chrono::duration<double>(fetch_end - fetch_start).count();
  result.core_batch_wait_s = core_metrics.core_batch_wait_s;
  result.core_chunk_download_s = core_metrics.core_chunk_download_s;
  result.core_arrow_decode_s = core_metrics.core_arrow_decode_s;
  result.wrapper_time_s = std::max(0.0, result.fetch_time_s - result.core_batch_wait_s);
  result.row_count = row_count;
  if (collect_cpu) {
    struct rusage usage_after;
    getrusage(RUSAGE_SELF, &usage_after);
    result.cpu_time_s = cpu_seconds(usage_after) - cpu_seconds(usage_before);
  }

  return result;
}
