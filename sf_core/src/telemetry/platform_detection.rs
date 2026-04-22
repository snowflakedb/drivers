type EnvDetector = (&'static str, fn() -> bool);

const ENV_DETECTORS: &[EnvDetector] = &[
    ("is_aws_lambda", is_aws_lambda),
    ("is_azure_function", is_azure_function),
    ("is_gce_cloud_run_service", is_gce_cloud_run_service),
    ("is_gce_cloud_run_job", is_gce_cloud_run_job),
    ("is_github_action", is_github_action),
];

pub async fn detect_platforms() -> Vec<String> {
    if std::env::var("SNOWFLAKE_DISABLE_PLATFORM_DETECTION")
        .map(|value| value.eq_ignore_ascii_case("true") || value == "1")
        .unwrap_or(false)
    {
        return vec!["disabled".to_string()];
    }

    ENV_DETECTORS
        .iter()
        .filter(|(_, probe)| probe())
        .map(|(name, _)| (*name).to_string())
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

#[cfg(any(test, feature = "test-utils"))]
const PLATFORM_DETECTION_ENV_KEYS: &[&str] = &[
    "SNOWFLAKE_DISABLE_PLATFORM_DETECTION",
    "LAMBDA_TASK_ROOT",
    "FUNCTIONS_WORKER_RUNTIME",
    "FUNCTIONS_EXTENSION_VERSION",
    "AzureWebJobsStorage",
    "K_SERVICE",
    "K_REVISION",
    "K_CONFIGURATION",
    "CLOUD_RUN_JOB",
    "CLOUD_RUN_EXECUTION",
    "GITHUB_ACTIONS",
];

/// Builds the `(key, Option<value>)` list to hand to `temp_env::with_vars`
/// or `temp_env::async_with_vars`. Every key in [`PLATFORM_DETECTION_ENV_KEYS`]
/// defaults to `None` (cleared) and is overridden by whatever the caller
/// supplies in `overrides`.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn returns_disabled_when_env_flag_true() {
        temp_env::async_with_vars(
            platform_detection_env_vars(&[("SNOWFLAKE_DISABLE_PLATFORM_DETECTION", "true")]),
            async {
                assert_eq!(detect_platforms().await, vec!["disabled"]);
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
                        detect_platforms().await,
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
                let platforms = detect_platforms().await;
                assert!(platforms.is_empty(), "expected empty, got {platforms:?}");
            },
        )
        .await;
    }

    #[tokio::test]
    async fn returns_empty_when_no_platform_detected() {
        temp_env::async_with_vars(platform_detection_env_vars(&[]), async {
            let platforms = detect_platforms().await;
            assert!(platforms.is_empty(), "expected empty, got {platforms:?}");
        })
        .await;
    }

    #[tokio::test]
    async fn detects_is_aws_lambda_from_env() {
        temp_env::async_with_vars(
            platform_detection_env_vars(&[("LAMBDA_TASK_ROOT", "/var/task")]),
            async {
                assert_eq!(detect_platforms().await, vec!["is_aws_lambda".to_string()]);
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
                    detect_platforms().await,
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
                    detect_platforms().await,
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
                    detect_platforms().await,
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
                    detect_platforms().await,
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
                let platforms = detect_platforms().await;
                assert_eq!(
                    platforms,
                    vec![
                        "is_aws_lambda".to_string(),
                        "is_gce_cloud_run_job".to_string(),
                        "is_github_action".to_string(),
                    ],
                    "detector order must match ENV_DETECTORS declaration order",
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
                    detect_platforms().await,
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
                let platforms = detect_platforms().await;
                assert!(
                    !platforms.contains(&"is_github_action".to_string()),
                    "empty string must be treated as unset, got {platforms:?}",
                );
            },
        )
        .await;
    }
}
