//! PUT/GET operation mock helpers.

use serde_json::json;
use wiremock::matchers::{body_string_contains, method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Mount a GET-result response at `GET /queries/{query_id}/result` that returns a
/// DOWNLOAD command with `sqlText` present. Used to test the async PUT/GET path in
/// `connection_get_query_result` where `sql_text` present → `StageInfoRefreshContext` is built.
///
/// `gcs_presigned_url` is the per-file presigned URL for the single source file. The
/// `src_locations` key is set to `["file.csv"]` and `localLocation` to `local_location`.
pub async fn mount_gcs_download_result_with_sql_text(
    server: &MockServer,
    query_id: &str,
    gcs_presigned_url: &str,
    local_location: &str,
) {
    let path_str = format!("/queries/{query_id}/result");
    Mock::given(method("GET"))
        .and(path(&path_str))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({
                    "success": true,
                    "data": {
                        "command": "DOWNLOAD",
                        "sqlText": "GET @mock_stage file:///local/ OVERWRITE=TRUE",
                        "src_locations": ["file.csv"],
                        "stageInfo": {
                            "locationType": "GCS",
                            "location": "test-bucket/prefix/",
                            "path": "prefix/",
                            "region": "us-central1",
                            "creds": { "GCS_ACCESS_TOKEN": null },
                            "presignedUrl": null,
                            "endPoint": null
                        },
                        "localLocation": local_location,
                        "presignedUrls": [gcs_presigned_url]
                    }
                }))
                .insert_header("Content-Type", "application/json"),
        )
        .mount(server)
        .await;
}

/// Mount a GET-result response at `GET /queries/{query_id}/result` that returns a
/// DOWNLOAD command WITHOUT `sqlText`. Used to test the `sql_text == None` fallback
/// in `connection_get_query_result` where no `StageInfoRefreshContext` is built.
pub async fn mount_gcs_download_result_no_sql_text(
    server: &MockServer,
    query_id: &str,
    gcs_presigned_url: &str,
    local_location: &str,
) {
    let path_str = format!("/queries/{query_id}/result");
    Mock::given(method("GET"))
        .and(path(&path_str))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({
                    "success": true,
                    "data": {
                        "command": "DOWNLOAD",
                        "src_locations": ["file.csv"],
                        "stageInfo": {
                            "locationType": "GCS",
                            "location": "test-bucket/prefix/",
                            "path": "prefix/",
                            "region": "us-central1",
                            "creds": { "GCS_ACCESS_TOKEN": null },
                            "presignedUrl": null,
                            "endPoint": null
                        },
                        "localLocation": local_location,
                        "presignedUrls": [gcs_presigned_url]
                    }
                }))
                .insert_header("Content-Type", "application/json"),
        )
        .mount(server)
        .await;
}

/// Mount the stage-info refresh SQL endpoint: `POST /queries/v1/query-request` for
/// any GET body. Returns a DOWNLOAD response with the fresh presigned URL, simulating
/// what Snowflake returns after the client re-issues the original PUT/GET SQL.
///
/// `fresh_gcs_presigned_url` is the fresh per-file presigned URL that replaces the
/// expired one.
pub async fn mount_gcs_download_refresh_sql_response(
    server: &MockServer,
    fresh_gcs_presigned_url: &str,
    local_location: &str,
) {
    Mock::given(method("POST"))
        .and(path_regex(r"/queries/v1/query-request.*"))
        .and(body_string_contains("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({
                    "success": true,
                    "data": {
                        "command": "DOWNLOAD",
                        "src_locations": ["file.csv"],
                        "stageInfo": {
                            "locationType": "GCS",
                            "location": "test-bucket/prefix/",
                            "path": "prefix/",
                            "region": "us-central1",
                            "creds": { "GCS_ACCESS_TOKEN": null },
                            "presignedUrl": null,
                            "endPoint": null
                        },
                        "localLocation": local_location,
                        "presignedUrls": [fresh_gcs_presigned_url]
                    }
                }))
                .insert_header("Content-Type", "application/json"),
        )
        .mount(server)
        .await;
}

/// Mount a PUT command response that triggers unsupported compression error.
///
/// The response contains src_locations pointing to .xz files which are not supported.
/// The `repo_root` parameter should be the workspace root path for the file pattern.
pub async fn mount_unsupported_compression(server: &MockServer, repo_root: &str) {
    let normalized_repo_root = repo_root.replace('\\', "/");
    let src_locations_pattern =
        format!("{normalized_repo_root}/tests/test_data/generated_test_data/compression/*.xz");

    Mock::given(method("POST"))
        .and(path_regex(r"/queries/v1/query-request.*"))
        .and(body_string_contains("PUT"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({
                    "success": true,
                    "data": {
                        "command": "UPLOAD",
                        "stageInfo": {
                            "locationType": "S3",
                            "location": "mock-stage/",
                            "path": "mock-stage/",
                            "region": "us-west-2",
                            "isClientSideEncrypted": true,
                            "creds": {
                                "AWS_KEY_ID": "mock_key",
                                "AWS_SECRET_KEY": "mock_secret",
                                "AWS_TOKEN": "mock_token"
                            }
                        },
                        "encryptionMaterial": {
                            "queryStageMasterKey": "mock_key==",
                            "queryId": "mock-query-id",
                            "smkId": "1"
                        },
                        "src_locations": [src_locations_pattern],
                        "autoCompress": true,
                        "overwrite": false,
                        "sourceCompression": "auto_detect"
                    }
                }))
                .insert_header("Content-Type", "application/json"),
        )
        .mount(server)
        .await;
}

/// Mount a PUT command response whose `stageInfo` points at a wiremock Azure
/// endpoint. The driver issues HEAD/PUT against `azure_uri` rather than a
/// real Azure account.
///
/// `auto_compress=false` and `sourceCompression="NONE"` keep the upload bytes
/// bit-identical to the file contents — required so the locally-computed
/// SHA-256 (`prepared.digest`) matches whatever digest the test plants in
/// the HEAD response.
pub async fn mount_azure_put_pointing_at(
    server: &MockServer,
    azure_uri: &str,
    src_file_path: &str,
) {
    Mock::given(method("POST"))
        .and(path_regex(r"/queries/v1/query-request.*"))
        .and(body_string_contains("PUT"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({
                    "success": true,
                    "data": {
                        "command": "UPLOAD",
                        "stageInfo": {
                            "locationType": "AZURE",
                            "location": "test-container/prefix/",
                            "path": "prefix/",
                            "region": "eastus2",
                            "endPoint": azure_uri,
                            "storageAccount": "test",
                            "isClientSideEncrypted": false,
                            "creds": {
                                "AZURE_SAS_TOKEN": "sv=2099-01-01&sig=mock-sig&se=2099-12-31"
                            }
                        },
                        "src_locations": [src_file_path],
                        "autoCompress": false,
                        "overwrite": true,
                        "sourceCompression": "NONE"
                    }
                }))
                .insert_header("Content-Type", "application/json"),
        )
        .mount(server)
        .await;
}
