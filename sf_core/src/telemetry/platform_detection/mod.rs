use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures::FutureExt;
use futures::future::BoxFuture;
use tracing::instrument::WithSubscriber;

use crate::config::param_names;
use crate::config::param_store::ParamStore;
use crate::config::settings::Setting;
use crate::env_vars;
use crate::telemetry::session_telemetry::SessionTelemetry;
use crate::tls::RootStoreProvider;

use self::aws::{CallerIdentityProvider, StsCallerIdentityProvider};

mod aws;
mod azure;
mod gcp;

#[cfg(test)]
mod tests;

const DETECTION_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_DETECTION_TIMEOUT_MS: i64 = 5_000;

#[derive(Clone)]
pub(crate) struct DetectionConfig {
    pub(crate) caller_identity_provider: Arc<dyn CallerIdentityProvider>,
    pub(crate) root_store_provider: RootStoreProvider,
    pub(crate) aws_metadata_base_url: String,
    pub(crate) azure_metadata_base_url: String,
    pub(crate) gce_metadata_root_url: String,
    pub(crate) gce_metadata_base_url: String,
    pub(crate) timeout: Duration,
    /// `Some` is the disable flag from the connection setting. `None` consults
    /// `SNOWFLAKE_DISABLE_PLATFORM_DETECTION` when detection runs.
    pub(crate) disabled: Option<bool>,
}

/// Timeout and disable flag from JDBC.
///
/// A negative timeout becomes 0, which skips metadata and STS probes. A timeout
/// above 5000 ms becomes 5000 ms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PlatformDetectionChoice {
    pub(crate) disabled: bool,
    pub(crate) timeout: Duration,
}

impl DetectionConfig {
    pub(crate) fn new(root_store_provider: RootStoreProvider) -> Self {
        Self {
            caller_identity_provider: Arc::new(StsCallerIdentityProvider {
                roots: root_store_provider.clone(),
            }),
            root_store_provider,
            aws_metadata_base_url: "http://169.254.169.254".to_string(),
            azure_metadata_base_url: "http://169.254.169.254".to_string(),
            gce_metadata_root_url: "http://metadata.google.internal".to_string(),
            gce_metadata_base_url: "http://metadata.google.internal/computeMetadata/v1".to_string(),
            timeout: DETECTION_TIMEOUT,
            disabled: None,
        }
    }
}

pub(crate) fn resolve_from_params(params: &ParamStore) -> PlatformDetectionChoice {
    let disable_env = std::env::var(env_vars::SNOWFLAKE_DISABLE_PLATFORM_DETECTION).ok();
    let timeout_env = std::env::var(env_vars::SNOWFLAKE_PLATFORM_DETECTION_TIMEOUT_MS).ok();
    resolve_platform_detection(
        connection_disabled(params.get(param_names::DISABLE_PLATFORM_DETECTION)),
        connection_timeout_ms(params.get(param_names::PLATFORM_DETECTION_TIMEOUT_MS)),
        disable_env.as_deref(),
        timeout_env.as_deref(),
    )
}

pub(crate) fn resolve_platform_detection(
    connection_disabled: Option<bool>,
    connection_timeout_ms: Option<i64>,
    disable_env: Option<&str>,
    timeout_env: Option<&str>,
) -> PlatformDetectionChoice {
    PlatformDetectionChoice {
        disabled: resolve_disabled(connection_disabled, disable_env),
        timeout: resolve_timeout(connection_timeout_ms, timeout_env),
    }
}

fn connection_disabled(setting: Option<&Setting>) -> Option<bool> {
    match setting? {
        Setting::Bool(value) => Some(*value),
        Setting::String(value) => crate::utils::parse_setting_bool_token(value.trim()),
        _ => None,
    }
}

fn connection_timeout_ms(setting: Option<&Setting>) -> Option<i64> {
    let setting = setting?;
    match setting {
        Setting::Int(value) => Some(*value),
        Setting::String(value) => parse_timeout_text(value, "platform_detection_timeout_ms"),
        _ => None,
    }
}

fn resolve_disabled(connection_disabled: Option<bool>, disable_env: Option<&str>) -> bool {
    if let Some(disabled) = connection_disabled {
        return disabled;
    }
    if let Some(value) = disable_env
        && let Some(disabled) =
            parse_disable_global(value, env_vars::SNOWFLAKE_DISABLE_PLATFORM_DETECTION)
    {
        return disabled;
    }
    false
}

fn parse_disable_global(value: &str, source: &'static str) -> Option<bool> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.eq_ignore_ascii_case("true") {
        return Some(true);
    }
    if !trimmed.eq_ignore_ascii_case("false") {
        tracing::warn!(
            source,
            value,
            "ignoring unrecognized platform-detection disable value; only \"true\" disables it"
        );
    }
    Some(false)
}

fn env_disables_platform_detection() -> bool {
    std::env::var(env_vars::SNOWFLAKE_DISABLE_PLATFORM_DETECTION)
        .ok()
        .and_then(|value| {
            parse_disable_global(&value, env_vars::SNOWFLAKE_DISABLE_PLATFORM_DETECTION)
        })
        .unwrap_or(false)
}

fn resolve_timeout(connection_timeout_ms: Option<i64>, timeout_env: Option<&str>) -> Duration {
    if let Some(timeout_ms) = connection_timeout_ms {
        return Duration::from_millis(clamp_timeout_ms(
            timeout_ms,
            "platform_detection_timeout_ms",
        ));
    }
    if let Some(value) = timeout_env
        && let Some(timeout_ms) =
            parse_timeout_text(value, env_vars::SNOWFLAKE_PLATFORM_DETECTION_TIMEOUT_MS)
    {
        return Duration::from_millis(clamp_timeout_ms(
            timeout_ms,
            env_vars::SNOWFLAKE_PLATFORM_DETECTION_TIMEOUT_MS,
        ));
    }
    DETECTION_TIMEOUT
}

fn parse_timeout_text(value: &str, source: &'static str) -> Option<i64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    match trimmed.parse::<i64>() {
        Ok(timeout_ms) => Some(timeout_ms),
        Err(_) => {
            tracing::warn!(
                source,
                value,
                "failed to parse platform detection timeout; trying the next source"
            );
            None
        }
    }
}

fn clamp_timeout_ms(timeout_ms: i64, source: &'static str) -> u64 {
    if timeout_ms < 0 {
        tracing::warn!(
            source,
            timeout_ms,
            "negative platform detection timeout is treated as 0, which skips network checks"
        );
        return 0;
    }
    if timeout_ms > MAX_DETECTION_TIMEOUT_MS {
        tracing::warn!(
            source,
            timeout_ms,
            max_ms = MAX_DETECTION_TIMEOUT_MS,
            "platform detection timeout exceeds the maximum and is clamped"
        );
        return MAX_DETECTION_TIMEOUT_MS as u64;
    }
    timeout_ms as u64
}

/// Run all detectors concurrently with a per-detector `DetectionConfig::timeout`.
///
/// A zero timeout runs only the environment-variable detectors; the ones that
/// query cloud-metadata or STS endpoints are left out.
/// Returns the names of only those detectors that succeeded. Order matches
/// the detector list below so the serialized array is stable across runs.
pub(crate) async fn detect_platforms(config: &DetectionConfig) -> Vec<String> {
    let disabled = config
        .disabled
        .unwrap_or_else(env_disables_platform_detection);
    if disabled {
        return vec!["disabled".to_string()];
    }

    // Env-only detection needs no HTTP client. Endpoint probes may run before
    // any connection. Their HTTPS client uses the linked module.
    // `require_fips_provider` decides whether a `fips` build runs detection.
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

        let builder =
            match crate::tls::client::default_verified_client_builder(&config.root_store_provider)
                .await
            {
                Ok(builder) => builder,
                Err(e) => {
                    tracing::warn!(
                        error_type = std::any::type_name_of_val(&e),
                        error = %e,
                        "failed to build platform detection HTTP client; skipping detection"
                    );
                    return vec!["disabled".to_string()];
                }
            };
        match builder.build() {
            Ok(c) => Some(c),
            Err(e) => {
                tracing::warn!(
                    error_type = std::any::type_name_of_val(&e),
                    "failed to build platform detection HTTP client; skipping detection"
                );
                tracing::debug!(
                    error = %e,
                    "failed to build platform detection HTTP client"
                );
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

    pub(crate) fn start_with(&self, params: &ParamStore) {
        if self.started.load(Ordering::Relaxed) {
            return;
        }
        let choice = resolve_from_params(params);
        let mut config = self.config.clone();
        config.timeout = choice.timeout;
        config.disabled = Some(choice.disabled);
        self.spawn(config);
    }

    fn spawn(&self, config: DetectionConfig) {
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            return;
        };
        if self.started.swap(true, Ordering::Relaxed) {
            return;
        }
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
    pub(crate) fn root_store_provider(&self) -> &RootStoreProvider {
        &self.config.root_store_provider
    }

    #[cfg(test)]
    pub(crate) fn has_started(&self) -> bool {
        self.started.load(Ordering::Relaxed)
    }

    pub(crate) fn report(&self, telemetry: &SessionTelemetry, telemetry_id: u64) {
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
        telemetry.add_log(telemetry_id, message.to_string(), timestamp_ms);
    }
}
