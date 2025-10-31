#pragma once

struct TestResult {
  int iteration;
  double query_time_s;
  double fetch_time_s;
  int row_count;
};
