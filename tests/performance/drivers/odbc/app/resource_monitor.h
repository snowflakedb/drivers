#pragma once

#include <atomic>
#include <chrono>
#include <cstdint>
#include <vector>

#ifdef __linux__
#include <unistd.h>

#include <fstream>
#include <thread>
#endif

struct MemorySample {
  uint64_t timestamp_ms;
  uint64_t rss_bytes;
  uint64_t vm_bytes;
};

/// Background thread that samples RSS/VM from /proc/self/statm (Linux only).
/// On non-Linux platforms start()/stop() are no-ops returning an empty timeline.
class ResourceMonitor {
 public:
  explicit ResourceMonitor(std::chrono::milliseconds interval = std::chrono::milliseconds(100))
      : interval_(interval), stop_(false) {}

  void start() {
#ifdef __linux__
    stop_.store(false, std::memory_order_relaxed);
    samples_.clear();
    thread_ = std::thread([this]() {
      while (!stop_.load(std::memory_order_relaxed)) {
        take_sample();
        std::this_thread::sleep_for(interval_);
      }
      take_sample();
    });
#endif
  }

  std::vector<MemorySample> stop() {
#ifdef __linux__
    stop_.store(true, std::memory_order_relaxed);
    if (thread_.joinable()) thread_.join();
    return std::move(samples_);
#else
    return {};
#endif
  }

 private:
#ifdef __linux__
  void take_sample() {
    auto [rss, vm] = read_statm();
    samples_.push_back({now_ms(), rss, vm});
  }

  static std::pair<uint64_t, uint64_t> read_statm() {
    std::ifstream f("/proc/self/statm");
    uint64_t vm_pages = 0, rss_pages = 0;
    if (f.is_open()) {
      f >> vm_pages >> rss_pages;
    }
    static const uint64_t page_size = static_cast<uint64_t>(sysconf(_SC_PAGE_SIZE));
    return {rss_pages * page_size, vm_pages * page_size};
  }

  static uint64_t now_ms() {
    return static_cast<uint64_t>(
        std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::system_clock::now().time_since_epoch())
            .count());
  }
#endif

  std::chrono::milliseconds interval_;
  std::atomic<bool> stop_;
#ifdef __linux__
  std::thread thread_;
#endif
  std::vector<MemorySample> samples_;
};
