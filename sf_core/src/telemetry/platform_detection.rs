use std::sync::Arc;
use std::time::Duration;

use futures::FutureExt;
use futures::future::BoxFuture;

use super::aws_identity::{CallerIdentityProvider, StsCallerIdentityProvider, has_aws_identity};

const DETECTION_TIMEOUT: Duration = Duration::from_millis(200);

const GCP_METADATA_FLAVOR_HEADER: &str = "Metadata-Flavor";
const GCP_METADATA_FLAVOR_VALUE: &str = "Google";

#[derive(Clone)]
pub struct DetectionConfig {
    pub(crate) caller_identity_provider: Arc<dyn CallerIdentityProvider>,
    pub(crate) ec2_metadata_url: String,
    pub(crate) azure_metadata_base_url: String,
    pub(crate) gce_metadata_root_url: String,
    pub(crate) gce_metadata_base_url: String,
}

impl Default for DetectionConfig {
    fn default() -> Self {
        Self {
            caller_identity_provider: Arc::new(StsCallerIdentityProvider),
            ec2_metadata_url: "http://169.254.169.254/latest/dynamic/instance-identity/document"
                .to_string(),
            azure_metadata_base_url: "http://169.254.169.254".to_string(),
            gce_metadata_root_url: "http://metadata.google.internal".to_string(),
            gce_metadata_base_url: "http://metadata.google.internal/computeMetadata/v1".to_string(),
        }
    }
}

/// Run all detectors concurrently with a per-detector [`DETECTION_TIMEOUT`].
///
/// Returns the names of only those detectors that succeeded. Order matches
/// the detector list below so the serialized array is stable across runs.
pub async fn detect_platforms(config: &DetectionConfig) -> Vec<String> {
    // Platform detection is temporarily opt-in: it adds up to ~200ms
    // (DETECTION_TIMEOUT) to every integration/e2e test, and we are in the
    // process of moving this feature from the login-request payload to
    // inband telemetry. Until that migration lands, detection only runs
    // when SNOWFLAKE_EXPERIMENTAL_ENABLE_PLATFORM_DETECTION is truthy.
    // SNOWFLAKE_DISABLE_PLATFORM_DETECTION remains as an explicit kill-switch
    // and wins over the enable flag.
    let disabled = std::env::var("SNOWFLAKE_DISABLE_PLATFORM_DETECTION")
        .map(|value| value.eq_ignore_ascii_case("true") || value == "1")
        .unwrap_or(false);
    let temporary_opt_in_enabled =
        std::env::var("SNOWFLAKE_EXPERIMENTAL_ENABLE_PLATFORM_DETECTION")
            .map(|value| value.eq_ignore_ascii_case("true") || value == "1")
            .unwrap_or(false);
    if disabled || !temporary_opt_in_enabled {
        return vec!["disabled".to_string()];
    }

    let http = reqwest::Client::new();

    let detectors: Vec<(&'static str, BoxFuture<'_, bool>)> = vec![
        ("is_aws_lambda", async { is_aws_lambda() }.boxed()),
        ("is_azure_function", async { is_azure_function() }.boxed()),
        (
            "is_gce_cloud_run_service",
            async { is_gce_cloud_run_service() }.boxed(),
        ),
        (
            "is_gce_cloud_run_job",
            async { is_gce_cloud_run_job() }.boxed(),
        ),
        ("is_github_action", async { is_github_action() }.boxed()),
        (
            "has_aws_identity",
            has_aws_identity(config.caller_identity_provider.as_ref()).boxed(),
        ),
        ("is_ec2_instance", is_ec2_instance(&http, config).boxed()),
        ("is_azure_vm", is_azure_vm(&http, config).boxed()),
        (
            "has_azure_managed_identity",
            has_azure_managed_identity(&http, config).boxed(),
        ),
        ("is_gce_vm", is_gce_vm(&http, config).boxed()),
        ("has_gcp_identity", has_gcp_identity(&http, config).boxed()),
    ];

    let results = futures::future::join_all(detectors.into_iter().map(|(name, fut)| async move {
        let detected = tokio::time::timeout(DETECTION_TIMEOUT, fut)
            .await
            .unwrap_or(false);
        (name, detected)
    }))
    .await;

    results
        .into_iter()
        .filter(|(_, detected)| *detected)
        .map(|(name, _)| name.to_string())
        .collect()
}

fn env_non_empty(key: &str) -> bool {
    std::env::var(key)
        .map(|value| !value.is_empty())
        .unwrap_or(false)
}

fn is_aws_lambda() -> bool {
    env_non_empty("LAMBDA_TASK_ROOT")
}

fn is_azure_function() -> bool {
    env_non_empty("FUNCTIONS_WORKER_RUNTIME")
        && env_non_empty("FUNCTIONS_EXTENSION_VERSION")
        && env_non_empty("AzureWebJobsStorage")
}

fn is_gce_cloud_run_service() -> bool {
    env_non_empty("K_SERVICE") && env_non_empty("K_REVISION") && env_non_empty("K_CONFIGURATION")
}

fn is_gce_cloud_run_job() -> bool {
    env_non_empty("CLOUD_RUN_JOB") && env_non_empty("CLOUD_RUN_EXECUTION")
}

fn is_github_action() -> bool {
    env_non_empty("GITHUB_ACTIONS")
}

async fn is_ec2_instance(http: &reqwest::Client, config: &DetectionConfig) -> bool {
    http.get(&config.ec2_metadata_url)
        .send()
        .await
        .map(|response| response.status().is_success())
        .unwrap_or(false)
}

async fn is_azure_vm(http: &reqwest::Client, config: &DetectionConfig) -> bool {
    let url = format!(
        "{}/metadata/instance?api-version=2019-03-11",
        config.azure_metadata_base_url
    );
    http.get(url)
        .header("Metadata", "true")
        .send()
        .await
        .map(|response| response.status().is_success())
        .unwrap_or(false)
}

async fn has_azure_managed_identity(http: &reqwest::Client, config: &DetectionConfig) -> bool {
    if is_azure_function() && env_non_empty("IDENTITY_HEADER") {
        return true;
    }
    let url = format!(
        "{}/metadata/identity/oauth2/token?api-version=2018-02-01&resource=https://management.azure.com",
        config.azure_metadata_base_url
    );
    http.get(url)
        .header("Metadata", "true")
        .send()
        .await
        .map(|response| response.status().is_success())
        .unwrap_or(false)
}

async fn is_gce_vm(http: &reqwest::Client, config: &DetectionConfig) -> bool {
    http.get(&config.gce_metadata_root_url)
        .send()
        .await
        .map(|response| {
            response
                .headers()
                .get(GCP_METADATA_FLAVOR_HEADER)
                .and_then(|value| value.to_str().ok())
                .map(|value| value == GCP_METADATA_FLAVOR_VALUE)
                .unwrap_or(false)
        })
        .unwrap_or(false)
}

async fn has_gcp_identity(http: &reqwest::Client, config: &DetectionConfig) -> bool {
    let url = format!(
        "{}/instance/service-accounts/default/email",
        config.gce_metadata_base_url
    );
    http.get(url)
        .header(GCP_METADATA_FLAVOR_HEADER, GCP_METADATA_FLAVOR_VALUE)
        .send()
        .await
        .map(|response| response.status().is_success())
        .unwrap_or(false)
}

#[cfg(any(test, feature = "test-utils"))]
const PLATFORM_DETECTION_ENV_KEYS: &[&str] = &[
    "SNOWFLAKE_DISABLE_PLATFORM_DETECTION",
    "SNOWFLAKE_EXPERIMENTAL_ENABLE_PLATFORM_DETECTION",
    "LAMBDA_TASK_ROOT",
    "FUNCTIONS_WORKER_RUNTIME",
    "FUNCTIONS_EXTENSION_VERSION",
    "AzureWebJobsStorage",
    "IDENTITY_HEADER",
    "K_SERVICE",
    "K_REVISION",
    "K_CONFIGURATION",
    "CLOUD_RUN_JOB",
    "CLOUD_RUN_EXECUTION",
    "GITHUB_ACTIONS",
];

/// Builds the `(key, Option<value>)` list to hand to `temp_env::with_vars`
/// or `temp_env::async_with_vars`. Every key in [`PLATFORM_DETECTION_ENV_KEYS`]
/// defaults to `None` (cleared), except
/// `SNOWFLAKE_EXPERIMENTAL_ENABLE_PLATFORM_DETECTION` which defaults to
/// `Some("true")` so tests that use this helper get detection running
/// without having to opt in everywhere. Callers can still override any key
/// via `overrides` (e.g. setting `SNOWFLAKE_DISABLE_PLATFORM_DETECTION=true`
/// to exercise the kill-switch path).
///
/// CI runners sometimes export keys like `GITHUB_ACTIONS=true`; passing the
/// returned vec to `temp_env` guarantees those leaks do not affect detector
/// behavior under test.
#[cfg(any(test, feature = "test-utils"))]
pub fn platform_detection_env_vars(
    overrides: &[(&'static str, &'static str)],
) -> Vec<(&'static str, Option<&'static str>)> {
    let mut env_vars: Vec<(&'static str, Option<&'static str>)> = PLATFORM_DETECTION_ENV_KEYS
        .iter()
        .map(|key| {
            // Detection is opt-in in production, but tests that use this helper
            // universally want it enabled; the explicit DISABLE flag still wins
            // when a caller sets it via `overrides`.
            let default = if *key == "SNOWFLAKE_EXPERIMENTAL_ENABLE_PLATFORM_DETECTION" {
                Some("true")
            } else {
                None
            };
            (*key, default)
        })
        .collect();

    for (key, value) in overrides {
        if let Some(slot) = env_vars.iter_mut().find(|(existing, _)| existing == key) {
            slot.1 = Some(*value);
        } else {
            env_vars.push((*key, Some(*value)));
        }
    }

    env_vars
}

#[cfg(test)]
mod tests {
    use super::super::aws_identity::tests::FakeCallerIdentityProvider;
    use super::*;

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
            ec2_metadata_url: unreachable_url.clone(),
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
                platform_detection_env_vars(&[(
                    "SNOWFLAKE_DISABLE_PLATFORM_DETECTION",
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

    mod has_aws_identity {
        use super::*;

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

        #[tokio::test(start_paused = true)]
        async fn drops_when_provider_exceeds_timeout() {
            let mut cfg = test_detection_config();
            let mut caller_identity_provider = FakeCallerIdentityProvider::new(Some(
                "arn:aws:iam::123456789012:user/alice".into(),
            ));
            caller_identity_provider.delay = DETECTION_TIMEOUT * 4;
            cfg.caller_identity_provider = Arc::new(caller_identity_provider);

            temp_env::async_with_vars(platform_detection_env_vars(&[]), async {
                let detect = tokio::spawn({
                    let cfg = cfg.clone();
                    async move { detect_platforms(&cfg).await }
                });
                tokio::time::advance(DETECTION_TIMEOUT + Duration::from_millis(1)).await;

                let platforms = detect.await.expect("detect_platforms timed out");
                assert!(
                    platforms.is_empty(),
                    "slow provider must be dropped by per-detector timeout, got {platforms:?}"
                );
            })
            .await;
        }
    }

    mod metadata_server_detectors {
        use std::time::Instant;

        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        use super::*;

        #[tokio::test]
        async fn detects_is_ec2_instance_via_imds() {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/latest/dynamic/instance-identity/document"))
                .respond_with(
                    ResponseTemplate::new(200).set_body_string(r#"{"instanceId":"i-12345"}"#),
                )
                .expect(1)
                .mount(&server)
                .await;

            let mut cfg = test_detection_config();
            cfg.ec2_metadata_url =
                format!("{}/latest/dynamic/instance-identity/document", server.uri());

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

        /// Every HTTP URL points at a mock that delays 3s; every detector
        /// must be aborted by the per-detector 200ms timeout rather than
        /// the whole call blocking for 3s.
        #[tokio::test]
        async fn aborts_within_200ms_when_metadata_endpoint_hangs() {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_string(r#"{"instanceId":"i-12345"}"#)
                        .set_delay(Duration::from_secs(3)),
                )
                .mount(&server)
                .await;

            let mut cfg = test_detection_config();
            cfg.ec2_metadata_url = format!("{}/ec2", server.uri());
            cfg.azure_metadata_base_url = server.uri();
            cfg.gce_metadata_root_url = server.uri();
            cfg.gce_metadata_base_url = server.uri();

            temp_env::async_with_vars(platform_detection_env_vars(&[]), async {
                let start = Instant::now();
                let platforms = detect_platforms(&cfg).await;
                let elapsed = start.elapsed();

                assert!(
                    !platforms.iter().any(|p| p.starts_with("is_ec2")
                        || p.starts_with("is_azure")
                        || p.starts_with("is_gce")
                        || p.starts_with("has_")),
                    "expected no HTTP detector to fire, got {platforms:?}"
                );

                assert!(
                    elapsed < DETECTION_TIMEOUT + Duration::from_millis(400),
                    "expected abort near {DETECTION_TIMEOUT:?}, got {elapsed:?}"
                );
            })
            .await;
        }
    }
}
