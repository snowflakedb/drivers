#include "concurrent_execution.h"

#include <chrono>
#include <condition_variable>
#include <cstdlib>
#include <exception>
#include <future>
#include <iomanip>
#include <iostream>
#include <mutex>
#include <stdexcept>
#include <vector>

#include "common.h"
#include "connection.h"
#include "query_execution.h"
#include "resource_monitor.h"
#include "results.h"

static constexpr auto kBarrierTimeout = std::chrono::seconds(120);

class BurstBarrier {
 public:
  BurstBarrier(int parties, std::chrono::high_resolution_clock::time_point* start)
      : parties_(parties), start_(start), count_(0), generation_(0) {}

  void arrive_and_wait() {
    std::unique_lock<std::mutex> lock(mutex_);
    const int gen = generation_;
    if (++count_ == parties_) {
      *start_ = std::chrono::high_resolution_clock::now();
      count_ = 0;
      ++generation_;
      cv_.notify_all();
      return;
    }
    if (!cv_.wait_for(lock, kBarrierTimeout, [this, gen] { return generation_ != gen; })) {
      throw std::runtime_error("Concurrent burst barrier timed out");
    }
  }

 private:
  int parties_;
  std::chrono::high_resolution_clock::time_point* start_;
  int count_;
  int generation_;
  std::mutex mutex_;
  std::condition_variable cv_;
};

struct WorkerConnections {
  std::vector<SQLHDBC> connections;

  WorkerConnections() = default;
  WorkerConnections(const WorkerConnections&) = delete;
  WorkerConnections& operator=(const WorkerConnections&) = delete;
  WorkerConnections(WorkerConnections&&) = default;
  WorkerConnections& operator=(WorkerConnections&&) = default;

  ~WorkerConnections() {
    for (SQLHDBC dbc : connections) {
      SQLDisconnect(dbc);
      SQLFreeHandle(SQL_HANDLE_DBC, dbc);
    }
  }
};

static WorkerConnections create_worker_connections(SQLHENV env, int worker_count,
                                                   const std::vector<std::string>& setup_queries) {
  WorkerConnections workers;
  workers.connections.reserve(static_cast<std::size_t>(worker_count));
  if (!setup_queries.empty()) {
    std::cout << "Running setup queries on " << worker_count << " worker connections...\n";
  }
  for (int i = 0; i < worker_count; ++i) {
    SQLHDBC dbc = create_connection(env);
    workers.connections.push_back(dbc);
    execute_setup_queries(dbc, setup_queries, false);
  }
  return workers;
}

static TestResult run_burst(const std::vector<SQLHDBC>& connections, const std::string& sql, BindMode bind_mode) {
  const int worker_count = static_cast<int>(connections.size());
  std::chrono::high_resolution_clock::time_point burst_start;

  BurstBarrier barrier(worker_count + 1, &burst_start);

  struct rusage usage_before;
  getrusage(RUSAGE_SELF, &usage_before);

  std::vector<std::future<QueryFetchResult>> futures;
  futures.reserve(static_cast<std::size_t>(worker_count));
  for (int i = 0; i < worker_count; ++i) {
    SQLHDBC dbc = connections[static_cast<std::size_t>(i)];
    futures.push_back(std::async(std::launch::async, [&, dbc]() {
      barrier.arrive_and_wait();
      return run_query_fetch(dbc, sql, bind_mode, nullptr, false);
    }));
  }

  barrier.arrive_and_wait();

  std::vector<QueryFetchResult> worker_results;
  worker_results.reserve(static_cast<std::size_t>(worker_count));
  std::exception_ptr first_error;
  for (auto& future : futures) {
    try {
      worker_results.push_back(future.get());
    } catch (...) {
      if (!first_error) {
        first_error = std::current_exception();
      }
    }
  }
  if (first_error) {
    std::rethrow_exception(first_error);
  }

  const auto burst_end = std::chrono::high_resolution_clock::now();
  const double burst_wall_s = std::chrono::duration<double>(burst_end - burst_start).count();

  const std::size_t per_worker_rows = worker_results[0].row_count;
  for (std::size_t i = 1; i < worker_results.size(); ++i) {
    if (worker_results[i].row_count != per_worker_rows) {
      std::cerr << "ERROR: Workers returned unequal row counts\n";
      exit(1);
    }
  }
  const std::size_t total_rows = per_worker_rows * static_cast<std::size_t>(worker_count);

  struct rusage usage_after;
  getrusage(RUSAGE_SELF, &usage_after);

  TestResult result;
  result.timestamp_ms =
      std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::system_clock::now().time_since_epoch())
          .count();
  result.fetch.query_time_s = burst_wall_s;
  result.fetch.fetch_time_s = burst_wall_s;
  result.fetch.row_count = total_rows;
  result.fetch.cpu_time_s = cpu_seconds(usage_after) - cpu_seconds(usage_before);
  result.peak_rss_mb = get_peak_rss_mb();
  result.worker_count = worker_count;
  result.throughput_rows_s = burst_wall_s > 0.0 ? static_cast<double>(total_rows) / burst_wall_s : 0.0;
  return result;
}

static void validate_concurrent_row_counts(const std::vector<TestResult>& results, int worker_count) {
  if (results.empty()) {
    return;
  }

  const std::size_t expected = results[0].fetch.row_count;
  if (expected == 0) {
    std::cerr << "ERROR: Row count baseline is 0 — refusing to use 0 as a concurrent-burst baseline.\n";
    exit(1);
  }

  const std::size_t per_worker = expected / static_cast<std::size_t>(worker_count);
  for (std::size_t i = 0; i < results.size(); ++i) {
    if (results[i].fetch.row_count != expected) {
      std::cerr << "ERROR: Row count mismatch: iteration " << i << " returned " << results[i].fetch.row_count
                << " rows, expected " << expected << " (" << worker_count << " workers × " << per_worker << ")\n";
      exit(1);
    }
  }

  std::cout << "✓ All " << results.size() << " bursts returned " << expected << " rows (" << worker_count << " × "
            << per_worker << ")\n";
}

static void print_concurrent_statistics(const std::vector<TestResult>& results) {
  if (results.empty()) {
    return;
  }

  std::vector<double> burst_times;
  std::vector<double> throughputs;
  burst_times.reserve(results.size());
  throughputs.reserve(results.size());
  for (const auto& result : results) {
    burst_times.push_back(result.fetch.query_time_s);
    throughputs.push_back(result.throughput_rows_s);
  }

  std::cout << "\nSummary:\n";
  print_timing_stats("Burst wall", burst_times);

  auto throughput_stats = calculate_stats(throughputs);
  std::cout << "  Throughput: median=" << std::fixed << std::setprecision(0) << throughput_stats.median
            << " rows/s  min=" << throughput_stats.min << "  max=" << throughput_stats.max << "\n";
}

void execute_concurrent_test(SQLHENV env, SQLHDBC setup_dbc, const std::string& sql_command, int warmup_iterations,
                             int iterations, int worker_count, const std::vector<std::string>& setup_queries,
                             const std::string& test_name, const std::string& driver_type_str,
                             const std::string& driver_version_str, time_t now) {
  if (worker_count < 1) {
    std::cerr << "ERROR: WORKER_COUNT must be >= 1, got " << worker_count << "\n";
    exit(1);
  }

  BindMode bind_mode = resolve_bind_mode();
  std::cout << "\n=== Executing Concurrent SELECT Test (bulk fetch" << ", bind=" << bind_mode_label(bind_mode)
            << ") ===\n";
  std::cout << "Query: " << sql_command << "\n";
  std::cout << "Workers: " << worker_count << " connections (one statement per connection)\n";

  std::cout << "Opening " << worker_count << " worker connections (excluded from burst timing)...\n";
  WorkerConnections worker_connections = create_worker_connections(env, worker_count, setup_queries);
  std::cout << "✓ Worker connections ready\n";

  for (int i = 1; i <= warmup_iterations; ++i) {
    std::cout << "  Warmup burst " << i << "/" << warmup_iterations << "\n";
    run_burst(worker_connections.connections, sql_command, bind_mode);
  }

  ResourceMonitor monitor(std::chrono::milliseconds(100));
  monitor.start();

  std::vector<TestResult> results;
  results.reserve(static_cast<std::size_t>(iterations));
  for (int i = 1; i <= iterations; ++i) {
    results.push_back(run_burst(worker_connections.connections, sql_command, bind_mode));
    const auto& result = results.back();
    std::cout << "  Iteration " << i << "/" << iterations << ": burst=" << std::fixed << std::setprecision(3)
              << result.fetch.query_time_s << "s  throughput=" << std::setprecision(0) << result.throughput_rows_s
              << " rows/s  rows=" << result.fetch.row_count << "\n";
  }

  auto memory_timeline = monitor.stop();

  validate_concurrent_row_counts(results, worker_count);

  std::string filename = generate_results_filename(test_name, driver_type_str, now);
  write_csv_results(results, filename, false);
  write_memory_timeline(memory_timeline, test_name, driver_type_str, now);

  print_concurrent_statistics(results);
  std::cout << "  Memory timeline: " << memory_timeline.size() << " samples collected\n";

  finalize_test_execution(setup_dbc, filename, driver_type_str, driver_version_str, now);
}
