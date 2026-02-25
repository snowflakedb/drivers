//! PUT/GET operation mock helpers.

use serde_json::json;
use wiremock::matchers::{body_string_contains, method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

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
                            "smkId": 1
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
