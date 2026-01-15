//! Protobuf API handlers for database driver protocol.
//!
//! This module provides the protobuf-based RPC interface for the database driver.
//!
//! ## Architecture
//!
//! - `unified`: Unified handler using the portable REST client (for WASM builds)
//! - `database_driver_v1`: Native handler with full features (async queries, retries, etc.)
//!
//! For native builds, we use `database_driver_v1` for full feature support.
//! For WASM builds, we use `unified` which works with the portable HTTP client.

// Native imports
#[cfg(feature = "native")]
use crate::protobuf_apis::database_driver_v1::DatabaseDriverImpl;
#[cfg(feature = "native")]
use crate::protobuf_gen::database_driver_v1::DatabaseDriverServer;

// WASM imports (using unified)
#[cfg(all(feature = "wasm", not(feature = "native")))]
use crate::protobuf_apis::unified::UnifiedDatabaseDriverImpl;
#[cfg(all(feature = "wasm", not(feature = "native")))]
use crate::protobuf_gen::database_driver_v1::DatabaseDriverServer;

use proto_utils::*;

// Shared type conversions
#[cfg(any(feature = "native", feature = "wasm"))]
mod conversions;

// Unified handler (used for WASM builds)
#[cfg(any(feature = "native", feature = "wasm"))]
pub mod unified;

// Native handler with full features
#[cfg(feature = "native")]
pub mod database_driver_v1;

/// Call a protobuf API method.
///
/// For native builds, uses `database_driver_v1` for full feature support.
/// For WASM builds, uses the `unified` handler with the portable REST client.
pub fn call_proto(api: &str, method: &str, message: &[u8]) -> Result<Vec<u8>, ProtoError<Vec<u8>>> {
    #[cfg(feature = "native")]
    {
        match api {
            "DatabaseDriver" => DatabaseDriverImpl::handle_message(method, message.to_vec()),
            _ => Err(ProtoError::Transport(format!("Unknown API: {}", api))),
        }
    }

    #[cfg(all(feature = "wasm", not(feature = "native")))]
    {
        match api {
            "DatabaseDriver" => UnifiedDatabaseDriverImpl::handle_message(method, message.to_vec()),
            _ => Err(ProtoError::Transport(format!("Unknown API: {}", api))),
        }
    }

    #[cfg(not(any(feature = "native", feature = "wasm")))]
    {
        let _ = (api, method, message);
        Err(ProtoError::Transport(
            "No driver implementation available".to_string(),
        ))
    }
}

pub struct RustTransport {}

impl Transport for RustTransport {
    fn handle_message(
        service: &str,
        method: &str,
        message: Vec<u8>,
    ) -> Result<Vec<u8>, ProtoError<Vec<u8>>> {
        call_proto(service, method, &message)
    }
}
