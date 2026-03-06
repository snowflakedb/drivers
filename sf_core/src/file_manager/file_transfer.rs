// Cloud-agnostic dispatch layer for file transfers
// This module routes upload/download operations to the appropriate cloud provider

use super::gcs_transfer;
use super::s3_transfer;
use super::types::{EncryptedFileMetadata, EncryptionResult, StageInfo, StageLocationType};
use snafu::{Location, ResultExt, Snafu};

/// Upload to cloud storage, skipping if file already exists and overwrite is false
pub async fn upload_to_cloud_or_skip(
    encryption_result: EncryptionResult,
    stage_info: &StageInfo,
    filename: &str,
    overwrite: bool,
) -> Result<String, UploadFileError> {
    match stage_info.location_type {
        StageLocationType::S3 => {
            s3_transfer::upload_to_s3_or_skip(encryption_result, stage_info, filename, overwrite)
                .await
                .context(UploadS3Snafu)
        }
        StageLocationType::Gcs => {
            gcs_transfer::upload_to_gcs_or_skip(encryption_result, stage_info, filename, overwrite)
                .await
                .context(UploadGcsSnafu)
        }
        StageLocationType::Azure => UploadUnsupportedSnafu {
            provider: "Azure".to_string(),
        }
        .fail(),
    }
}

/// Download from cloud storage
pub async fn download_from_cloud(
    stage_info: &StageInfo,
    filename: &str,
) -> Result<(Vec<u8>, EncryptedFileMetadata), DownloadFileError> {
    match stage_info.location_type {
        StageLocationType::S3 => s3_transfer::download_from_s3(stage_info, filename)
            .await
            .context(DownloadS3Snafu),
        StageLocationType::Gcs => gcs_transfer::download_from_gcs(stage_info, filename)
            .await
            .context(DownloadGcsSnafu),
        StageLocationType::Azure => DownloadUnsupportedSnafu {
            provider: "Azure".to_string(),
        }
        .fail(),
    }
}

#[derive(Snafu, Debug, error_trace::ErrorTrace)]
pub enum UploadFileError {
    #[snafu(display("S3 upload error"))]
    UploadS3 {
        source: s3_transfer::UploadFileError,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("GCS upload error"))]
    UploadGcs {
        source: gcs_transfer::GcsTransferError,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Unsupported cloud provider for upload: {provider}"))]
    UploadUnsupported {
        provider: String,
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Snafu, Debug, error_trace::ErrorTrace)]
pub enum DownloadFileError {
    #[snafu(display("S3 download error"))]
    DownloadS3 {
        source: s3_transfer::DownloadFileError,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("GCS download error"))]
    DownloadGcs {
        source: gcs_transfer::GcsTransferError,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Unsupported cloud provider for download: {provider}"))]
    DownloadUnsupported {
        provider: String,
        #[snafu(implicit)]
        location: Location,
    },
}
