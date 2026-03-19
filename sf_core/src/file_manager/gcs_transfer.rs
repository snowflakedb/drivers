use super::types::{
    CloudCredentials, EncryptedFileMetadata, EncryptionResult, MaterialDescription, StageInfo,
};
use snafu::{Location, OptionExt, ResultExt, Snafu};
use std::sync::Arc;
use std::time::Duration;

const MAX_RETRIES: u32 = 5;
const MAX_BACKOFF_EXPONENT: u32 = 4; // 2^4 = 16 seconds max
const REQUEST_TIMEOUT_SECS: u64 = 300;

/// Retryable HTTP status codes for GCS operations.
/// Note: 401 is NOT here — it triggers TokenExpired instead of a retry.
/// Note: 400 is conditionally retryable (only for presigned URLs) and handled separately.
const RETRYABLE_STATUS_CODES: &[u16] = &[403, 408, 429, 500, 502, 503, 504];

// GCS metadata header names
const GCS_META_SFC_DIGEST: &str = "x-goog-meta-sfc-digest";
const GCS_META_ENCRYPTIONDATA: &str = "x-goog-meta-encryptiondata";
const GCS_META_MATDESC: &str = "x-goog-meta-matdesc";

/// Uploads a file to GCS, skipping if it already exists and `overwrite` is false.
pub async fn upload_to_gcs_or_skip(
    encryption_result: EncryptionResult,
    stage_info: &StageInfo,
    filename: &str,
    overwrite: bool,
) -> Result<String, GcsTransferError> {
    let client = create_gcs_client()?;
    let key = format!("{}{filename}", stage_info.key_prefix);
    let (url, token) = resolve_url_and_token(stage_info, &key)?;

    if !overwrite && check_file_exists_gcs(&client, &url, token.as_deref()).await? {
        tracing::info!("File already exists in GCS: {}", key);
        return Ok("SKIPPED".to_string());
    }

    upload_to_gcs(&client, &url, token.as_deref(), encryption_result).await?;
    Ok("UPLOADED".to_string())
}

/// Downloads a file from GCS and returns encrypted data with metadata.
pub async fn download_from_gcs(
    stage_info: &StageInfo,
    filename: &str,
) -> Result<(Vec<u8>, EncryptedFileMetadata), GcsTransferError> {
    let client = create_gcs_client()?;
    let key = format!("{}{filename}", stage_info.key_prefix);
    let (url, token) = resolve_url_and_token(stage_info, &key)?;
    let using_presigned_url = stage_info.presigned_url.is_some();

    let response = execute_with_retry(
        &client,
        || {
            let mut req = client.get(&url);
            if let Some(ref t) = token {
                req = req.bearer_auth(t);
            }
            req
        },
        using_presigned_url,
    )
    .await?;

    // Extract metadata from response headers
    let headers = response.headers();
    let digest = get_header(headers, GCS_META_SFC_DIGEST)?;
    let encryption_data_str = get_header(headers, GCS_META_ENCRYPTIONDATA)?;
    let mat_desc_str = get_header(headers, GCS_META_MATDESC)?;

    // Parse encryption data JSON to extract key and IV
    let enc_data: serde_json::Value =
        serde_json::from_str(&encryption_data_str).context(DeserializationSnafu)?;

    let encrypted_key = enc_data["WrappedContentKey"]["EncryptedKey"]
        .as_str()
        .context(MissingMetadataSnafu {
            field: "WrappedContentKey.EncryptedKey",
        })?
        .to_string();

    let iv = enc_data["ContentEncryptionIV"]
        .as_str()
        .context(MissingMetadataSnafu {
            field: "ContentEncryptionIV",
        })?
        .to_string();

    let material_desc: MaterialDescription =
        serde_json::from_str(&mat_desc_str).context(DeserializationSnafu)?;

    let file_metadata = EncryptedFileMetadata {
        encrypted_key,
        iv,
        material_desc,
        digest,
    };

    let encrypted_data = response.bytes().await.context(HttpSnafu)?.to_vec();
    Ok((encrypted_data, file_metadata))
}

/// Check if a file exists in GCS via HEAD request.
async fn check_file_exists_gcs(
    client: &reqwest::Client,
    url: &str,
    token: Option<&str>,
) -> Result<bool, GcsTransferError> {
    let mut request = client.head(url);
    if let Some(t) = token {
        request = request.bearer_auth(t);
    }

    match request.send().await {
        Ok(resp) => match resp.status().as_u16() {
            200 => Ok(true),
            404 => Ok(false),
            403 => {
                tracing::warn!(
                    "Access denied checking file existence in GCS, proceeding with upload"
                );
                Ok(false)
            }
            status => {
                tracing::warn!(
                    "Unexpected status {} checking GCS file existence, proceeding with upload",
                    status
                );
                Ok(false)
            }
        },
        Err(e) => {
            tracing::warn!(
                "Error checking GCS file existence, proceeding with upload: {}",
                e
            );
            Ok(false)
        }
    }
}

/// Upload encrypted data to GCS with retry logic.
async fn upload_to_gcs(
    client: &reqwest::Client,
    url: &str,
    token: Option<&str>,
    encryption_result: EncryptionResult,
) -> Result<(), GcsTransferError> {
    // Build encryption metadata JSON (matching JDBC/Python format)
    let encryption_data = serde_json::json!({
        "EncryptionMode": "FullBlob",
        "WrappedContentKey": {
            "KeyId": "symmKey1",
            "EncryptedKey": encryption_result.metadata.encrypted_key,
            "Algorithm": "AES_CBC_256"
        },
        "EncryptionAgent": {
            "Protocol": "1.0",
            "EncryptionAlgorithm": "AES_CBC_256"
        },
        "ContentEncryptionIV": encryption_result.metadata.iv,
        "KeyWrappingMetadata": {
            "EncryptionLibrary": "Rust(OpenSSL)"
        }
    });
    let encryption_data_str =
        serde_json::to_string(&encryption_data).context(SerializationSnafu)?;

    let mat_desc = serde_json::to_string(&encryption_result.metadata.material_desc)
        .context(SerializationSnafu)?;

    // Wrap in Arc for cheap sharing on retries (avoids cloning large Vec on each retry)
    let data: Arc<Vec<u8>> = Arc::new(encryption_result.data);
    let digest = encryption_result.metadata.digest;

    execute_with_retry(
        client,
        || {
            let body_bytes = (*data).clone();
            let mut req = client
                .put(url)
                .header(GCS_META_SFC_DIGEST, &digest)
                .header(GCS_META_ENCRYPTIONDATA, &encryption_data_str)
                .header(GCS_META_MATDESC, &mat_desc)
                .header("content-encoding", "")
                .body(body_bytes);

            if let Some(t) = token {
                req = req.bearer_auth(t);
            }
            req
        },
        false,
    )
    .await?;

    tracing::debug!("GCS upload successful");
    Ok(())
}

// --- Retry logic (shared between upload and download) ---

/// Executes an HTTP request with retry logic and exponential backoff.
async fn execute_with_retry<F>(
    _client: &reqwest::Client,
    build_request: F,
    using_presigned_url: bool,
) -> Result<reqwest::Response, GcsTransferError>
where
    F: Fn() -> reqwest::RequestBuilder,
{
    let mut attempt = 0u32;
    loop {
        let response = match build_request().send().await {
            Ok(resp) => resp,
            Err(e) => {
                if attempt < MAX_RETRIES {
                    attempt += 1;
                    tracing::warn!(
                        "GCS network error (attempt {}/{}): {}",
                        attempt,
                        MAX_RETRIES,
                        e
                    );
                    tokio::time::sleep(backoff_delay(attempt)).await;
                    continue;
                }
                return Err(e).context(HttpSnafu);
            }
        };

        if response.status().is_success() {
            return Ok(response);
        }

        let status_code = response.status().as_u16();

        // 401: token expired — propagate up so the query layer can re-execute
        if status_code == 401 {
            return TokenExpiredSnafu.fail();
        }

        // 400: retryable only when using presigned URLs (URL may have expired)
        if status_code == 400 {
            if using_presigned_url && attempt < MAX_RETRIES {
                attempt += 1;
                tracing::warn!(
                    "GCS presigned URL may have expired (HTTP 400, attempt {}/{})",
                    attempt,
                    MAX_RETRIES
                );
                tokio::time::sleep(backoff_delay(attempt)).await;
                continue;
            }
            let body = read_error_body(response).await;
            return GcsHttpSnafu { status_code, body }.fail();
        }

        // 404: hard failure
        if status_code == 404 {
            let body = read_error_body(response).await;
            return GcsHttpSnafu { status_code, body }.fail();
        }

        // Retryable errors: backoff and retry
        if RETRYABLE_STATUS_CODES.contains(&status_code) && attempt < MAX_RETRIES {
            attempt += 1;
            tracing::warn!(
                "GCS retryable error {} (attempt {}/{})",
                status_code,
                attempt,
                MAX_RETRIES
            );
            tokio::time::sleep(backoff_delay(attempt)).await;
            continue;
        }

        // Exhausted retries or non-retryable status
        let body = read_error_body(response).await;
        return GcsHttpSnafu { status_code, body }.fail();
    }
}

// --- Helpers ---

fn create_gcs_client() -> Result<reqwest::Client, GcsTransferError> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .context(HttpSnafu)
}

/// Constructs the GCS URL and extracts the bearer token from stage info.
///
/// URL strategy priority (matching JDBC/ODBC/Python):
/// 1. Presigned URL — use directly, no token
/// 2. Custom endpoint — `https://{end_point}/{bucket}/{key}`
/// 3. Virtual host — `https://{bucket}.storage.googleapis.com/{key}`
/// 4. Regional — `https://storage.{region}.rep.googleapis.com/{bucket}/{key}`
/// 5. Default — `https://storage.googleapis.com/{bucket}/{key}`
fn resolve_url_and_token(
    stage_info: &StageInfo,
    key: &str,
) -> Result<(String, Option<String>), GcsTransferError> {
    // Strategy 1: presigned URL
    if let Some(presigned) = &stage_info.presigned_url {
        return Ok((presigned.clone(), None));
    }

    // Extract token (may be None in presigned-URL-only mode, but we already checked above)
    let token = match &stage_info.creds {
        CloudCredentials::Gcs { gcs_access_token } => {
            gcs_access_token.as_ref().map(|t| t.reveal().to_string())
        }
        _ => return MissingGcsCredentialsSnafu.fail(),
    };

    let url = build_gcs_url(stage_info, key);
    Ok((url, token))
}

/// Builds the GCS URL based on endpoint/virtual/regional flags.
fn build_gcs_url(stage_info: &StageInfo, key: &str) -> String {
    // Strategy 2: custom endpoint
    if let Some(ref ep) = stage_info.end_point
        && !ep.is_empty()
    {
        let base = if ep.starts_with("https://") || ep.starts_with("http://") {
            ep.clone()
        } else {
            format!("https://{ep}")
        };
        return format!("{base}/{}/{key}", stage_info.bucket);
    }

    // Strategy 3: virtual host
    if stage_info.use_virtual_url {
        return format!("https://{}.storage.googleapis.com/{key}", stage_info.bucket);
    }

    // Strategy 4: regional
    if stage_info.use_regional_url {
        return format!(
            "https://storage.{}.rep.googleapis.com/{}/{key}",
            stage_info.region.to_lowercase(),
            stage_info.bucket
        );
    }

    // Strategy 5: default
    format!("https://storage.googleapis.com/{}/{key}", stage_info.bucket)
}

fn backoff_delay(attempt: u32) -> Duration {
    let exponent = attempt.saturating_sub(1).min(MAX_BACKOFF_EXPONENT);
    let secs = 1u64.checked_shl(exponent).unwrap_or(16);
    Duration::from_secs(secs.min(16))
}

fn get_header(
    headers: &reqwest::header::HeaderMap,
    name: &str,
) -> Result<String, GcsTransferError> {
    headers
        .get(name)
        .context(MissingMetadataSnafu {
            field: name.to_string(),
        })?
        .to_str()
        .context(InvalidHeaderValueSnafu)
        .map(|s| s.to_string())
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

// --- Unified error type ---

#[derive(Snafu, Debug, error_trace::ErrorTrace)]
pub enum GcsTransferError {
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
        assert_eq!(token, Some("fake-token".to_string()));
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
    fn resolve_with_no_token_and_no_presigned_url() {
        // When GCS_ACCESS_TOKEN is absent and no presigned URL, token should be None
        let stage = make_stage_info(StageInfoOverrides {
            creds: Some(CloudCredentials::Gcs {
                gcs_access_token: None,
            }),
            ..Default::default()
        });
        let (url, token) = resolve_url_and_token(&stage, "file.csv.gz").unwrap();
        assert_eq!(url, "https://storage.googleapis.com/my-bucket/file.csv.gz");
        assert!(token.is_none());
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
            Err(GcsTransferError::MissingGcsCredentials { .. })
        ));
    }

    // ---------------------------------------------------------------
    // 3. Retryable status codes
    //    (matches ODBC test_retryable_http_code, JDBC RestRequestTest)
    // ---------------------------------------------------------------

    #[test]
    fn retryable_status_codes_include_403() {
        assert!(
            RETRYABLE_STATUS_CODES.contains(&403),
            "403 should be retryable (matches JDBC/ODBC)"
        );
    }

    #[test]
    fn retryable_status_codes_include_standard_set() {
        for code in &[408, 429, 500, 502, 503, 504] {
            assert!(
                RETRYABLE_STATUS_CODES.contains(code),
                "{code} should be retryable"
            );
        }
    }

    #[test]
    fn retryable_status_codes_exclude_401() {
        assert!(
            !RETRYABLE_STATUS_CODES.contains(&401),
            "401 must NOT be in retryable set — it triggers TokenExpired"
        );
    }

    #[test]
    fn retryable_status_codes_exclude_400() {
        assert!(
            !RETRYABLE_STATUS_CODES.contains(&400),
            "400 is only retryable for presigned URLs (handled separately)"
        );
    }

    #[test]
    fn retryable_status_codes_exclude_404() {
        assert!(
            !RETRYABLE_STATUS_CODES.contains(&404),
            "404 should be a hard failure"
        );
    }

    // ---------------------------------------------------------------
    // 4. Backoff delay calculation
    // ---------------------------------------------------------------

    #[test]
    fn backoff_delay_values() {
        assert_eq!(backoff_delay(1), Duration::from_secs(1));
        assert_eq!(backoff_delay(2), Duration::from_secs(2));
        assert_eq!(backoff_delay(3), Duration::from_secs(4));
        assert_eq!(backoff_delay(4), Duration::from_secs(8));
        assert_eq!(backoff_delay(5), Duration::from_secs(16));
        // Capped at 16s
        assert_eq!(backoff_delay(6), Duration::from_secs(16));
        assert_eq!(backoff_delay(100), Duration::from_secs(16));
    }
}
