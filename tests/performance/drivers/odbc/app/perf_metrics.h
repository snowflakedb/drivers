#pragma once

#include <cstdint>

/// Mirrors #[repr(C)] CoreInstrumentationData from sf_core::perf_timing.
/// All durations are nanoseconds; callers convert to seconds as needed.
struct SfCorePerfData {
  uint64_t core_batch_wait_ns = 0;
  uint64_t core_chunk_download_ns = 0;
  uint64_t core_arrow_decode_ns = 0;
};

struct CoreInstrumentationData {
  double core_batch_wait_s = 0.0;
  double core_chunk_download_s = 0.0;
  double core_arrow_decode_s = 0.0;
};

/// Dynamically resolves sf_core perf symbols from the already-loaded ODBC
/// driver (via dlopen RTLD_NOLOAD) and provides a simple interface for
/// resetting / reading counters.
/// Reports available only when the driver was built with perf_timing enabled
/// (sf_core_perf_enabled returns true), not merely when the symbols exist.
class CoreInstrumentation {
 public:
  CoreInstrumentation();
  ~CoreInstrumentation();

  CoreInstrumentation(const CoreInstrumentation&) = delete;
  CoreInstrumentation& operator=(const CoreInstrumentation&) = delete;

  bool available() const;
  void reset();
  CoreInstrumentationData collect();

 private:
  using GetPerfDataFn = SfCorePerfData (*)();
  using ResetMetricsFn = void (*)();
  using PerfEnabledFn = bool (*)();

  void* handle_ = nullptr;
  GetPerfDataFn get_perf_data_ = nullptr;
  ResetMetricsFn reset_metrics_ = nullptr;
  bool enabled_ = false;
};
