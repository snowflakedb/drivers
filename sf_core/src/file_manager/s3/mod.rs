//! S3 client abstraction for platform-independent S3 operations.
#![allow(dead_code)]
//!
//! This module provides traits for S3 operations that can be implemented
//! by different backends (aws-sdk-s3 for native builds, wasi-http for WASM).

use snafu::{Location, Snafu};
use std::collections::HashMap;

#[cfg(feature = "native")]
pub mod aws_sdk;

#[cfg(feature = "wasm")]
pub mod wasi;

// Re-export the appropriate implementation based on features
#[cfg(feature = "native")]
pub use aws_sdk::AwsSdkS3Client as DefaultS3Client;

#[cfg(all(feature = "wasm", not(feature = "native")))]
pub use wasi::WasiS3Client as DefaultS3Client;

/// Credentials for S3 authentication.
#[derive(Debug, Clone)]
pub struct S3Credentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: Option<String>,
}

/// Configuration for S3 client.
#[derive(Debug, Clone)]
pub struct S3Config {
    pub region: String,
    pub credentials: S3Credentials,
    pub endpoint: Option<String>,
}

/// Result of a GET object operation.
#[derive(Debug)]
pub struct S3Object {
    pub body: Vec<u8>,
    pub metadata: HashMap<String, String>,
    pub content_length: Option<i64>,
    pub content_type: Option<String>,
}

/// Errors that can occur during S3 operations.
#[derive(Debug, Snafu)]
pub enum S3Error {
    #[snafu(display("Failed to upload object: {message}"))]
    Upload {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to download object: {message}"))]
    Download {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to check object existence: {message}"))]
    HeadObject {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Object not found: {key}"))]
    NotFound {
        key: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Access denied: {message}"))]
    AccessDenied {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Invalid credentials: {message}"))]
    InvalidCredentials {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Connection error: {message}"))]
    Connection {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Serialization error: {message}"))]
    Serialization {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },
}

/// Trait for S3 client operations.
///
/// This trait abstracts over different S3 client implementations,
/// allowing the same code to work with aws-sdk-s3 (native) or
/// a custom implementation using wasi-http (WASM).
pub trait S3Client: Send + Sync {
    /// Upload an object to S3.
    ///
    /// # Arguments
    /// * `bucket` - The S3 bucket name
    /// * `key` - The object key (path)
    /// * `body` - The object content
    /// * `content_type` - Optional content type
    /// * `metadata` - Optional metadata key-value pairs
    fn put_object(
        &self,
        bucket: &str,
        key: &str,
        body: Vec<u8>,
        content_type: Option<&str>,
        metadata: HashMap<String, String>,
    ) -> impl std::future::Future<Output = Result<(), S3Error>> + Send;

    /// Download an object from S3.
    ///
    /// # Arguments
    /// * `bucket` - The S3 bucket name
    /// * `key` - The object key (path)
    fn get_object(
        &self,
        bucket: &str,
        key: &str,
    ) -> impl std::future::Future<Output = Result<S3Object, S3Error>> + Send;

    /// Check if an object exists in S3.
    ///
    /// # Arguments
    /// * `bucket` - The S3 bucket name
    /// * `key` - The object key (path)
    ///
    /// # Returns
    /// * `Ok(true)` if the object exists
    /// * `Ok(false)` if the object does not exist
    /// * `Err(_)` on error
    fn head_object(
        &self,
        bucket: &str,
        key: &str,
    ) -> impl std::future::Future<Output = Result<bool, S3Error>> + Send;
}

/// Create a default S3 client with the given configuration.
pub fn create_s3_client(config: S3Config) -> DefaultS3Client {
    DefaultS3Client::new(config)
}
