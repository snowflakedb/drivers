#pragma once

#include <cstddef>
#include <ctime>

struct TestResult {
  int iteration;
  time_t timestamp;
  double query_time_s;
  double fetch_time_s;
  std::size_t row_count;
  double cpu_time_s;
  double peak_rss_mb;
};
