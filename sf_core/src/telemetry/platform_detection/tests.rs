use super::aws::tests::FakeCallerIdentityProvider;
use super::*;
use crate::config::param_names;
use crate::config::param_store::ParamStore;
use crate::config::settings::Setting;
use crate::env_vars;
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
        root_store_provider: crate::tls::RootStoreProvider::new(),
        aws_metadata_base_url: unreachable_url.clone(),
        azure_metadata_base_url: unreachable_url.clone(),
        gce_metadata_root_url: unreachable_url.clone(),
        gce_metadata_base_url: unreachable_url,
        timeout: DETECTION_TIMEOUT,
        disabled: None,
    }
}

#[tokio::test]
async fn returns_disabled_when_env_flag_true() {
    temp_env::async_with_vars(
        platform_detection_env_vars(&[(env_vars::SNOWFLAKE_DISABLE_PLATFORM_DETECTION, "true")]),
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
    for flag_value in ["true", "TRUE", "True", "tRuE", "  true  "] {
        temp_env::async_with_vars(
            platform_detection_env_vars(&[(
                env_vars::SNOWFLAKE_DISABLE_PLATFORM_DETECTION,
                flag_value,
            )]),
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
        platform_detection_env_vars(&[(env_vars::SNOWFLAKE_DISABLE_PLATFORM_DETECTION, "false")]),
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
            (env_vars::SNOWFLAKE_DISABLE_PLATFORM_DETECTION, "true"),
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

#[tokio::test]
async fn zero_timeout_skips_endpoint_detectors_on_cloud() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"instanceId":"i-12345"}"#))
        .expect(0)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(200).set_body_string("imds-token"))
        .expect(0)
        .mount(&server)
        .await;

    let mut cfg = test_detection_config();
    cfg.timeout = Duration::ZERO;
    cfg.aws_metadata_base_url = server.uri();
    cfg.azure_metadata_base_url = server.uri();
    cfg.gce_metadata_root_url = server.uri();
    cfg.gce_metadata_base_url = server.uri();
    cfg.caller_identity_provider = Arc::new(FakeCallerIdentityProvider::new(Some(
        "arn:aws:iam::123456789012:user/alice".into(),
    )));

    temp_env::async_with_vars(
        platform_detection_env_vars(&[("LAMBDA_TASK_ROOT", "/var/task")]),
        async {
            let platforms = detect_platforms(&cfg).await;
            assert_eq!(
                platforms,
                vec!["is_aws_lambda".to_string()],
                "zero timeout must keep env-only detectors and skip HTTP/STS, got {platforms:?}"
            );
        },
    )
    .await;
}

#[tokio::test]
async fn zero_timeout_skips_endpoint_detectors_out_of_cloud() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
        .expect(0)
        .mount(&server)
        .await;

    let mut cfg = test_detection_config();
    cfg.timeout = Duration::ZERO;
    cfg.aws_metadata_base_url = server.uri();
    cfg.azure_metadata_base_url = server.uri();
    cfg.gce_metadata_root_url = server.uri();
    cfg.gce_metadata_base_url = server.uri();

    temp_env::async_with_vars(platform_detection_env_vars(&[]), async {
        let platforms = detect_platforms(&cfg).await;
        assert!(
            platforms.is_empty(),
            "zero timeout out of cloud must not report endpoint labels, got {platforms:?}"
        );
    })
    .await;
}

async fn wait_for_platforms(detector: &PlatformDetector) -> Vec<String> {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(platforms) = detector.platforms() {
                return platforms.to_vec();
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("background detection should finish")
}

#[test]
fn start_outside_a_runtime_leaves_detection_to_a_later_start() {
    let detector = PlatformDetector::new(test_detection_config());
    let params = ParamStore::new();
    detector.start_with(&params);
    assert!(!detector.has_started());

    let runtime = tokio::runtime::Runtime::new().expect("test runtime");
    temp_env::with_vars(
        platform_detection_env_vars(&[("LAMBDA_TASK_ROOT", "/var/task")]),
        || {
            runtime.block_on(async {
                detector.start_with(&params);
                assert_eq!(wait_for_platforms(&detector).await, vec!["is_aws_lambda"]);
            });
        },
    );
}

fn choice(
    connection_disabled: Option<bool>,
    connection_timeout_ms: Option<i64>,
    disable_env: Option<&str>,
    timeout_env: Option<&str>,
) -> PlatformDetectionChoice {
    resolve_platform_detection(
        connection_disabled,
        connection_timeout_ms,
        disable_env,
        timeout_env,
    )
}

#[test]
fn defaults_to_enabled_with_builtin_timeout() {
    let resolved = choice(None, None, None, None);
    assert!(!resolved.disabled);
    assert_eq!(resolved.timeout, DETECTION_TIMEOUT);
}

#[test]
fn connection_disable_wins_over_env() {
    assert!(choice(Some(true), None, Some("false"), None).disabled);
    assert!(!choice(Some(false), None, Some("true"), None).disabled);
}

#[test]
fn env_disable_applies_when_the_connection_setting_is_unset() {
    assert!(choice(None, None, Some("true"), None).disabled);
    assert!(!choice(None, None, Some("   "), None).disabled);
}

#[test]
fn disable_env_accepts_only_padded_literal_true() {
    for value in ["TRUE", "True", "  true  "] {
        assert!(choice(None, None, Some(value), None).disabled, "{value}");
    }
}

#[test]
fn disable_env_rejects_values_other_than_true() {
    for value in ["1", "yes", "on", "", "false", "garbage"] {
        assert!(!choice(None, None, Some(value), None).disabled, "{value}");
    }
}

#[test]
fn connection_disable_uses_setting_bool_tokens() {
    assert_eq!(connection_disabled(None), None);
    assert_eq!(connection_disabled(Some(&Setting::Bool(true))), Some(true));
    assert_eq!(
        connection_disabled(Some(&Setting::Bool(false))),
        Some(false)
    );
    for value in ["true", "TRUE", "1", "on", "ON", "  true  ", " 1 "] {
        assert_eq!(
            connection_disabled(Some(&Setting::String(value.to_string()))),
            Some(true),
            "{value}"
        );
    }
    for value in ["false", "FALSE", "0", "off", "OFF", "  false  "] {
        assert_eq!(
            connection_disabled(Some(&Setting::String(value.to_string()))),
            Some(false),
            "{value}"
        );
    }
    for value in ["yes", "no", "", "trueish", "garbage"] {
        assert_eq!(
            connection_disabled(Some(&Setting::String(value.to_string()))),
            None,
            "{value}"
        );
    }
    assert_eq!(connection_disabled(Some(&Setting::Int(1))), None);
}

#[test]
fn timeout_precedence_is_connection_then_env() {
    assert_eq!(
        choice(None, Some(11), None, Some("33")).timeout,
        Duration::from_millis(11)
    );
    assert_eq!(
        choice(None, None, None, Some("33")).timeout,
        Duration::from_millis(33)
    );
    assert_eq!(
        choice(None, None, None, Some("")).timeout,
        DETECTION_TIMEOUT
    );
    assert_eq!(
        choice(None, None, None, Some("   ")).timeout,
        DETECTION_TIMEOUT
    );
}

#[test]
fn timeout_above_5000ms_is_clamped() {
    for configured in [5001_i64, 300_000, i64::MAX] {
        assert_eq!(
            choice(None, Some(configured), None, None).timeout,
            Duration::from_millis(5000),
            "{configured}"
        );
        let text = configured.to_string();
        assert_eq!(
            choice(None, None, None, Some(&text)).timeout,
            Duration::from_millis(5000),
            "{configured}"
        );
    }
}

#[test]
fn timeout_at_5000ms_is_preserved() {
    assert_eq!(
        choice(None, Some(5000), None, None).timeout,
        Duration::from_millis(5000)
    );
}

#[test]
fn zero_timeout_is_preserved_from_each_source() {
    assert!(choice(None, Some(0), None, None).timeout.is_zero());
    assert!(choice(None, None, None, Some("0")).timeout.is_zero());
}

#[test]
fn negative_timeout_is_clamped_to_zero() {
    for configured in [-1_i64, -200] {
        assert!(
            choice(None, Some(configured), None, None).timeout.is_zero(),
            "{configured}"
        );
        let text = configured.to_string();
        assert!(
            choice(None, None, None, Some(&text)).timeout.is_zero(),
            "{configured}"
        );
    }
}

#[test]
fn unparseable_timeout_falls_through() {
    for value in ["abc", "1.5"] {
        assert_eq!(
            connection_timeout_ms(Some(&Setting::String(value.to_string()))),
            None,
            "{value}"
        );
        assert_eq!(
            choice(None, None, None, Some(value)).timeout,
            DETECTION_TIMEOUT,
            "{value}"
        );
    }
}

#[test]
fn resolve_from_params_prefers_connection_property_over_env() {
    temp_env::with_var(
        env_vars::SNOWFLAKE_PLATFORM_DETECTION_TIMEOUT_MS,
        Some("33"),
        || {
            let mut params = ParamStore::new();
            params.insert(
                param_names::PLATFORM_DETECTION_TIMEOUT_MS
                    .as_str()
                    .to_owned(),
                Setting::Int(11),
            );
            params.insert(
                param_names::DISABLE_PLATFORM_DETECTION.as_str().to_owned(),
                Setting::String("on".to_string()),
            );
            let resolved = resolve_from_params(&params);
            assert!(resolved.disabled);
            assert_eq!(resolved.timeout, Duration::from_millis(11));
        },
    );
}

#[test]
fn resolve_from_params_clamps_string_timeouts() {
    let cases = [
        ("0", 0),
        ("-1", 0),
        ("-200", 0),
        ("5000", 5000),
        ("5001", 5000),
        ("300000", 5000),
        ("2147483647", 5000),
    ];
    temp_env::with_var(
        env_vars::SNOWFLAKE_PLATFORM_DETECTION_TIMEOUT_MS,
        Some("33"),
        || {
            for (configured, expected_ms) in cases {
                let mut params = ParamStore::new();
                params.insert(
                    param_names::PLATFORM_DETECTION_TIMEOUT_MS
                        .as_str()
                        .to_owned(),
                    Setting::String(configured.to_string()),
                );
                assert_eq!(
                    resolve_from_params(&params).timeout,
                    Duration::from_millis(expected_ms),
                    "{configured}"
                );
            }
        },
    );
}

#[test]
fn resolve_from_params_string_disable_wins_over_env() {
    let cases = [
        ("true", "false", true),
        ("false", "true", false),
        ("1", "false", true),
        ("on", "false", true),
        ("off", "true", false),
        ("  true  ", "false", true),
    ];
    for (connection, env, expected) in cases {
        temp_env::with_var(
            env_vars::SNOWFLAKE_DISABLE_PLATFORM_DETECTION,
            Some(env),
            || {
                let mut params = ParamStore::new();
                params.insert(
                    param_names::DISABLE_PLATFORM_DETECTION.as_str().to_owned(),
                    Setting::String(connection.to_string()),
                );
                assert_eq!(
                    resolve_from_params(&params).disabled,
                    expected,
                    "connection {connection}, env {env}"
                );
            },
        );
    }
}

#[test]
fn resolve_from_params_unrecognized_disable_falls_through_to_env() {
    temp_env::with_var(
        env_vars::SNOWFLAKE_DISABLE_PLATFORM_DETECTION,
        Some("true"),
        || {
            let mut params = ParamStore::new();
            params.insert(
                param_names::DISABLE_PLATFORM_DETECTION.as_str().to_owned(),
                Setting::String("yes".to_string()),
            );
            assert!(resolve_from_params(&params).disabled);
        },
    );
}

#[tokio::test]
async fn start_with_runs_detection_once_per_detector() {
    let detector = PlatformDetector::new(test_detection_config());
    let mut params = ParamStore::new();
    params.insert(
        param_names::PLATFORM_DETECTION_TIMEOUT_MS
            .as_str()
            .to_owned(),
        Setting::Int(0),
    );
    let metrics = tokio::runtime::Handle::current().metrics();

    temp_env::async_with_vars(
        platform_detection_env_vars(&[("LAMBDA_TASK_ROOT", "/var/task")]),
        async {
            detector.start_with(&params);
            assert_eq!(wait_for_platforms(&detector).await, vec!["is_aws_lambda"]);
            let first = detector.platforms().expect("first detection").as_ptr();

            let alive_before = metrics.num_alive_tasks();
            detector.start_with(&params);
            assert_eq!(metrics.num_alive_tasks(), alive_before);
            assert!(std::ptr::eq(
                first,
                detector.platforms().expect("cached detection").as_ptr()
            ));
        },
    )
    .await;
}

#[tokio::test]
async fn disable_env_value_one_does_not_disable() {
    temp_env::async_with_vars(
        platform_detection_env_vars(&[(env_vars::SNOWFLAKE_DISABLE_PLATFORM_DETECTION, "1")]),
        async {
            let platforms = detect_platforms(&test_detection_config()).await;
            assert!(
                platforms.is_empty(),
                "only the literal true disables detection, got {platforms:?}"
            );
        },
    )
    .await;
}

#[tokio::test]
async fn start_with_connection_timeout_zero_skips_endpoint_probes() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
        .expect(0)
        .mount(&server)
        .await;

    let mut cfg = test_detection_config();
    cfg.aws_metadata_base_url = server.uri();
    cfg.azure_metadata_base_url = server.uri();
    cfg.gce_metadata_root_url = server.uri();
    cfg.gce_metadata_base_url = server.uri();
    let detector = PlatformDetector::new(cfg);

    let mut params = ParamStore::new();
    params.insert(
        param_names::PLATFORM_DETECTION_TIMEOUT_MS
            .as_str()
            .to_owned(),
        Setting::Int(0),
    );

    temp_env::async_with_vars(
        platform_detection_env_vars(&[("LAMBDA_TASK_ROOT", "/var/task")]),
        async {
            detector.start_with(&params);
            assert_eq!(
                wait_for_platforms(&detector).await,
                vec!["is_aws_lambda".to_string()]
            );
        },
    )
    .await;
}

#[tokio::test]
async fn start_with_disable_reports_disabled_without_probes() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
        .expect(0)
        .mount(&server)
        .await;

    let mut cfg = test_detection_config();
    cfg.aws_metadata_base_url = server.uri();
    cfg.azure_metadata_base_url = server.uri();
    cfg.gce_metadata_root_url = server.uri();
    cfg.gce_metadata_base_url = server.uri();
    cfg.caller_identity_provider = Arc::new(FakeCallerIdentityProvider::new(Some(
        "arn:aws:iam::123456789012:user/alice".into(),
    )));
    let detector = PlatformDetector::new(cfg);

    let mut params = ParamStore::new();
    params.insert(
        param_names::DISABLE_PLATFORM_DETECTION.as_str().to_owned(),
        Setting::Bool(true),
    );

    temp_env::async_with_vars(
        platform_detection_env_vars(&[("LAMBDA_TASK_ROOT", "/var/task")]),
        async {
            detector.start_with(&params);
            assert_eq!(
                wait_for_platforms(&detector).await,
                vec!["disabled".to_string()]
            );
        },
    )
    .await;
}

#[tokio::test]
async fn negative_connection_timeout_skips_endpoint_detectors() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
        .expect(0)
        .mount(&server)
        .await;

    let mut cfg = test_detection_config();
    let resolved = choice(None, Some(-200), None, None);
    assert!(resolved.timeout.is_zero());
    cfg.timeout = resolved.timeout;
    cfg.disabled = Some(resolved.disabled);
    cfg.aws_metadata_base_url = server.uri();
    cfg.azure_metadata_base_url = server.uri();
    cfg.gce_metadata_root_url = server.uri();
    cfg.gce_metadata_base_url = server.uri();

    temp_env::async_with_vars(platform_detection_env_vars(&[]), async {
        let platforms = detect_platforms(&cfg).await;
        assert!(
            platforms.is_empty(),
            "a negative timeout skips endpoint detectors, got {platforms:?}"
        );
    })
    .await;
}
