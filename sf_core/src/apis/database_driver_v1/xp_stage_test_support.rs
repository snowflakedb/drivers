//! A driver whose connection serves stage I/O from an in-process XP backend,
//! for unit tests of the XP stage-transfer paths.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use super::connection::Connection;
use super::global_state::{DatabaseDriverV1, DriverProviders, WrapperPresets};
use crate::config::rest_parameters::{LoginParameters, QueryParameters};
use crate::handle_manager::Handle;
use crate::rest::snowflake::{LoginResult, QueryInput, query_response};
use crate::utils::sync::MutexRecoverExt;
use crate::xp_backend::{
    BackendCancelOutcome, BackendCancelTarget, BackendQueryOptions, BackendResult, SnowflakeBackend,
};

/// Answers `list_stage_files` with `listing` and `download_stream` with
/// `body`, recording each download source. With `hang_downloads`,
/// `download_stream` never returns.
pub(super) struct StageBackend {
    pub(super) listing: &'static str,
    pub(super) body: &'static [u8],
    pub(super) hang_downloads: bool,
    pub(super) download_sources: std::sync::Mutex<Vec<String>>,
}

impl StageBackend {
    pub(super) fn new(listing: &'static str, body: &'static [u8]) -> Self {
        Self {
            listing,
            body,
            hang_downloads: false,
            download_sources: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub(super) fn download_sources(&self) -> Vec<String> {
        self.download_sources.lock_recover().clone()
    }
}

#[async_trait]
impl SnowflakeBackend for StageBackend {
    async fn authenticate(
        &self,
        _params: &LoginParameters,
        _session_parameters: Option<&HashMap<String, String>>,
    ) -> BackendResult<LoginResult> {
        unimplemented!()
    }

    async fn execute_query(
        &self,
        _input: &QueryInput<'_>,
        _params: &QueryParameters,
        _options: BackendQueryOptions,
    ) -> BackendResult<query_response::Response> {
        unimplemented!()
    }

    async fn get_query_status(&self, _query_id: &str) -> BackendResult<String> {
        unimplemented!()
    }

    async fn get_query_result(&self, _query_id: &str) -> BackendResult<query_response::Response> {
        unimplemented!()
    }

    async fn cancel_query(
        &self,
        _target: BackendCancelTarget<'_>,
    ) -> BackendResult<BackendCancelOutcome> {
        unimplemented!()
    }

    async fn get_session_parameters(&self) -> BackendResult<HashMap<String, String>> {
        unimplemented!()
    }

    async fn download_stream(&self, source: &str, _metadata_json: &str) -> BackendResult<Vec<u8>> {
        self.download_sources
            .lock_recover()
            .push(source.to_string());
        if self.hang_downloads {
            std::future::pending::<()>().await;
        }
        Ok(self.body.to_vec())
    }

    async fn list_stage_files(
        &self,
        _stage_location: &str,
        _is_exact_match: bool,
    ) -> BackendResult<String> {
        Ok(self.listing.to_string())
    }
}

/// A driver with `presets` whose single connection is served by `backend`
/// and has the transport fields a query would need, without a login.
pub(super) fn xp_stage_driver(
    presets: WrapperPresets,
    backend: Arc<StageBackend>,
) -> (DatabaseDriverV1, Handle) {
    let driver = DatabaseDriverV1::with_providers(DriverProviders {
        wrapper_presets: presets,
        running_inside_xp: Some(true),
        xp_backend: Some(backend),
        ..Default::default()
    });
    let mut conn = Connection::new();
    conn.xp_slot = Arc::clone(&driver.xp_slot);
    conn.server_url = Some("https://snowflake.com".to_string());
    conn.client_info = Some(crate::config::rest_parameters::test_fixtures::test_client_info());
    conn.http_client = Some(reqwest::Client::new());
    let handle = driver.connections.add_handle(Mutex::new(conn));
    (driver, handle)
}
