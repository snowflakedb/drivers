use super::aws::tests::FakeCallerIdentityProvider;
use super::*;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Returns a [`DetectionConfig`] wired with inert fakes for every field,
/// so detectors never reach the network. Tests override individual fields
/// (e.g. `cfg.caller_identity_provider = ...`) to exercise specific paths.
///
/// Every HTTP URL is pointed at `http://127.0.0.1:1`, which reliably
/// refuses connections on all supported platforms, so HTTP detectors
/// fail fast and cannot flake an env-only or STS-only assertion.
fn test_detection_config() -> DetectionConfig {
    let unreachable_url = "http://127.0.0.1:1".to_string();
    DetectionConfig {
        caller_identity_provider: Arc::new(FakeCallerIdentityProvider::new(None)),
        aws_metadata_base_url: unreachable_url.clone(),
        azure_metadata_base_url: unreachable_url.clone(),
        gce_metadata_root_url: unreachable_url.clone(),
        gce_metadata_base_url: unreachable_url,
    }
}

#[tokio::test]
async fn returns_disabled_when_opt_in_flag_not_set() {
    temp_env::async_with_vars(
        [(
            "SNOWFLAKE_EXPERIMENTAL_ENABLE_PLATFORM_DETECTION",
            None::<&str>,
        )],
        async {
            assert_eq!(
                detect_platforms(&test_detection_config()).await,
                vec!["disabled"]
            );
        },
    )
    .await;
}

#[tokio::test]
async fn returns_disabled_when_env_flag_true() {
    temp_env::async_with_vars(
        platform_detection_env_vars(&[("SNOWFLAKE_DISABLE_PLATFORM_DETECTION", "true")]),
        async {
            assert_eq!(
                detect_platforms(&test_detection_config()).await,
                vec!["disabled"]
            );
        },
    )
    .await;
}

#[tokio::test]
async fn disabled_flag_accepts_truthy_values() {
    for flag_value in ["true", "TRUE", "True", "tRuE", "1"] {
        temp_env::async_with_vars(
            platform_detection_env_vars(&[("SNOWFLAKE_DISABLE_PLATFORM_DETECTION", flag_value)]),
            async {
                assert_eq!(
                    detect_platforms(&test_detection_config()).await,
                    vec!["disabled"],
                    "disable flag {flag_value} should short-circuit detection",
                );
            },
        )
        .await;
    }
}

#[tokio::test]
async fn returns_empty_when_disable_flag_is_false() {
    temp_env::async_with_vars(
        platform_detection_env_vars(&[("SNOWFLAKE_DISABLE_PLATFORM_DETECTION", "false")]),
        async {
            let platforms = detect_platforms(&test_detection_config()).await;
            assert!(platforms.is_empty(), "expected empty, got {platforms:?}");
        },
    )
    .await;
}

#[tokio::test]
async fn returns_empty_when_no_platform_detected() {
    temp_env::async_with_vars(platform_detection_env_vars(&[]), async {
        let platforms = detect_platforms(&test_detection_config()).await;
        assert!(platforms.is_empty(), "expected empty, got {platforms:?}");
    })
    .await;
}

#[tokio::test]
async fn detects_is_aws_lambda_from_env() {
    temp_env::async_with_vars(
        platform_detection_env_vars(&[("LAMBDA_TASK_ROOT", "/var/task")]),
        async {
            assert_eq!(
                detect_platforms(&test_detection_config()).await,
                vec!["is_aws_lambda".to_string()]
            );
        },
    )
    .await;
}

#[tokio::test]
async fn detects_is_azure_function_from_env() {
    temp_env::async_with_vars(
        platform_detection_env_vars(&[
            ("FUNCTIONS_WORKER_RUNTIME", "node"),
            ("FUNCTIONS_EXTENSION_VERSION", "~4"),
            ("AzureWebJobsStorage", "DefaultEndpointsProtocol=https"),
        ]),
        async {
            assert_eq!(
                detect_platforms(&test_detection_config()).await,
                vec!["is_azure_function".to_string()]
            );
        },
    )
    .await;
}

#[tokio::test]
async fn detects_is_gce_cloud_run_service_from_env() {
    temp_env::async_with_vars(
        platform_detection_env_vars(&[
            ("K_SERVICE", "svc"),
            ("K_REVISION", "rev"),
            ("K_CONFIGURATION", "cfg"),
        ]),
        async {
            assert_eq!(
                detect_platforms(&test_detection_config()).await,
                vec!["is_gce_cloud_run_service".to_string()]
            );
        },
    )
    .await;
}

#[tokio::test]
async fn detects_is_gce_cloud_run_job_from_env() {
    temp_env::async_with_vars(
        platform_detection_env_vars(&[
            ("CLOUD_RUN_JOB", "my-job"),
            ("CLOUD_RUN_EXECUTION", "exec-1"),
        ]),
        async {
            assert_eq!(
                detect_platforms(&test_detection_config()).await,
                vec!["is_gce_cloud_run_job".to_string()]
            );
        },
    )
    .await;
}

#[tokio::test]
async fn detects_is_github_action_from_env() {
    temp_env::async_with_vars(
        platform_detection_env_vars(&[("GITHUB_ACTIONS", "true")]),
        async {
            assert_eq!(
                detect_platforms(&test_detection_config()).await,
                vec!["is_github_action".to_string()]
            );
        },
    )
    .await;
}

#[tokio::test]
async fn detects_multiple_env_platforms_simultaneously() {
    temp_env::async_with_vars(
        platform_detection_env_vars(&[
            ("LAMBDA_TASK_ROOT", "/var/task"),
            ("CLOUD_RUN_JOB", "my-job"),
            ("CLOUD_RUN_EXECUTION", "exec-1"),
            ("GITHUB_ACTIONS", "true"),
        ]),
        async {
            let platforms = detect_platforms(&test_detection_config()).await;
            assert_eq!(
                platforms,
                vec![
                    "is_aws_lambda".to_string(),
                    "is_gce_cloud_run_job".to_string(),
                    "is_github_action".to_string(),
                ],
                "detector order must match",
            );
        },
    )
    .await;
}

#[tokio::test]
async fn disabled_flag_wins_over_other_env_signals() {
    temp_env::async_with_vars(
        platform_detection_env_vars(&[
            ("SNOWFLAKE_DISABLE_PLATFORM_DETECTION", "true"),
            ("GITHUB_ACTIONS", "true"),
            ("LAMBDA_TASK_ROOT", "/var/task"),
        ]),
        async {
            assert_eq!(
                detect_platforms(&test_detection_config()).await,
                vec!["disabled"],
                "disabled flag must short-circuit before any detectors run",
            );
        },
    )
    .await;
}

#[tokio::test]
async fn empty_env_value_does_not_trigger_detector() {
    temp_env::async_with_vars(
        platform_detection_env_vars(&[("GITHUB_ACTIONS", "")]),
        async {
            let platforms = detect_platforms(&test_detection_config()).await;
            assert!(
                platforms.is_empty(),
                "empty string must be treated as unset, got {platforms:?}"
            );
        },
    )
    .await;
}

#[tokio::test(start_paused = true)]
async fn drops_detectors_when_timeout_reached() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_delay(DETECTION_TIMEOUT * 4))
        .mount(&server)
        .await;

    let mut slow_sts =
        FakeCallerIdentityProvider::new(Some("arn:aws:iam::123456789012:user/alice".into()));
    slow_sts.delay = DETECTION_TIMEOUT * 4;

    let mut cfg = test_detection_config();
    cfg.caller_identity_provider = Arc::new(slow_sts);
    cfg.aws_metadata_base_url = server.uri();
    cfg.azure_metadata_base_url = server.uri();
    cfg.gce_metadata_root_url = server.uri();
    cfg.gce_metadata_base_url = server.uri();

    temp_env::async_with_vars(platform_detection_env_vars(&[]), async {
        let detect = tokio::spawn({
            let cfg = cfg.clone();
            async move { detect_platforms(&cfg).await }
        });
        tokio::time::advance(DETECTION_TIMEOUT + Duration::from_millis(1)).await;

        let platforms = detect.await.expect("detect_platforms timed out");
        assert!(
            platforms.is_empty(),
            "all slow detectors must be dropped by per-detector timeout, got {platforms:?}"
        );
    })
    .await;
}

#[tokio::test]
async fn detects_has_aws_identity_when_provider_returns_wif_arn() {
    let mut cfg = test_detection_config();
    cfg.caller_identity_provider = Arc::new(FakeCallerIdentityProvider::new(Some(
        "arn:aws:iam::123456789012:user/alice".into(),
    )));
    temp_env::async_with_vars(platform_detection_env_vars(&[]), async {
        let platforms = detect_platforms(&cfg).await;
        assert_eq!(platforms, vec!["has_aws_identity".to_string()]);
    })
    .await;
}

#[tokio::test]
async fn rejects_when_sts_returns_no_arn() {
    let mut cfg = test_detection_config();
    cfg.caller_identity_provider = Arc::new(FakeCallerIdentityProvider::new(None));
    temp_env::async_with_vars(platform_detection_env_vars(&[]), async {
        let platforms = detect_platforms(&cfg).await;
        assert!(
            platforms.is_empty(),
            "missing ARN must produce an empty platforms array, got {platforms:?}"
        );
    })
    .await;
}

#[tokio::test]
async fn detects_is_ec2_instance_via_imdsv2() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/latest/api/token"))
        .and(header("X-aws-ec2-metadata-token-ttl-seconds", "21600"))
        .respond_with(ResponseTemplate::new(200).set_body_string("imds-token"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/latest/dynamic/instance-identity/document"))
        .and(header("X-aws-ec2-metadata-token", "imds-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"instanceId":"i-12345"}"#))
        .expect(1)
        .mount(&server)
        .await;

    let mut cfg = test_detection_config();
    cfg.aws_metadata_base_url = server.uri();

    temp_env::async_with_vars(platform_detection_env_vars(&[]), async {
        let platforms = detect_platforms(&cfg).await;
        assert_eq!(
            platforms,
            vec!["is_ec2_instance".to_string()],
            "expected is_ec2_instance, got {platforms:?}"
        );
    })
    .await;
}

#[tokio::test]
async fn detects_is_ec2_instance_via_imdsv1_when_token_request_fails() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/latest/api/token"))
        .respond_with(ResponseTemplate::new(403))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/latest/dynamic/instance-identity/document"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"instanceId":"i-12345"}"#))
        .expect(1)
        .mount(&server)
        .await;

    let mut cfg = test_detection_config();
    cfg.aws_metadata_base_url = server.uri();

    temp_env::async_with_vars(platform_detection_env_vars(&[]), async {
        let platforms = detect_platforms(&cfg).await;
        assert_eq!(
            platforms,
            vec!["is_ec2_instance".to_string()],
            "expected is_ec2_instance via IMDSv1 fallback, got {platforms:?}"
        );
    })
    .await;
}

#[tokio::test]
async fn rejects_is_ec2_instance_when_document_lacks_instance_id() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/latest/api/token"))
        .respond_with(ResponseTemplate::new(200).set_body_string("imds-token"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/latest/dynamic/instance-identity/document"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"region":"us-east-1"}"#))
        .mount(&server)
        .await;

    let mut cfg = test_detection_config();
    cfg.aws_metadata_base_url = server.uri();

    temp_env::async_with_vars(platform_detection_env_vars(&[]), async {
        let platforms = detect_platforms(&cfg).await;
        assert!(
            platforms.is_empty(),
            "missing instanceId must not trigger is_ec2_instance, got {platforms:?}"
        );
    })
    .await;
}

#[tokio::test]
async fn rejects_is_ec2_instance_when_document_returns_non_2xx() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/latest/api/token"))
        .respond_with(ResponseTemplate::new(200).set_body_string("imds-token"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/latest/dynamic/instance-identity/document"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let mut cfg = test_detection_config();
    cfg.aws_metadata_base_url = server.uri();

    temp_env::async_with_vars(platform_detection_env_vars(&[]), async {
        let platforms = detect_platforms(&cfg).await;
        assert!(
            platforms.is_empty(),
            "non-2xx document response must not trigger is_ec2_instance, got {platforms:?}"
        );
    })
    .await;
}

#[tokio::test]
async fn detects_is_azure_vm_with_metadata_header() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/metadata/instance"))
        .and(header("Metadata", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
        .mount(&server)
        .await;

    let mut cfg = test_detection_config();
    cfg.azure_metadata_base_url = server.uri();

    temp_env::async_with_vars(platform_detection_env_vars(&[]), async {
        let platforms = detect_platforms(&cfg).await;
        assert_eq!(
            platforms,
            vec!["is_azure_vm".to_string()],
            "expected is_azure_vm, got {platforms:?}"
        );
    })
    .await;
}

#[tokio::test]
async fn detects_has_azure_managed_identity_via_functions_env() {
    temp_env::async_with_vars(
        platform_detection_env_vars(&[
            ("FUNCTIONS_WORKER_RUNTIME", "node"),
            ("FUNCTIONS_EXTENSION_VERSION", "~4"),
            ("AzureWebJobsStorage", "DefaultEndpoint=..."),
            ("IDENTITY_HEADER", "header"),
        ]),
        async {
            let platforms = detect_platforms(&test_detection_config()).await;
            assert_eq!(
                platforms,
                vec![
                    "is_azure_function".to_string(),
                    "has_azure_managed_identity".to_string()
                ],
                "expected is_azure_function + has_azure_managed_identity, got {platforms:?}"
            );
        },
    )
    .await;
}

#[tokio::test]
async fn detects_has_azure_managed_identity_via_metadata() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/metadata/identity/oauth2/token"))
        .and(header("Metadata", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
        .mount(&server)
        .await;

    let mut cfg = test_detection_config();
    cfg.azure_metadata_base_url = server.uri();

    temp_env::async_with_vars(platform_detection_env_vars(&[]), async {
        let platforms = detect_platforms(&cfg).await;
        assert_eq!(
            platforms,
            vec!["has_azure_managed_identity".to_string()],
            "expected has_azure_managed_identity, got {platforms:?}"
        );
    })
    .await;
}

#[tokio::test]
async fn detects_is_gce_vm_via_metadata_flavor_header() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).insert_header("Metadata-Flavor", "Google"))
        .mount(&server)
        .await;

    let mut cfg = test_detection_config();
    cfg.gce_metadata_root_url = server.uri();

    temp_env::async_with_vars(platform_detection_env_vars(&[]), async {
        let platforms = detect_platforms(&cfg).await;
        assert_eq!(
            platforms,
            vec!["is_gce_vm".to_string()],
            "expected is_gce_vm, got {platforms:?}"
        );
    })
    .await;
}

#[tokio::test]
async fn detects_has_gcp_identity_via_metadata_service() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instance/service-accounts/default/email"))
        .and(header("Metadata-Flavor", "Google"))
        .respond_with(ResponseTemplate::new(200).set_body_string("sa@example.iam"))
        .mount(&server)
        .await;

    let mut cfg = test_detection_config();
    cfg.gce_metadata_base_url = server.uri();

    temp_env::async_with_vars(platform_detection_env_vars(&[]), async {
        let platforms = detect_platforms(&cfg).await;
        assert_eq!(
            platforms,
            vec!["has_gcp_identity".to_string()],
            "expected has_gcp_identity, got {platforms:?}"
        );
    })
    .await;
}
