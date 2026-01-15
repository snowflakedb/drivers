//! AWS SDK S3 client implementation.
#![allow(dead_code)]
//!
//! This module provides the aws-sdk-s3-based S3 client for native builds.

use super::{S3Client, S3Config, S3Credentials, S3Error, S3Object};
use aws_config::{BehaviorVersion, Region};
use aws_credential_types::Credentials;
use aws_sdk_s3::Client;
use aws_sdk_s3::primitives::ByteStream;
use std::collections::HashMap;

/// AWS SDK S3 client.
pub struct AwsSdkS3Client {
    config: S3Config,
}

impl AwsSdkS3Client {
    /// Create a new AWS SDK S3 client with the given configuration.
    pub fn new(config: S3Config) -> Self {
        Self { config }
    }

    /// Create the internal AWS S3 client.
    async fn create_client(&self) -> Client {
        let credentials = Credentials::new(
            &self.config.credentials.access_key_id,
            &self.config.credentials.secret_access_key,
            self.config.credentials.session_token.clone(),
            None,
            "sf_core",
        );

        let config = aws_config::defaults(BehaviorVersion::latest())
            .credentials_provider(credentials)
            .region(Region::new(self.config.region.clone()))
            .load()
            .await;

        Client::new(&config)
    }
}

impl S3Client for AwsSdkS3Client {
    async fn put_object(
        &self,
        bucket: &str,
        key: &str,
        body: Vec<u8>,
        content_type: Option<&str>,
        metadata: HashMap<String, String>,
    ) -> Result<(), S3Error> {
        let client = self.create_client().await;

        let mut request = client
            .put_object()
            .bucket(bucket)
            .key(key)
            .body(ByteStream::from(body));

        if let Some(ct) = content_type {
            request = request.content_type(ct);
        }

        for (k, v) in metadata {
            request = request.metadata(k, v);
        }

        request.send().await.map_err(|e| S3Error::Upload {
            message: e.to_string(),
            location: snafu::Location::new(file!(), line!(), column!()),
        })?;

        Ok(())
    }

    async fn get_object(&self, bucket: &str, key: &str) -> Result<S3Object, S3Error> {
        let client = self.create_client().await;

        let response = client
            .get_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| {
                let err_str = e.to_string();
                if err_str.contains("NoSuchKey") || err_str.contains("404") {
                    S3Error::NotFound {
                        key: key.to_string(),
                        location: snafu::Location::new(file!(), line!(), column!()),
                    }
                } else if err_str.contains("AccessDenied") || err_str.contains("403") {
                    S3Error::AccessDenied {
                        message: err_str,
                        location: snafu::Location::new(file!(), line!(), column!()),
                    }
                } else {
                    S3Error::Download {
                        message: err_str,
                        location: snafu::Location::new(file!(), line!(), column!()),
                    }
                }
            })?;

        // Extract metadata
        let metadata = response
            .metadata()
            .map(|m| {
                m.iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let content_length = response.content_length();
        let content_type = response.content_type().map(|s| s.to_string());

        // Read body
        let body = response
            .body
            .collect()
            .await
            .map_err(|e| S3Error::Download {
                message: e.to_string(),
                location: snafu::Location::new(file!(), line!(), column!()),
            })?
            .into_bytes()
            .to_vec();

        Ok(S3Object {
            body,
            metadata,
            content_length,
            content_type,
        })
    }

    async fn head_object(&self, bucket: &str, key: &str) -> Result<bool, S3Error> {
        let client = self.create_client().await;

        match client.head_object().bucket(bucket).key(key).send().await {
            Ok(_) => Ok(true),
            Err(e) => {
                let err = e.into_service_error();
                if err.is_not_found() {
                    Ok(false)
                } else {
                    Err(S3Error::HeadObject {
                        message: format!("{err:?}"),
                        location: snafu::Location::new(file!(), line!(), column!()),
                    })
                }
            }
        }
    }
}

/// Convert from our S3Credentials to the types used elsewhere.
impl From<&super::super::types::Credentials> for S3Credentials {
    fn from(creds: &super::super::types::Credentials) -> Self {
        S3Credentials {
            access_key_id: creds.aws_key_id.clone(),
            secret_access_key: creds.aws_secret_key.clone(),
            session_token: Some(creds.aws_token.clone()),
        }
    }
}

/// Convert from StageInfo to S3Config.
impl From<&super::super::types::StageInfo> for S3Config {
    fn from(stage_info: &super::super::types::StageInfo) -> Self {
        S3Config {
            region: stage_info.region.clone(),
            credentials: S3Credentials::from(&stage_info.creds),
            endpoint: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let config = S3Config {
            region: "us-west-2".to_string(),
            credentials: S3Credentials {
                access_key_id: "test".to_string(),
                secret_access_key: "test".to_string(),
                session_token: None,
            },
            endpoint: None,
        };
        let _client = AwsSdkS3Client::new(config);
    }
}
