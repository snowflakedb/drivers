//! API implementations for database driver protocols.
//!
//! This module contains database driver API implementations.
//!
//! ## Architecture
//!
//! - `unified`: Unified implementation that uses the portable REST client (for WASM)
//! - `database_driver_v1`: Native implementation with full features (async, retries, etc.)
//!
//! For native builds, `database_driver_v1` provides full feature support.
//! For WASM builds, `unified` provides a simpler implementation using portable HTTP.

// Unified implementation (used for WASM builds)
#[cfg(any(feature = "native", feature = "wasm"))]
pub mod unified;

// Native implementation with full features
#[cfg(feature = "native")]
pub mod database_driver_v1;
