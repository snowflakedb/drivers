#include "binding_execution.h"

#include <sqlext.h>

#include <chrono>
#include <cstdlib>
#include <cstring>
#include <iostream>
#include <sstream>
#include <stdexcept>
#include <utility>
#include <vector>

#include <nlohmann/json.hpp>

#include "common.h"
#include "config.h"
#include "connection.h"
#include "resource_monitor.h"
#include "results.h"

namespace {

constexpr const char* BIND_PERF_TABLE = "bind_perf_wide_t";

using json = nlohmann::json;

struct FreeStmt {
  SQLHSTMT handle;
  ~FreeStmt() {
    if (handle != SQL_NULL_HSTMT) {
      SQLFreeHandle(SQL_HANDLE_STMT, handle);
    }
  }
};

std::string get_env_or_empty(const char* name) {
  const char* value = std::getenv(name);
  return value ? std::string(value) : std::string();
}

bool via_wiremock_replay() {
  const char* replay = std::getenv("WIREMOCK_REPLAY");
  return replay != nullptr && std::string(replay) == "true";
}

bool is_byte_array(const json& value) {
  if (!value.is_array() || value.empty()) {
    return false;
  }
  for (const auto& byte : value) {
    if (!byte.is_number_integer()) {
      return false;
    }
    const auto byte_value = byte.get<long long>();
    if (byte_value < 0 || byte_value > 255) {
      return false;
    }
  }
  return true;
}

std::string value_to_string(const json& value) {
  if (value.is_null()) {
    return "";
  }
  if (value.is_boolean()) {
    return value.get<bool>() ? "true" : "false";
  }
  if (value.is_number_integer()) {
    return std::to_string(value.get<long long>());
  }
  if (value.is_number_float()) {
    std::ostringstream out;
    out << value.get<double>();
    return out.str();
  }
  if (is_byte_array(value)) {
    std::string bytes;
    for (const auto& byte : value) {
      bytes.push_back(static_cast<char>(byte.get<int>()));
    }
    return bytes;
  }
  return value.get<std::string>();
}

void bind_scalar_parameter(SQLHSTMT stmt, SQLUSMALLINT index, const json& value,
                           std::vector<std::string>& binary_buffers, std::vector<std::vector<char>>& char_buffers,
                           std::vector<SQLLEN>& lengths) {
  const SQLUSMALLINT param_index = static_cast<SQLUSMALLINT>(index + 1);
  if (value.is_null()) {
    lengths[index] = SQL_NULL_DATA;
    check_odbc_error(SQLBindParameter(stmt, param_index, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 0, 0, nullptr, 0,
                                      &lengths[index]),
                     SQL_HANDLE_STMT, stmt, "SQLBindParameter scalar null");
    return;
  }
  if (is_byte_array(value)) {
    binary_buffers.push_back(value_to_string(value));
    const std::string& bytes = binary_buffers.back();
    lengths[index] = static_cast<SQLLEN>(bytes.size());
    check_odbc_error(SQLBindParameter(stmt, param_index, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_BINARY,
                                      static_cast<SQLULEN>(bytes.size()), 0,
                                      const_cast<SQLCHAR*>(reinterpret_cast<const SQLCHAR*>(bytes.data())),
                                      bytes.size(), &lengths[index]),
                     SQL_HANDLE_STMT, stmt, "SQLBindParameter scalar binary");
    return;
  }

  const std::string param = value_to_string(value);
  char_buffers.emplace_back(param.begin(), param.end());
  char_buffers.back().push_back('\0');
  lengths[index] = static_cast<SQLLEN>(param.size());
  check_odbc_error(
      SQLBindParameter(stmt, param_index, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, static_cast<SQLULEN>(param.size()),
                       0, char_buffers.back().data(), char_buffers.back().size(), &lengths[index]),
      SQL_HANDLE_STMT, stmt, "SQLBindParameter scalar");
}

void truncate_bind_table(SQLHDBC dbc) {
  if (via_wiremock_replay()) {
    return;
  }
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  check_odbc_error(SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt), SQL_HANDLE_DBC, dbc, "SQLAllocHandle truncate");
  FreeStmt free_stmt{stmt};
  std::string sql = std::string("TRUNCATE TABLE IF EXISTS ") + BIND_PERF_TABLE;
  SQLRETURN ret = SQLExecDirect(stmt, reinterpret_cast<SQLCHAR*>(sql.data()), SQL_NTS);
  if (ret != SQL_SUCCESS && ret != SQL_SUCCESS_WITH_INFO) {
    sql = std::string("DELETE FROM ") + BIND_PERF_TABLE;
    check_odbc_error(SQLExecDirect(stmt, reinterpret_cast<SQLCHAR*>(sql.data()), SQL_NTS), SQL_HANDLE_STMT, stmt,
                     "DELETE bind table");
  }
}

std::size_t inserted_row_count(SQLHDBC dbc) {
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  check_odbc_error(SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt), SQL_HANDLE_DBC, dbc, "SQLAllocHandle count");
  FreeStmt free_stmt{stmt};
  std::string sql = std::string("SELECT COUNT(*) FROM ") + BIND_PERF_TABLE;
  check_odbc_error(SQLExecDirect(stmt, reinterpret_cast<SQLCHAR*>(sql.data()), SQL_NTS), SQL_HANDLE_STMT, stmt,
                   "SELECT COUNT(*)");
  SQLRETURN fetch_ret = SQLFetch(stmt);
  if (fetch_ret == SQL_NO_DATA) {
    throw std::runtime_error(sql + " returned no rows");
  }
  check_odbc_error(fetch_ret, SQL_HANDLE_STMT, stmt, "SQLFetch COUNT(*)");
  SQLINTEGER count = 0;
  SQLLEN indicator = 0;
  check_odbc_error(SQLGetData(stmt, 1, SQL_C_SLONG, &count, sizeof(count), &indicator), SQL_HANDLE_STMT, stmt,
                   "SQLGetData COUNT(*)");
  if (indicator == SQL_NULL_DATA || count < 0) {
    throw std::runtime_error(sql + " returned NULL");
  }
  return static_cast<std::size_t>(count);
}

TestResult run_scalar_query(SQLHDBC dbc, const std::string& sql, const json& params) {
  SQLHSTMT stmt = SQL_NULL_HSTMT;
  check_odbc_error(SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt), SQL_HANDLE_DBC, dbc, "SQLAllocHandle scalar");
  FreeStmt free_stmt{stmt};

  check_odbc_error(SQLPrepare(stmt, reinterpret_cast<SQLCHAR*>(const_cast<char*>(sql.data())), SQL_NTS),
                   SQL_HANDLE_STMT, stmt, "SQLPrepare scalar");

  std::vector<std::string> binary_buffers;
  std::vector<std::vector<char>> char_buffers;
  std::vector<SQLLEN> lengths(params.size());
  auto bind_start = std::chrono::steady_clock::now();
  for (std::size_t i = 0; i < params.size(); ++i) {
    bind_scalar_parameter(stmt, static_cast<SQLUSMALLINT>(i), params[i], binary_buffers, char_buffers, lengths);
  }
  auto bind_end = std::chrono::steady_clock::now();

  check_odbc_error(SQLExecute(stmt), SQL_HANDLE_STMT, stmt, "SQLExecute scalar");

  auto fetch_start = std::chrono::steady_clock::now();
  std::size_t row_count = 0;
  // SQLFetch here counts rows only; it does not bind columns or convert cells.
  while (true) {
    SQLRETURN fetch_ret = SQLFetch(stmt);
    if (fetch_ret == SQL_NO_DATA) {
      break;
    }
    check_odbc_error(fetch_ret, SQL_HANDLE_STMT, stmt, "SQLFetch scalar");
    row_count++;
  }
  auto fetch_end = std::chrono::steady_clock::now();

  TestResult result{};
  result.timestamp_ms =
      std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::system_clock::now().time_since_epoch())
          .count();
  result.fetch.query_time_s = std::chrono::duration<double>(bind_end - bind_start).count();
  result.fetch.fetch_time_s = std::chrono::duration<double>(fetch_end - fetch_start).count();
  result.fetch.row_count = row_count;
  result.fetch.cpu_time_s = 0.0;
  result.peak_rss_mb = get_peak_rss_mb();
  return result;
}

TestResult run_executemany_insert(SQLHDBC dbc, const std::string& sql, const json& rows) {
  if (rows.empty()) {
    throw std::runtime_error("BINDING_PARAMS_JSON must contain at least one row for executemany");
  }
  const std::size_t row_count = rows.size();
  const std::size_t col_count = rows.front().size();
  for (const auto& row : rows) {
    if (row.size() != col_count) {
      throw std::runtime_error("executemany rows must all have the same column count");
    }
  }

  SQLHSTMT stmt = SQL_NULL_HSTMT;
  check_odbc_error(SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt), SQL_HANDLE_DBC, dbc, "SQLAllocHandle executemany");
  FreeStmt free_stmt{stmt};
  check_odbc_error(SQLPrepare(stmt, reinterpret_cast<SQLCHAR*>(const_cast<char*>(sql.data())), SQL_NTS),
                   SQL_HANDLE_STMT, stmt, "SQLPrepare executemany");

  auto bind_start = std::chrono::steady_clock::now();
  std::vector<std::vector<std::string>> cells(col_count);
  std::vector<std::vector<char>> nulls(col_count);
  std::vector<SQLULEN> widths(col_count, 1);
  for (const auto& row : rows) {
    for (std::size_t col = 0; col < col_count; ++col) {
      if (row[col].is_null()) {
        cells[col].emplace_back();
        nulls[col].push_back(1);
        continue;
      }
      std::string value = value_to_string(row[col]);
      widths[col] = std::max(widths[col], static_cast<SQLULEN>(value.size() + 1));
      cells[col].push_back(std::move(value));
      nulls[col].push_back(0);
    }
  }

  struct BoundColumn {
    std::vector<char> data;
    std::vector<SQLLEN> lengths;
    SQLULEN width;
  };
  std::vector<BoundColumn> columns(col_count);
  for (std::size_t col = 0; col < col_count; ++col) {
    columns[col].width = widths[col];
    columns[col].data.assign(row_count * widths[col], '\0');
    columns[col].lengths.resize(row_count);
    for (std::size_t row = 0; row < row_count; ++row) {
      if (nulls[col][row]) {
        columns[col].lengths[row] = SQL_NULL_DATA;
        continue;
      }
      const std::string& value = cells[col][row];
      std::memcpy(&columns[col].data[row * widths[col]], value.data(), value.size());
      columns[col].lengths[row] = static_cast<SQLLEN>(value.size());
    }
  }

  check_odbc_error(SQLSetStmtAttr(stmt, SQL_ATTR_PARAM_BIND_TYPE, SQL_PARAM_BIND_BY_COLUMN, 0), SQL_HANDLE_STMT, stmt,
                   "SQLSetStmtAttr PARAM_BIND_TYPE");
  check_odbc_error(
      SQLSetStmtAttr(stmt, SQL_ATTR_PARAMSET_SIZE, reinterpret_cast<SQLPOINTER>(static_cast<SQLULEN>(row_count)), 0),
      SQL_HANDLE_STMT, stmt, "SQLSetStmtAttr PARAMSET_SIZE");
  for (std::size_t col = 0; col < col_count; ++col) {
    check_odbc_error(SQLBindParameter(stmt, static_cast<SQLUSMALLINT>(col + 1), SQL_PARAM_INPUT, SQL_C_CHAR,
                                      SQL_VARCHAR, columns[col].width, 0, columns[col].data.data(),
                                      static_cast<SQLLEN>(columns[col].width), columns[col].lengths.data()),
                     SQL_HANDLE_STMT, stmt, "SQLBindParameter executemany");
  }
  auto bind_end = std::chrono::steady_clock::now();

  check_odbc_error(SQLExecute(stmt), SQL_HANDLE_STMT, stmt, "SQLExecute executemany");

  TestResult result{};
  result.timestamp_ms =
      std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::system_clock::now().time_since_epoch())
          .count();
  result.fetch.query_time_s = std::chrono::duration<double>(bind_end - bind_start).count();
  result.fetch.fetch_time_s = 0.0;
  result.fetch.row_count = via_wiremock_replay() ? rows.size() : inserted_row_count(dbc);
  result.fetch.cpu_time_s = 0.0;
  result.peak_rss_mb = get_peak_rss_mb();
  return result;
}

void validate_row_counts(const std::vector<TestResult>& results, std::size_t expected) {
  for (std::size_t i = 0; i < results.size(); ++i) {
    if (results[i].fetch.row_count != expected) {
      throw std::runtime_error("Row count mismatch at iteration " + std::to_string(i));
    }
  }
  std::cout << "✓ All " << results.size() << " iterations returned " << expected << " rows\n";
}

void print_statistics(const std::vector<TestResult>& results) {
  std::vector<double> query_times;
  std::vector<double> fetch_times;
  query_times.reserve(results.size());
  fetch_times.reserve(results.size());
  bool any_fetch = false;
  for (const auto& result : results) {
    query_times.push_back(result.fetch.query_time_s);
    fetch_times.push_back(result.fetch.fetch_time_s);
    if (result.fetch.fetch_time_s != 0.0) {
      any_fetch = true;
    }
  }
  std::cout << "\nSummary:\n";
  print_timing_stats("Query", query_times);
  if (any_fetch) {
    print_timing_stats("Fetch", fetch_times);
  }
}

}  // namespace

void execute_binding_test(SQLHDBC dbc, const std::string& sql_command, int warmup_iterations, int iterations,
                          const std::string& test_name, const std::string& driver_type_str,
                          const std::string& driver_version_str, time_t now) {
  std::string binding_mode = get_env_optional("BINDING_MODE", "execute");
  std::string binding_params_json = get_env_required("BINDING_PARAMS_JSON");
  json raw = json::parse(binding_params_json);

  std::cout << "\n=== Executing Parameter Binding Test ===\n";
  std::cout << "Query: " << sql_command << "\n";
  std::cout << "Binding mode: " << binding_mode << "\n";

  std::size_t expected_rows = 1;
  if (binding_mode == "execute") {
    if (!raw.is_array()) {
      throw std::runtime_error("BINDING_PARAMS_JSON must be a JSON array for execute mode");
    }
    for (int i = 0; i < warmup_iterations; ++i) {
      (void)run_scalar_query(dbc, sql_command, raw);
    }
    ResourceMonitor monitor(std::chrono::milliseconds(100));
    monitor.start();
    std::vector<TestResult> results;
    for (int i = 0; i < iterations; ++i) {
      results.push_back(run_scalar_query(dbc, sql_command, raw));
    }
    auto memory_timeline = monitor.stop();
    if (auto expected_env = get_env_or_empty("EXPECTED_ROW_COUNT"); !expected_env.empty()) {
      expected_rows = static_cast<std::size_t>(std::stoul(expected_env));
    }
    validate_row_counts(results, expected_rows);
    print_statistics(results);
    std::cout << "  Memory timeline: " << memory_timeline.size() << " samples collected\n";
    std::string filename = generate_results_filename(test_name, driver_type_str, now);
    write_csv_results(results, filename, false);
    write_memory_timeline(memory_timeline, test_name, driver_type_str, now);
    finalize_test_execution(dbc, filename, driver_type_str, driver_version_str, now);
    return;
  }

  if (binding_mode == "executemany") {
    if (!raw.is_array()) {
      throw std::runtime_error("BINDING_PARAMS_JSON must be a JSON array for executemany mode");
    }
    expected_rows = raw.size();
    for (int i = 0; i < warmup_iterations; ++i) {
      truncate_bind_table(dbc);
      (void)run_executemany_insert(dbc, sql_command, raw);
    }
    ResourceMonitor monitor(std::chrono::milliseconds(100));
    monitor.start();
    std::vector<TestResult> results;
    for (int i = 0; i < iterations; ++i) {
      truncate_bind_table(dbc);
      results.push_back(run_executemany_insert(dbc, sql_command, raw));
    }
    auto memory_timeline = monitor.stop();
    if (auto expected_env = get_env_or_empty("EXPECTED_ROW_COUNT"); !expected_env.empty()) {
      expected_rows = static_cast<std::size_t>(std::stoul(expected_env));
    }
    validate_row_counts(results, expected_rows);
    print_statistics(results);
    std::cout << "  Memory timeline: " << memory_timeline.size() << " samples collected\n";
    std::string filename = generate_results_filename(test_name, driver_type_str, now);
    write_csv_results(results, filename, false);
    write_memory_timeline(memory_timeline, test_name, driver_type_str, now);
    finalize_test_execution(dbc, filename, driver_type_str, driver_version_str, now);
    return;
  }

  throw std::runtime_error("Unsupported binding mode: " + binding_mode);
}
