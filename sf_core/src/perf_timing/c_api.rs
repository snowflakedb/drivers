/// Returns `true` when the `perf_timing` feature was compiled in.
#[unsafe(no_mangle)]
pub extern "C" fn sf_core_perf_enabled() -> bool {
    super::perf_enabled()
}

/// Atomically read-and-reset all perf counters into a flat C struct.
/// No heap allocation, no string to free.
#[unsafe(no_mangle)]
pub extern "C" fn sf_core_get_perf_data() -> super::CoreInstrumentationData {
    super::get_perf_data()
}

/// Reset all accumulated performance metrics.
#[unsafe(no_mangle)]
pub extern "C" fn sf_core_reset_perf_metrics() {
    super::reset_perf_counters();
}
