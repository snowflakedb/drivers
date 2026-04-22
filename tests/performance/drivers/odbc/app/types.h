#pragma once

#include <cstddef>
#include <cstdint>

struct TestResult {
  int iteration;
  int64_t timestamp_ms;
  double query_time_s;
  double fetch_time_s;
  double core_batch_wait_s = 0.0;
  double core_chunk_download_s = 0.0;
  double core_arrow_decode_s = 0.0;
  double wrapper_time_s = 0.0;
  std::size_t row_count;
  double cpu_time_s;
  double peak_rss_mb;
};
