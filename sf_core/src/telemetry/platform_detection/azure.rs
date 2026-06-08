use super::{DetectionConfig, env_non_empty};

pub(super) fn is_azure_function() -> bool {
    env_non_empty("FUNCTIONS_WORKER_RUNTIME")
        && env_non_empty("FUNCTIONS_EXTENSION_VERSION")
        && env_non_empty("AzureWebJobsStorage")
}

pub(super) async fn is_azure_vm(http: &reqwest::Client, config: &DetectionConfig) -> bool {
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

pub(super) async fn has_azure_managed_identity(
    http: &reqwest::Client,
    config: &DetectionConfig,
) -> bool {
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
