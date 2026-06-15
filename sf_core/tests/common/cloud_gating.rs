//! Per-CSP test gating: detect which cloud the current run targets,
//! and gate tests that only make sense on a subset of clouds.
//!
//! Driven by the `TEST_CLOUD_PROVIDER` env var; recognized values map
//! to `CloudProvider::{Aws, Gcp, Azure, Dev}`. Default is `Dev` on
//! unset / unrecognized — a fresh `cargo test` from a developer box
//! runs all cloud-gated tests by default. Mirrors the legacy
//! `snowflake-connector-python` `conftest.py:135-144` convention. CI
//! lanes set `TEST_CLOUD_PROVIDER=<aws|azure|gcp>` explicitly via the
//! matrix to scope themselves.
//!
//! `Dev` only sets the gate; tests still need real credentials /
//! network / stage to actually pass.

use std::fmt;
use std::str::FromStr;

pub(crate) const ENV_VAR: &str = "TEST_CLOUD_PROVIDER";

/// Recognized values for the `TEST_CLOUD_PROVIDER` env var.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudProvider {
    Aws,
    Gcp,
    Azure,
    /// Developer-machine wildcard; runs all cloud-gated tests.
    Dev,
}

impl FromStr for CloudProvider {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "aws" => Ok(Self::Aws),
            "gcp" => Ok(Self::Gcp),
            "azure" => Ok(Self::Azure),
            "dev" => Ok(Self::Dev),
            _ => Err(()),
        }
    }
}

impl fmt::Display for CloudProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Aws => "aws",
            Self::Gcp => "gcp",
            Self::Azure => "azure",
            Self::Dev => "dev",
        })
    }
}

/// Returns the current cloud, defaulting to `Dev` on unset / unrecognized.
fn current_cloud_provider() -> CloudProvider {
    std::env::var(ENV_VAR)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(CloudProvider::Dev)
}

pub fn is_running_on_dev() -> bool {
    current_cloud_provider() == CloudProvider::Dev
}

pub fn is_running_on_aws() -> bool {
    current_cloud_provider() == CloudProvider::Aws
}

pub fn is_running_on_azure() -> bool {
    current_cloud_provider() == CloudProvider::Azure
}

pub fn is_running_on_gcp() -> bool {
    current_cloud_provider() == CloudProvider::Gcp
}

/// Skip the test unless the run targets Azure (or the `Dev` wildcard
/// applies). On `Aws` / `Gcp`, return early with a visible CI log line.
///
/// The `eprintln!` line is load-bearing: without it, a CI-skipped run
/// looks identical to a real pass.
#[macro_export]
macro_rules! require_running_on_azure {
    () => {
        if !($crate::common::cloud_gating::is_running_on_dev()
            || $crate::common::cloud_gating::is_running_on_azure())
        {
            eprintln!(
                "SKIPPED: {}={} (Azure-only test); unset {} for dev wildcard or set =azure to run",
                $crate::common::cloud_gating::ENV_VAR,
                std::env::var($crate::common::cloud_gating::ENV_VAR)
                    .unwrap_or_else(|_| "<unset>".into()),
                $crate::common::cloud_gating::ENV_VAR,
            );
            return;
        }
    };
}

/// Skip the test unless the run targets AWS (or the `Dev` wildcard applies).
#[macro_export]
macro_rules! require_running_on_aws {
    () => {
        if !($crate::common::cloud_gating::is_running_on_dev()
            || $crate::common::cloud_gating::is_running_on_aws())
        {
            eprintln!(
                "SKIPPED: {}={} (AWS-only test); unset {} for dev wildcard or set =aws to run",
                $crate::common::cloud_gating::ENV_VAR,
                std::env::var($crate::common::cloud_gating::ENV_VAR)
                    .unwrap_or_else(|_| "<unset>".into()),
                $crate::common::cloud_gating::ENV_VAR,
            );
            return;
        }
    };
}

/// Skip the test unless the run targets GCP (or the `Dev` wildcard applies).
#[macro_export]
macro_rules! require_running_on_gcp {
    () => {
        if !($crate::common::cloud_gating::is_running_on_dev()
            || $crate::common::cloud_gating::is_running_on_gcp())
        {
            eprintln!(
                "SKIPPED: {}={} (GCP-only test); unset {} for dev wildcard or set =gcp to run",
                $crate::common::cloud_gating::ENV_VAR,
                std::env::var($crate::common::cloud_gating::ENV_VAR)
                    .unwrap_or_else(|_| "<unset>".into()),
                $crate::common::cloud_gating::ENV_VAR,
            );
            return;
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    // `temp_env::with_var` serializes env mutations across the test
    // process via a global mutex, so these tests can run in parallel
    // without observing each other's mid-flight env state.

    #[test]
    fn unset_env_var_treats_as_dev() {
        temp_env::with_var(ENV_VAR, None::<&str>, || {
            assert!(is_running_on_dev());
            assert!(!is_running_on_aws());
            assert!(!is_running_on_azure());
            assert!(!is_running_on_gcp());
        });
    }

    #[test]
    fn explicit_dev_runs_everything() {
        temp_env::with_var(ENV_VAR, Some("dev"), || {
            assert!(is_running_on_dev());
            assert!(!is_running_on_aws());
            assert!(!is_running_on_azure());
            assert!(!is_running_on_gcp());
        });
    }

    #[test]
    fn aws_lane() {
        temp_env::with_var(ENV_VAR, Some("aws"), || {
            assert!(!is_running_on_dev());
            assert!(is_running_on_aws());
            assert!(!is_running_on_azure());
            assert!(!is_running_on_gcp());
        });
    }

    #[test]
    fn azure_lane() {
        temp_env::with_var(ENV_VAR, Some("azure"), || {
            assert!(!is_running_on_dev());
            assert!(!is_running_on_aws());
            assert!(is_running_on_azure());
            assert!(!is_running_on_gcp());
        });
    }

    #[test]
    fn gcp_lane() {
        temp_env::with_var(ENV_VAR, Some("gcp"), || {
            assert!(!is_running_on_dev());
            assert!(!is_running_on_aws());
            assert!(!is_running_on_azure());
            assert!(is_running_on_gcp());
        });
    }

    #[test]
    fn unknown_value_treats_as_dev() {
        temp_env::with_var(ENV_VAR, Some("something-else"), || {
            assert!(is_running_on_dev());
            assert!(!is_running_on_aws());
            assert!(!is_running_on_azure());
            assert!(!is_running_on_gcp());
        });
    }

    #[test]
    fn miscased_value_normalized() {
        temp_env::with_var(ENV_VAR, Some("Azure"), || {
            assert!(is_running_on_azure());
        });
        temp_env::with_var(ENV_VAR, Some("  AWS  "), || {
            assert!(is_running_on_aws());
        });
    }

    #[test]
    fn cloud_provider_display_round_trips_through_from_str() {
        for cloud in [
            CloudProvider::Aws,
            CloudProvider::Gcp,
            CloudProvider::Azure,
            CloudProvider::Dev,
        ] {
            assert_eq!(cloud.to_string().parse::<CloudProvider>().unwrap(), cloud);
        }
    }

    #[test]
    fn test_require_running_on_azure() {
        // The macro `return`s from its enclosing fn on skip, so we
        // wrap each invocation in a closure and observe whether the
        // post-macro sentinel was reached.
        fn would_pass() -> bool {
            let mut reached = false;
            (|| {
                require_running_on_azure!();
                reached = true;
            })();
            reached
        }

        // azure / dev / unset (dev-wildcard default) -> fall through
        temp_env::with_var(ENV_VAR, Some("azure"), || assert!(would_pass()));
        temp_env::with_var(ENV_VAR, Some("dev"), || assert!(would_pass()));
        temp_env::with_var(ENV_VAR, None::<&str>, || assert!(would_pass()));

        // aws / gcp -> skip
        temp_env::with_var(ENV_VAR, Some("aws"), || assert!(!would_pass()));
        temp_env::with_var(ENV_VAR, Some("gcp"), || assert!(!would_pass()));
    }

    #[test]
    fn test_require_running_on_aws() {
        fn would_pass() -> bool {
            let mut reached = false;
            (|| {
                require_running_on_aws!();
                reached = true;
            })();
            reached
        }

        // aws / dev / unset (dev-wildcard default) -> fall through
        temp_env::with_var(ENV_VAR, Some("aws"), || assert!(would_pass()));
        temp_env::with_var(ENV_VAR, Some("dev"), || assert!(would_pass()));
        temp_env::with_var(ENV_VAR, None::<&str>, || assert!(would_pass()));

        // azure / gcp -> skip
        temp_env::with_var(ENV_VAR, Some("azure"), || assert!(!would_pass()));
        temp_env::with_var(ENV_VAR, Some("gcp"), || assert!(!would_pass()));
    }

    #[test]
    fn test_require_running_on_gcp() {
        fn would_pass() -> bool {
            let mut reached = false;
            (|| {
                require_running_on_gcp!();
                reached = true;
            })();
            reached
        }

        // gcp / dev / unset (dev-wildcard default) -> fall through
        temp_env::with_var(ENV_VAR, Some("gcp"), || assert!(would_pass()));
        temp_env::with_var(ENV_VAR, Some("dev"), || assert!(would_pass()));
        temp_env::with_var(ENV_VAR, None::<&str>, || assert!(would_pass()));

        // aws / azure -> skip
        temp_env::with_var(ENV_VAR, Some("aws"), || assert!(!would_pass()));
        temp_env::with_var(ENV_VAR, Some("azure"), || assert!(!would_pass()));
    }
}
