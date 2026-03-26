use super::types::{
    CloudCredentials, EncryptedFileMetadata, MaterialDescription, PreparedUpload, StageInfo,
    UploadStatus, build_encryption_metadata_json, percent_encode_path,
};
use crate::config::retry::{BackoffConfig, Jitter, RetryPolicy};
use crate::http::retry::{HttpContext, HttpError, execute_with_retry as http_execute_with_retry};
use reqwest::{Method, StatusCode};
use snafu::{Location, OptionExt, ResultExt, Snafu};
use std::time::Duration;

const REQUEST_TIMEOUT_SECS: u64 = 300;

// Azure metadata header names
const AZURE_META_SFC_DIGEST: &str = "x-ms-meta-sfcdigest";
const AZURE_META_ENCRYPTIONDATA: &str = "x-ms-meta-encryptiondata";
const AZURE_META_MATDESC: &str = "x-ms-meta-matdesc";

/// Uploads a file to Azure Blob Storage, skipping if it already exists and `overwrite` is false.
pub async fn upload_to_azure_or_skip(
    prepared: PreparedUpload,
    stage_info: &StageInfo,
    filename: &str,
    overwrite: bool,
) -> Result<UploadStatus, AzureUploadError> {
    let client = create_azure_client()?;
    let key = format!("{}{filename}", stage_info.key_prefix);
    let (url, sas_token) = resolve_url_and_token(stage_info, &key)?;

    if !overwrite && check_blob_exists(&client, &url, sas_token).await {
        tracing::info!("Blob already exists in Azure: {}", key);
        return Ok(UploadStatus::Skipped);
    }

    upload_to_azure(&client, &url, sas_token, prepared).await?;
    Ok(UploadStatus::Uploaded)
}

/// Downloads a file from Azure Blob Storage and returns data with optional encryption metadata.
/// For SSE stages the metadata headers will be absent and `None` is returned.
pub async fn download_from_azure(
    stage_info: &StageInfo,
    filename: &str,
) -> Result<(Vec<u8>, Option<String>, Option<EncryptedFileMetadata>), AzureDownloadError> {
    let client = create_azure_client()?;
    let key = format!("{}{filename}", stage_info.key_prefix);
    let (url, sas_token) = resolve_url_and_token(stage_info, &key)?;
    let full_url = build_sas_url(&url, sas_token);

    let response = azure_request_with_retry(|| client.get(&full_url), Method::GET).await?;

    // Extract metadata from response headers
    let headers = response.headers();
    let digest = try_get_header(headers, AZURE_META_SFC_DIGEST)?;

    let file_metadata = match try_get_header(headers, AZURE_META_ENCRYPTIONDATA)? {
        Some(encryption_data_str) => {
            let enc_data: serde_json::Value = serde_json::from_str(&encryption_data_str)
                .context(azure_download_error::DeserializationSnafu)?;

            let encrypted_key = enc_data["WrappedContentKey"]["EncryptedKey"]
                .as_str()
                .context(azure_download_error::MissingMetadataSnafu {
                    field: "WrappedContentKey.EncryptedKey",
                })?
                .to_string();

            let iv = enc_data["ContentEncryptionIV"]
                .as_str()
                .context(azure_download_error::MissingMetadataSnafu {
                    field: "ContentEncryptionIV",
                })?
                .to_string();

            let mat_desc_str = try_get_header(headers, AZURE_META_MATDESC)?.context(
                azure_download_error::MissingMetadataSnafu {
                    field: AZURE_META_MATDESC,
                },
            )?;
            let material_desc: MaterialDescription = serde_json::from_str(&mat_desc_str)
                .context(azure_download_error::DeserializationSnafu)?;

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
        .map_err(|e| AzureRequestError::Http {
            detail: sanitize_sas(e.to_string()),
        })?
        .to_vec();

    Ok((data, digest, file_metadata))
}

/// Check if a blob exists in Azure via HEAD request.
/// Returns false on any error or non-200 status so the caller proceeds with upload.
async fn check_blob_exists(client: &reqwest::Client, url: &str, sas_token: &str) -> bool {
    let full_url = build_sas_url(url, sas_token);
    match client.head(&full_url).send().await {
        Ok(resp) => match resp.status() {
            StatusCode::OK => true,
            StatusCode::NOT_FOUND => false,
            StatusCode::FORBIDDEN => {
                tracing::warn!(
                    "Access denied checking blob existence in Azure, proceeding with upload"
                );
                false
            }
            status => {
                tracing::warn!(
                    "Unexpected status {} checking Azure blob existence, proceeding with upload",
                    status
                );
                false
            }
        },
        Err(e) => {
            tracing::warn!(
                "Error checking Azure blob existence, proceeding with upload: {}",
                sanitize_sas(e.to_string())
            );
            false
        }
    }
}

/// Upload data to Azure with retry logic.
/// Sets encryption metadata headers only when client-side encryption was used.
async fn upload_to_azure(
    client: &reqwest::Client,
    url: &str,
    sas_token: &str,
    prepared: PreparedUpload,
) -> Result<(), AzureUploadError> {
    let encryption_data_str = prepared
        .encryption_metadata
        .as_ref()
        .map(|enc_meta| {
            let encryption_data = build_encryption_metadata_json(enc_meta);
            serde_json::to_string(&encryption_data)
        })
        .transpose()
        .context(azure_upload_error::SerializationSnafu)?;

    let mat_desc_str = prepared
        .encryption_metadata
        .as_ref()
        .map(|enc_meta| serde_json::to_string(&enc_meta.material_desc))
        .transpose()
        .context(azure_upload_error::SerializationSnafu)?;

    let data = prepared.data;
    let digest = prepared.digest;
    let full_url = build_sas_url(url, sas_token);

    azure_request_with_retry(
        || {
            let mut req = client
                .put(&full_url)
                .header("x-ms-blob-type", "BlockBlob")
                // Empty content-encoding matches JDBC/ODBC behavior: explicitly clears
                // any default encoding to ensure the blob is stored as-is.
                .header("content-encoding", "")
                .header(AZURE_META_SFC_DIGEST, &digest)
                .body(data.clone());

            if let Some(ref enc_str) = encryption_data_str {
                req = req.header(AZURE_META_ENCRYPTIONDATA, enc_str);
            }
            if let Some(ref md_str) = mat_desc_str {
                req = req.header(AZURE_META_MATDESC, md_str);
            }
            req
        },
        Method::PUT,
    )
    .await?;

    tracing::debug!("Azure blob upload successful");
    Ok(())
}

// --- Retry logic (delegates to http::retry) ---

/// Returns a retry policy tuned for Azure file-transfer operations.
///
/// Azure treats 403 as retryable (SAS token clock skew / replication delays),
/// matching JDBC/ODBC behavior.
fn azure_retry_policy() -> RetryPolicy {
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
        extra_retryable_statuses: vec![403],
        ..RetryPolicy::default()
    }
}

/// Executes an Azure HTTP request with retry, then checks for Azure-specific status codes.
///
/// Unlike GCS, Azure does not have a `TokenExpired` (401) fast-fail path.
/// Azure SAS tokens are URL-embedded and produce 403 on expiry (which is already retried).
/// SAS tokens cannot be refreshed mid-request — a new query execution is needed.
async fn azure_request_with_retry<F>(
    build_request: F,
    method: Method,
) -> Result<reqwest::Response, AzureRequestError>
where
    F: Fn() -> reqwest::RequestBuilder,
{
    let ctx = HttpContext::new(method, "azure-transfer");
    let policy = azure_retry_policy();

    let response = http_execute_with_retry(build_request, &ctx, &policy, |r| async move { Ok(r) })
        .await
        .map_err(map_http_error)?;

    if response.status().is_success() {
        return Ok(response);
    }

    let status_code = response.status().as_u16();
    let body = read_error_body(response).await;
    Err(AzureRequestError::AzureHttp { status_code, body })
}

fn map_http_error(e: HttpError) -> AzureRequestError {
    match e {
        HttpError::Transport { source, .. } => AzureRequestError::Http {
            detail: sanitize_sas(source.to_string()),
        },
        other => AzureRequestError::RetryExhausted {
            detail: sanitize_sas(other.to_string()),
        },
    }
}

// --- Helpers ---

fn create_azure_client() -> Result<reqwest::Client, AzureRequestError> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| AzureRequestError::Http {
            detail: e.to_string(),
        })
}

/// Constructs the Azure Blob Storage URL and extracts the SAS token from stage info.
///
/// URL format: `https://{storageAccount}.{blob_endpoint}/{container}/{blob_path}`
///
/// The endpoint value comes from Snowflake and may vary by environment
/// (commercial, government, China). It is used as-is from the server response,
/// with a `blob.` prefix prepended only if absent.
fn resolve_url_and_token<'a>(
    stage_info: &'a StageInfo,
    key: &str,
) -> Result<(String, &'a str), AzureRequestError> {
    let sas_token = match &stage_info.creds {
        CloudCredentials::Azure { sas_token } => sas_token.reveal(),
        _ => return Err(AzureRequestError::MissingAzureCredentials),
    };

    let url = build_azure_url(stage_info, key)?;
    Ok((url, sas_token))
}

/// Builds the Azure Blob Storage URL for a given object key.
fn build_azure_url(stage_info: &StageInfo, key: &str) -> Result<String, AzureRequestError> {
    let storage_account = stage_info
        .storage_account
        .as_ref()
        .filter(|sa| !sa.is_empty())
        .ok_or(AzureRequestError::MissingMetadata {
            field: "storage_account".to_string(),
        })?;

    let raw_endpoint = stage_info
        .end_point
        .as_deref()
        .unwrap_or("blob.core.windows.net");

    // Normalize the endpoint to a bare hostname (strip any URL scheme or path).
    let endpoint = {
        let without_scheme = raw_endpoint
            .split_once("://")
            .map(|(_, rest)| rest)
            .unwrap_or(raw_endpoint);
        without_scheme
            .trim_start_matches('/')
            .split('/')
            .next()
            .unwrap_or(without_scheme)
    };

    // The Snowflake server may provide the endpoint with or without the "blob." prefix.
    // Azure Government uses "blob.core.usgovcloudapi.net", Azure China uses
    // "blob.core.chinacloudapi.cn". We prepend "blob." only when it's missing.
    let blob_endpoint = if endpoint.starts_with("blob.") {
        endpoint.to_string()
    } else {
        format!("blob.{endpoint}")
    };

    let encoded_key = percent_encode_path(key);

    Ok(format!(
        "https://{storage_account}.{blob_endpoint}/{}/{encoded_key}",
        stage_info.bucket
    ))
}

/// Appends the SAS token to a URL as a query parameter.
fn build_sas_url(base_url: &str, sas_token: &str) -> String {
    let token = sas_token.strip_prefix('?').unwrap_or(sas_token);
    let separator = if base_url.contains('?') { "&" } else { "?" };
    format!("{base_url}{separator}{token}")
}

fn try_get_header(
    headers: &reqwest::header::HeaderMap,
    name: &str,
) -> Result<Option<String>, AzureDownloadError> {
    match headers.get(name) {
        Some(value) => {
            let s = value
                .to_str()
                .context(azure_download_error::InvalidHeaderValueSnafu)?;
            Ok(Some(s.to_string()))
        }
        None => Ok(None),
    }
}

async fn read_error_body(response: reqwest::Response) -> String {
    let body = match response.text().await {
        Ok(text) => text,
        Err(e) => {
            tracing::warn!("Failed to read Azure error response body: {}", e);
            return format!("<could not read body: {}>", e);
        }
    };
    // Scrub SAS tokens (sig=…) from Azure error responses which may echo the request URL.
    sanitize_sas(body)
}

/// Removes SAS token signature values from a string to prevent credential leakage in logs.
/// Handles multiple `sig=` occurrences (e.g., when error bodies echo URLs more than once).
fn sanitize_sas(input: String) -> String {
    let mut result = String::with_capacity(input.len());
    let mut remaining = input.as_str();
    while let Some(start) = remaining.find("sig=") {
        result.push_str(&remaining[..start]);
        result.push_str("sig=REDACTED");
        let value_start = start + 4;
        let value_end = remaining[value_start..]
            .find('&')
            .map(|i| value_start + i)
            .unwrap_or(remaining.len());
        remaining = &remaining[value_end..];
    }
    result.push_str(remaining);
    result
}

// --- Error types ---

/// Internal error for shared helpers (retry, client creation, URL resolution).
/// Converted into `AzureUploadError` or `AzureDownloadError` via `From` impls.
#[derive(Debug, Snafu)]
enum AzureRequestError {
    #[snafu(display("Azure HTTP error: {detail}"))]
    Http { detail: String },
    #[snafu(display("Azure request failed: HTTP {status_code}: {body}"))]
    AzureHttp { status_code: u16, body: String },
    #[snafu(display("Missing Azure credentials"))]
    MissingAzureCredentials,
    #[snafu(display("Missing Azure metadata: {field}"))]
    MissingMetadata { field: String },
    #[snafu(display("Azure retry exhausted: {detail}"))]
    RetryExhausted { detail: String },
}

impl From<AzureRequestError> for AzureUploadError {
    fn from(e: AzureRequestError) -> Self {
        match e {
            AzureRequestError::Http { detail } => AzureUploadError::Http {
                detail,
                location: Location::default(),
            },
            AzureRequestError::AzureHttp { status_code, body } => AzureUploadError::AzureHttp {
                status_code,
                body,
                location: Location::default(),
            },
            AzureRequestError::MissingAzureCredentials => {
                AzureUploadError::MissingAzureCredentials {
                    location: Location::default(),
                }
            }
            AzureRequestError::MissingMetadata { field } => AzureUploadError::MissingMetadata {
                field,
                location: Location::default(),
            },
            AzureRequestError::RetryExhausted { detail } => AzureUploadError::RetryExhausted {
                detail,
                location: Location::default(),
            },
        }
    }
}

impl From<AzureRequestError> for AzureDownloadError {
    fn from(e: AzureRequestError) -> Self {
        match e {
            AzureRequestError::Http { detail } => AzureDownloadError::Http {
                detail,
                location: Location::default(),
            },
            AzureRequestError::AzureHttp { status_code, body } => AzureDownloadError::AzureHttp {
                status_code,
                body,
                location: Location::default(),
            },
            AzureRequestError::MissingAzureCredentials => {
                AzureDownloadError::MissingAzureCredentials {
                    location: Location::default(),
                }
            }
            AzureRequestError::MissingMetadata { field } => AzureDownloadError::MissingMetadata {
                field,
                location: Location::default(),
            },
            AzureRequestError::RetryExhausted { detail } => AzureDownloadError::RetryExhausted {
                detail,
                location: Location::default(),
            },
        }
    }
}

#[derive(Snafu, Debug, error_trace::ErrorTrace)]
#[snafu(module)]
pub enum AzureUploadError {
    #[snafu(display("Azure HTTP error: {detail}"))]
    Http {
        detail: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Azure request failed: HTTP {status_code}: {body}"))]
    AzureHttp {
        status_code: u16,
        body: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to serialize Azure metadata"))]
    Serialization {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Missing Azure credentials"))]
    MissingAzureCredentials {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Missing Azure metadata: {field}"))]
    MissingMetadata {
        field: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Azure retry exhausted: {detail}"))]
    RetryExhausted {
        detail: String,
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Snafu, Debug, error_trace::ErrorTrace)]
#[snafu(module)]
pub enum AzureDownloadError {
    #[snafu(display("Azure HTTP error: {detail}"))]
    Http {
        detail: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Azure request failed: HTTP {status_code}: {body}"))]
    AzureHttp {
        status_code: u16,
        body: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to deserialize Azure metadata"))]
    Deserialization {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Missing Azure metadata: {field}"))]
    MissingMetadata {
        field: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Invalid Azure header value"))]
    InvalidHeaderValue {
        source: reqwest::header::ToStrError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Missing Azure credentials"))]
    MissingAzureCredentials {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Azure retry exhausted: {detail}"))]
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
            location_type: super::super::types::LocationType::Azure,
            bucket: overrides.bucket.unwrap_or("my-container".to_string()),
            key_prefix: overrides.key_prefix.unwrap_or("prefix/".to_string()),
            region: overrides.region.unwrap_or("eastus2".to_string()),
            creds: overrides.creds.unwrap_or(CloudCredentials::Azure {
                sas_token: SensitiveString::from("fake-sas-token"),
            }),
            end_point: overrides.end_point,
            presigned_url: None,
            use_virtual_url: false,
            use_regional_url: false,
            storage_account: overrides
                .storage_account
                .or(Some("mystorageaccount".to_string())),
        }
    }

    #[derive(Default)]
    struct StageInfoOverrides {
        bucket: Option<String>,
        key_prefix: Option<String>,
        region: Option<String>,
        creds: Option<CloudCredentials>,
        end_point: Option<String>,
        storage_account: Option<String>,
    }

    // ---------------------------------------------------------------
    // 1. URL construction
    // ---------------------------------------------------------------

    #[test]
    fn url_default_endpoint() {
        let stage = make_stage_info(StageInfoOverrides::default());
        let url = build_azure_url(&stage, "prefix/file.csv.gz").unwrap();
        assert_eq!(
            url,
            "https://mystorageaccount.blob.core.windows.net/my-container/prefix/file.csv.gz"
        );
    }

    #[test]
    fn url_custom_endpoint_with_blob_prefix() {
        let stage = make_stage_info(StageInfoOverrides {
            end_point: Some("blob.core.usgovcloudapi.net".to_string()),
            ..Default::default()
        });
        let url = build_azure_url(&stage, "prefix/file.csv.gz").unwrap();
        assert_eq!(
            url,
            "https://mystorageaccount.blob.core.usgovcloudapi.net/my-container/prefix/file.csv.gz"
        );
    }

    #[test]
    fn url_custom_endpoint_without_blob_prefix() {
        let stage = make_stage_info(StageInfoOverrides {
            end_point: Some("core.chinacloudapi.cn".to_string()),
            ..Default::default()
        });
        let url = build_azure_url(&stage, "prefix/file.csv.gz").unwrap();
        assert_eq!(
            url,
            "https://mystorageaccount.blob.core.chinacloudapi.cn/my-container/prefix/file.csv.gz"
        );
    }

    #[test]
    fn url_endpoint_without_trailing_slash() {
        let stage = make_stage_info(StageInfoOverrides {
            end_point: Some("core.windows.net".to_string()),
            ..Default::default()
        });
        let url = build_azure_url(&stage, "prefix/file.csv.gz").unwrap();
        assert_eq!(
            url,
            "https://mystorageaccount.blob.core.windows.net/my-container/prefix/file.csv.gz"
        );
    }

    #[test]
    fn url_missing_storage_account() {
        let mut stage = make_stage_info(StageInfoOverrides::default());
        stage.storage_account = None;
        let result = build_azure_url(&stage, "prefix/file.csv.gz");
        assert!(result.is_err());
    }

    #[test]
    fn url_with_nested_path() {
        let stage = make_stage_info(StageInfoOverrides::default());
        let url = build_azure_url(&stage, "deep/nested/path/file.csv.gz").unwrap();
        assert!(url.contains("deep/nested/path/file.csv.gz"));
    }

    // ---------------------------------------------------------------
    // 2. SAS token handling
    // ---------------------------------------------------------------

    #[test]
    fn sas_url_appends_token() {
        let url = build_sas_url(
            "https://example.blob.core.windows.net/c/f",
            "sv=2021&sig=abc",
        );
        assert_eq!(
            url,
            "https://example.blob.core.windows.net/c/f?sv=2021&sig=abc"
        );
    }

    #[test]
    fn sas_url_strips_leading_question_mark() {
        let url = build_sas_url(
            "https://example.blob.core.windows.net/c/f",
            "?sv=2021&sig=abc",
        );
        assert_eq!(
            url,
            "https://example.blob.core.windows.net/c/f?sv=2021&sig=abc"
        );
    }

    #[test]
    fn resolve_with_sas_token() {
        let stage = make_stage_info(StageInfoOverrides::default());
        let (url, token) = resolve_url_and_token(&stage, "prefix/file.csv.gz").unwrap();
        assert!(url.starts_with("https://mystorageaccount.blob.core.windows.net/"));
        assert_eq!(token, "fake-sas-token");
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
        let result = resolve_url_and_token(&stage, "prefix/file.csv.gz");
        assert!(matches!(
            result,
            Err(AzureRequestError::MissingAzureCredentials)
        ));
    }

    // ---------------------------------------------------------------
    // 3. Retry policy configuration
    // ---------------------------------------------------------------

    #[test]
    fn azure_retry_policy_includes_403() {
        let policy = azure_retry_policy();
        assert!(
            policy.extra_retryable_statuses.contains(&403),
            "403 should be retryable (SAS token clock skew / replication delays)"
        );
    }

    #[test]
    fn azure_retry_policy_max_elapsed_exceeds_request_timeout() {
        let policy = azure_retry_policy();
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
    fn azure_retry_policy_max_attempts() {
        let policy = azure_retry_policy();
        assert_eq!(policy.max_attempts, 6);
    }

    // ---------------------------------------------------------------
    // 4. SAS token sanitization
    // ---------------------------------------------------------------

    #[test]
    fn sanitize_sas_redacts_signature() {
        let input =
            "https://acct.blob.core.windows.net/c/f?sv=2021&sig=secret123&se=2026".to_string();
        let result = sanitize_sas(input);
        assert_eq!(
            result,
            "https://acct.blob.core.windows.net/c/f?sv=2021&sig=REDACTED&se=2026"
        );
    }

    #[test]
    fn sanitize_sas_handles_sig_at_end() {
        let input = "https://acct.blob.core.windows.net/c/f?sv=2021&sig=secret123".to_string();
        let result = sanitize_sas(input);
        assert_eq!(
            result,
            "https://acct.blob.core.windows.net/c/f?sv=2021&sig=REDACTED"
        );
    }

    #[test]
    fn sanitize_sas_no_sig_unchanged() {
        let input = "no signature here".to_string();
        let result = sanitize_sas(input);
        assert_eq!(result, "no signature here");
    }

    #[test]
    fn sanitize_sas_redacts_multiple_occurrences() {
        let input = "url1?sig=secret1&se=2026 url2?sig=secret2&se=2027".to_string();
        let result = sanitize_sas(input);
        assert!(!result.contains("secret1"));
        assert!(!result.contains("secret2"));
        assert!(result.contains("sig=REDACTED"));
    }

    #[test]
    fn url_endpoint_with_scheme_is_normalized() {
        let stage = make_stage_info(StageInfoOverrides {
            end_point: Some("https://blob.core.windows.net/".to_string()),
            ..Default::default()
        });
        let url = build_azure_url(&stage, "prefix/file.csv.gz").unwrap();
        assert_eq!(
            url,
            "https://mystorageaccount.blob.core.windows.net/my-container/prefix/file.csv.gz"
        );
    }

    // ---------------------------------------------------------------
    // 5. URL with special characters (uses shared percent_encode_path)
    // ---------------------------------------------------------------

    #[test]
    fn url_encodes_special_chars_in_key() {
        let stage = make_stage_info(StageInfoOverrides::default());
        let url = build_azure_url(&stage, "dir/my file (1).csv").unwrap();
        assert_eq!(
            url,
            "https://mystorageaccount.blob.core.windows.net/my-container/dir/my%20file%20%281%29.csv"
        );
    }

    // ---------------------------------------------------------------
    // 6. Upload status enum
    // ---------------------------------------------------------------

    #[test]
    fn upload_status_display() {
        assert_eq!(UploadStatus::Uploaded.to_string(), "UPLOADED");
        assert_eq!(UploadStatus::Skipped.to_string(), "SKIPPED");
    }
}
