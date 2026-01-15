//! Snowflake Universal Driver Core Library
//!
//! This library provides the core functionality for the Snowflake database driver.
//! It supports both native builds (with full functionality) and WASM builds
//! (with limited functionality).
//!
//! # Features
//!
//! - `native` (default): Enables native-only functionality including TLS, HTTP client,
//!   file transfers, and full cryptographic operations using FIPS-compliant libraries.
//!
//! - `wasm`: Enables WASM-compatible functionality using pure-Rust crypto implementations.

extern crate tracing;
extern crate tracing_subscriber;

pub mod apis;

pub mod arrow_utils;
#[cfg(feature = "wasm")]
pub mod arrow_wasm;
mod auth;
pub mod c_api;
#[cfg(feature = "native")]
mod chunks;
mod compression;
mod compression_types;
pub mod config;
pub mod crl;
pub mod crypto;
mod file_manager;
pub mod handle_manager;
#[cfg(any(feature = "native", feature = "wasm"))]
pub mod http;
pub mod logging;
#[cfg(any(feature = "native", feature = "wasm"))]
pub mod net;
pub mod protobuf_apis;
pub mod protobuf_gen;
pub mod query_types;
pub mod rest;
#[cfg(any(feature = "native", feature = "wasm"))]
pub mod runtime;
pub mod tls;
