use super::{DetectionConfig, env_non_empty};

const GCP_METADATA_FLAVOR_HEADER: &str = "Metadata-Flavor";
const GCP_METADATA_FLAVOR_VALUE: &str = "Google";

pub(super) fn is_gce_cloud_run_service() -> bool {
    env_non_empty("K_SERVICE") && env_non_empty("K_REVISION") && env_non_empty("K_CONFIGURATION")
}

pub(super) fn is_gce_cloud_run_job() -> bool {
    env_non_empty("CLOUD_RUN_JOB") && env_non_empty("CLOUD_RUN_EXECUTION")
}

pub(super) async fn is_gce_vm(http: &reqwest::Client, config: &DetectionConfig) -> bool {
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

pub(super) async fn has_gcp_identity(http: &reqwest::Client, config: &DetectionConfig) -> bool {
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
