//! Platform detection for `CLIENT_ENVIRONMENT.PLATFORM`.
//!
//! Detectors fall into three buckets:
//! - env-only (synchronous env-var probes)
//! - HTTP metadata (cloud provider IMDS endpoints)
//! - AWS SDK (STS `GetCallerIdentity` for workload identity)
//!
//! Each detector runs under its own [`DETECTION_TIMEOUT`]. Wrapping `join_all`
//! with a single outer timeout would drop all results on expiry; per-detector
//! timeout bounds the total wall time to ~200 ms while still keeping partial
//! results for detectors that finished in time.
//!
//! The `SNOWFLAKE_DISABLE_PLATFORM_DETECTION=true` env var short-circuits the
//! entire pipeline and returns `["disabled"]`.

use std::sync::Arc;
use std::time::Duration;

use aws_sdk_sts::config::BehaviorVersion;
use futures::future::BoxFuture;

pub const DETECTION_TIMEOUT: Duration = Duration::from_millis(200);

const DISABLE_ENV: &str = "SNOWFLAKE_DISABLE_PLATFORM_DETECTION";

const GCP_METADATA_FLAVOR_HEADER: &str = "Metadata-Flavor";
const GCP_METADATA_FLAVOR_VALUE: &str = "Google";

/// HTTP endpoints used by metadata-based detectors. Injectable so tests can
/// redirect them at a wiremock server (or at a deliberately unreachable
/// address to guarantee HTTP paths fail fast).
#[derive(Debug, Clone)]
pub struct DetectionConfig {
    pub ec2_metadata_url: String,
    pub azure_metadata_base_url: String,
    pub gce_metadata_root_url: String,
    pub gce_metadata_base_url: String,
    pub caller_identity_provider: Arc<dyn CallerIdentityProvider>,
}

impl Default for DetectionConfig {
    fn default() -> Self {
        Self {
            ec2_metadata_url: "http://169.254.169.254/latest/dynamic/instance-identity/document"
                .to_string(),
            azure_metadata_base_url: "http://169.254.169.254".to_string(),
            gce_metadata_root_url: "http://metadata.google.internal".to_string(),
            gce_metadata_base_url: "http://metadata.google.internal/computeMetadata/v1".to_string(),
            caller_identity_provider: Arc::new(StsCallerIdentityProvider),
        }
    }
}

impl DetectionConfig {
    /// Test helper: every HTTP endpoint points at `http://127.0.0.1:1`, which
    /// reliably refuses connections on all supported platforms. HTTP detectors
    /// therefore fail quickly and cannot flake an env-only test assertion.
    #[cfg(test)]
    pub fn unreachable() -> Self {
        let unreachable_url = "http://127.0.0.1:1".to_string();
        Self {
            ec2_metadata_url: unreachable_url.clone(),
            azure_metadata_base_url: unreachable_url.clone(),
            gce_metadata_root_url: unreachable_url.clone(),
            gce_metadata_base_url: unreachable_url,
            caller_identity_provider: Arc::new(NoopCallerIdentityProvider),
        }
    }
}

/// Abstraction over AWS STS `GetCallerIdentity`, injectable for tests.
///
/// Production is [`StsCallerIdentityProvider`] which loads the default AWS
/// config chain and calls STS. Tests inject a stub that returns a canned ARN
/// or an error without hitting the network.
pub trait CallerIdentityProvider: Send + Sync + std::fmt::Debug {
    fn arn<'a>(&'a self) -> BoxFuture<'a, Option<String>>;
}

#[derive(Debug)]
pub struct StsCallerIdentityProvider;

impl CallerIdentityProvider for StsCallerIdentityProvider {
    fn arn<'a>(&'a self) -> BoxFuture<'a, Option<String>> {
        Box::pin(async move {
            let config = aws_config::defaults(BehaviorVersion::latest()).load().await;
            let client = aws_sdk_sts::Client::new(&config);
            client.get_caller_identity().send().await.ok()?.arn
        })
    }
}

#[cfg(test)]
#[derive(Debug)]
pub struct NoopCallerIdentityProvider;

#[cfg(test)]
impl CallerIdentityProvider for NoopCallerIdentityProvider {
    fn arn<'a>(&'a self) -> BoxFuture<'a, Option<String>> {
        Box::pin(async move { None })
    }
}

/// Run all 11 detectors concurrently with per-detector [`DETECTION_TIMEOUT`].
///
/// Returns the names of detectors that fired (same string used in the Node
/// reference). Order matches the detector list below so the serialized array
/// is stable across runs.
pub async fn detect_platforms(config: &DetectionConfig) -> Vec<String> {
    if std::env::var(DISABLE_ENV)
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        return vec!["disabled".to_string()];
    }

    let http = reqwest::Client::builder()
        .timeout(DETECTION_TIMEOUT)
        .build()
        .unwrap_or_default();

    let detectors: Vec<(&'static str, BoxFuture<'_, bool>)> = vec![
        ("is_aws_lambda", Box::pin(async { is_aws_lambda() })),
        ("is_azure_function", Box::pin(async { is_azure_function() })),
        (
            "is_gce_cloud_run_service",
            Box::pin(async { is_gce_cloud_run_service() }),
        ),
        (
            "is_gce_cloud_run_job",
            Box::pin(async { is_gce_cloud_run_job() }),
        ),
        ("is_github_action", Box::pin(async { is_github_action() })),
        ("is_ec2_instance", Box::pin(is_ec2_instance(&http, config))),
        (
            "has_aws_identity",
            Box::pin(has_aws_identity(config.caller_identity_provider.as_ref())),
        ),
        ("is_azure_vm", Box::pin(is_azure_vm(&http, config))),
        (
            "has_azure_managed_identity",
            Box::pin(has_azure_managed_identity(&http, config)),
        ),
        ("is_gce_vm", Box::pin(is_gce_vm(&http, config))),
        (
            "has_gcp_identity",
            Box::pin(has_gcp_identity(&http, config)),
        ),
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
    std::env::var(key).map(|v| !v.is_empty()).unwrap_or(false)
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
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

async fn has_aws_identity(provider: &dyn CallerIdentityProvider) -> bool {
    provider
        .arn()
        .await
        .as_deref()
        .map(is_valid_arn_for_wif)
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
        .map(|r| r.status().is_success())
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
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

async fn is_gce_vm(http: &reqwest::Client, config: &DetectionConfig) -> bool {
    http.get(&config.gce_metadata_root_url)
        .send()
        .await
        .map(|r| {
            r.headers()
                .get(GCP_METADATA_FLAVOR_HEADER)
                .and_then(|v| v.to_str().ok())
                .map(|v| v == GCP_METADATA_FLAVOR_VALUE)
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
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

/// Matches the Go reference regex. Only IAM users and STS assumed roles
/// qualify as valid workload-identity federation ARNs.
///
/// Expected layout is `arn:<partition>:<service>::<account>:<resource>`
/// (region segment is always empty for IAM/STS).
fn is_valid_arn_for_wif(arn: &str) -> bool {
    let parts: Vec<&str> = arn.splitn(6, ':').collect();
    if parts.len() != 6 || parts[0] != "arn" || parts[1].is_empty() || parts[4].is_empty() {
        return false;
    }
    match parts[2] {
        "iam" => parts[5].starts_with("user/") && parts[5].len() > "user/".len(),
        "sts" => parts[5].starts_with("assumed-role/") && parts[5].len() > "assumed-role/".len(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::time::Instant;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const ENV_KEYS: &[&str] = &[
        "SNOWFLAKE_DISABLE_PLATFORM_DETECTION",
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

    /// Clear every env var platform detection reads. `temp_env` only scopes
    /// the keys the caller enumerates, so a CI runner that happens to export
    /// e.g. `GITHUB_ACTIONS` would otherwise bleed into unrelated tests.
    fn clean_env() -> Vec<(&'static str, Option<&'static str>)> {
        ENV_KEYS.iter().map(|k| (*k, None)).collect()
    }

    #[derive(Debug)]
    struct StaticArnProvider(Option<String>);

    impl CallerIdentityProvider for StaticArnProvider {
        fn arn<'a>(&'a self) -> BoxFuture<'a, Option<String>> {
            let value = self.0.clone();
            Box::pin(async move { value })
        }
    }

    #[derive(Debug, Default)]
    struct CountingArnProvider {
        calls: Mutex<usize>,
        arn: Option<String>,
    }

    impl CallerIdentityProvider for CountingArnProvider {
        fn arn<'a>(&'a self) -> BoxFuture<'a, Option<String>> {
            *self.calls.lock().unwrap() += 1;
            let value = self.arn.clone();
            Box::pin(async move { value })
        }
    }

    fn with_provider(
        mut cfg: DetectionConfig,
        provider: Arc<dyn CallerIdentityProvider>,
    ) -> DetectionConfig {
        cfg.caller_identity_provider = provider;
        cfg
    }

    // -----------------------------------------------------------------
    // Test 1 — disabled via env
    // -----------------------------------------------------------------

    #[tokio::test]
    async fn returns_disabled_when_env_flag_true() {
        let mut env = clean_env();
        env[0] = ("SNOWFLAKE_DISABLE_PLATFORM_DETECTION", Some("true"));

        temp_env::async_with_vars(env, async {
            let cfg = DetectionConfig::unreachable();
            let start = Instant::now();
            let platforms = detect_platforms(&cfg).await;
            assert_eq!(platforms, vec!["disabled".to_string()]);
            assert!(
                start.elapsed() < Duration::from_millis(50),
                "disabled path must not do HTTP or env work"
            );
        })
        .await;
    }

    #[tokio::test]
    async fn disabled_flag_matches_case_insensitive() {
        let mut env = clean_env();
        env[0] = ("SNOWFLAKE_DISABLE_PLATFORM_DETECTION", Some("TRUE"));

        temp_env::async_with_vars(env, async {
            let cfg = DetectionConfig::unreachable();
            assert_eq!(detect_platforms(&cfg).await, vec!["disabled".to_string()]);
        })
        .await;
    }

    // -----------------------------------------------------------------
    // Test 2 — env-only detectors
    // -----------------------------------------------------------------

    #[tokio::test]
    async fn detects_is_aws_lambda_from_env() {
        let mut env = clean_env();
        env[1] = ("LAMBDA_TASK_ROOT", Some("/var/task"));

        temp_env::async_with_vars(env, async {
            let cfg = DetectionConfig::unreachable();
            let platforms = detect_platforms(&cfg).await;
            assert!(platforms.contains(&"is_aws_lambda".to_string()));
        })
        .await;
    }

    #[tokio::test]
    async fn detects_is_azure_function_requires_all_three_env_vars() {
        let mut env = clean_env();
        env[2] = ("FUNCTIONS_WORKER_RUNTIME", Some("node"));
        env[3] = ("FUNCTIONS_EXTENSION_VERSION", Some("~4"));
        env[4] = ("AzureWebJobsStorage", Some("DefaultEndpoint=..."));

        temp_env::async_with_vars(env, async {
            let cfg = DetectionConfig::unreachable();
            let platforms = detect_platforms(&cfg).await;
            assert!(platforms.contains(&"is_azure_function".to_string()));
        })
        .await;
    }

    #[tokio::test]
    async fn does_not_detect_azure_function_when_partial_env() {
        let mut env = clean_env();
        env[2] = ("FUNCTIONS_WORKER_RUNTIME", Some("node"));

        temp_env::async_with_vars(env, async {
            let cfg = DetectionConfig::unreachable();
            let platforms = detect_platforms(&cfg).await;
            assert!(!platforms.contains(&"is_azure_function".to_string()));
        })
        .await;
    }

    #[tokio::test]
    async fn detects_is_gce_cloud_run_service_from_env() {
        let mut env = clean_env();
        env[6] = ("K_SERVICE", Some("svc"));
        env[7] = ("K_REVISION", Some("rev"));
        env[8] = ("K_CONFIGURATION", Some("cfg"));

        temp_env::async_with_vars(env, async {
            let cfg = DetectionConfig::unreachable();
            let platforms = detect_platforms(&cfg).await;
            assert!(platforms.contains(&"is_gce_cloud_run_service".to_string()));
        })
        .await;
    }

    #[tokio::test]
    async fn detects_is_gce_cloud_run_job_from_env() {
        let mut env = clean_env();
        env[9] = ("CLOUD_RUN_JOB", Some("j"));
        env[10] = ("CLOUD_RUN_EXECUTION", Some("e"));

        temp_env::async_with_vars(env, async {
            let cfg = DetectionConfig::unreachable();
            let platforms = detect_platforms(&cfg).await;
            assert!(platforms.contains(&"is_gce_cloud_run_job".to_string()));
        })
        .await;
    }

    #[tokio::test]
    async fn detects_is_github_action_from_env() {
        let mut env = clean_env();
        env[11] = ("GITHUB_ACTIONS", Some("true"));

        temp_env::async_with_vars(env, async {
            let cfg = DetectionConfig::unreachable();
            let platforms = detect_platforms(&cfg).await;
            assert!(platforms.contains(&"is_github_action".to_string()));
        })
        .await;
    }

    #[tokio::test]
    async fn detects_multiple_env_platforms_simultaneously() {
        let mut env = clean_env();
        env[1] = ("LAMBDA_TASK_ROOT", Some("/var/task"));
        env[9] = ("CLOUD_RUN_JOB", Some("j"));
        env[10] = ("CLOUD_RUN_EXECUTION", Some("e"));

        temp_env::async_with_vars(env, async {
            let cfg = DetectionConfig::unreachable();
            let platforms = detect_platforms(&cfg).await;
            assert!(platforms.contains(&"is_aws_lambda".to_string()));
            assert!(platforms.contains(&"is_gce_cloud_run_job".to_string()));
        })
        .await;
    }

    #[tokio::test]
    async fn returns_empty_when_no_platform_detected() {
        temp_env::async_with_vars(clean_env(), async {
            let cfg = DetectionConfig::unreachable();
            let platforms = detect_platforms(&cfg).await;
            assert!(platforms.is_empty(), "expected empty, got {platforms:?}");
        })
        .await;
    }

    // -----------------------------------------------------------------
    // Test 3 — HTTP detectors
    // -----------------------------------------------------------------

    #[tokio::test]
    async fn detects_is_ec2_instance_via_imds() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/latest/dynamic/instance-identity/document"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"instanceId":"i-12345"}"#))
            .expect(1)
            .mount(&server)
            .await;

        let cfg = DetectionConfig {
            ec2_metadata_url: format!("{}/latest/dynamic/instance-identity/document", server.uri()),
            ..DetectionConfig::unreachable()
        };

        temp_env::async_with_vars(clean_env(), async {
            let platforms = detect_platforms(&cfg).await;
            assert!(platforms.contains(&"is_ec2_instance".to_string()));
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

        let cfg = DetectionConfig {
            azure_metadata_base_url: server.uri(),
            ..DetectionConfig::unreachable()
        };

        temp_env::async_with_vars(clean_env(), async {
            let platforms = detect_platforms(&cfg).await;
            assert!(platforms.contains(&"is_azure_vm".to_string()));
        })
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

        let cfg = DetectionConfig {
            azure_metadata_base_url: server.uri(),
            ..DetectionConfig::unreachable()
        };

        temp_env::async_with_vars(clean_env(), async {
            let platforms = detect_platforms(&cfg).await;
            assert!(platforms.contains(&"has_azure_managed_identity".to_string()));
        })
        .await;
    }

    #[tokio::test]
    async fn detects_has_azure_managed_identity_via_functions_env() {
        let mut env = clean_env();
        env[2] = ("FUNCTIONS_WORKER_RUNTIME", Some("node"));
        env[3] = ("FUNCTIONS_EXTENSION_VERSION", Some("~4"));
        env[4] = ("AzureWebJobsStorage", Some("DefaultEndpoint=..."));
        env[5] = ("IDENTITY_HEADER", Some("header"));

        temp_env::async_with_vars(env, async {
            let cfg = DetectionConfig::unreachable();
            let platforms = detect_platforms(&cfg).await;
            assert!(platforms.contains(&"has_azure_managed_identity".to_string()));
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

        let cfg = DetectionConfig {
            gce_metadata_root_url: server.uri(),
            ..DetectionConfig::unreachable()
        };

        temp_env::async_with_vars(clean_env(), async {
            let platforms = detect_platforms(&cfg).await;
            assert!(platforms.contains(&"is_gce_vm".to_string()));
        })
        .await;
    }

    #[tokio::test]
    async fn does_not_detect_gce_vm_without_metadata_flavor_header() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let cfg = DetectionConfig {
            gce_metadata_root_url: server.uri(),
            ..DetectionConfig::unreachable()
        };

        temp_env::async_with_vars(clean_env(), async {
            let platforms = detect_platforms(&cfg).await;
            assert!(!platforms.contains(&"is_gce_vm".to_string()));
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

        let cfg = DetectionConfig {
            gce_metadata_base_url: server.uri(),
            ..DetectionConfig::unreachable()
        };

        temp_env::async_with_vars(clean_env(), async {
            let platforms = detect_platforms(&cfg).await;
            assert!(platforms.contains(&"has_gcp_identity".to_string()));
        })
        .await;
    }

    // -----------------------------------------------------------------
    // Test 4 — timeout / abort behavior
    // -----------------------------------------------------------------

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

        let cfg = DetectionConfig {
            ec2_metadata_url: format!("{}/ec2", server.uri()),
            azure_metadata_base_url: server.uri(),
            gce_metadata_root_url: server.uri(),
            gce_metadata_base_url: server.uri(),
            caller_identity_provider: Arc::new(NoopCallerIdentityProvider),
        };

        temp_env::async_with_vars(clean_env(), async {
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
                elapsed >= DETECTION_TIMEOUT
                    && elapsed < DETECTION_TIMEOUT + Duration::from_millis(400),
                "expected abort at ~{DETECTION_TIMEOUT:?}, got {elapsed:?}"
            );
        })
        .await;
    }

    #[tokio::test]
    async fn slow_http_does_not_block_fast_env_detector() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(3)))
            .mount(&server)
            .await;

        let cfg = DetectionConfig {
            ec2_metadata_url: format!("{}/ec2", server.uri()),
            azure_metadata_base_url: server.uri(),
            gce_metadata_root_url: server.uri(),
            gce_metadata_base_url: server.uri(),
            caller_identity_provider: Arc::new(NoopCallerIdentityProvider),
        };

        let mut env = clean_env();
        env[11] = ("GITHUB_ACTIONS", Some("true"));

        let start = Instant::now();
        let platforms =
            temp_env::async_with_vars(env, async { detect_platforms(&cfg).await }).await;

        assert!(
            platforms.contains(&"is_github_action".to_string()),
            "env detector must fire even while HTTP detectors hang, got {platforms:?}"
        );
        assert!(
            start.elapsed() < Duration::from_millis(600),
            "elapsed {:?} suggests env detector was blocked on HTTP",
            start.elapsed()
        );
    }

    // -----------------------------------------------------------------
    // Test 5 — has_aws_identity via injected STS provider
    // -----------------------------------------------------------------

    #[tokio::test]
    async fn has_aws_identity_detects_iam_user_arn() {
        let provider = Arc::new(StaticArnProvider(Some(
            "arn:aws:iam::123456789012:user/alice".to_string(),
        )));
        let cfg = with_provider(DetectionConfig::unreachable(), provider);

        temp_env::async_with_vars(clean_env(), async {
            let platforms = detect_platforms(&cfg).await;
            assert!(platforms.contains(&"has_aws_identity".to_string()));
        })
        .await;
    }

    #[tokio::test]
    async fn has_aws_identity_detects_assumed_role_arn() {
        let provider = Arc::new(StaticArnProvider(Some(
            "arn:aws:sts::123456789012:assumed-role/my-role/session".to_string(),
        )));
        let cfg = with_provider(DetectionConfig::unreachable(), provider);

        temp_env::async_with_vars(clean_env(), async {
            let platforms = detect_platforms(&cfg).await;
            assert!(platforms.contains(&"has_aws_identity".to_string()));
        })
        .await;
    }

    #[tokio::test]
    async fn has_aws_identity_rejects_invalid_arn() {
        // IAM role ARNs (role/..., not user/... or assumed-role/...) do not
        // qualify as WIF identities per the Go reference regex.
        let provider = Arc::new(StaticArnProvider(Some(
            "arn:aws:iam::123456789012:role/my-role".to_string(),
        )));
        let cfg = with_provider(DetectionConfig::unreachable(), provider);

        temp_env::async_with_vars(clean_env(), async {
            let platforms = detect_platforms(&cfg).await;
            assert!(!platforms.contains(&"has_aws_identity".to_string()));
        })
        .await;
    }

    #[tokio::test]
    async fn has_aws_identity_handles_sts_error() {
        let provider = Arc::new(StaticArnProvider(None));
        let cfg = with_provider(DetectionConfig::unreachable(), provider);

        temp_env::async_with_vars(clean_env(), async {
            let platforms = detect_platforms(&cfg).await;
            assert!(!platforms.contains(&"has_aws_identity".to_string()));
        })
        .await;
    }

    #[test]
    fn is_valid_arn_for_wif_covers_reference_cases() {
        assert!(is_valid_arn_for_wif("arn:aws:iam::1:user/u"));
        assert!(is_valid_arn_for_wif("arn:aws:sts::1:assumed-role/r/s"));
        assert!(!is_valid_arn_for_wif("arn:aws:iam::1:role/r"));
        assert!(!is_valid_arn_for_wif("arn:aws:s3:::bucket/key"));
        assert!(!is_valid_arn_for_wif("not-an-arn"));
        assert!(!is_valid_arn_for_wif("arn:aws:iam::1:user/"));
    }

    // -----------------------------------------------------------------
    // Test 6 — caller identity provider is invoked only when STS path runs
    // -----------------------------------------------------------------

    #[tokio::test]
    async fn caller_identity_provider_invoked_once_per_detect_call() {
        let provider = Arc::new(CountingArnProvider::default());
        let cfg = with_provider(DetectionConfig::unreachable(), provider.clone());

        temp_env::async_with_vars(clean_env(), async {
            let _ = detect_platforms(&cfg).await;
        })
        .await;

        assert_eq!(*provider.calls.lock().unwrap(), 1);
    }
}
