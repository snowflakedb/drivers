use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures::FutureExt;
use futures::future::BoxFuture;
use tracing::instrument::WithSubscriber;

use crate::env_vars;
use crate::telemetry::session_telemetry::SessionTelemetry;

use self::aws::{CallerIdentityProvider, StsCallerIdentityProvider};

mod aws;
mod azure;
mod gcp;

#[cfg(test)]
mod tests;

const DETECTION_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Clone)]
pub struct DetectionConfig {
    pub(crate) caller_identity_provider: Arc<dyn CallerIdentityProvider>,
    pub(crate) aws_metadata_base_url: String,
    pub(crate) azure_metadata_base_url: String,
    pub(crate) gce_metadata_root_url: String,
    pub(crate) gce_metadata_base_url: String,
    pub(crate) timeout: Duration,
}

impl Default for DetectionConfig {
    fn default() -> Self {
        Self {
            caller_identity_provider: Arc::new(StsCallerIdentityProvider),
            aws_metadata_base_url: "http://169.254.169.254".to_string(),
            azure_metadata_base_url: "http://169.254.169.254".to_string(),
            gce_metadata_root_url: "http://metadata.google.internal".to_string(),
            gce_metadata_base_url: "http://metadata.google.internal/computeMetadata/v1".to_string(),
            timeout: DETECTION_TIMEOUT,
        }
    }
}

/// Run all detectors concurrently with a per-detector `DetectionConfig::timeout`.
///
/// A zero timeout runs only the environment-variable detectors; the ones that
/// query cloud-metadata or STS endpoints are left out.
/// Returns the names of only those detectors that succeeded. Order matches
/// the detector list below so the serialized array is stable across runs.
pub async fn detect_platforms(config: &DetectionConfig) -> Vec<String> {
    if crate::utils::env_flag(env_vars::SNOWFLAKE_DISABLE_PLATFORM_DETECTION) {
        return vec!["disabled".to_string()];
    }

    // Env-only detection needs no HTTP client. Endpoint probes may run before
    // any connection, so pin the process-global crypto provider before building
    // their client; in a FIPS build, do not send traffic through another provider.
    // `http` outlives the detector futures that borrow it.
    let http = if config.timeout.is_zero() {
        None
    } else {
        crate::tls::ensure_crypto_provider();

        // The probe client's HTTPS uses the linked module explicitly. The gate
        // still runs so a `fips` build with a non-FIPS process provider skips
        // detection: every connection this driver would go on to make fails the
        // same gate, so the probes could only report on a session that cannot be
        // established. `detect_platforms` has no error channel, so the failure
        // folds into the existing `disabled` result.
        #[cfg(feature = "fips")]
        if let Err(e) = crate::tls::require_fips_provider() {
            tracing::error!(
                error = %e,
                "skipping platform detection: crypto provider is not FIPS in a `fips` build"
            );
            return vec!["disabled".to_string()];
        }

        let client = crate::tls::client::default_verified_client_builder()
            .map_err(|e| e.to_string())
            .and_then(|builder| builder.build().map_err(|e| e.to_string()));
        match client {
            Ok(c) => Some(c),
            Err(e) => {
                tracing::warn!(
                    "failed to build platform detection HTTP client; skipping detection"
                );
                tracing::debug!("failed to build platform detection HTTP client: {e}");
                return vec!["disabled".to_string()];
            }
        }
    };

    let mut detectors: Vec<(&'static str, BoxFuture<'_, bool>)> = vec![
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
    ];

    if let Some(http) = &http {
        detectors.extend([
            (
                "has_aws_identity",
                aws::has_aws_identity(config.caller_identity_provider.as_ref()).boxed(),
            ),
            (
                "is_ec2_instance",
                aws::is_ec2_instance(http, config).boxed(),
            ),
            ("is_azure_vm", azure::is_azure_vm(http, config).boxed()),
            (
                "has_azure_managed_identity",
                azure::has_azure_managed_identity(http, config).boxed(),
            ),
            ("is_gce_vm", gcp::is_gce_vm(http, config).boxed()),
            (
                "has_gcp_identity",
                gcp::has_gcp_identity(http, config).boxed(),
            ),
        ]);
    }

    let results = futures::future::join_all(detectors.into_iter().map(|(name, fut)| async move {
        let detected = tokio::time::timeout(config.timeout, fut)
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
    env_vars::SNOWFLAKE_DISABLE_PLATFORM_DETECTION,
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
/// defaults to `None` (cleared). Callers override any key via `overrides`
/// (e.g. setting `SNOWFLAKE_DISABLE_PLATFORM_DETECTION=true` to exercise
/// the kill-switch path).
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
        .map(|key| (*key, None))
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

pub(crate) struct PlatformDetector {
    config: DetectionConfig,
    started: AtomicBool,
    platforms: Arc<OnceLock<Vec<String>>>,
}

impl PlatformDetector {
    pub(crate) fn new(config: DetectionConfig) -> Self {
        Self {
            config,
            started: AtomicBool::new(false),
            platforms: Arc::default(),
        }
    }

    pub(crate) fn start(&self) {
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            return;
        };
        if self.started.swap(true, Ordering::Relaxed) {
            return;
        }
        let config = self.config.clone();
        let platforms = Arc::clone(&self.platforms);
        runtime.spawn(
            async move {
                let _ = platforms.set(detect_platforms(&config).await);
            }
            .with_current_subscriber(),
        );
    }

    pub(crate) fn platforms(&self) -> Option<&[String]> {
        self.platforms.get().map(Vec::as_slice)
    }

    #[cfg(test)]
    pub(crate) fn has_started(&self) -> bool {
        self.started.load(Ordering::Relaxed)
    }

    pub(crate) fn report(&self, telemetry: &SessionTelemetry, session_id: i64) {
        let Some(platforms) = self.platforms() else {
            return;
        };
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let message = serde_json::json!({
            "type": "platform_detection_status",
            "value": platforms,
        });
        telemetry.add_log(session_id, message.to_string(), timestamp_ms);
    }
}
