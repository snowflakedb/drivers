use sf_core::file_manager::{CloudCredentials, LocationType, StageInfo};
use sf_core::sensitive::SensitiveString;

/// Helper to build an Azure StageInfo.
fn azure_stage() -> StageInfo {
    StageInfo {
        location_type: LocationType::Azure,
        bucket: "test-container".to_string(),
        key_prefix: "prefix/".to_string(),
        region: "eastus2".to_string(),
        creds: CloudCredentials::Azure {
            sas_token: SensitiveString::from("sv=2021-08-06&sig=test-secret-sig&se=2099-01-01"),
        },
        end_point: Some("blob.core.windows.net".to_string()),
        presigned_url: None,
        use_virtual_url: false,
        use_regional_url: false,
        storage_account: Some("nonexistentaccount".to_string()),
    }
}

// ---------------------------------------------------------------
// Transport errors do NOT leak SAS tokens
// (Azure URLs embed SAS tokens as query parameters; reqwest::Error
// includes the full URL, so we must sanitize before surfacing)
// ---------------------------------------------------------------

#[tokio::test]
async fn azure_download_transport_error_does_not_leak_sas_token() {
    let stage = azure_stage();
    let result = sf_core::file_manager::download_from_azure(&stage, "file.csv").await;

    let err = result.unwrap_err();
    let err_display = format!("{err}");
    let err_debug = format!("{err:?}");

    assert!(
        !err_display.contains("test-secret-sig"),
        "Display should not contain SAS signature, got: {err_display}"
    );
    assert!(
        !err_debug.contains("test-secret-sig"),
        "Debug should not contain SAS signature, got: {err_debug}"
    );
}

// ---------------------------------------------------------------
// Missing credentials produce a clear error
// ---------------------------------------------------------------

#[tokio::test]
async fn azure_download_with_wrong_creds_type_fails() {
    let mut stage = azure_stage();
    stage.creds = CloudCredentials::Gcs {
        gcs_access_token: None,
    };

    let result = sf_core::file_manager::download_from_azure(&stage, "file.csv").await;

    let err = result.unwrap_err();
    let err_str = format!("{err}");
    assert!(
        err_str.contains("Missing Azure credentials"),
        "Should report missing credentials, got: {err_str}"
    );
}

// ---------------------------------------------------------------
// Missing storage account produces a clear error
// ---------------------------------------------------------------

#[tokio::test]
async fn azure_download_with_missing_storage_account_fails() {
    let mut stage = azure_stage();
    stage.storage_account = None;

    let result = sf_core::file_manager::download_from_azure(&stage, "file.csv").await;

    let err = result.unwrap_err();
    let err_str = format!("{err}");
    assert!(
        err_str.contains("storage_account"),
        "Should report missing storage_account, got: {err_str}"
    );
}
