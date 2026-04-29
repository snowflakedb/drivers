use std::sync::Arc;
use std::time::Duration;

use futures::FutureExt;
use futures::future::BoxFuture;

use self::aws::{CallerIdentityProvider, StsCallerIdentityProvider};

mod aws;
mod azure;
mod gcp;

#[cfg(test)]
mod tests;

const DETECTION_TIMEOUT: Duration = Duration::from_millis(200);

#[derive(Clone)]
pub struct DetectionConfig {
    pub(crate) caller_identity_provider: Arc<dyn CallerIdentityProvider>,
    pub(crate) aws_metadata_base_url: String,
    pub(crate) azure_metadata_base_url: String,
    pub(crate) gce_metadata_root_url: String,
    pub(crate) gce_metadata_base_url: String,
}

impl Default for DetectionConfig {
    fn default() -> Self {
        Self {
            caller_identity_provider: Arc::new(StsCallerIdentityProvider),
            aws_metadata_base_url: "http://169.254.169.254".to_string(),
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
        ("is_aws_lambda", async { aws::is_aws_lambda() }.boxed()),
        (
            "is_azure_function",
            async { azure::is_azure_function() }.boxed(),
        ),
        (
            "is_gce_cloud_run_service",
            async { gcp::is_gce_cloud_run_service() }.boxed(),
        ),
        (
            "is_gce_cloud_run_job",
            async { gcp::is_gce_cloud_run_job() }.boxed(),
        ),
        ("is_github_action", async { is_github_action() }.boxed()),
        (
            "has_aws_identity",
            aws::has_aws_identity(config.caller_identity_provider.as_ref()).boxed(),
        ),
        (
            "is_ec2_instance",
            aws::is_ec2_instance(&http, config).boxed(),
        ),
        ("is_azure_vm", azure::is_azure_vm(&http, config).boxed()),
        (
            "has_azure_managed_identity",
            azure::has_azure_managed_identity(&http, config).boxed(),
        ),
        ("is_gce_vm", gcp::is_gce_vm(&http, config).boxed()),
        (
            "has_gcp_identity",
            gcp::has_gcp_identity(&http, config).boxed(),
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

pub(super) fn env_non_empty(key: &str) -> bool {
    std::env::var(key)
        .map(|value| !value.is_empty())
        .unwrap_or(false)
}

pub(super) fn is_github_action() -> bool {
    env_non_empty("GITHUB_ACTIONS")
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
