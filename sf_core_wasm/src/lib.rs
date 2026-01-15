//! WASM component wrapper for sf_core.
//!
//! This crate provides a WASM component that exposes the sf_core driver
//! functionality via the WIT-defined interface.

#![allow(unused)]

// Generate bindings from WIT
wit_bindgen::generate!({
    world: "driver",
    path: "wit",
});

use exports::snowflake::driver::api::{ApiResult, Guest};
use proto_utils::ProtoError;
use sf_core::protobuf_apis::call_proto;

/// Implementation of the driver API.
struct DriverComponent;

impl Guest for DriverComponent {
    /// Call a protobuf API method.
    fn api_call(api: String, method: String, request: Vec<u8>) -> ApiResult {
        match call_proto(&api, &method, &request) {
            Ok(response) => ApiResult::Ok(response),
            Err(ProtoError::Application(error_bytes)) => ApiResult::ApplicationError(error_bytes),
            Err(ProtoError::Transport(message)) => ApiResult::TransportError(message),
        }
    }

    /// Initialize the logger.
    fn init_logger(level: u32) -> u32 {
        // TODO: Initialize logging for WASM
        // For now, just return success
        0
    }

    /// Get the driver version.
    fn get_version() -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }
}

// Export the component implementation
export!(DriverComponent);

#[cfg(test)]
mod tests {
    #[test]
    fn test_version() {
        assert!(!env!("CARGO_PKG_VERSION").is_empty());
    }
}
