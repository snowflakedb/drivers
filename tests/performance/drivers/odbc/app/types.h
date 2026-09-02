#pragma once

#include <cstddef>
#include <cstdint>

struct QueryFetchResult {
  double query_time_s = 0.0;
  double fetch_time_s = 0.0;
  std::size_t row_count = 0;
  double cpu_time_s = 0.0;
  double core_batch_wait_s = 0.0;
  double core_chunk_download_s = 0.0;
  double core_arrow_decode_s = 0.0;
  double wrapper_time_s = 0.0;
};

struct TestResult {
  int iteration = 0;
  int64_t timestamp_ms = 0;
  QueryFetchResult fetch;
  double peak_rss_mb = 0.0;
  int worker_count = 0;
  double throughput_rows_s = 0.0;
};
