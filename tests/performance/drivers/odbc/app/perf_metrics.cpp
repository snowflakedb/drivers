#include "perf_metrics.h"

#include <dlfcn.h>

CoreInstrumentation::CoreInstrumentation() {
  handle_ = dlopen("libsfodbc.so", RTLD_LAZY | RTLD_NOLOAD);
  if (!handle_) {
    return;
  }

  get_perf_data_ = reinterpret_cast<GetPerfDataFn>(dlsym(handle_, "sf_core_get_perf_data"));
  reset_metrics_ = reinterpret_cast<ResetMetricsFn>(dlsym(handle_, "sf_core_reset_perf_metrics"));
  auto perf_enabled = reinterpret_cast<PerfEnabledFn>(dlsym(handle_, "sf_core_perf_enabled"));

  if (!get_perf_data_ || !reset_metrics_ || !perf_enabled) {
    get_perf_data_ = nullptr;
    reset_metrics_ = nullptr;
    dlclose(handle_);
    handle_ = nullptr;
    return;
  }

  enabled_ = perf_enabled();
  if (!enabled_) {
    get_perf_data_ = nullptr;
    reset_metrics_ = nullptr;
    dlclose(handle_);
    handle_ = nullptr;
  }
}

CoreInstrumentation::~CoreInstrumentation() {
  if (handle_) {
    dlclose(handle_);
  }
}

bool CoreInstrumentation::available() const { return handle_ != nullptr; }

void CoreInstrumentation::reset() {
  if (reset_metrics_) {
    reset_metrics_();
  }
}

CoreInstrumentationData CoreInstrumentation::collect() {
  if (!get_perf_data_) {
    return {};
  }

  SfCorePerfData raw = get_perf_data_();
  CoreInstrumentationData data{};
  data.core_batch_wait_s = static_cast<double>(raw.core_batch_wait_ns) / 1e9;
  data.core_chunk_download_s = static_cast<double>(raw.core_chunk_download_ns) / 1e9;
  data.core_arrow_decode_s = static_cast<double>(raw.core_arrow_decode_ns) / 1e9;
  return data;
}
