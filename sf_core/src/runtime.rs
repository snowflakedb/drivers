//! Runtime abstraction for async code execution.
//!
//! This module provides a platform-independent way to run async code.
//! - Native builds use tokio
//! - WASM builds use wstd (requires WASI Preview 2)

/// Run an async block to completion.
///
/// This function blocks the current thread until the future completes.
#[cfg(feature = "native")]
pub fn block_on<F, T>(future: F) -> T
where
    F: std::future::Future<Output = T>,
{
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    rt.block_on(future)
}

#[cfg(all(feature = "wasm", not(feature = "native")))]
pub fn block_on<F, T>(future: F) -> T
where
    F: std::future::Future<Output = T>,
{
    wstd::runtime::block_on(future)
}
