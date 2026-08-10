// Same reason as `jdbc_bridge` and `nodejs_bridge`: this crate instantiates the
// generated client's ~60 methods against `sf_core`'s transport, so rustc
// computes each operation future's layout from deeper in the query stack than
// `sf_core` does. An operation's `await` chain is ~130 levels on its own, which
// clears the default `recursion_limit` of 128 from here.
//
// See the comment in `jdbc_bridge/src/lib.rs` for why the ceiling is raised
// instead of `Box::pin`ning individual operations in `sf_core` (the deepest
// operation differs per target), and for the two reasons this error is easy to
// miss locally.
#![recursion_limit = "256"]

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
