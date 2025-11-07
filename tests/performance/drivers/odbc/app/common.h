#pragma once

#include <algorithm>
#include <iomanip>
#include <iostream>
#include <string>
#include <vector>

struct TimingStats {
  double median;
  double min;
  double max;
};

inline TimingStats calculate_stats(std::vector<double> values) {
  if (values.empty()) {
    return {0.0, 0.0, 0.0};
  }

  std::sort(values.begin(), values.end());

  double median = (values.size() % 2 == 0)
                      ? (values[values.size() / 2 - 1] + values[values.size() / 2]) / 2.0
                      : values[values.size() / 2];

  double min_val = values.front();
  double max_val = values.back();

  return {median, min_val, max_val};
}

inline void print_timing_stats(const std::string& label, const std::vector<double>& values) {
  auto stats = calculate_stats(values);
  std::cout << "  " << label << ": median=" << std::fixed << std::setprecision(3) << stats.median
            << "s  min=" << stats.min << "s  max=" << stats.max << "s\n";
}
