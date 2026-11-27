use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::mocks;
use crate::common::put_get_common::GetResult;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Simulates a GET from an external stage without client-side encryption. For such a
/// stage Snowflake answers with `"encryptionMaterial": [null]`, one `null` per source
/// file. A live test would need AWS credentials for an external S3 stage; the mocked
/// GCS download exercises the same response parsing.
#[tokio::test(flavor = "multi_thread")]
async fn get_downloads_file_when_encryption_material_is_a_null_array() {
    let sf_server = MockServer::start().await;
    let gcs_server = MockServer::start().await;
    mocks::auth::mount_jwt_login_success(&sf_server).await;

    let download_dir = tempfile::tempdir().expect("tempdir");
    let local_location = download_dir.path().to_string_lossy().to_string();
    let presigned_url = format!("{}/file", gcs_server.uri());
    mocks::put_get::mount_gcs_download_with_null_encryption_material(
        &sf_server,
        &presigned_url,
        &local_location,
    )
    .await;

    Mock::given(method("GET"))
        .and(path("/file"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(b"plain-content".to_vec())
                .insert_header("x-goog-meta-sfc-digest", "test-digest"),
        )
        .expect(1)
        .mount(&gcs_server)
        .await;

    let sf_uri = sf_server.uri();
    tokio::task::spawn_blocking(move || {
        let client = SnowflakeTestClient::connect_integration_test(Some(&sf_uri));
        let sql = format!("GET @external_stage/file.csv 'file://{local_location}/'");
        let stream = client.execute_query(&sql);

        let mut helper = ArrowResultHelper::from_result(stream);
        let row: GetResult = helper.fetch_one().expect("fetch GetResult");
        assert_eq!(row.status, "DOWNLOADED");
        assert_eq!(
            std::fs::read(download_dir.path().join("file.csv")).expect("downloaded file"),
            b"plain-content"
        );
    })
    .await
    .unwrap();
}
