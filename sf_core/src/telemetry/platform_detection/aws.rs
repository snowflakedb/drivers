use aws_sdk_sts::config::BehaviorVersion;
use futures::FutureExt;
use futures::future::BoxFuture;

use super::{DetectionConfig, env_non_empty};

/// Abstraction over the STS `GetCallerIdentity` lookup used by `has_aws_identity`,
/// so tests can inject a canned ARN, `None`, or a delay instead of hitting live
/// AWS. See [`tests::FakeCallerIdentityProvider`] for the test double.
pub(crate) trait CallerIdentityProvider: Send + Sync {
    fn caller_identity_arn(&self) -> BoxFuture<'_, Option<String>>;
}

pub(crate) struct StsCallerIdentityProvider;

impl CallerIdentityProvider for StsCallerIdentityProvider {
    fn caller_identity_arn(&self) -> BoxFuture<'_, Option<String>> {
        async move {
            let config = aws_config::defaults(BehaviorVersion::latest()).load().await;
            let client = aws_sdk_sts::Client::new(&config);
            match client.get_caller_identity().send().await {
                Ok(response) => response.arn,
                Err(err) => {
                    tracing::debug!(
                        error = ?err,
                        "STS GetCallerIdentity failed; treating has_aws_identity as false",
                    );
                    None
                }
            }
        }
        .boxed()
    }
}

pub(super) fn is_aws_lambda() -> bool {
    env_non_empty("LAMBDA_TASK_ROOT")
}

pub(super) async fn is_ec2_instance(http: &reqwest::Client, config: &DetectionConfig) -> bool {
    let token = async {
        let response = http
            .put(format!("{}/latest/api/token", config.aws_metadata_base_url))
            .header("X-aws-ec2-metadata-token-ttl-seconds", "21600")
            .send()
            .await
            .ok()?
            .error_for_status()
            .ok()?;
        let body = response.text().await.ok()?;
        Some(body.trim().to_string()).filter(|t| !t.is_empty())
    }
    .await;

    let document: reqwest::Result<serde_json::Value> = async {
        let mut request = http.get(format!(
            "{}/latest/dynamic/instance-identity/document",
            config.aws_metadata_base_url,
        ));
        if let Some(token) = token {
            request = request.header("X-aws-ec2-metadata-token", token);
        }
        request.send().await?.error_for_status()?.json().await
    }
    .await;

    document
        .ok()
        .and_then(|doc| doc.get("instanceId")?.as_str().map(str::to_owned))
        .is_some_and(|id| !id.is_empty())
}

pub(super) async fn has_aws_identity(provider: &dyn CallerIdentityProvider) -> bool {
    provider
        .caller_identity_arn()
        .await
        .as_deref()
        .map(is_valid_arn_for_wif)
        .unwrap_or(false)
}

/// Only IAM users and STS assumed roles qualify as valid workload-identity federation ARNs.
///
/// Expected layout is `arn:<partition>:<service>::<account>:<resource>`:
/// - `<partition>` must be non-empty (e.g. `aws`, `aws-cn`, `aws-us-gov`),
/// - `<service>` must be exactly `iam` or `sts`,
/// - the region segment must be empty (IAM/STS ARNs are global),
/// - `<account>` must be non-empty,
/// - `<resource>` must be `user/<name>` (for `iam`) or `assumed-role/<...>` (for `sts`).
fn is_valid_arn_for_wif(arn: &str) -> bool {
    let parts: Vec<&str> = arn.splitn(6, ':').collect();
    if parts.len() != 6
        || parts[0] != "arn"
        || parts[1].is_empty()
        || !parts[3].is_empty()
        || parts[4].is_empty()
    {
        return false;
    }
    match parts[2] {
        "iam" => parts[5]
            .strip_prefix("user/")
            .is_some_and(|rest| !rest.is_empty()),
        "sts" => parts[5]
            .strip_prefix("assumed-role/")
            .is_some_and(|rest| !rest.is_empty()),
        _ => false,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::time::Duration;

    use super::*;

    pub(crate) struct FakeCallerIdentityProvider {
        arn: Option<String>,
        pub delay: Duration,
    }

    impl FakeCallerIdentityProvider {
        pub(crate) fn new(arn: Option<String>) -> Self {
            Self {
                arn,
                delay: Duration::ZERO,
            }
        }
    }

    impl CallerIdentityProvider for FakeCallerIdentityProvider {
        fn caller_identity_arn(&self) -> BoxFuture<'_, Option<String>> {
            let delay = self.delay;
            let arn = self.arn.clone();
            Box::pin(async move {
                if !delay.is_zero() {
                    tokio::time::sleep(delay).await;
                }
                arn
            })
        }
    }

    #[test]
    fn accepts_iam_user_and_assumed_role() {
        assert!(is_valid_arn_for_wif("arn:aws:iam::123456789012:user/alice"));
        assert!(is_valid_arn_for_wif(
            "arn:aws:sts::123456789012:assumed-role/my-role/session-name"
        ));
        assert!(is_valid_arn_for_wif("arn:aws-cn:iam::1:user/u"));
    }

    #[test]
    fn rejects_malformed_arns() {
        let cases = [
            (
                "role/ instead of user/ under iam",
                "arn:aws:iam::123456789012:role/my-role",
            ),
            ("service other than iam/sts", "arn:aws:s3:::my-bucket/key"),
            ("missing arn: prefix", "aws:iam::123456789012:user/alice"),
            ("too few segments", "arn:aws:iam"),
            ("empty account id", "arn:aws:iam:::user/alice"),
            ("empty partition", "arn::iam::123456789012:user/alice"),
            (
                "non-empty region (IAM ARNs are global)",
                "arn:aws:iam:us-east-1:123456789012:user/alice",
            ),
            ("user/ with no suffix", "arn:aws:iam::123456789012:user/"),
            (
                "assumed-role/ with no suffix",
                "arn:aws:sts::123456789012:assumed-role/",
            ),
            ("random string", "not-an-arn"),
            ("empty", ""),
        ];
        for (reason, arn) in cases {
            assert!(
                !is_valid_arn_for_wif(arn),
                "{reason}: {arn:?} should be rejected",
            );
        }
    }
}
