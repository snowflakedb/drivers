use super::types::{
    CloudCredentials, EncryptedFileMetadata, EncryptionResult, MaterialDescription, StageInfo,
};
use crate::config::retry::{BackoffConfig, HttpPolicy, Jitter, RetryPolicy};
use crate::http::retry::{HttpContext, HttpError, execute_with_retry};
use snafu::{Location, OptionExt, ResultExt, Snafu};
use std::time::Duration;

const GCS_DEFAULT_ENDPOINT: &str = "https://storage.googleapis.com";
const GCS_META_SFC_DIGEST: &str = "x-goog-meta-sfc-digest";
const GCS_META_MATDESC: &str = "x-goog-meta-matdesc";
const GCS_META_ENCRYPTIONDATA: &str = "x-goog-meta-encryptiondata";

// Encryption metadata constants
const ENCRYPTION_MODE: &str = "FullBlob";
const ENCRYPTION_KEY_ID: &str = "symmKey1";
const ENCRYPTION_ALGORITHM: &str = "AES_CBC_256";
const ENCRYPTION_PROTOCOL_VERSION: &str = "1.0";
const ENCRYPTION_LIBRARY: &str = "UniversalDriver";

// Upload status constants
const STATUS_SKIPPED: &str = "SKIPPED";
const STATUS_UPLOADED: &str = "UPLOADED";

// HTTP status codes
const HTTP_STATUS_FORBIDDEN: u16 = 403;
const HTTP_STATUS_UNAUTHORIZED: u16 = 401;
const HTTP_STATUS_NOT_FOUND: u16 = 404;

// TODO: streaming instead of loading the whole file into memory

/// Create a retry policy suitable for cloud storage operations.
/// Uses exponential backoff with decorrelated jitter to avoid thundering herd.
fn create_cloud_retry_policy() -> RetryPolicy {
    RetryPolicy {
        http: HttpPolicy {
            retry_safe_reads: true,
            retry_idempotent_writes: true,
            retry_post_patch: false,
        },
        max_attempts: 7,
        backoff: BackoffConfig {
            base: Duration::from_secs(1),
            factor: 2.0,
            cap: Duration::from_secs(16),
            jitter: Jitter::Decorrelated,
        },
        max_elapsed: Duration::from_secs(120),
    }
}

/// Build the GCS key by concatenating the stage prefix and filename.
fn build_gcs_key(stage_info: &StageInfo, filename: &str) -> String {
    format!("{}{}", stage_info.key_prefix, filename)
}

fn build_gcs_url(stage_info: &StageInfo, key: &str) -> String {
    let encoded_key = urlencoding::encode(key);
    if stage_info.use_virtual_url {
        format!(
            "https://{}.storage.googleapis.com/{}",
            stage_info.bucket, encoded_key
        )
    } else {
        let endpoint = stage_info
            .end_point
            .as_deref()
            .unwrap_or(GCS_DEFAULT_ENDPOINT);
        format!("{}/{}/{}", endpoint, stage_info.bucket, encoded_key)
    }
}

/// Build the encryption metadata JSON in the format expected by GCS and other drivers.
fn build_encryption_metadata_json(
    metadata: &EncryptedFileMetadata,
) -> Result<String, serde_json::Error> {
    let json = serde_json::json!({
        "EncryptionMode": ENCRYPTION_MODE,
        "WrappedContentKey": {
            "KeyId": ENCRYPTION_KEY_ID,
            "EncryptedKey": metadata.encrypted_key,
            "Algorithm": ENCRYPTION_ALGORITHM
        },
        "EncryptionAgent": {
            "Protocol": ENCRYPTION_PROTOCOL_VERSION,
            "EncryptionAlgorithm": ENCRYPTION_ALGORITHM
        },
        "ContentEncryptionIV": metadata.iv,
        "KeyWrappingMetadata": {
            "EncryptionLibrary": ENCRYPTION_LIBRARY
        }
    });
    serde_json::to_string(&json)
}

/// Check if a file exists in GCS using HEAD request.
/// When access is denied (403), returns false so the caller proceeds with upload.
async fn check_file_exists_gcs(
    client: &reqwest::Client,
    stage_info: &StageInfo,
    gcs_key: &str,
) -> Result<bool, GcsTransferError> {
    let url = build_gcs_url(stage_info, gcs_key);
    let CloudCredentials::Gcs { access_token } = &stage_info.creds else {
        return InvalidCredentialsSnafu.fail();
    };

    let response = client
        .head(&url)
        .bearer_auth(access_token.reveal())
        .send()
        .await
        .context(TransportSnafu)?;

    match response.status().as_u16() {
        200..=299 => Ok(true),
        HTTP_STATUS_NOT_FOUND => Ok(false),
        HTTP_STATUS_FORBIDDEN => {
            tracing::warn!(
                "Access denied checking GCS file existence ({gcs_key}), proceeding with upload"
            );
            Ok(false)
        }
        HTTP_STATUS_UNAUTHORIZED => TokenExpiredSnafu.fail(),
        status => UnexpectedStatusSnafu { status }.fail(),
    }
}

/// Upload a file to GCS with retry support.
/// Skips upload if the file already exists and overwrite is false.
pub async fn upload_to_gcs_or_skip(
    encryption_result: EncryptionResult,
    stage_info: &StageInfo,
    filename: &str,
    overwrite: bool,
) -> Result<String, GcsTransferError> {
    let gcs_key = build_gcs_key(stage_info, filename);
    // Reuse a single client for both existence check and upload
    let client = reqwest::Client::builder().build().context(TransportSnafu)?;

    // Check existence (skip if overwrite=true)
    if !overwrite && check_file_exists_gcs(&client, stage_info, &gcs_key).await? {
        tracing::info!("File already exists in GCS: {gcs_key}");
        return Ok(STATUS_SKIPPED.into());
    }

    // Upload with retry
    let url = build_gcs_url(stage_info, &gcs_key);
    let ctx = HttpContext::new(reqwest::Method::PUT, &url).with_idempotent(true);

    let gcs_retry_policy = create_cloud_retry_policy();

    // Build the request with encrypted data and metadata
    let CloudCredentials::Gcs { access_token } = &stage_info.creds else {
        return InvalidCredentialsSnafu.fail();
    };

    let mat_desc_json = serde_json::to_string(&encryption_result.metadata.material_desc)
        .context(SerializationSnafu)?;
    let encryption_json =
        build_encryption_metadata_json(&encryption_result.metadata).context(SerializationSnafu)?;

    // Clone only what's needed for the retry closure
    let data = encryption_result.data;
    let digest = &encryption_result.metadata.digest;
    let token = access_token.reveal();

    // Execute with retry - the handler returns the response for us to check
    let response = execute_with_retry(
        || {
            client
                .put(&url)
                .bearer_auth(token)
                .header("content-encoding", "") // CRITICAL: prevents GCS auto-decompression
                .header(GCS_META_SFC_DIGEST, digest)
                .header(GCS_META_MATDESC, &mat_desc_json)
                .header(GCS_META_ENCRYPTIONDATA, &encryption_json)
                .body(data.clone())
        },
        &ctx,
        &gcs_retry_policy,
        |response| async move { Ok(response) },
    )
    .await
    .context(HttpRetrySnafu)?;

    // Check the final response status
    match response.status().as_u16() {
        200..=299 => Ok(STATUS_UPLOADED.into()),
        HTTP_STATUS_UNAUTHORIZED => TokenExpiredSnafu.fail(),
        status => {
            let body = response.text().await.unwrap_or_default();
            UploadSnafu { status, body }.fail()
        }
    }
}

/// Extract a required header value from the response, returning a descriptive error if missing.
fn get_required_header(
    headers: &reqwest::header::HeaderMap,
    name: &str,
) -> Result<String, GcsTransferError> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .context(MissingMetadataSnafu {
            field: name.to_string(),
        })
}

/// Parse the x-goog-meta-encryptiondata JSON to extract the encrypted key and IV.
fn parse_encryption_data_json(json_str: &str) -> Result<(String, String), GcsTransferError> {
    let data: serde_json::Value = serde_json::from_str(json_str).context(DeserializationSnafu)?;

    let encrypted_key = data
        .get("WrappedContentKey")
        .and_then(|wck| wck.get("EncryptedKey"))
        .and_then(|ek| ek.as_str())
        .map(|s| s.to_string())
        .context(MissingMetadataSnafu {
            field: "WrappedContentKey.EncryptedKey".to_string(),
        })?;

    let iv = data
        .get("ContentEncryptionIV")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .context(MissingMetadataSnafu {
            field: "ContentEncryptionIV".to_string(),
        })?;

    Ok((encrypted_key, iv))
}

/// Download a file from GCS, extracting metadata from response headers.
pub async fn download_from_gcs(
    stage_info: &StageInfo,
    filename: &str,
) -> Result<(Vec<u8>, EncryptedFileMetadata), GcsTransferError> {
    let gcs_key = build_gcs_key(stage_info, filename);
    let url = build_gcs_url(stage_info, &gcs_key);

    let CloudCredentials::Gcs { access_token } = &stage_info.creds else {
        return InvalidCredentialsSnafu.fail();
    };

    let client = reqwest::Client::builder().build().context(TransportSnafu)?;
    let response = client
        .get(&url)
        .bearer_auth(access_token.reveal())
        .send()
        .await
        .context(TransportSnafu)?;

    match response.status().as_u16() {
        200..=299 => {}
        HTTP_STATUS_UNAUTHORIZED => return TokenExpiredSnafu.fail(),
        HTTP_STATUS_NOT_FOUND => return FileNotFoundSnafu { key: gcs_key }.fail(),
        status => {
            let body = response.text().await.unwrap_or_default();
            return DownloadSnafu { status, body }.fail();
        }
    }

    // CRITICAL: Clone headers BEFORE consuming the response body.
    // response.bytes().await moves the response, making headers inaccessible.
    let headers = response.headers().clone();

    // Extract metadata from response headers
    let digest = get_required_header(&headers, GCS_META_SFC_DIGEST)?;
    let encryption_json = get_required_header(&headers, GCS_META_ENCRYPTIONDATA)?;
    let mat_desc_str = get_required_header(&headers, GCS_META_MATDESC)?;

    // Parse nested encryption JSON
    let (encrypted_key, iv) = parse_encryption_data_json(&encryption_json)?;

    // Parse material description
    let material_desc: MaterialDescription =
        serde_json::from_str(&mat_desc_str).context(DeserializationSnafu)?;

    let file_metadata = EncryptedFileMetadata {
        encrypted_key,
        iv,
        material_desc,
        digest,
    };

    // Now consume the response body
    let encrypted_data = response.bytes().await.context(TransportSnafu)?.to_vec();

    Ok((encrypted_data, file_metadata))
}

#[derive(Snafu, Debug, error_trace::ErrorTrace)]
pub enum GcsTransferError {
    #[snafu(display("Invalid credentials for GCS"))]
    InvalidCredentials {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("GCS access token expired"))]
    TokenExpired {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("HTTP transport error"))]
    Transport {
        source: reqwest::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("HTTP retry error"))]
    HttpRetry {
        source: HttpError,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Missing GCS metadata: {field}"))]
    MissingMetadata {
        field: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Unexpected HTTP status {status}"))]
    UnexpectedStatus {
        status: u16,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("GCS upload error: status {status}, {body}"))]
    Upload {
        status: u16,
        body: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to serialize metadata"))]
    Serialization {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("GCS download error: status {status}, {body}"))]
    Download {
        status: u16,
        body: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("GCS file not found: {key}"))]
    FileNotFound {
        key: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to deserialize metadata"))]
    Deserialization {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_gcs_url_standard() {
        let stage_info = StageInfo {
            location_type: super::super::types::StageLocationType::Gcs,
            bucket: "test-bucket".to_string(),
            key_prefix: "prefix/".to_string(),
            region: None,
            creds: CloudCredentials::Gcs {
                access_token: "token".to_string().into(),
            },
            end_point: None,
            use_regional_url: false,
            use_virtual_url: false,
        };

        let url = build_gcs_url(&stage_info, "file.txt");
        assert_eq!(url, "https://storage.googleapis.com/test-bucket/file.txt");
    }

    #[test]
    fn test_build_gcs_url_virtual() {
        let stage_info = StageInfo {
            location_type: super::super::types::StageLocationType::Gcs,
            bucket: "test-bucket".to_string(),
            key_prefix: "prefix/".to_string(),
            region: None,
            creds: CloudCredentials::Gcs {
                access_token: "token".to_string().into(),
            },
            end_point: None,
            use_regional_url: false,
            use_virtual_url: true,
        };

        let url = build_gcs_url(&stage_info, "file.txt");
        assert_eq!(url, "https://test-bucket.storage.googleapis.com/file.txt");
    }

    #[test]
    fn test_build_gcs_url_regional() {
        let stage_info = StageInfo {
            location_type: super::super::types::StageLocationType::Gcs,
            bucket: "test-bucket".to_string(),
            key_prefix: "prefix/".to_string(),
            region: None,
            creds: CloudCredentials::Gcs {
                access_token: "token".to_string().into(),
            },
            end_point: Some("https://storage.us-west1.rep.googleapis.com".to_string()),
            use_regional_url: true,
            use_virtual_url: false,
        };

        let url = build_gcs_url(&stage_info, "file.txt");
        assert_eq!(
            url,
            "https://storage.us-west1.rep.googleapis.com/test-bucket/file.txt"
        );
    }

    #[test]
    fn test_parse_encryption_data_json_valid() {
        let json = r#"{
            "EncryptionMode": "FullBlob",
            "WrappedContentKey": {
                "KeyId": "symmKey1",
                "EncryptedKey": "base64key",
                "Algorithm": "AES_CBC_256"
            },
            "ContentEncryptionIV": "base64iv"
        }"#;

        let (key, iv) = parse_encryption_data_json(json).unwrap();
        assert_eq!(key, "base64key");
        assert_eq!(iv, "base64iv");
    }

    #[test]
    fn test_parse_encryption_data_json_missing_key() {
        let json = r#"{
            "EncryptionMode": "FullBlob",
            "ContentEncryptionIV": "base64iv"
        }"#;

        let result = parse_encryption_data_json(json);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("WrappedContentKey.EncryptedKey")
        );
    }

    #[test]
    fn test_parse_encryption_data_json_missing_iv() {
        let json = r#"{
            "EncryptionMode": "FullBlob",
            "WrappedContentKey": {
                "KeyId": "symmKey1",
                "EncryptedKey": "base64key",
                "Algorithm": "AES_CBC_256"
            }
        }"#;

        let result = parse_encryption_data_json(json);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("ContentEncryptionIV")
        );
    }
}
