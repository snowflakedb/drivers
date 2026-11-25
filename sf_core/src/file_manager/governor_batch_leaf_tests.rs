use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use crate::apis::database_driver_v1::PutGetResultsetFlavor;
use crate::config::param_store::ParamStore;
use crate::config::retry::RetryPolicy;
use crate::file_manager::types::{
    CloudCredentials, LocationType, SourceCompressionParam, StageInfo, StageTransport,
};
use crate::file_manager::{
    DownloadData, MultipartParams, TransferCtx, UploadData, download_files, upload_files,
};
use crate::governor::{Governor, TransferOp};

use super::governor_test_util::{wait_for_arrival_peak, wait_for_arrivals};

const CREATE_MP_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<InitiateMultipartUploadResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><Bucket>test-bucket</Bucket><Key>prefix/f.dat</Key><UploadId>test-upload-id</UploadId></InitiateMultipartUploadResult>"#;

const COMPLETE_MP_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<CompleteMultipartUploadResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><Location>https://snowflake.com/stage/f.dat</Location><Bucket>test-bucket</Bucket><Key>prefix/f.dat</Key><ETag>"final-etag"</ETag></CompleteMultipartUploadResult>"#;

const MULTIPART_FILE_BYTES: usize = 16 * 1024 * 1024;
const RANGED_OBJECT_BYTES: usize = 16 * 1024 * 1024;

fn parse_byte_range(range_header: &str, object_bytes: usize) -> (usize, usize) {
    let spec = range_header
        .strip_prefix("bytes=")
        .unwrap_or_else(|| panic!("expected bytes range, got {range_header}"));
    let (start_str, end_str) = spec
        .split_once('-')
        .unwrap_or_else(|| panic!("range must contain '-': {range_header}"));
    let start: usize = start_str
        .parse()
        .unwrap_or_else(|_| panic!("range start must be numeric: {range_header}"));
    let end: usize = if end_str.is_empty() {
        object_bytes - 1
    } else {
        end_str
            .parse()
            .unwrap_or_else(|_| panic!("range end must be numeric: {range_header}"))
    };
    assert!(
        start <= end && end < object_bytes,
        "range [{start}, {end}] must fit object size {object_bytes}"
    );
    (start, end)
}

fn put_get_policy() -> RetryPolicy {
    RetryPolicy::put_get(&ParamStore::new())
}

fn s3_stage(mock_uri: &str) -> StageInfo {
    StageInfo {
        location_type: LocationType::S3,
        bucket: "test-bucket".to_string(),
        key_prefix: "prefix/".to_string(),
        region: "us-east-1".to_string(),
        creds: CloudCredentials::S3 {
            aws_key_id: crate::sensitive::SensitiveString::from("AKIA-TEST".to_string()),
            aws_secret_key: crate::sensitive::SensitiveString::from("secret".to_string()),
            aws_token: Some(crate::sensitive::SensitiveString::from("token".to_string())),
        },
        endpoint: Some(mock_uri.to_string()),
        presigned_url: None,
        use_virtual_url: false,
        use_regional_url: false,
        use_s3_regional_url: false,
        transport: StageTransport {
            tls_config: crate::tls::config::TlsConfig::default(),
            crl_manager: crate::crl::CrlManager::new().connection_crl(),
            proxy_config: crate::tls::config::ProxyConfig::default(),
            root_store_provider: crate::tls::RootStoreProvider::new(),
            transfer: Governor::new().register_command(1),
        },
        storage_account: None,
    }
}

fn low_threshold_multipart(parallel: i64) -> MultipartParams {
    MultipartParams::from_server(Some(1), Some(parallel))
}

fn register_transfer_on_governor(governor: &Governor, parallel: i64) -> TransferOp {
    governor.register_command(super::resolve_command_parallel(Some(parallel)))
}

fn batch_upload_data(
    dir: &std::path::Path,
    mock_uri: &str,
    overwrite: bool,
    parallel: i64,
    governor: &Governor,
) -> UploadData {
    UploadData {
        src_location_pattern: dir.join("*.dat").to_string_lossy().into_owned(),
        stage_info: s3_stage(mock_uri),
        encryption_material: None,
        auto_compress: false,
        source_compression: SourceCompressionParam::None,
        overwrite,
        flavor: PutGetResultsetFlavor::Python,
        legacy_odbc_compression_autodetect: false,
        skip_upload_on_content_match: false,
        multipart: low_threshold_multipart(parallel),
        put_fastfail: false,
        put_compress_level: 9,
        put_tempdir: None,
        cwd: None,
        transfer: register_transfer_on_governor(governor, parallel),
    }
}

fn batch_download_data(
    names: &[&str],
    local_dir: &std::path::Path,
    mock_uri: &str,
    parallel: i64,
    governor: &Governor,
) -> DownloadData {
    let upload = batch_upload_data(local_dir, mock_uri, true, parallel, governor);
    DownloadData {
        src_locations: names.iter().map(|n| (*n).to_string()).collect(),
        local_location: local_dir.to_string_lossy().into_owned(),
        stage_info: upload.stage_info,
        encryption_materials: names.iter().map(|_| None).collect(),
        presigned_urls: names.iter().map(|_| None).collect(),
        flavor: PutGetResultsetFlavor::Python,
        multipart: upload.multipart,
        unsafe_file_write: false,
        get_fastfail: false,
        cwd: None,
        transfer: upload.transfer,
    }
}

fn write_multipart_fixture_files(dir: &std::path::Path, count: usize) {
    for i in 0..count {
        std::fs::write(
            dir.join(format!("f{i:02}.dat")),
            vec![b'x'; MULTIPART_FILE_BYTES],
        )
        .unwrap();
    }
}

#[derive(Clone)]
struct MultipartBatchMock {
    part_arrivals: Arc<AtomicUsize>,
    delay: Duration,
}

impl wiremock::Respond for MultipartBatchMock {
    fn respond(&self, req: &wiremock::Request) -> wiremock::ResponseTemplate {
        let method = req.method.as_str();
        let has_uploads = req.url.query_pairs().any(|(key, _)| key == "uploads");
        let has_upload_id = req.url.query_pairs().any(|(key, _)| key == "uploadId");
        let has_part_number = req.url.query_pairs().any(|(key, _)| key == "partNumber");

        if method == "POST" && has_uploads {
            return wiremock::ResponseTemplate::new(200)
                .set_body_raw(CREATE_MP_XML, "application/xml");
        }
        if method == "PUT" && has_upload_id && has_part_number {
            self.part_arrivals.fetch_add(1, Ordering::SeqCst);
            return wiremock::ResponseTemplate::new(200)
                .insert_header("ETag", "\"part-etag\"")
                .set_delay(self.delay);
        }
        if method == "POST" && has_upload_id {
            return wiremock::ResponseTemplate::new(200)
                .set_body_raw(COMPLETE_MP_XML, "application/xml");
        }
        wiremock::ResponseTemplate::new(404)
    }
}

async fn mount_multipart_batch_mock(mock: &wiremock::MockServer, part_arrivals: &Arc<AtomicUsize>) {
    wiremock::Mock::given(|req: &wiremock::Request| {
        matches!(
            req.method.as_str(),
            "POST" | "PUT" if req.url.path().starts_with("/test-bucket/prefix/")
        )
    })
    .respond_with(MultipartBatchMock {
        part_arrivals: Arc::clone(part_arrivals),
        delay: Duration::from_secs(30),
    })
    .mount(mock)
    .await;
}

#[derive(Clone)]
struct RangedDownloadBatchMock {
    range_arrivals: Arc<AtomicUsize>,
    object_bytes: usize,
    delay: Duration,
}

impl wiremock::Respond for RangedDownloadBatchMock {
    fn respond(&self, req: &wiremock::Request) -> wiremock::ResponseTemplate {
        match req.method.as_str() {
            "HEAD" => wiremock::ResponseTemplate::new(200)
                .insert_header("content-length", self.object_bytes.to_string()),
            "GET" => {
                let range_header = req
                    .headers
                    .get("range")
                    .unwrap_or_else(|| panic!("ranged GET must carry Range"))
                    .to_str()
                    .unwrap_or_else(|_| panic!("Range header must be valid UTF-8"));
                let (start, end) = parse_byte_range(range_header, self.object_bytes);
                let len = end - start + 1;
                self.range_arrivals.fetch_add(1, Ordering::SeqCst);
                wiremock::ResponseTemplate::new(206)
                    .insert_header("content-length", len.to_string())
                    .insert_header(
                        "content-range",
                        format!("bytes {start}-{end}/{}", self.object_bytes),
                    )
                    .set_body_bytes(vec![b'x'; len])
                    .set_delay(self.delay)
            }
            _ => wiremock::ResponseTemplate::new(404),
        }
    }
}

async fn mount_ranged_download_batch_mock(
    mock: &wiremock::MockServer,
    range_arrivals: &Arc<AtomicUsize>,
) {
    wiremock::Mock::given(|req: &wiremock::Request| matches!(req.method.as_str(), "HEAD" | "GET"))
        .respond_with(RangedDownloadBatchMock {
            range_arrivals: Arc::clone(range_arrivals),
            object_bytes: RANGED_OBJECT_BYTES,
            delay: Duration::from_secs(30),
        })
        .mount(mock)
        .await;
}

#[derive(Clone)]
struct HeadThenPutBatchMock {
    head_arrivals: Arc<AtomicUsize>,
    put_arrivals: Arc<AtomicUsize>,
    head_delay: Duration,
}

impl wiremock::Respond for HeadThenPutBatchMock {
    fn respond(&self, req: &wiremock::Request) -> wiremock::ResponseTemplate {
        match req.method.as_str() {
            "HEAD" => {
                self.head_arrivals.fetch_add(1, Ordering::SeqCst);
                wiremock::ResponseTemplate::new(404).set_delay(self.head_delay)
            }
            "PUT" => {
                self.put_arrivals.fetch_add(1, Ordering::SeqCst);
                wiremock::ResponseTemplate::new(200).set_delay(Duration::from_secs(30))
            }
            _ => wiremock::ResponseTemplate::new(404),
        }
    }
}

async fn mount_head_then_put_batch_mock(
    mock: &wiremock::MockServer,
    head_arrivals: &Arc<AtomicUsize>,
    put_arrivals: &Arc<AtomicUsize>,
) {
    wiremock::Mock::given(|req: &wiremock::Request| matches!(req.method.as_str(), "HEAD" | "PUT"))
        .respond_with(HeadThenPutBatchMock {
            head_arrivals: Arc::clone(head_arrivals),
            put_arrivals: Arc::clone(put_arrivals),
            head_delay: Duration::from_secs(30),
        })
        .mount(mock)
        .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn upload_multipart_batch_shares_one_operation_allocation() {
    const FILES: usize = 4;
    const PARALLEL: i64 = 4;
    const PROCESS_CAP: usize = 32;
    let tmp = tempfile::TempDir::new().unwrap();
    write_multipart_fixture_files(tmp.path(), FILES);
    let governor =
        Arc::new(Governor::try_with_capacity(PROCESS_CAP).expect("test capacity must be nonzero"));
    let part_arrivals = Arc::new(AtomicUsize::new(0));
    let mock = wiremock::MockServer::start().await;
    mount_multipart_batch_mock(&mock, &part_arrivals).await;
    let data = batch_upload_data(tmp.path(), &mock.uri(), true, PARALLEL, &governor);
    let upload = async { upload_files(&data, &put_get_policy(), TransferCtx::default()).await };
    let observe = async {
        let (peak_parts, peak_in_flight) =
            wait_for_arrival_peak(&part_arrivals, &governor, PARALLEL as usize).await;
        assert_eq!(
            peak_parts, PARALLEL as usize,
            "multipart parts across files must share one TransferOp budget, not \
             file_count * MAX_PART_CONCURRENCY"
        );
        assert_eq!(peak_in_flight, PARALLEL as usize);
    };

    tokio::select! {
        _ = upload => panic!("stalled UploadPart mock must not let the batch finish"),
        () = observe => {}
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn upload_multipart_batch_respects_process_cap_over_parts() {
    const FILES: usize = 4;
    const PARALLEL: i64 = 4;
    const PROCESS_CAP: usize = 2;
    let tmp = tempfile::TempDir::new().unwrap();
    write_multipart_fixture_files(tmp.path(), FILES);
    let governor =
        Arc::new(Governor::try_with_capacity(PROCESS_CAP).expect("test capacity must be nonzero"));
    let part_arrivals = Arc::new(AtomicUsize::new(0));
    let mock = wiremock::MockServer::start().await;
    mount_multipart_batch_mock(&mock, &part_arrivals).await;
    let data = batch_upload_data(tmp.path(), &mock.uri(), true, PARALLEL, &governor);
    let upload = async { upload_files(&data, &put_get_policy(), TransferCtx::default()).await };
    let observe = async {
        let (peak_parts, peak_in_flight) =
            wait_for_arrival_peak(&part_arrivals, &governor, PROCESS_CAP).await;
        assert_eq!(
            peak_parts, PROCESS_CAP,
            "the process cap must bound multipart UploadPart leaves even when data.parallel is \
             higher"
        );
        assert_eq!(peak_in_flight, PROCESS_CAP);
    };

    tokio::select! {
        _ = upload => panic!("stalled UploadPart mock must not let the batch finish"),
        () = observe => {}
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn download_ranged_batch_shares_one_operation_allocation() {
    const FILES: usize = 4;
    const PARALLEL: i64 = 4;
    const PROCESS_CAP: usize = 32;
    let tmp = tempfile::TempDir::new().unwrap();
    let names: Vec<String> = (0..FILES).map(|i| format!("d{i:02}.csv")).collect();
    let name_refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let governor =
        Arc::new(Governor::try_with_capacity(PROCESS_CAP).expect("test capacity must be nonzero"));
    let range_arrivals = Arc::new(AtomicUsize::new(0));
    let mock = wiremock::MockServer::start().await;
    mount_ranged_download_batch_mock(&mock, &range_arrivals).await;
    let data = batch_download_data(&name_refs, tmp.path(), &mock.uri(), PARALLEL, &governor);
    let download = async { download_files(data, &put_get_policy(), TransferCtx::default()).await };
    let observe = async {
        let (peak_ranges, peak_in_flight) =
            wait_for_arrival_peak(&range_arrivals, &governor, PARALLEL as usize).await;
        assert_eq!(
            peak_ranges, PARALLEL as usize,
            "ranged GET leaves across files must share one TransferOp budget"
        );
        assert_eq!(peak_in_flight, PARALLEL as usize);
    };

    tokio::select! {
        _ = download => panic!("stalled ranged GET mock must not let the batch finish"),
        () = observe => {}
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn download_ranged_batch_respects_process_cap_over_ranges() {
    const FILES: usize = 4;
    const PARALLEL: i64 = 4;
    const PROCESS_CAP: usize = 2;
    let tmp = tempfile::TempDir::new().unwrap();
    let names: Vec<String> = (0..FILES).map(|i| format!("d{i:02}.csv")).collect();
    let name_refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let governor =
        Arc::new(Governor::try_with_capacity(PROCESS_CAP).expect("test capacity must be nonzero"));
    let range_arrivals = Arc::new(AtomicUsize::new(0));
    let mock = wiremock::MockServer::start().await;
    mount_ranged_download_batch_mock(&mock, &range_arrivals).await;
    let data = batch_download_data(&name_refs, tmp.path(), &mock.uri(), PARALLEL, &governor);
    let download = async { download_files(data, &put_get_policy(), TransferCtx::default()).await };
    let observe = async {
        let (peak_ranges, peak_in_flight) =
            wait_for_arrival_peak(&range_arrivals, &governor, PROCESS_CAP).await;
        assert_eq!(
            peak_ranges, PROCESS_CAP,
            "the process cap must bound ranged GET leaves even when data.parallel is higher"
        );
        assert_eq!(peak_in_flight, PROCESS_CAP);
    };

    tokio::select! {
        _ = download => panic!("stalled ranged GET mock must not let the batch finish"),
        () = observe => {}
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn upload_batch_with_head_respects_cap_without_overlapping_head_and_put() {
    const FILES: usize = 8;
    const PARALLEL: i64 = 4;
    const PROCESS_CAP: usize = 32;
    let tmp = tempfile::TempDir::new().unwrap();
    for i in 0..FILES {
        std::fs::write(tmp.path().join(format!("f{i:02}.dat")), b"tiny").unwrap();
    }
    let governor =
        Arc::new(Governor::try_with_capacity(PROCESS_CAP).expect("test capacity must be nonzero"));
    let head_arrivals = Arc::new(AtomicUsize::new(0));
    let put_arrivals = Arc::new(AtomicUsize::new(0));
    let mock = wiremock::MockServer::start().await;
    mount_head_then_put_batch_mock(&mock, &head_arrivals, &put_arrivals).await;
    let data = batch_upload_data(tmp.path(), &mock.uri(), false, PARALLEL, &governor);
    let transfer = data.transfer.clone();
    let upload = async { upload_files(&data, &put_get_policy(), TransferCtx::default()).await };
    let observe = async {
        let seen = wait_for_arrivals(&head_arrivals, PARALLEL as usize).await;
        assert_eq!(
            seen, PARALLEL as usize,
            "overwrite=false must stall HEAD probes until the command budget fills"
        );
        assert_eq!(
            put_arrivals.load(Ordering::SeqCst),
            0,
            "PUT must not start while HEAD probes for the same files are still in flight"
        );
        assert_eq!(
            governor.in_flight(),
            PARALLEL as usize,
            "HEAD probes must consume the same governed slots as data requests"
        );
        assert_eq!(
            transfer.command_in_flight(),
            PARALLEL as usize,
            "HEAD probes must count toward the command-local cap"
        );
    };

    tokio::select! {
        _ = upload => panic!("stalled HEAD mock must not let the batch finish"),
        () = observe => {}
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn upload_multipart_16x16_command_limit_binds_in_flight() {
    const FILES: usize = 16;
    const PARALLEL: i64 = 16;
    const PROCESS_CAP: usize = 32;
    let tmp = tempfile::TempDir::new().unwrap();
    write_multipart_fixture_files(tmp.path(), FILES);
    let governor =
        Arc::new(Governor::try_with_capacity(PROCESS_CAP).expect("test capacity must be nonzero"));
    let part_arrivals = Arc::new(AtomicUsize::new(0));
    let mock = wiremock::MockServer::start().await;
    mount_multipart_batch_mock(&mock, &part_arrivals).await;
    let data = batch_upload_data(tmp.path(), &mock.uri(), true, PARALLEL, &governor);
    let transfer = data.transfer.clone();
    let upload = async { upload_files(&data, &put_get_policy(), TransferCtx::default()).await };
    let observe = async {
        let mut peak_queued = 0usize;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        let (peak_parts, peak_in_flight) = loop {
            peak_queued = peak_queued.max(governor.queued());
            let arrivals = part_arrivals.load(Ordering::SeqCst);
            let in_flight = governor.in_flight();
            if arrivals >= PARALLEL as usize && in_flight == PARALLEL as usize {
                break (arrivals, in_flight);
            }
            if tokio::time::Instant::now() > deadline {
                panic!(
                    "timed out waiting for 16 in-flight parts (arrivals={arrivals}, in_flight={in_flight}, queued={peak_queued})"
                );
            }
            tokio::task::yield_now().await;
        };
        assert_eq!(peak_parts, PARALLEL as usize);
        assert_eq!(peak_in_flight, PARALLEL as usize);
        assert_eq!(transfer.command_in_flight(), PARALLEL as usize);
        assert!(
            peak_queued <= FILES,
            "waiter queue is bounded by one pending part acquire per running file; peak queued={peak_queued}"
        );
    };

    tokio::select! {
        _ = upload => panic!("stalled UploadPart mock must not let the batch finish"),
        () = observe => {}
    }
}
