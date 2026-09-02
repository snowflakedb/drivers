//! Cancelling a GET must stop the in-flight transfer and leave no debris, rather
//! than drop the local future and let the download finish in the background.
//!
//! The upload half of the contract lives in unit tests beside each cloud's
//! transfer code, where the mock fixtures for those protocols already are. This
//! file covers what is observable from outside the crate: the local filesystem,
//! and whether bytes keep arriving after the cancel.
//!
//! **Why a raw TCP server and not wiremock.** These tests must cancel *mid-body* —
//! past the response headers, so the transfer has spawned its byte-stream producer
//! and created `<dst>.part`. `set_delay` delays the whole response, headers
//! included, so a cancel during it merely drops a pending request, which reqwest
//! tears down cleanly: the test then passes with or without the fix. The server
//! below sends headers immediately, drips the body, and counts what it writes, so
//! "the download stopped" is asserted on the wire.
//!
//! No sleeping on cleanup is needed: `cancelled_by` reports only after
//! `OperationCtx::run` has awaited the cleanup tracker.

use sf_core::apis::database_driver_v1::PutGetResultsetFlavor;
use sf_core::apis::operation_ctx::cancelled_by;
use sf_core::config::param_store::ParamStore;
use sf_core::config::retry::RetryPolicy;
use sf_core::file_manager::{
    CloudCredentials, DownloadData, LocationType, MultipartParams, SingleDownloadData, StageInfo,
    TransferCtx, download_files, download_single_file,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use wiremock::matchers::{method, path as path_matcher};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

/// Far larger than any socket buffer, so the server cannot reach the total by
/// writing ahead — continued progress means the client is genuinely still reading.
const ADVERTISED_LEN: usize = 32 * 1024 * 1024;
const DRIP_CHUNK: usize = 16 * 1024;

/// A download source that sends response headers immediately, then drips the body
/// until the client goes away.
struct StallingBodyServer {
    url: String,
    /// Body bytes written to the socket so far.
    bytes_sent: Arc<AtomicUsize>,
    /// Set when a write fails, i.e. the client closed the connection. This — not a
    /// byte count — is the exact signal that a cancelled download let go of the
    /// socket: the count can still tick up by a chunk that was already in flight,
    /// whereas a live download never closes at all.
    client_disconnected: Arc<AtomicBool>,
}

impl StallingBodyServer {
    async fn spawn() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let bytes_sent = Arc::new(AtomicUsize::new(0));
        let client_disconnected = Arc::new(AtomicBool::new(false));

        tokio::spawn({
            let (bytes_sent, client_disconnected) =
                (bytes_sent.clone(), client_disconnected.clone());
            async move {
                let (mut stream, _) = listener.accept().await.expect("accept");
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf).await;

                // No `x-goog-meta-encryptiondata`, so this is an SSE object: the body
                // goes straight to `<dst>.part`, undecrypted and unverified.
                let head = format!(
                    "HTTP/1.1 200 OK\r\n\
                     Content-Length: {ADVERTISED_LEN}\r\n\
                     x-goog-meta-sfc-digest: test-digest\r\n\r\n"
                );
                if stream.write_all(head.as_bytes()).await.is_err() {
                    return;
                }

                let chunk = vec![9u8; DRIP_CHUNK];
                while bytes_sent.load(Ordering::SeqCst) < ADVERTISED_LEN {
                    // Fails once the client drops the connection.
                    if stream.write_all(&chunk).await.is_err() {
                        client_disconnected.store(true, Ordering::SeqCst);
                        break;
                    }
                    bytes_sent.fetch_add(DRIP_CHUNK, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(2)).await;
                }
            }
        });

        Self {
            url: format!("http://{addr}/stalling-download"),
            bytes_sent,
            client_disconnected,
        }
    }

    /// Waits (bounded) for the client to close the connection. A live download
    /// keeps draining the socket, so this stays `false` for the whole window.
    async fn saw_client_disconnect(&self) -> bool {
        const DEADLINE: Duration = Duration::from_secs(3);
        let start = std::time::Instant::now();
        while start.elapsed() < DEADLINE {
            if self.client_disconnected.load(Ordering::SeqCst) {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        false
    }

    fn bytes_sent(&self) -> usize {
        self.bytes_sent.load(Ordering::SeqCst)
    }
}

/// A GCS stage reached through per-file presigned URLs, so no access token is
/// needed and every request lands on the test server.
fn presigned_gcs_stage() -> StageInfo {
    StageInfo {
        location_type: LocationType::Gcs,
        bucket: "test-bucket".to_string(),
        key_prefix: "prefix/".to_string(),
        region: "us-central1".to_string(),
        creds: CloudCredentials::Gcs {
            gcs_access_token: None,
        },
        endpoint: None,
        presigned_url: None,
        use_virtual_url: false,
        use_regional_url: false,
        use_s3_regional_url: false,
        tls_config: sf_core::tls::config::TlsConfig::default(),
        crl_worker: sf_core::crl::CrlWorker::shared_lazy(),
        proxy_config: sf_core::tls::config::ProxyConfig::default(),
        storage_account: None,
    }
}

fn single_download(src: &str, local_location: &str, presigned_url: String) -> SingleDownloadData {
    SingleDownloadData {
        src_location: src.to_string(),
        local_location: local_location.to_string(),
        stage_info: presigned_gcs_stage(),
        encryption_material: None,
        presigned_url: Some(presigned_url),
        flavor: PutGetResultsetFlavor::Python,
        multipart: MultipartParams::default(),
        unsafe_file_write: false,
    }
}

/// The `<dst>.part` staging path a download writes to before its atomic rename.
fn partial_path(dir: &Path, name: &str) -> PathBuf {
    let mut p = dir.join(name).into_os_string();
    p.push(".part");
    PathBuf::from(p)
}

/// How long [`once_partial_file_exists`] waits for a transfer to reach mid-body.
/// `<dst>.part` appears within milliseconds of the response headers being parsed, so
/// this is ample headroom — and deliberately shorter than the drip server needs to
/// finish the whole body, so a `.part` that never appears trips the precondition
/// assertion instead of masquerading as a completed download.
const MID_BODY_BOUND: Duration = Duration::from_secs(5);

/// Resolves once `<dst>.part` exists — proof that the transfer is mid-body: headers
/// parsed, producer draining the socket, blocking writer holding the staging file.
///
/// Waiting on the *server* having sent headers is not enough. It wins that race
/// long before the client has processed them, so the cancel lands on a
/// still-pending request and exercises nothing.
///
/// Bounded by [`MID_BODY_BOUND`], reporting through the returned flag whether the
/// file was actually observed. On timeout it resolves **anyway**, so the caller
/// still cancels: a trigger left pending would hang the test rather than fail it,
/// and not only when `.part` is missing — if the blocking writer never starts, the
/// producer channel fills, the producer parks, and the drip server blocks on a full
/// socket buffer, a cycle with no timeout anywhere in it. Firing the cancel breaks
/// that, and the flag turns the resulting failure into a message naming the real
/// cause. A panic here could not: this runs on a task the test never joins.
fn once_partial_file_exists(
    partial: PathBuf,
) -> (
    Arc<AtomicBool>,
    impl std::future::Future<Output = ()> + Send,
) {
    let observed = Arc::new(AtomicBool::new(false));
    let flag = observed.clone();
    let wait = async move {
        let deadline = std::time::Instant::now() + MID_BODY_BOUND;
        while std::time::Instant::now() < deadline {
            if partial.exists() {
                flag.store(true, Ordering::SeqCst);
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    };
    (observed, wait)
}

/// A cancelled GET must stop pulling bytes rather than finishing in detached tasks
/// after the caller was told it was cancelled, and must leave no `<dst>.part`.
///
/// Both halves fail pre-fix: the byte-stream producer's `AbortHandle` was
/// discarded, so it drained the rest of the body while the detached writer
/// committed it to `.part`, which then survived because the removal only ran on the
/// error path.
#[tokio::test(flavor = "multi_thread")]
async fn should_stop_fetching_and_remove_the_partial_file_when_a_download_is_cancelled() {
    let server = StallingBodyServer::spawn().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path().to_path_buf();
    let local_location = dir.to_string_lossy().to_string();
    let url = server.url.clone();

    let (reached_mid_body, mid_body) = once_partial_file_exists(partial_path(&dir, "stalls"));
    let outcome = cancelled_by(mid_body, move |scope| async move {
        download_single_file(
            single_download("stalls", &local_location, url),
            &RetryPolicy::put_get(&ParamStore::new()),
            0,
            TransferCtx::new(None, Some(&scope)),
        )
        .await
    })
    .await;

    // Checked first: without it, a `.part` that never appeared would trip one of the
    // assertions below and point at the wrong cause.
    assert!(
        reached_mid_body.load(Ordering::SeqCst),
        "`<dst>.part` never appeared within {MID_BODY_BOUND:?}, so the cancel did not \
         land mid-body and this test exercised nothing"
    );
    assert!(
        outcome.is_none(),
        "the stalled download must be cancelled, not completed"
    );

    // Asserted on the disconnect rather than a frozen byte count: a chunk already
    // in flight can still land, but a download still running never closes at all.
    // Pre-fix this stayed connected and pulled megabytes more.
    assert!(
        server.saw_client_disconnect().await,
        "a cancelled download must close its connection instead of draining the body \
         in the background (server had written {} of {ADVERTISED_LEN} bytes)",
        server.bytes_sent()
    );
    assert!(
        server.bytes_sent() < ADVERTISED_LEN,
        "the download must not have run to completion"
    );

    assert!(
        !partial_path(&dir, "stalls").exists(),
        "the partial `.part` file must be removed when a download is cancelled"
    );
    assert!(
        !dir.join("stalls").exists(),
        "a cancelled download must never publish a destination file"
    );
}

/// Cancelling partway through a multi-file GET keeps the files already finished
/// (each was published by an atomic rename, so it is whole), never starts the
/// files after it, and reports only cancellation — no partial result rows.
#[tokio::test(flavor = "multi_thread")]
async fn should_keep_completed_files_and_skip_the_rest_when_a_batch_is_cancelled() {
    // Files 1 and 3 respond normally; file 2 stalls mid-body, where the cancel lands.
    let completed = MockServer::start().await;
    let stalling = StallingBodyServer::spawn().await;
    let third_requests = Arc::new(AtomicUsize::new(0));

    Mock::given(method("GET"))
        .and(path_matcher("/presigned/one"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(b"first-file".to_vec())
                .insert_header("x-goog-meta-sfc-digest", "test-digest"),
        )
        .mount(&completed)
        .await;

    // Must never be requested: the sequential loop is dropped while on file 2.
    let third = third_requests.clone();
    Mock::given(method("GET"))
        .and(path_matcher("/presigned/three"))
        .respond_with(move |_: &Request| {
            third.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(200)
                .set_body_bytes(b"third-file".to_vec())
                .insert_header("x-goog-meta-sfc-digest", "test-digest")
        })
        .mount(&completed)
        .await;

    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path().to_path_buf();
    let base = completed.uri();

    let data = DownloadData {
        src_locations: vec!["one".to_string(), "two".to_string(), "three".to_string()],
        local_location: dir.to_string_lossy().to_string(),
        stage_info: presigned_gcs_stage(),
        encryption_materials: vec![None, None, None],
        presigned_urls: vec![
            Some(format!("{base}/presigned/one")),
            Some(stalling.url.clone()),
            Some(format!("{base}/presigned/three")),
        ],
        flavor: PutGetResultsetFlavor::Python,
        multipart: MultipartParams::default(),
        unsafe_file_write: false,
        get_fastfail: true,
    };

    let (reached_mid_body, mid_body) = once_partial_file_exists(partial_path(&dir, "two"));
    let outcome = cancelled_by(mid_body, move |scope| async move {
        download_files(
            data,
            &RetryPolicy::put_get(&ParamStore::new()),
            TransferCtx::new(None, Some(&scope)),
        )
        .await
    })
    .await;

    assert!(
        reached_mid_body.load(Ordering::SeqCst),
        "file 2's `<dst>.part` never appeared within {MID_BODY_BOUND:?}, so the cancel \
         did not land during file 2 and this test exercised nothing"
    );
    assert!(
        outcome.is_none(),
        "a cancelled batch reports cancellation, not partial result rows"
    );
    assert_eq!(
        std::fs::read(dir.join("one")).expect("file 1 finished before the cancel"),
        b"first-file",
        "a file completed before the cancel must be left intact"
    );
    assert!(
        !dir.join("two").exists() && !partial_path(&dir, "two").exists(),
        "the in-flight file must leave neither a destination nor `.part` debris"
    );
    assert_eq!(
        third_requests.load(Ordering::SeqCst),
        0,
        "files after the cancelled one must never be requested"
    );
}

/// The `connection_get_query_result` entry point must forward its operation ctx
/// into the PUT/GET transfer, not just observe cancellation at the RPC boundary.
///
/// The distinction matters because `run_opt` at the boundary returns `Cancelled`
/// whether or not the ctx reached `TransferCtx`, so the *outcome* of the call
/// proves nothing. Two other candidate signals are equally useless here:
/// `ProducerAbortGuard` is `Drop`-based, so the socket closes even with no ctx;
/// and on a client-side-encrypted object the digest check removes `.part` on its
/// own. This asserts on the one effect only the ctx-registered cleanup produces —
/// `remove_partial_after_cancel` deleting `<dst>.part` after a cancelled
/// *server-side-encrypted* download, which is why [`StallingBodyServer`] sends no
/// encryption metadata.
///
/// Unlike the tests above, which hand `download_single_file` a synthetic
/// `TransferCtx`, this drives the real RPC: GS answers the fetch-by-query-id with
/// a `command: DOWNLOAD` body, so the request travels
/// dispatch → `connection_get_query_result` → `extract_rowset_data`
/// → `perform_put_get_transfer` → `TransferCtx`. Reverting that call site to
/// `extract_rowset_data(None, …)` fails this test and nothing else.
#[tokio::test(flavor = "multi_thread")]
async fn should_remove_the_partial_file_when_a_fetch_by_query_id_is_cancelled_mid_transfer() {
    const QUERY_ID: &str = "00000000-0000-0000-0000-000000000009";

    let cloud = StallingBodyServer::spawn().await;
    let sf_server = MockServer::start().await;
    crate::common::mocks::auth::mount_jwt_login_success(&sf_server).await;

    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path().to_path_buf();
    let local_location = dir.to_string_lossy().to_string();

    crate::common::mocks::put_get::mount_gcs_download_result_with_sql_text(
        &sf_server,
        QUERY_ID,
        &cloud.url,
        &local_location,
    )
    .await;

    let sf_uri = sf_server.uri();
    let client = tokio::task::spawn_blocking(move || {
        Arc::new(
            crate::common::snowflake_test_client::SnowflakeTestClient::connect_integration_test(
                Some(&sf_uri),
            ),
        )
    })
    .await
    .expect("connect");

    // Minted before dispatch so the cancel below can name the operation while the
    // blocking thread is still inside it.
    let operation = client.register_operation();
    let partial = partial_path(&dir, "file.csv");
    let (reached_mid_body, mid_body) = once_partial_file_exists(partial.clone());

    let fetch = tokio::task::spawn_blocking({
        let client = client.clone();
        move || client.connection_get_query_result_cancellable_raw(QUERY_ID, operation)
    });

    mid_body.await;
    assert!(
        reached_mid_body.load(Ordering::SeqCst),
        "precondition: the transfer never reached mid-body, so this cancelled \
         something other than an in-flight PUT/GET"
    );
    client.cancel_operation(operation);

    let result = fetch.await.expect("fetch task");

    // `SnowflakeTestClient::drop` releases handles through the *blocking* client,
    // which `block_on`s — dropping it on this runtime's thread panics with
    // "Cannot start a runtime from within a runtime". Retire it on a blocking
    // thread while the assertions below still have everything they need.
    tokio::task::spawn_blocking(move || drop(client))
        .await
        .expect("client teardown");

    let err = result.expect_err("a cancelled fetch must not succeed");
    assert!(
        format!("{err:?}").contains("Cancelled"),
        "expected a cancelled DriverException, got {err:?}"
    );

    assert!(
        !partial.exists(),
        "cancelling the fetch must remove {} — its survival means the operation \
         ctx never reached the transfer's TransferCtx",
        partial.display()
    );
    assert!(
        cloud.saw_client_disconnect().await,
        "the cancelled transfer must let go of the socket"
    );
}
