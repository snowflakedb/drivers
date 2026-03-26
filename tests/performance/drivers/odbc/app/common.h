#pragma once

#include <sys/resource.h>

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

  double median = (values.size() % 2 == 0) ? (values[values.size() / 2 - 1] + values[values.size() / 2]) / 2.0
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

/// CPU time (user + system) for the process from a rusage snapshot.
inline double cpu_seconds(const struct rusage& u) {
  return static_cast<double>(u.ru_utime.tv_sec) + static_cast<double>(u.ru_utime.tv_usec) / 1e6 +
         static_cast<double>(u.ru_stime.tv_sec) + static_cast<double>(u.ru_stime.tv_usec) / 1e6;
}

/// Process peak RSS in MB.  ru_maxrss is KB on Linux, bytes on macOS.
inline double get_peak_rss_mb() {
  struct rusage usage;
  getrusage(RUSAGE_SELF, &usage);
#ifdef __APPLE__
  return static_cast<double>(usage.ru_maxrss) / (1024.0 * 1024.0);
#else
  return static_cast<double>(usage.ru_maxrss) / 1024.0;
#endif
}
