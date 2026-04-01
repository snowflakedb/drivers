//! Performance timing using a custom [`tracing::Layer`].
//!
//! The `perf_timing` Cargo feature controls whether [`PerfTimingLayer`] and
//! the counter-accumulation implementation are compiled in. The public and
//! FFI-facing perf accessors remain available regardless of feature state so
//! callers can always link against the same symbols. When the feature is
//! **off**, the layer doesn't exist, `get_perf_data` returns zeroes, and reset
//! operations are no-ops. When **on**, the layer intercepts perf spans and
//! accumulates wall-clock durations into atomic counters.

pub mod c_api;
// ---------------------------------------------------------------------------
// Feature-gated implementation
// ---------------------------------------------------------------------------

#[cfg(feature = "perf_timing")]
mod inner {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, OnceLock};
    use std::time::Instant;

    use tracing::Subscriber;
    use tracing::span::{Attributes, Id};
    use tracing_subscriber::Layer;
    use tracing_subscriber::layer::Context;
    use tracing_subscriber::registry::LookupSpan;

    static SHARED_COUNTERS: OnceLock<Arc<PerfCounters>> = OnceLock::new();

    pub struct PerfCounters {
        core_batch_wait_ns: AtomicU64,
        core_chunk_download_ns: AtomicU64,
        core_arrow_decode_ns: AtomicU64,
    }

    impl Default for PerfCounters {
        fn default() -> Self {
            Self::new()
        }
    }

    impl PerfCounters {
        pub fn new() -> Self {
            Self {
                core_batch_wait_ns: AtomicU64::new(0),
                core_chunk_download_ns: AtomicU64::new(0),
                core_arrow_decode_ns: AtomicU64::new(0),
            }
        }

        fn accumulate(&self, span_name: &str, nanos: u64) {
            let counter = match span_name {
                "core_batch_wait" => &self.core_batch_wait_ns,
                "core_chunk_download" => &self.core_chunk_download_ns,
                "core_arrow_decode" => &self.core_arrow_decode_ns,
                _ => return,
            };
            counter.fetch_add(nanos, Ordering::Relaxed);
        }

        pub fn get_data(&self) -> super::CoreInstrumentationData {
            super::CoreInstrumentationData {
                core_batch_wait_ns: self.core_batch_wait_ns.swap(0, Ordering::Relaxed),
                core_chunk_download_ns: self.core_chunk_download_ns.swap(0, Ordering::Relaxed),
                core_arrow_decode_ns: self.core_arrow_decode_ns.swap(0, Ordering::Relaxed),
            }
        }

        pub fn reset(&self) {
            self.core_batch_wait_ns.store(0, Ordering::Relaxed);
            self.core_chunk_download_ns.store(0, Ordering::Relaxed);
            self.core_arrow_decode_ns.store(0, Ordering::Relaxed);
        }
    }

    struct SpanCreatedAt(Instant);

    pub struct PerfTimingLayer {
        counters: Arc<PerfCounters>,
    }

    impl<S> Layer<S> for PerfTimingLayer
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
            if attrs.metadata().target() != super::PERF_TARGET {
                return;
            }
            if let Some(span) = ctx.span(id) {
                span.extensions_mut().insert(SpanCreatedAt(Instant::now()));
            }
        }

        fn on_close(&self, id: Id, ctx: Context<'_, S>) {
            let Some(span) = ctx.span(&id) else { return };
            let ext = span.extensions();
            let Some(created) = ext.get::<SpanCreatedAt>() else {
                return;
            };
            let nanos = created.0.elapsed().as_nanos() as u64;
            if nanos > 0 {
                self.counters.accumulate(span.name(), nanos);
            }
        }
    }

    pub fn create_perf_layer() -> PerfTimingLayer {
        let counters = Arc::clone(SHARED_COUNTERS.get_or_init(|| Arc::new(PerfCounters::new())));
        PerfTimingLayer { counters }
    }

    pub fn get_perf_data() -> super::CoreInstrumentationData {
        SHARED_COUNTERS
            .get()
            .map(|c| c.get_data())
            .unwrap_or_default()
    }

    pub fn reset_perf_counters() {
        if let Some(c) = SHARED_COUNTERS.get() {
            c.reset();
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn counters_accumulate_read_and_reset() {
            let counters = PerfCounters::new();

            counters.accumulate("core_batch_wait", 100_000_000);
            counters.accumulate("core_chunk_download", 200_000_000);
            counters.accumulate("core_arrow_decode", 50_000_000);
            counters.accumulate("core_batch_wait", 150_000_000);

            let data = counters.get_data();
            assert_eq!(data.core_batch_wait_ns, 250_000_000);
            assert_eq!(data.core_chunk_download_ns, 200_000_000);
            assert_eq!(data.core_arrow_decode_ns, 50_000_000);

            let data2 = counters.get_data();
            assert_eq!(
                data2.core_batch_wait_ns, 0,
                "should be zero after read-and-reset"
            );
            assert_eq!(data2.core_chunk_download_ns, 0);
            assert_eq!(data2.core_arrow_decode_ns, 0);
        }

        #[test]
        fn explicit_reset_clears_counters() {
            let counters = PerfCounters::new();
            counters.accumulate("core_batch_wait", 300_000_000);
            counters.reset();

            let data = counters.get_data();
            assert_eq!(data.core_batch_wait_ns, 0);
        }

        #[test]
        fn unknown_span_name_ignored() {
            let counters = PerfCounters::new();
            counters.accumulate("unknown_metric", 999);

            let data = counters.get_data();
            assert_eq!(data.core_batch_wait_ns, 0);
            assert_eq!(data.core_chunk_download_ns, 0);
            assert_eq!(data.core_arrow_decode_ns, 0);
        }
    }
}

// ---------------------------------------------------------------------------
// Public API — always available; delegates to `inner` or returns no-ops
// ---------------------------------------------------------------------------

/// Target string used by perf span sites. Always available so instrumentation
/// sites don't need `#[cfg]` guards — when `perf_timing` is off the spans are
/// still created but no layer listens, making them effectively free (~1-2ns).
pub const PERF_TARGET: &str = "sf_core::perf";

#[cfg(feature = "perf_timing")]
pub use inner::{PerfTimingLayer, create_perf_layer};

/// FFI-safe struct returned by [`get_perf_data`]. All durations are in
/// nanoseconds; callers convert to seconds as needed.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct CoreInstrumentationData {
    pub core_batch_wait_ns: u64,
    pub core_chunk_download_ns: u64,
    pub core_arrow_decode_ns: u64,
}

/// Atomically read-and-reset all counters, returning a struct.
/// Returns zeroed struct when the `perf_timing` feature is disabled.
pub fn get_perf_data() -> CoreInstrumentationData {
    #[cfg(feature = "perf_timing")]
    {
        inner::get_perf_data()
    }
    #[cfg(not(feature = "perf_timing"))]
    {
        CoreInstrumentationData::default()
    }
}

/// Reset all accumulated counters without returning values.
pub fn reset_perf_counters() {
    #[cfg(feature = "perf_timing")]
    inner::reset_perf_counters();
}

/// Returns `true` when the `perf_timing` feature is compiled in.
pub fn perf_enabled() -> bool {
    cfg!(feature = "perf_timing")
}
