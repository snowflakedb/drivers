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
const RETRYABLE_STATUS_CODES: &[u16] = &[401, 408, 429, 500, 502, 503, 504];

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

    let response = execute_with_retry(&client, || {
        let mut req = client.get(&url);
        if let Some(ref t) = token {
            req = req.bearer_auth(t);
        }
        req
    })
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

    execute_with_retry(client, || {
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
    })
    .await?;

    tracing::debug!("GCS upload successful");
    Ok(())
}

// --- Retry logic (shared between upload and download) ---

/// Executes an HTTP request with retry logic and exponential backoff.
async fn execute_with_retry<F>(
    _client: &reqwest::Client,
    build_request: F,
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

        // Non-retryable errors: fail immediately
        if status_code == 400 || status_code == 404 {
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

fn resolve_url_and_token(
    stage_info: &StageInfo,
    key: &str,
) -> Result<(String, Option<String>), GcsTransferError> {
    if let Some(presigned) = &stage_info.presigned_url {
        return Ok((presigned.clone(), None));
    }

    let CloudCredentials::Gcs {
        ref gcs_access_token,
    } = stage_info.creds
    else {
        return MissingGcsCredentialsSnafu.fail();
    };

    let url = format!(
        "https://storage.googleapis.com/{}/{}",
        stage_info.bucket, key
    );
    Ok((url, Some(gcs_access_token.reveal().to_string())))
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
