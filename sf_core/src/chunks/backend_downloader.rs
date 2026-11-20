//! Routes result-chunk downloads through the registered XP backend instead
//! of HTTP. Mirrors `http_downloader.rs`'s shape so [`ChunkTransport`] can
//! pick between the two without the prefetch pipeline itself knowing which
//! transport is in play.

use std::sync::Arc;
use std::time::Duration;

use arrow::error::ArrowError;
use reqwest::Client;

use super::http_downloader::HttpChunkDownloader;
use super::prefetch::DownloadChunk;
use crate::config::retry::RetryPolicy;
use crate::xp_backend::SnowflakeBackend;

use super::ChunkDownloadData;

/// Downloads a result chunk through [`SnowflakeBackend::download_query_chunk`]
/// rather than an outbound HTTP request — the path XP mode uses, since the
/// sandbox has no route to cloud storage.
#[derive(Clone)]
pub(crate) struct BackendChunkDownloader {
    pub(crate) backend: Arc<dyn SnowflakeBackend>,
}

impl DownloadChunk for BackendChunkDownloader {
    #[tracing::instrument(
        name = "core_chunk_download_backend",
        target = "sf_core::perf",
        level = "trace",
        skip_all
    )]
    async fn download_chunk(&self, chunk: ChunkDownloadData) -> Result<Vec<u8>, ArrowError> {
        let bytes = self
            .backend
            .download_query_chunk(&chunk.url, &chunk.headers)
            .await
            .map_err(|e| ArrowError::ExternalError(Box::new(e)))?;
        super::finish_backend_chunk_body(bytes, &chunk)
            .await
            .map_err(|e| ArrowError::ExternalError(Box::new(e)))
    }
}

/// Picks HTTP or the registered XP backend once per prefetch pipeline, so
/// every construction site in `chunks/mod.rs` threads the same choice rather
/// than re-deriving it. `Some(backend)` always wins: a process running under
/// `SNOWFLAKE_RUNNING_INSIDE_XP` has no outbound HTTP path to fall back to.
///
/// `read_timeout` applies only to [`ChunkTransport::Http`]. The backend
/// callback returns the whole body in one blocking call that sf_core cannot
/// cancel, so the host owns timeouts on that path.
#[derive(Clone)]
pub(crate) enum ChunkTransport {
    Http(HttpChunkDownloader),
    Backend(BackendChunkDownloader),
}

impl ChunkTransport {
    pub(crate) fn new(
        client: Client,
        read_timeout: Option<Duration>,
        retry_policy: RetryPolicy,
        xp_backend: Option<Arc<dyn SnowflakeBackend>>,
    ) -> Self {
        match xp_backend {
            Some(backend) => Self::Backend(BackendChunkDownloader { backend }),
            None => Self::Http(HttpChunkDownloader {
                client,
                read_timeout,
                retry_policy,
            }),
        }
    }
}

impl DownloadChunk for ChunkTransport {
    async fn download_chunk(&self, chunk: ChunkDownloadData) -> Result<Vec<u8>, ArrowError> {
        match self {
            Self::Http(downloader) => downloader.download_chunk(chunk).await,
            Self::Backend(downloader) => downloader.download_chunk(chunk).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use async_trait::async_trait;

    use super::*;
    use crate::config::rest_parameters::LoginParameters;
    use crate::rest::snowflake::{LoginResult, QueryInput, query_response};
    use crate::xp_backend::{
        BackendCancelOutcome, BackendCancelTarget, BackendError, BackendQueryOptions,
        BackendResult, error_codes,
    };

    struct RecordingBackend {
        requested_url: std::sync::Mutex<Option<String>>,
        requested_headers: std::sync::Mutex<Option<HashMap<String, String>>>,
        response: BackendResult<Vec<u8>>,
    }

    #[async_trait]
    impl SnowflakeBackend for RecordingBackend {
        async fn authenticate(
            &self,
            _params: &LoginParameters,
            _session_parameters: Option<&HashMap<String, String>>,
        ) -> BackendResult<LoginResult> {
            unimplemented!()
        }

        async fn execute_query(
            &self,
            _input: &QueryInput<'_>,
            _params: &crate::config::rest_parameters::QueryParameters,
            _options: BackendQueryOptions,
        ) -> BackendResult<query_response::Response> {
            unimplemented!()
        }

        async fn get_query_status(&self, _query_id: &str) -> BackendResult<String> {
            unimplemented!()
        }

        async fn get_query_result(
            &self,
            _query_id: &str,
        ) -> BackendResult<query_response::Response> {
            unimplemented!()
        }

        async fn cancel_query(
            &self,
            _target: BackendCancelTarget<'_>,
        ) -> BackendResult<BackendCancelOutcome> {
            unimplemented!()
        }

        async fn get_session_parameters(&self) -> BackendResult<HashMap<String, String>> {
            unimplemented!()
        }

        async fn download_query_chunk(
            &self,
            url: &str,
            headers: &HashMap<String, String>,
        ) -> BackendResult<Vec<u8>> {
            *self.requested_url.lock().unwrap() = Some(url.to_string());
            *self.requested_headers.lock().unwrap() = Some(headers.clone());
            self.response.clone()
        }
    }

    fn chunk_with_headers() -> ChunkDownloadData {
        ChunkDownloadData {
            url: "https://chunk.test/c1".to_string(),
            row_count: 1,
            uncompressed_size: 10,
            compressed_size: 10,
            headers: HashMap::from([("x-chunk-token".to_string(), "secret".to_string())]),
        }
    }

    #[tokio::test]
    async fn backend_chunk_downloader_forwards_url_and_headers() {
        let backend = Arc::new(RecordingBackend {
            requested_url: std::sync::Mutex::new(None),
            requested_headers: std::sync::Mutex::new(None),
            response: Ok(b"chunk bytes".to_vec()),
        });
        let downloader = BackendChunkDownloader {
            backend: Arc::clone(&backend) as Arc<dyn SnowflakeBackend>,
        };

        let bytes = downloader
            .download_chunk(chunk_with_headers())
            .await
            .expect("backend should serve the chunk");

        assert_eq!(bytes, b"chunk bytes");
        assert_eq!(
            backend.requested_url.lock().unwrap().as_deref(),
            Some("https://chunk.test/c1")
        );
        assert_eq!(
            backend
                .requested_headers
                .lock()
                .unwrap()
                .as_ref()
                .and_then(|h| h.get("x-chunk-token")),
            Some(&"secret".to_string())
        );
    }

    #[tokio::test]
    async fn backend_chunk_downloader_inflates_a_gzip_body() {
        use std::io::Write as _;

        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(b"chunk bytes").unwrap();
        let backend = Arc::new(RecordingBackend {
            requested_url: std::sync::Mutex::new(None),
            requested_headers: std::sync::Mutex::new(None),
            response: Ok(encoder.finish().unwrap()),
        });
        let downloader = BackendChunkDownloader {
            backend: backend as Arc<dyn SnowflakeBackend>,
        };

        let bytes = downloader
            .download_chunk(chunk_with_headers())
            .await
            .expect("a gzip body from the host should be inflated");
        assert_eq!(bytes, b"chunk bytes");
    }

    #[tokio::test]
    async fn backend_chunk_downloader_surfaces_a_backend_failure_as_an_arrow_external_error() {
        let backend = Arc::new(RecordingBackend {
            requested_url: std::sync::Mutex::new(None),
            requested_headers: std::sync::Mutex::new(None),
            response: Err(BackendError::new(error_codes::PROTOCOL, "boom")),
        });
        let downloader = BackendChunkDownloader {
            backend: backend as Arc<dyn SnowflakeBackend>,
        };

        let err = downloader
            .download_chunk(chunk_with_headers())
            .await
            .expect_err("a backend failure should not be silently swallowed");
        assert!(matches!(err, ArrowError::ExternalError(_)));
        assert!(err.to_string().contains("boom"));
    }

    #[tokio::test]
    async fn chunk_transport_prefers_the_backend_when_one_is_registered() {
        let backend = Arc::new(RecordingBackend {
            requested_url: std::sync::Mutex::new(None),
            requested_headers: std::sync::Mutex::new(None),
            response: Ok(b"from backend".to_vec()),
        });
        let transport = ChunkTransport::new(
            Client::new(),
            None,
            RetryPolicy::default(),
            Some(Arc::clone(&backend) as Arc<dyn SnowflakeBackend>),
        );

        let bytes = transport
            .download_chunk(chunk_with_headers())
            .await
            .expect("backend transport should serve the chunk");
        assert_eq!(bytes, b"from backend");
        assert!(
            backend.requested_url.lock().unwrap().is_some(),
            "the backend, not HTTP, must have been asked for the chunk"
        );
    }

    #[test]
    fn chunk_transport_falls_back_to_http_without_a_registered_backend() {
        let transport = ChunkTransport::new(Client::new(), None, RetryPolicy::default(), None);
        assert!(matches!(transport, ChunkTransport::Http(_)));
    }
}
