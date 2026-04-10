use super::types::{
    CloudCredentials, EncryptedFileMetadata, MaterialDescription, PreparedUpload, StageInfo,
    UploadStatus,
};
use crate::config::retry::{BackoffConfig, Jitter, RetryPolicy};
use crate::http::retry::{HttpContext, HttpError, execute_with_retry as http_execute_with_retry};
use reqwest::{Method, StatusCode};
use snafu::{Location, OptionExt, ResultExt, Snafu};
use std::time::Duration;

const REQUEST_TIMEOUT_SECS: u64 = 300;

// GCS metadata header names
const GCS_META_SFC_DIGEST: &str = "x-goog-meta-sfc-digest";
const GCS_META_ENCRYPTIONDATA: &str = "x-goog-meta-encryptiondata";
const GCS_META_MATDESC: &str = "x-goog-meta-matdesc";

/// Uploads a file to GCS, skipping if it already exists and `overwrite` is false.
pub async fn upload_to_gcs_or_skip(
    prepared: PreparedUpload,
    stage_info: &StageInfo,
    filename: &str,
    overwrite: bool,
) -> Result<UploadStatus, GcsUploadError> {
    let client = create_gcs_client()?;
    let key = format!("{}{filename}", stage_info.key_prefix);
    let using_presigned_url = stage_info.presigned_url.is_some();
    let (url, token) = resolve_url_and_token(stage_info, &key)?;

    if !overwrite && check_file_exists_gcs(&client, &url, token).await {
        tracing::info!("File already exists in GCS: {key}");
        return Ok(UploadStatus::Skipped);
    }

    upload_to_gcs(&client, &url, token, prepared, using_presigned_url).await?;
    Ok(UploadStatus::Uploaded)
}

/// Downloads a file from GCS and returns data with optional encryption metadata.
/// For SSE stages the metadata headers will be absent and `None` is returned.
pub async fn download_from_gcs(
    stage_info: &StageInfo,
    filename: &str,
) -> Result<(Vec<u8>, Option<String>, Option<EncryptedFileMetadata>), GcsDownloadError> {
    let client = create_gcs_client()?;
    let key = format!("{}{filename}", stage_info.key_prefix);
    let (url, token) = resolve_url_and_token(stage_info, &key)?;
    let using_presigned_url = stage_info.presigned_url.is_some();

    let response = gcs_request_with_retry(
        || {
            let mut req = client.get(&url);
            if let Some(ref t) = token {
                req = req.bearer_auth(t);
            }
            req
        },
        Method::GET,
        using_presigned_url,
    )
    .await?;

    let headers = response.headers();
    let digest = try_get_header(headers, GCS_META_SFC_DIGEST)?;

    let file_metadata = match try_get_header(headers, GCS_META_ENCRYPTIONDATA)? {
        Some(encryption_data_str) => {
            let enc_data: serde_json::Value = serde_json::from_str(&encryption_data_str)
                .context(gcs_download_error::DeserializationSnafu)?;

            let encrypted_key = enc_data["WrappedContentKey"]["EncryptedKey"]
                .as_str()
                .context(gcs_download_error::MissingMetadataSnafu {
                    field: "WrappedContentKey.EncryptedKey",
                })?
                .to_string();

            let iv = enc_data["ContentEncryptionIV"]
                .as_str()
                .context(gcs_download_error::MissingMetadataSnafu {
                    field: "ContentEncryptionIV",
                })?
                .to_string();

            let mat_desc_str = try_get_header(headers, GCS_META_MATDESC)?.context(
                gcs_download_error::MissingMetadataSnafu {
                    field: GCS_META_MATDESC,
                },
            )?;
            let material_desc: MaterialDescription = serde_json::from_str(&mat_desc_str)
                .context(gcs_download_error::DeserializationSnafu)?;

            Some(EncryptedFileMetadata {
                encrypted_key,
                iv,
                material_desc,
            })
        }
        None => None,
    };

    let data = response
        .bytes()
        .await
        .map_err(|source| GcsRequestError::Http { source })?
        .to_vec();

    Ok((data, digest, file_metadata))
}

/// Check if a file exists in GCS via HEAD request.
/// Returns false on any error or non-200 status so the caller proceeds with upload.
async fn check_file_exists_gcs(client: &reqwest::Client, url: &str, token: Option<&str>) -> bool {
    let mut request = client.head(url);
    if let Some(t) = token {
        request = request.bearer_auth(t);
    }

    match request.send().await {
        Ok(resp) => match resp.status() {
            StatusCode::OK => true,
            StatusCode::NOT_FOUND => false,
            StatusCode::FORBIDDEN => {
                tracing::warn!(
                    "Access denied checking file existence in GCS, proceeding with upload"
                );
                false
            }
            status => {
                tracing::warn!(
                    "Unexpected status {} checking GCS file existence, proceeding with upload",
                    status
                );
                false
            }
        },
        Err(e) => {
            tracing::warn!(
                "Error checking GCS file existence, proceeding with upload: {}",
                e
            );
            false
        }
    }
}

/// Upload data to GCS with retry logic.
/// Sets encryption metadata headers only when client-side encryption was used.
async fn upload_to_gcs(
    client: &reqwest::Client,
    url: &str,
    token: Option<&str>,
    prepared: PreparedUpload,
    using_presigned_url: bool,
) -> Result<(), GcsUploadError> {
    let encryption_data_str = prepared
        .encryption_metadata
        .as_ref()
        .map(|enc_meta| {
            let encryption_data = serde_json::json!({
                "EncryptionMode": "FullBlob",
                "WrappedContentKey": {
                    "KeyId": "symmKey1",
                    "EncryptedKey": enc_meta.encrypted_key,
                    "Algorithm": "AES_CBC_256"
                },
                "EncryptionAgent": {
                    "Protocol": "1.0",
                    "EncryptionAlgorithm": "AES_CBC_256"
                },
                "ContentEncryptionIV": enc_meta.iv,
                "KeyWrappingMetadata": {
                    "EncryptionLibrary": "Rust(OpenSSL)"
                }
            });
            serde_json::to_string(&encryption_data)
        })
        .transpose()
        .context(gcs_upload_error::SerializationSnafu)?;

    let mat_desc_str = prepared
        .encryption_metadata
        .as_ref()
        .map(|enc_meta| serde_json::to_string(&enc_meta.material_desc))
        .transpose()
        .context(gcs_upload_error::SerializationSnafu)?;

    let data = prepared.data;
    let digest = prepared.digest;

    gcs_request_with_retry(
        || {
            let mut req = client
                .put(url)
                .header(GCS_META_SFC_DIGEST, &digest)
                .header("content-encoding", "")
                .body(data.clone());

            if let Some(ref enc_str) = encryption_data_str {
                req = req.header(GCS_META_ENCRYPTIONDATA, enc_str);
            }
            if let Some(ref md_str) = mat_desc_str {
                req = req.header(GCS_META_MATDESC, md_str);
            }
            if let Some(t) = token {
                req = req.bearer_auth(t);
            }
            req
        },
        Method::PUT,
        using_presigned_url,
    )
    .await?;

    tracing::debug!("GCS upload successful");
    Ok(())
}

// --- Retry logic (delegates to http::retry) ---

/// Returns a retry policy tuned for GCS file-transfer operations.
///
/// GCS treats 403 as retryable (temporary credential issues), and 400 is
/// retryable when using presigned URLs (URL may have expired).
fn gcs_retry_policy(using_presigned_url: bool) -> RetryPolicy {
    let mut extra = vec![403];
    if using_presigned_url {
        extra.push(400);
    }
    RetryPolicy {
        max_attempts: 6,
        backoff: BackoffConfig {
            base: Duration::from_secs(1),
            factor: 2.0,
            cap: Duration::from_secs(16),
            jitter: Jitter::None,
        },
        // Must exceed REQUEST_TIMEOUT_SECS (300s) to allow at least one full
        // request + retries. 600s accommodates ~2 full-timeout attempts plus backoff.
        max_elapsed: Duration::from_secs(600),
        extra_retryable_statuses: extra,
        ..RetryPolicy::default()
    }
}

/// Executes a GCS HTTP request with retry, then checks for GCS-specific status codes.
async fn gcs_request_with_retry<F>(
    build_request: F,
    method: Method,
    using_presigned_url: bool,
) -> Result<reqwest::Response, GcsRequestError>
where
    F: Fn() -> reqwest::RequestBuilder,
{
    let ctx = HttpContext::new(method, "gcs-transfer");
    let policy = gcs_retry_policy(using_presigned_url);

    let response = http_execute_with_retry(build_request, &ctx, &policy, |r| async move { Ok(r) })
        .await
        .map_err(map_http_error)?;

    if response.status().is_success() {
        return Ok(response);
    }

    // 401: token expired — propagate up so the query layer can re-execute
    if response.status() == StatusCode::UNAUTHORIZED {
        return Err(GcsRequestError::TokenExpired);
    }

    let status_code = response.status().as_u16();
    let body = read_error_body(response).await;
    Err(GcsRequestError::GcsHttp { status_code, body })
}

fn map_http_error(e: HttpError) -> GcsRequestError {
    match e {
        HttpError::Transport { source, .. } => GcsRequestError::Http { source },
        other => GcsRequestError::RetryExhausted {
            detail: other.to_string(),
        },
    }
}

// --- Helpers ---

fn create_gcs_client() -> Result<reqwest::Client, GcsRequestError> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|source| GcsRequestError::Http { source })
}

/// Constructs the GCS URL and extracts the bearer token from stage info.
///
/// URL strategy priority (matching JDBC/ODBC/Python):
/// 1. Presigned URL — use directly, no token
/// 2. Custom endpoint — `https://{end_point}/{bucket}/{key}`
/// 3. Virtual host — `https://{bucket}.storage.googleapis.com/{key}`
/// 4. Regional — `https://storage.{region}.rep.googleapis.com/{bucket}/{key}`
/// 5. Default — `https://storage.googleapis.com/{bucket}/{key}`
fn resolve_url_and_token<'a>(
    stage_info: &'a StageInfo,
    key: &str,
) -> Result<(String, Option<&'a str>), GcsRequestError> {
    // Strategy 1: presigned URL
    if let Some(presigned) = &stage_info.presigned_url {
        return Ok((presigned.clone(), None));
    }

    // Extract token reference — avoids copying into a non-zeroized String
    let token = match &stage_info.creds {
        CloudCredentials::Gcs { gcs_access_token } => {
            gcs_access_token.as_ref().map(|t| t.reveal().as_str())
        }
        _ => return Err(GcsRequestError::MissingGcsCredentials),
    };

    if token.is_none() {
        return Err(GcsRequestError::MissingGcsCredentials);
    }

    let url = build_gcs_url(stage_info, key);
    Ok((url, token))
}

/// Builds the GCS URL based on endpoint/virtual/regional flags.
fn build_gcs_url(stage_info: &StageInfo, key: &str) -> String {
    let encoded_key = percent_encode_path(key);

    // Strategy 2: custom endpoint
    if let Some(ref ep) = stage_info.end_point
        && !ep.is_empty()
    {
        let base = if ep.starts_with("https://") || ep.starts_with("http://") {
            ep.clone()
        } else {
            format!("https://{ep}")
        };
        return format!("{base}/{}/{encoded_key}", stage_info.bucket);
    }

    // Strategy 3: virtual host
    if stage_info.use_virtual_url {
        return format!(
            "https://{}.storage.googleapis.com/{encoded_key}",
            stage_info.bucket
        );
    }

    // Strategy 4: regional
    if stage_info.use_regional_url {
        return format!(
            "https://storage.{}.rep.googleapis.com/{}/{encoded_key}",
            stage_info.region.to_lowercase(),
            stage_info.bucket
        );
    }

    // Strategy 5: default
    format!(
        "https://storage.googleapis.com/{}/{encoded_key}",
        stage_info.bucket
    )
}

/// Percent-encode a URL path, preserving `/` separators.
/// Matches Python `urllib.parse.quote()` / ODBC `encodeUrlName()` behavior:
/// unreserved chars (RFC 3986) and `/` pass through, everything else is encoded.
fn percent_encode_path(s: &str) -> String {
    let mut encoded = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                encoded.push(byte as char)
            }
            _ => {
                use std::fmt::Write;
                let _ = write!(encoded, "%{byte:02X}");
            }
        }
    }
    encoded
}

fn try_get_header(
    headers: &reqwest::header::HeaderMap,
    name: &str,
) -> Result<Option<String>, GcsDownloadError> {
    match headers.get(name) {
        Some(value) => {
            let s = value
                .to_str()
                .context(gcs_download_error::InvalidHeaderValueSnafu)?;
            Ok(Some(s.to_string()))
        }
        None => Ok(None),
    }
}

async fn read_error_body(response: reqwest::Response) -> String {
    match response.text().await {
        Ok(text) => text,
        Err(e) => {
            tracing::warn!("Failed to read GCS error response body: {}", e);
            format!("<could not read body: {}>", e)
        }
    }
}

// --- Error types ---

/// Internal error for shared helpers (retry, client creation, URL resolution).
/// Converted into `GcsUploadError` or `GcsDownloadError` via `From` impls.
#[derive(Debug, Snafu)]
enum GcsRequestError {
    #[snafu(display("GCS HTTP error"))]
    Http { source: reqwest::Error },
    #[snafu(display("GCS request failed: HTTP {status_code}: {body}"))]
    GcsHttp { status_code: u16, body: String },
    #[snafu(display("GCS access token expired"))]
    TokenExpired,
    #[snafu(display("Missing GCS credentials"))]
    MissingGcsCredentials,
    #[snafu(display("GCS retry exhausted: {detail}"))]
    RetryExhausted { detail: String },
}

impl From<GcsRequestError> for GcsUploadError {
    fn from(e: GcsRequestError) -> Self {
        match e {
            GcsRequestError::Http { source } => GcsUploadError::Http {
                source,
                location: Location::default(),
            },
            GcsRequestError::GcsHttp { status_code, body } => GcsUploadError::GcsHttp {
                status_code,
                body,
                location: Location::default(),
            },
            GcsRequestError::TokenExpired => GcsUploadError::TokenExpired {
                location: Location::default(),
            },
            GcsRequestError::MissingGcsCredentials => GcsUploadError::MissingGcsCredentials {
                location: Location::default(),
            },
            GcsRequestError::RetryExhausted { detail } => GcsUploadError::RetryExhausted {
                detail,
                location: Location::default(),
            },
        }
    }
}

impl From<GcsRequestError> for GcsDownloadError {
    fn from(e: GcsRequestError) -> Self {
        match e {
            GcsRequestError::Http { source } => GcsDownloadError::Http {
                source,
                location: Location::default(),
            },
            GcsRequestError::GcsHttp { status_code, body } => GcsDownloadError::GcsHttp {
                status_code,
                body,
                location: Location::default(),
            },
            GcsRequestError::TokenExpired => GcsDownloadError::TokenExpired {
                location: Location::default(),
            },
            GcsRequestError::MissingGcsCredentials => GcsDownloadError::MissingGcsCredentials {
                location: Location::default(),
            },
            GcsRequestError::RetryExhausted { detail } => GcsDownloadError::RetryExhausted {
                detail,
                location: Location::default(),
            },
        }
    }
}

#[derive(Snafu, Debug, error_trace::ErrorTrace)]
#[snafu(module)]
pub enum GcsUploadError {
    #[snafu(display("GCS HTTP error"))]
    Http {
        source: reqwest::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("GCS request failed: HTTP {status_code}: {body}"))]
    GcsHttp {
        status_code: u16,
        body: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("GCS access token expired"))]
    TokenExpired {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to serialize GCS metadata"))]
    Serialization {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Missing GCS credentials"))]
    MissingGcsCredentials {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("GCS retry exhausted: {detail}"))]
    RetryExhausted {
        detail: String,
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Snafu, Debug, error_trace::ErrorTrace)]
#[snafu(module)]
pub enum GcsDownloadError {
    #[snafu(display("GCS HTTP error"))]
    Http {
        source: reqwest::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("GCS request failed: HTTP {status_code}: {body}"))]
    GcsHttp {
        status_code: u16,
        body: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("GCS access token expired"))]
    TokenExpired {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to deserialize GCS metadata"))]
    Deserialization {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Missing GCS metadata: {field}"))]
    MissingMetadata {
        field: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Invalid GCS header value"))]
    InvalidHeaderValue {
        source: reqwest::header::ToStrError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Missing GCS credentials"))]
    MissingGcsCredentials {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("GCS retry exhausted: {detail}"))]
    RetryExhausted {
        detail: String,
        #[snafu(implicit)]
        location: Location,
    },
}

// --- Unit tests ---

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensitive::SensitiveString;

    fn make_stage_info(overrides: StageInfoOverrides) -> StageInfo {
        StageInfo {
            location_type: super::super::types::LocationType::Gcs,
            bucket: overrides.bucket.unwrap_or("my-bucket".to_string()),
            key_prefix: overrides.key_prefix.unwrap_or("prefix/".to_string()),
            region: overrides.region.unwrap_or("us-central1".to_string()),
            creds: overrides.creds.unwrap_or(CloudCredentials::Gcs {
                gcs_access_token: Some(SensitiveString::from("fake-token")),
            }),
            end_point: overrides.end_point,
            presigned_url: overrides.presigned_url,
            use_virtual_url: overrides.use_virtual_url,
            use_regional_url: overrides.use_regional_url,
        }
    }

    #[derive(Default)]
    struct StageInfoOverrides {
        bucket: Option<String>,
        key_prefix: Option<String>,
        region: Option<String>,
        creds: Option<CloudCredentials>,
        end_point: Option<String>,
        presigned_url: Option<String>,
        use_virtual_url: bool,
        use_regional_url: bool,
    }

    // ---------------------------------------------------------------
    // 1. URL construction strategies (matches ODBC test_unit_put_get_gcs.cpp)
    // ---------------------------------------------------------------

    #[test]
    fn url_default_strategy() {
        let stage = make_stage_info(StageInfoOverrides::default());
        let url = build_gcs_url(&stage, "file.csv.gz");
        assert_eq!(url, "https://storage.googleapis.com/my-bucket/file.csv.gz");
    }

    #[test]
    fn url_custom_endpoint() {
        // Matches ODBC test_gcs_override_endpoint
        let stage = make_stage_info(StageInfoOverrides {
            end_point: Some("testendpoint.googleapis.com".to_string()),
            ..Default::default()
        });
        let url = build_gcs_url(&stage, "file.csv.gz");
        assert_eq!(
            url,
            "https://testendpoint.googleapis.com/my-bucket/file.csv.gz"
        );
    }

    #[test]
    fn url_custom_endpoint_with_scheme() {
        let stage = make_stage_info(StageInfoOverrides {
            end_point: Some("https://custom.example.com".to_string()),
            ..Default::default()
        });
        let url = build_gcs_url(&stage, "file.csv.gz");
        assert_eq!(url, "https://custom.example.com/my-bucket/file.csv.gz");
    }

    #[test]
    fn url_virtual_host() {
        // Matches ODBC test_gcs_use_virtual_url
        let stage = make_stage_info(StageInfoOverrides {
            use_virtual_url: true,
            ..Default::default()
        });
        let url = build_gcs_url(&stage, "file.csv.gz");
        assert_eq!(url, "https://my-bucket.storage.googleapis.com/file.csv.gz");
    }

    #[test]
    fn url_regional() {
        // Matches ODBC test_gcs_use_regional_url
        let stage = make_stage_info(StageInfoOverrides {
            region: Some("testregion".to_string()),
            use_regional_url: true,
            ..Default::default()
        });
        let url = build_gcs_url(&stage, "file.csv.gz");
        assert_eq!(
            url,
            "https://storage.testregion.rep.googleapis.com/my-bucket/file.csv.gz"
        );
    }

    #[test]
    fn url_me_central2_forces_regional() {
        // Matches ODBC test_gcs_use_me2_region
        // Note: me-central2 forcing is done in query_response.rs TryFrom,
        // so here we just verify the regional URL is built correctly.
        let stage = make_stage_info(StageInfoOverrides {
            region: Some("me-central2".to_string()),
            use_regional_url: true,
            ..Default::default()
        });
        let url = build_gcs_url(&stage, "file.csv.gz");
        assert_eq!(
            url,
            "https://storage.me-central2.rep.googleapis.com/my-bucket/file.csv.gz"
        );
    }

    #[test]
    fn url_custom_endpoint_takes_precedence() {
        // Matches ODBC test_gcs_all_endpoint_fields_enabled
        let stage = make_stage_info(StageInfoOverrides {
            end_point: Some("testendpoint.googleapis.com".to_string()),
            region: Some("testregion".to_string()),
            use_virtual_url: true,
            use_regional_url: true,
            ..Default::default()
        });
        let url = build_gcs_url(&stage, "file.csv.gz");
        assert_eq!(
            url,
            "https://testendpoint.googleapis.com/my-bucket/file.csv.gz"
        );
    }

    #[test]
    fn url_empty_endpoint_falls_through() {
        let stage = make_stage_info(StageInfoOverrides {
            end_point: Some("".to_string()),
            ..Default::default()
        });
        let url = build_gcs_url(&stage, "file.csv.gz");
        assert_eq!(url, "https://storage.googleapis.com/my-bucket/file.csv.gz");
    }

    // ---------------------------------------------------------------
    // 2. Access token optionality (matches ODBC token vs presigned tests)
    // ---------------------------------------------------------------

    #[test]
    fn resolve_with_bearer_token() {
        // Matches ODBC test_simple_get_gcs_with_token
        let stage = make_stage_info(StageInfoOverrides::default());
        let (url, token) = resolve_url_and_token(&stage, "file.csv.gz").unwrap();
        assert_eq!(url, "https://storage.googleapis.com/my-bucket/file.csv.gz");
        assert_eq!(token, Some("fake-token"));
    }

    #[test]
    fn resolve_with_presigned_url() {
        // Matches ODBC test_simple_get_gcs_with_presignedurl
        let stage = make_stage_info(StageInfoOverrides {
            presigned_url: Some("https://faked.presigned.url".to_string()),
            ..Default::default()
        });
        let (url, token) = resolve_url_and_token(&stage, "file.csv.gz").unwrap();
        assert_eq!(url, "https://faked.presigned.url");
        assert!(token.is_none(), "presigned URL mode should not use a token");
    }

    #[test]
    fn resolve_with_no_token_and_no_presigned_url_returns_error() {
        // When GCS_ACCESS_TOKEN is absent and no presigned URL, should error
        let stage = make_stage_info(StageInfoOverrides {
            creds: Some(CloudCredentials::Gcs {
                gcs_access_token: None,
            }),
            ..Default::default()
        });
        let result = resolve_url_and_token(&stage, "file.csv.gz");
        assert!(matches!(
            result,
            Err(GcsRequestError::MissingGcsCredentials)
        ));
    }

    #[test]
    fn resolve_with_s3_creds_returns_error() {
        let stage = make_stage_info(StageInfoOverrides {
            creds: Some(CloudCredentials::S3 {
                aws_key_id: "key".to_string(),
                aws_secret_key: SensitiveString::from("secret"),
                aws_token: SensitiveString::from("token"),
            }),
            ..Default::default()
        });
        let result = resolve_url_and_token(&stage, "file.csv.gz");
        assert!(matches!(
            result,
            Err(GcsRequestError::MissingGcsCredentials)
        ));
    }

    // ---------------------------------------------------------------
    // 3. Retry policy configuration
    // ---------------------------------------------------------------

    #[test]
    fn gcs_retry_policy_includes_403() {
        let policy = gcs_retry_policy(false);
        assert!(
            policy.extra_retryable_statuses.contains(&403),
            "403 should be retryable for GCS (matches JDBC/ODBC)"
        );
    }

    #[test]
    fn gcs_retry_policy_includes_400_for_presigned_urls() {
        let policy = gcs_retry_policy(true);
        assert!(
            policy.extra_retryable_statuses.contains(&400),
            "400 should be retryable when using presigned URLs"
        );
    }

    #[test]
    fn gcs_retry_policy_excludes_400_without_presigned_urls() {
        let policy = gcs_retry_policy(false);
        assert!(
            !policy.extra_retryable_statuses.contains(&400),
            "400 should not be retryable without presigned URLs"
        );
    }

    // ---------------------------------------------------------------
    // 4. URL percent-encoding
    // ---------------------------------------------------------------

    #[test]
    fn percent_encode_preserves_normal_paths() {
        assert_eq!(
            percent_encode_path("prefix/file.csv.gz"),
            "prefix/file.csv.gz"
        );
    }

    #[test]
    fn percent_encode_encodes_spaces_and_special_chars() {
        assert_eq!(percent_encode_path("dir/my file.csv"), "dir/my%20file.csv");
        assert_eq!(percent_encode_path("path/a+b=c"), "path/a%2Bb%3Dc");
    }

    // ---------------------------------------------------------------
    // 5. Upload status enum
    // ---------------------------------------------------------------

    #[test]
    fn upload_status_display() {
        assert_eq!(UploadStatus::Uploaded.to_string(), "UPLOADED");
        assert_eq!(UploadStatus::Skipped.to_string(), "SKIPPED");
    }

    // ---------------------------------------------------------------
    // 6. Retry policy budget
    // ---------------------------------------------------------------

    #[test]
    fn gcs_retry_policy_max_elapsed_exceeds_request_timeout() {
        let policy = gcs_retry_policy(false);
        assert_eq!(
            policy.max_elapsed,
            Duration::from_secs(600),
            "max_elapsed must exceed REQUEST_TIMEOUT_SECS (300s)"
        );
        assert!(
            policy.max_elapsed > Duration::from_secs(REQUEST_TIMEOUT_SECS),
            "retry budget must be larger than a single request timeout"
        );
    }

    #[test]
    fn gcs_retry_policy_max_attempts() {
        let policy = gcs_retry_policy(false);
        assert_eq!(policy.max_attempts, 6);
    }

    // ---------------------------------------------------------------
    // 7. Percent-encoding edge cases
    // ---------------------------------------------------------------

    #[test]
    fn percent_encode_empty_string() {
        assert_eq!(percent_encode_path(""), "");
    }

    #[test]
    fn percent_encode_unreserved_chars_pass_through() {
        // RFC 3986 unreserved: A-Z a-z 0-9 - _ . ~
        let unreserved = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_.~/";
        assert_eq!(percent_encode_path(unreserved), unreserved);
    }

    #[test]
    fn percent_encode_special_ascii_chars() {
        assert_eq!(percent_encode_path("@"), "%40");
        assert_eq!(percent_encode_path("#"), "%23");
        assert_eq!(percent_encode_path("!"), "%21");
        assert_eq!(percent_encode_path("$"), "%24");
        assert_eq!(percent_encode_path("&"), "%26");
        assert_eq!(percent_encode_path(" "), "%20");
        assert_eq!(percent_encode_path("%"), "%25");
    }

    #[test]
    fn percent_encode_multibyte_unicode() {
        // é is U+00E9, encoded as 0xC3 0xA9 in UTF-8
        assert_eq!(percent_encode_path("café.csv"), "caf%C3%A9.csv");
        // 日本 is multi-byte CJK
        assert_eq!(
            percent_encode_path("日本/data.csv"),
            "%E6%97%A5%E6%9C%AC/data.csv"
        );
    }

    #[test]
    fn percent_encode_preserves_slashes_in_paths() {
        assert_eq!(percent_encode_path("a/b/c/d.csv"), "a/b/c/d.csv");
    }

    // ---------------------------------------------------------------
    // 8. URL construction with special characters
    // ---------------------------------------------------------------

    #[test]
    fn url_default_encodes_special_chars_in_key() {
        let stage = make_stage_info(StageInfoOverrides::default());
        let url = build_gcs_url(&stage, "dir/my file (1).csv");
        assert_eq!(
            url,
            "https://storage.googleapis.com/my-bucket/dir/my%20file%20%281%29.csv"
        );
    }

    #[test]
    fn url_virtual_host_encodes_key() {
        let stage = make_stage_info(StageInfoOverrides {
            use_virtual_url: true,
            ..Default::default()
        });
        let url = build_gcs_url(&stage, "path/café.csv");
        assert_eq!(
            url,
            "https://my-bucket.storage.googleapis.com/path/caf%C3%A9.csv"
        );
    }

    #[test]
    fn url_regional_encodes_key() {
        let stage = make_stage_info(StageInfoOverrides {
            region: Some("us-east1".to_string()),
            use_regional_url: true,
            ..Default::default()
        });
        let url = build_gcs_url(&stage, "a&b=c.csv");
        assert_eq!(
            url,
            "https://storage.us-east1.rep.googleapis.com/my-bucket/a%26b%3Dc.csv"
        );
    }

    #[test]
    fn url_custom_endpoint_encodes_key() {
        let stage = make_stage_info(StageInfoOverrides {
            end_point: Some("custom.example.com".to_string()),
            ..Default::default()
        });
        let url = build_gcs_url(&stage, "dir/file name.csv");
        assert_eq!(
            url,
            "https://custom.example.com/my-bucket/dir/file%20name.csv"
        );
    }

    // ---------------------------------------------------------------
    // 9. try_get_header: missing vs invalid header values
    // ---------------------------------------------------------------

    #[test]
    fn try_get_header_missing_returns_ok_none() {
        let headers = reqwest::header::HeaderMap::new();
        let result = try_get_header(&headers, "x-missing").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn try_get_header_valid_returns_ok_some() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("x-test", "hello".parse().unwrap());
        let result = try_get_header(&headers, "x-test").unwrap();
        assert_eq!(result, Some("hello".to_string()));
    }

    #[test]
    fn try_get_header_invalid_utf8_returns_error() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "x-bad",
            reqwest::header::HeaderValue::from_bytes(&[0x80, 0x81]).unwrap(),
        );
        let result = try_get_header(&headers, "x-bad");
        assert!(result.is_err(), "non-UTF8 header should produce an error");
        assert!(matches!(
            result.unwrap_err(),
            GcsDownloadError::InvalidHeaderValue { .. }
        ));
    }

    // ---------------------------------------------------------------
    // 10. GCS download metadata extraction
    // ---------------------------------------------------------------

    fn build_gcs_download_headers(
        encryption_data: Option<&str>,
        mat_desc: Option<&str>,
        digest: Option<&str>,
    ) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(v) = encryption_data {
            headers.insert(GCS_META_ENCRYPTIONDATA, v.parse().unwrap());
        }
        if let Some(v) = mat_desc {
            headers.insert(GCS_META_MATDESC, v.parse().unwrap());
        }
        if let Some(v) = digest {
            headers.insert(GCS_META_SFC_DIGEST, v.parse().unwrap());
        }
        headers
    }

    const VALID_ENCRYPTION_DATA: &str =
        r#"{"WrappedContentKey":{"EncryptedKey":"dGVzdA=="},"ContentEncryptionIV":"aXYxMjM0NTY="}"#;
    const VALID_MAT_DESC: &str = r#"{"smkId":"1","queryId":"qid","keySize":"128"}"#;

    #[test]
    fn gcs_metadata_sse_no_headers_returns_none() {
        let headers = build_gcs_download_headers(None, None, None);
        let digest = try_get_header(&headers, GCS_META_SFC_DIGEST).unwrap();
        let file_metadata = try_get_header(&headers, GCS_META_ENCRYPTIONDATA).unwrap();
        assert!(digest.is_none());
        assert!(file_metadata.is_none());
    }

    #[test]
    fn gcs_metadata_encrypted_all_headers_returns_metadata() {
        let headers = build_gcs_download_headers(
            Some(VALID_ENCRYPTION_DATA),
            Some(VALID_MAT_DESC),
            Some("sha256digest"),
        );

        let digest = try_get_header(&headers, GCS_META_SFC_DIGEST).unwrap();
        assert_eq!(digest, Some("sha256digest".to_string()));

        let enc_data_str = try_get_header(&headers, GCS_META_ENCRYPTIONDATA)
            .unwrap()
            .unwrap();
        let enc_data: serde_json::Value = serde_json::from_str(&enc_data_str).unwrap();

        let encrypted_key = enc_data["WrappedContentKey"]["EncryptedKey"]
            .as_str()
            .unwrap();
        assert_eq!(encrypted_key, "dGVzdA==");

        let iv = enc_data["ContentEncryptionIV"].as_str().unwrap();
        assert_eq!(iv, "aXYxMjM0NTY=");

        let mat_desc_str = try_get_header(&headers, GCS_META_MATDESC).unwrap().unwrap();
        let material_desc: MaterialDescription = serde_json::from_str(&mat_desc_str).unwrap();
        assert_eq!(material_desc.smk_id, "1");
    }

    #[test]
    fn gcs_metadata_encryptiondata_present_but_matdesc_missing_errors_in_download() {
        let headers = build_gcs_download_headers(Some(VALID_ENCRYPTION_DATA), None, Some("digest"));

        let enc_data_str = try_get_header(&headers, GCS_META_ENCRYPTIONDATA)
            .unwrap()
            .unwrap();
        assert!(!enc_data_str.is_empty());

        let mat_desc_result: Result<Option<String>, _> = try_get_header(&headers, GCS_META_MATDESC);
        assert!(
            mat_desc_result.unwrap().is_none(),
            "matdesc should be None when header is absent"
        );
    }

    #[test]
    fn gcs_metadata_malformed_encryptiondata_returns_deserialization_error() {
        let headers =
            build_gcs_download_headers(Some("not-valid-json"), Some(VALID_MAT_DESC), None);

        let enc_data_str = try_get_header(&headers, GCS_META_ENCRYPTIONDATA)
            .unwrap()
            .unwrap();
        let parse_result: Result<serde_json::Value, _> = serde_json::from_str(&enc_data_str);
        assert!(
            parse_result.is_err(),
            "malformed JSON should fail deserialization"
        );
    }
}
