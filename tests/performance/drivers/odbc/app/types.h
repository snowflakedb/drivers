#pragma once

#include <cstddef>
#include <cstdint>

struct TestResult {
  int iteration;
  int64_t timestamp_ms;
  double query_time_s;
  double fetch_time_s;
  std::size_t row_count;
  double cpu_time_s;
  double peak_rss_mb;
};
