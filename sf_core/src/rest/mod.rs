//! REST API client modules.
//!
//! This module provides REST API client functionality for Snowflake.
//!
//! ## Architecture
//!
//! - `client`: Unified `SnowflakeRestClient` trait for both platforms
//! - `client_native`: Native implementation using reqwest
//! - `client_wasm`: WASM implementation using portable HTTP
//! - `snowflake`: Native-only module with additional features (async queries, etc.)

// Unified REST client trait and types
#[cfg(any(feature = "native", feature = "wasm"))]
pub mod client;

// Platform-specific implementations
#[cfg(feature = "native")]
pub mod client_native;

#[cfg(feature = "wasm")]
pub mod client_wasm;

// Native-only extended functionality
#[cfg(feature = "native")]
pub mod snowflake;

// Re-export common types
#[cfg(any(feature = "native", feature = "wasm"))]
pub use client::{
    BindParameter, ChunkInfo, QueryExecutionMode, QueryResponse, QueryResponseData,
    RestClientError, RowType, SnowflakeRestClient,
};

#[cfg(feature = "native")]
pub use client_native::NativeRestClient;

#[cfg(feature = "wasm")]
pub use client_wasm::WasmRestClient;

/// Create a REST client appropriate for the current platform.
#[cfg(any(feature = "native", feature = "wasm"))]
pub fn create_client(base_url: &str) -> Box<dyn SnowflakeRestClient> {
    #[cfg(feature = "native")]
    {
        Box::new(NativeRestClient::new(base_url))
    }
    #[cfg(all(feature = "wasm", not(feature = "native")))]
    {
        Box::new(WasmRestClient::new(base_url))
    }
}
