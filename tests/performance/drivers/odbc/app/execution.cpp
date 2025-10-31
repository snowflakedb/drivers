#include "execution.h"

#include <algorithm>
#include <chrono>
#include <iomanip>
#include <iostream>

#include "connection.h"

TestResult run_query(SQLHDBC dbc, const std::string& sql_command, int iteration,
                     bool use_bulk_fetch) {
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

  SQLFreeHandle(SQL_HANDLE_STMT, stmt);

  return result;
}

void run_warmup(SQLHDBC dbc, const std::string& sql, int warmup_iterations, bool use_bulk_fetch) {
  if (warmup_iterations == 0) {
    return;
  }

  for (int i = 1; i <= warmup_iterations; i++) {
    run_query(dbc, sql, i, use_bulk_fetch);
  }
}

std::vector<TestResult> run_test_iterations(SQLHDBC dbc, const std::string& sql, int iterations,
                                            bool use_bulk_fetch) {
  std::vector<TestResult> results;

  for (int i = 1; i <= iterations; i++) {
    auto result = run_query(dbc, sql, i, use_bulk_fetch);
    results.push_back(result);
  }

  return results;
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

  std::sort(query_times.begin(), query_times.end());
  std::sort(fetch_times.begin(), fetch_times.end());

  double query_median =
      (query_times.size() % 2 == 0)
          ? (query_times[query_times.size() / 2 - 1] + query_times[query_times.size() / 2]) / 2.0
          : query_times[query_times.size() / 2];
  double fetch_median =
      (fetch_times.size() % 2 == 0)
          ? (fetch_times[fetch_times.size() / 2 - 1] + fetch_times[fetch_times.size() / 2]) / 2.0
          : fetch_times[fetch_times.size() / 2];

  double query_min = query_times.front();
  double query_max = query_times.back();
  double fetch_min = fetch_times.front();
  double fetch_max = fetch_times.back();

  std::cout << "\nSummary:\n";
  std::cout << std::fixed << std::setprecision(3);
  std::cout << "  Query: median=" << query_median << "s  min=" << query_min
            << "s  max=" << query_max << "s\n";
  std::cout << "  Fetch: median=" << fetch_median << "s  min=" << fetch_min
            << "s  max=" << fetch_max << "s\n";
}
