//! Real-account baseline for process-wide transfer admission.
//!
//! This complements the governor's hermetic queue tests by executing two
//! independent SQL PUT commands through separate `DatabaseDriverV1` roots and
//! the deployment's actual Azure or GCS stage.

use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::put_get_common::{PutResult, build_put_command, write_payload};
use crate::common::snowflake_test_client::SnowflakeTestClient;
use sf_core::governor::Governor;
use sf_core::protobuf::apis::database_driver_v1::DriverProviders;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant};
use uuid::Uuid;

const PAYLOAD_LEN: u64 = 128 * 1024 * 1024;

fn stage_name(suffix: &str) -> String {
    format!("TEST_GOVERNOR_E2E_{}_{}", suffix, Uuid::new_v4().simple())
}

fn execute_put(client: SnowflakeTestClient, sql: String) -> PutResult {
    let result = client.execute_query(&sql);
    ArrowResultHelper::from_result(result)
        .fetch_one()
        .expect("real-cloud PUT failed")
}

#[test]
#[ignore = "large_ real-cloud governor baseline"]
fn large_governor_caps_concurrent_real_put_commands() {
    // Given two driver roots share a cap-1 governor and two 128 MiB files
    let governor = Governor::try_with_capacity(1).expect("test capacity");
    let client_a = SnowflakeTestClient::connect_with_default_auth_using(DriverProviders {
        governor: Some(governor.clone()),
        ..Default::default()
    });

    let client_b = SnowflakeTestClient::connect_with_default_auth_using(DriverProviders {
        governor: Some(governor.clone()),
        ..Default::default()
    });

    let stage_a = stage_name("A");
    let stage_b = stage_name("B");
    client_a.create_temporary_stage(&stage_a);
    client_b.create_temporary_stage(&stage_b);

    let upload_dir = tempfile::TempDir::new().expect("upload tempdir");
    let file_a = upload_dir.path().join("a.bin");
    let file_b = upload_dir.path().join("b.bin");
    write_payload(&file_a, PAYLOAD_LEN);
    write_payload(&file_b, PAYLOAD_LEN);

    let sql_a = build_put_command(
        &stage_a,
        file_a.to_str().expect("UTF-8 path"),
        "AUTO_COMPRESS=FALSE OVERWRITE=TRUE PARALLEL=4",
    );
    let sql_b = build_put_command(
        &stage_b,
        file_b.to_str().expect("UTF-8 path"),
        "AUTO_COMPRESS=FALSE OVERWRITE=TRUE PARALLEL=4",
    );

    let start = Arc::new(Barrier::new(3));
    let completed = Arc::new(AtomicUsize::new(0));
    let wall_start = Instant::now();
    let mut max_in_flight = 0;
    let mut observed_contention = false;

    // When both PUT commands start together
    let (result_a, result_b) = std::thread::scope(|scope| {
        let start_a = Arc::clone(&start);
        let completed_a = Arc::clone(&completed);
        let upload_a = scope.spawn(move || {
            start_a.wait();
            let result = execute_put(client_a, sql_a);
            completed_a.fetch_add(1, Ordering::Release);
            result
        });

        let start_b = Arc::clone(&start);
        let completed_b = Arc::clone(&completed);
        let upload_b = scope.spawn(move || {
            start_b.wait();
            let result = execute_put(client_b, sql_b);
            completed_b.fetch_add(1, Ordering::Release);
            result
        });

        start.wait();
        while completed.load(Ordering::Acquire) < 2 {
            max_in_flight = max_in_flight.max(governor.in_flight());
            observed_contention |= governor.queued() > 0;
            std::thread::sleep(Duration::from_millis(1));
        }

        (
            upload_a.join().expect("PUT A thread"),
            upload_b.join().expect("PUT B thread"),
        )
    });

    // Then both uploads succeed and at most one transfer is in flight
    assert_eq!(result_a.status, "UPLOADED");
    assert_eq!(result_b.status, "UPLOADED");
    assert_eq!(max_in_flight, 1, "real PUTs must exercise the governor");
    assert!(
        observed_contention,
        "two synchronized 128 MiB PUTs should contend for the cap-1 governor"
    );
    assert_eq!(governor.in_flight(), 0);
    assert_eq!(governor.queued(), 0);

    eprintln!(
        "governor real-cloud baseline: bytes={}, commands=2, wall_s={:.3}, max_in_flight={max_in_flight}",
        PAYLOAD_LEN * 2,
        wall_start.elapsed().as_secs_f64(),
    );
}
