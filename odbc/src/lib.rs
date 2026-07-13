mod api;
pub mod c_api;
mod conversion;
#[cfg(target_os = "windows")]
mod setup_common;
#[cfg(target_os = "windows")]
mod setup_dialog;

/// Internal conversion-pipeline items exposed ONLY under the `bench` feature
/// (and `#[doc(hidden)]`) so the `conversion` criterion bench can drive the
/// otherwise-private fetch path. NOT part of the public API.
#[cfg(feature = "bench")]
#[doc(hidden)]
pub mod bench_support;

extern crate sf_core;
extern crate tracing;
extern crate tracing_subscriber;
// #[macro_use]
// extern crate lazy_static;
extern crate odbc_sys;
