//! WASM S3 client implementation using wstd HTTP + AWS Signature V4.
//!
//! This module provides an S3 client for WASM builds that uses wstd's
//! HTTP client and implements AWS Signature V4 signing.

use super::{S3Client, S3Config, S3Error, S3Object};
use crate::http::{DefaultHttpClient, HttpClient, HttpRequest};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// WASM S3 client using wstd HTTP with AWS SigV4 signing.
pub struct WasiS3Client {
    config: S3Config,
    http_client: DefaultHttpClient,
}

impl WasiS3Client {
    /// Create a new WASM S3 client with the given configuration.
    pub fn new(config: S3Config) -> Self {
        Self {
            config,
            http_client: DefaultHttpClient::default(),
        }
    }

    /// Generate the S3 endpoint URL for the configured region.
    fn endpoint_url(&self, bucket: &str, key: &str) -> String {
        if let Some(ref endpoint) = self.config.endpoint {
            format!("{}/{}/{}", endpoint, bucket, key)
        } else {
            format!(
                "https://{}.s3.{}.amazonaws.com/{}",
                bucket, self.config.region, key
            )
        }
    }

    /// Get the S3 host for the given bucket.
    fn host(&self, bucket: &str) -> String {
        if let Some(ref endpoint) = self.config.endpoint {
            // Extract host from endpoint
            endpoint
                .trim_start_matches("https://")
                .trim_start_matches("http://")
                .split('/')
                .next()
                .unwrap_or("")
                .to_string()
        } else {
            format!("{}.s3.{}.amazonaws.com", bucket, self.config.region)
        }
    }

    /// Sign a request using AWS Signature Version 4.
    fn sign_request(
        &self,
        method: &str,
        url: &str,
        headers: &mut HashMap<String, String>,
        payload: &[u8],
    ) {
        use chrono::Utc;

        let datetime = Utc::now();
        let date_str = datetime.format("%Y%m%d").to_string();
        let datetime_str = datetime.format("%Y%m%dT%H%M%SZ").to_string();

        // Calculate payload hash
        let payload_hash = hex::encode(Sha256::digest(payload));

        // Add required headers
        headers.insert("x-amz-date".to_string(), datetime_str.clone());
        headers.insert("x-amz-content-sha256".to_string(), payload_hash.clone());

        if let Some(ref token) = self.config.credentials.session_token {
            headers.insert("x-amz-security-token".to_string(), token.clone());
        }

        // Parse URL
        let parsed_url = url::Url::parse(url).unwrap();
        let host = parsed_url.host_str().unwrap_or("");
        let path = parsed_url.path();
        let query = parsed_url.query().unwrap_or("");

        headers.insert("host".to_string(), host.to_string());

        // Build canonical request
        let canonical_request =
            self.build_canonical_request(method, path, query, headers, &payload_hash);

        // Build string to sign
        let credential_scope = format!("{}/{}/s3/aws4_request", date_str, self.config.region);
        let canonical_request_hash = hex::encode(Sha256::digest(canonical_request.as_bytes()));
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{}\n{}\n{}",
            datetime_str, credential_scope, canonical_request_hash
        );

        // Calculate signature
        let signature = self.calculate_signature(&date_str, &string_to_sign);

        // Build authorization header
        let signed_headers = self.get_signed_headers(headers);
        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            self.config.credentials.access_key_id, credential_scope, signed_headers, signature
        );

        headers.insert("authorization".to_string(), authorization);
    }

    fn build_canonical_request(
        &self,
        method: &str,
        path: &str,
        query: &str,
        headers: &HashMap<String, String>,
        payload_hash: &str,
    ) -> String {
        // Get sorted headers
        let mut header_names: Vec<_> = headers.keys().map(|k| k.to_lowercase()).collect();
        header_names.sort();

        let canonical_headers: String = header_names
            .iter()
            .map(|name| {
                let value = headers
                    .iter()
                    .find(|(k, _)| k.to_lowercase() == *name)
                    .map(|(_, v)| v.trim())
                    .unwrap_or("");
                format!("{}:{}\n", name, value)
            })
            .collect();

        let signed_headers = header_names.join(";");

        // Sort query parameters
        let canonical_query = if query.is_empty() {
            String::new()
        } else {
            let mut params: Vec<_> = query.split('&').collect();
            params.sort();
            params.join("&")
        };

        format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            method, path, canonical_query, canonical_headers, signed_headers, payload_hash
        )
    }

    fn get_signed_headers(&self, headers: &HashMap<String, String>) -> String {
        let mut names: Vec<_> = headers.keys().map(|k| k.to_lowercase()).collect();
        names.sort();
        names.join(";")
    }

    fn calculate_signature(&self, date: &str, string_to_sign: &str) -> String {
        use hmac::Hmac;
        type HmacSha256 = Hmac<Sha256>;

        let k_date = hmac_sha256(
            format!("AWS4{}", self.config.credentials.secret_access_key).as_bytes(),
            date.as_bytes(),
        );
        let k_region = hmac_sha256(&k_date, self.config.region.as_bytes());
        let k_service = hmac_sha256(&k_region, b"s3");
        let k_signing = hmac_sha256(&k_service, b"aws4_request");

        hex::encode(hmac_sha256(&k_signing, string_to_sign.as_bytes()))
    }
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    use hmac::{Hmac, Mac};
    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key length is always valid");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

impl S3Client for WasiS3Client {
    async fn put_object(
        &self,
        bucket: &str,
        key: &str,
        body: Vec<u8>,
        content_type: Option<&str>,
        metadata: HashMap<String, String>,
    ) -> Result<(), S3Error> {
        let url = self.endpoint_url(bucket, key);
        let mut headers = HashMap::new();

        // Add content type
        if let Some(ct) = content_type {
            headers.insert("content-type".to_string(), ct.to_string());
        }

        // Add metadata with x-amz-meta- prefix
        for (k, v) in &metadata {
            headers.insert(format!("x-amz-meta-{}", k), v.clone());
        }

        // Sign the request
        self.sign_request("PUT", &url, &mut headers, &body);

        // Build and send request
        let mut request = HttpRequest::put(&url);
        for (k, v) in headers {
            request = request.header(k, v);
        }
        request = request.body(body);

        let response = self
            .http_client
            .request(request)
            .await
            .map_err(|e| S3Error::Upload {
                message: e.to_string(),
                location: snafu::Location::new(file!(), line!(), column!()),
            })?;

        if response.status.is_success() {
            Ok(())
        } else if response.status.as_u16() == 403 {
            Err(S3Error::AccessDenied {
                message: format!("Access denied uploading to {}/{}", bucket, key),
                location: snafu::Location::new(file!(), line!(), column!()),
            })
        } else {
            Err(S3Error::Upload {
                message: format!(
                    "S3 PUT failed with status {}: {}",
                    response.status,
                    response.text().unwrap_or_default()
                ),
                location: snafu::Location::new(file!(), line!(), column!()),
            })
        }
    }

    async fn get_object(&self, bucket: &str, key: &str) -> Result<S3Object, S3Error> {
        let url = self.endpoint_url(bucket, key);
        let mut headers = HashMap::new();

        // Sign the request (empty body for GET)
        self.sign_request("GET", &url, &mut headers, &[]);

        // Build and send request
        let mut request = HttpRequest::get(&url);
        for (k, v) in headers {
            request = request.header(k, v);
        }

        let response = self
            .http_client
            .request(request)
            .await
            .map_err(|e| S3Error::Download {
                message: e.to_string(),
                location: snafu::Location::new(file!(), line!(), column!()),
            })?;

        if response.status.is_success() {
            // Extract metadata from response headers
            let mut metadata = HashMap::new();
            for (k, v) in &response.headers {
                if k.to_lowercase().starts_with("x-amz-meta-") {
                    let meta_key = k.trim_start_matches("x-amz-meta-").to_string();
                    metadata.insert(meta_key, v.clone());
                }
            }

            let content_length = response
                .header("content-length")
                .and_then(|v| v.parse().ok());
            let content_type = response.header("content-type").cloned();

            Ok(S3Object {
                body: response.body,
                metadata,
                content_length,
                content_type,
            })
        } else if response.status.as_u16() == 404 {
            Err(S3Error::NotFound {
                key: key.to_string(),
                location: snafu::Location::new(file!(), line!(), column!()),
            })
        } else if response.status.as_u16() == 403 {
            Err(S3Error::AccessDenied {
                message: format!("Access denied downloading {}/{}", bucket, key),
                location: snafu::Location::new(file!(), line!(), column!()),
            })
        } else {
            Err(S3Error::Download {
                message: format!(
                    "S3 GET failed with status {}: {}",
                    response.status,
                    response.text().unwrap_or_default()
                ),
                location: snafu::Location::new(file!(), line!(), column!()),
            })
        }
    }

    async fn head_object(&self, bucket: &str, key: &str) -> Result<bool, S3Error> {
        let url = self.endpoint_url(bucket, key);
        let mut headers = HashMap::new();

        // Sign the request (empty body for HEAD)
        self.sign_request("HEAD", &url, &mut headers, &[]);

        // Build and send request
        let mut request = HttpRequest::head(&url);
        for (k, v) in headers {
            request = request.header(k, v);
        }

        let response =
            self.http_client
                .request(request)
                .await
                .map_err(|e| S3Error::HeadObject {
                    message: e.to_string(),
                    location: snafu::Location::new(file!(), line!(), column!()),
                })?;

        match response.status.as_u16() {
            200 => Ok(true),
            404 => Ok(false),
            403 => Err(S3Error::AccessDenied {
                message: format!("Access denied checking {}/{}", bucket, key),
                location: snafu::Location::new(file!(), line!(), column!()),
            }),
            _ => Err(S3Error::HeadObject {
                message: format!("S3 HEAD failed with status {}", response.status),
                location: snafu::Location::new(file!(), line!(), column!()),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_manager::s3::S3Credentials;

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
        let _client = WasiS3Client::new(config);
    }

    #[test]
    fn test_endpoint_url() {
        let config = S3Config {
            region: "us-west-2".to_string(),
            credentials: S3Credentials {
                access_key_id: "test".to_string(),
                secret_access_key: "test".to_string(),
                session_token: None,
            },
            endpoint: None,
        };
        let client = WasiS3Client::new(config);
        assert_eq!(
            client.endpoint_url("my-bucket", "path/to/file.txt"),
            "https://my-bucket.s3.us-west-2.amazonaws.com/path/to/file.txt"
        );
    }
}
