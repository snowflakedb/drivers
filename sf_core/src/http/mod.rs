//! HTTP client abstractions and retry logic.
//!
//! This module provides platform-independent HTTP client traits and implementations.

pub mod client;

#[cfg(feature = "native")]
pub mod client_native;

#[cfg(feature = "wasm")]
pub mod client_wasm;

#[cfg(feature = "native")]
pub mod retry;

// Re-export common types
pub use client::{
    Headers, HttpClient, HttpClientConfig, HttpClientError, HttpRequest, HttpResponse, Method,
    StatusCode,
};

// Re-export the appropriate client implementation based on features
#[cfg(feature = "native")]
pub use client_native::NativeHttpClient as DefaultHttpClient;

#[cfg(all(feature = "wasm", not(feature = "native")))]
pub use client_wasm::WasmHttpClient as DefaultHttpClient;
