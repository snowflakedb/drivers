#pragma once

struct TestResult {
  int iteration;
  time_t timestamp;
  double query_time_s;
  double fetch_time_s;
  int row_count;
};
