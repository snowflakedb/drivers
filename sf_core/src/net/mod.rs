//! Network abstractions for WASM builds.
//!
//! This module provides TCP connectivity for WASM builds using wasi-sockets.
//!
//! Note: TLS is handled by the Go host shim for port 443 connections.
//! The WASM code sends plaintext HTTP, and Go wraps the connection with TLS.
//!
//! Native builds use reqwest directly (with rustls + aws-lc-rs for FIPS).

mod error;
pub use error::TcpConnectorError;

// DNS resolution (uses host function for WASM, std::net for native)
#[cfg(any(feature = "native", feature = "wasm"))]
pub mod dns;

// TCP streams for WASM feature (uses wstd + wasi:sockets)
#[cfg(feature = "wasm")]
mod stream_wasm;

#[cfg(feature = "wasm")]
pub use stream_wasm::{WasmTcpConnector, WasmTcpStream};

// TLS types not available in WASM - requires host-level TLS support
// #[cfg(feature = "wasm")]
// pub use tls_wasm::{CrlCheckMode, WasmTlsConfig, WasmTlsConnector, WasmTlsError, WasmTlsStream};
