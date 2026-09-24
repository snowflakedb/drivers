use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use snafu::{Location, ResultExt, Snafu};
use tokio::sync::RwLock as AsyncRwLock;
use uuid::Uuid;

use crate::apis::database_driver_v1::{Setting, WrapperPresets};
use crate::apis::operation_ctx::CleanupScope;
use crate::config::rest_parameters::QueryParameters;
use crate::config::retry::RetryPolicy;
use crate::file_manager;
use crate::file_manager::upload_in_memory_file;
use crate::rest::snowflake::query_response::{Data, QueryResponseError, Response};
use crate::rest::snowflake::{QueryInput, QueryOptions, RestError, snowflake_query_with_client};
use crate::sensitive::SensitiveString;

pub const BIND_STAGE_NAME: &str = "SYSTEM$BIND";

const PYTHON_SNOWPARK_USE_SCOPED_TEMP_OBJECTS: &str = "PYTHON_SNOWPARK_USE_SCOPED_TEMP_OBJECTS";

/// Three-state lifecycle for the per-connection `SYSTEM$BIND` stage.
///
/// Encoding all states in a single value makes the illegal fourth state
/// (`stage_created = true` **and** `stage_binding_disabled = true` simultaneously)
/// unrepresentable by construction.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StageState {
    /// Initial state: no attempt has been made yet.
    Unknown = 0,
    /// `CREATE TEMPORARY STAGE … SYSTEM$BIND` succeeded; the stage is ready.
    Created = 1,
    /// Stage creation failed (e.g. missing `CREATE STAGE` privilege or no
    /// default database/schema). All subsequent CSV uploads on this connection
    /// must be skipped in favour of inline JSON bindings.
    Disabled = 2,
}

impl StageState {
    fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Created,
            2 => Self::Disabled,
            _ => Self::Unknown,
        }
    }
}

/// Atomically readable/writable `StageState` backed by an `AtomicU8`.
///
/// Wraps the raw integer so callers always work with the typed enum and
/// can never accidentally store an out-of-range value.
pub struct AtomicStageState(AtomicU8);

impl AtomicStageState {
    pub fn new(state: StageState) -> Self {
        Self(AtomicU8::new(state as u8))
    }

    pub fn load(&self, order: Ordering) -> StageState {
        StageState::from_u8(self.0.load(order))
    }

    pub fn store(&self, state: StageState, order: Ordering) {
        self.0.store(state as u8, order);
    }
}

const CREATE_SESSION_STAGE_SQL: &str = "CREATE TEMPORARY STAGE IF NOT EXISTS SYSTEM$BIND \
     file_format=(type=csv field_optionally_enclosed_by='\"' encoding='UTF8' escape_unenclosed_field=NONE)";
const CREATE_SCOPED_STAGE_SQL: &str = "CREATE OR REPLACE SCOPED TEMPORARY STAGE SYSTEM$BIND \
     file_format=(type=csv field_optionally_enclosed_by='\"' encoding='UTF8' escape_unenclosed_field=NONE)";

#[derive(Debug, Snafu, error_trace::ErrorTrace)]
#[snafu(visibility(pub(crate)))]
pub enum StageBindingError {
    #[snafu(display(
        "Stage binding is disabled on this connection (a previous CREATE STAGE \
         failed or the session lacks a default database/schema). The driver \
         must re-issue this query with inline JSON bindings."
    ))]
    Disabled {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to create the SYSTEM$BIND temporary stage"))]
    CreateStage {
        #[snafu(source(from(RestError, Box::new)))]
        source: Box<RestError>,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("PUT query to @SYSTEM$BIND failed"))]
    PutQuery {
        #[snafu(source(from(RestError, Box::new)))]
        source: Box<RestError>,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("PUT response is missing fields required for stage upload"))]
    MalformedPutResponse {
        #[snafu(source(from(QueryResponseError, Box::new)))]
        source: Box<QueryResponseError>,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to upload CSV bind data to the stage"))]
    Upload {
        #[snafu(source(from(file_manager::FileManagerError, Box::new)))]
        source: Box<file_manager::FileManagerError>,
        #[snafu(implicit)]
        location: Location,
    },
}

pub struct StageBindingContext<'a> {
    pub client: &'a reqwest::Client,
    pub query_parameters: &'a QueryParameters,
    pub session_token: &'a SensitiveString,
    pub retry_policy: &'a RetryPolicy,
    pub put_get_policy: &'a RetryPolicy,
    pub use_s3_regional_url_session_param: bool,
    /// When true the bind stage is created `SCOPED TEMPORARY`, so the server
    /// drops it at transaction end and every upload must recreate it.
    pub use_scoped_stage: bool,
    /// Driver-owned CRL worker for the storage TLS client, threaded through so
    /// the bind-stage upload honours CRL like the file-path PUT/GET does.
    pub crl_worker: crate::crl::worker::SharedCrlWorker,
    /// Where the upload registers cancellation cleanup. Matters only for a CSV
    /// payload at or above the multipart threshold, which has an abort to register;
    /// a single PUT is discarded by the cloud when the connection is torn down.
    pub cleanup: Option<&'a CleanupScope>,
    pub xp_backend: Option<std::sync::Arc<dyn crate::xp_backend::SnowflakeBackend>>,
}

impl<'a> StageBindingContext<'a> {
    #[allow(clippy::too_many_arguments)]
    pub async fn new(
        client: &'a reqwest::Client,
        query_parameters: &'a QueryParameters,
        session_token: &'a SensitiveString,
        retry_policy: &'a RetryPolicy,
        put_get_policy: &'a RetryPolicy,
        use_s3_regional_url_session_param: bool,
        session_parameters: &AsyncRwLock<HashMap<String, Setting>>,
        wrapper_presets: &WrapperPresets,
        crl_worker: crate::crl::worker::SharedCrlWorker,
        cleanup: Option<&'a CleanupScope>,
        xp_backend: Option<std::sync::Arc<dyn crate::xp_backend::SnowflakeBackend>>,
    ) -> Self {
        let use_scoped_stage =
            resolve_scoped_stage(wrapper_presets, &*session_parameters.read().await);
        Self {
            client,
            query_parameters,
            session_token,
            retry_policy,
            put_get_policy,
            use_s3_regional_url_session_param,
            use_scoped_stage,
            crl_worker,
            cleanup,
            xp_backend,
        }
    }
}

/// The bind stage is `SCOPED TEMPORARY` only when the wrapper opts in *and* the
/// session reports `PYTHON_SNOWPARK_USE_SCOPED_TEMP_OBJECTS`. Only Snowpark's
/// Python sessions set the parameter, and only the Python wrapper honours it.
fn resolve_scoped_stage(
    wrapper_presets: &WrapperPresets,
    session_parameters: &HashMap<String, Setting>,
) -> bool {
    wrapper_presets.honor_scoped_temp_bind_stage
        && session_parameters
            .get(PYTHON_SNOWPARK_USE_SCOPED_TEMP_OBJECTS)
            .and_then(Setting::coerce_bool)
            .unwrap_or(false)
}

#[derive(Clone)]
pub struct StageBindingFlags {
    pub stage_state: Arc<AtomicStageState>,
}

pub async fn upload_csv_bindings(
    stage_binding_ctx: &StageBindingContext<'_>,
    flags: &StageBindingFlags,
    request_id: Uuid,
    csv_bytes: &[u8],
) -> Result<String, StageBindingError> {
    if flags.stage_state.load(Ordering::Relaxed) == StageState::Disabled {
        return DisabledSnafu.fail();
    }

    ensure_stage(stage_binding_ctx, flags).await?;
    let put_response = issue_put_query(stage_binding_ctx, request_id).await?;
    upload_blob(stage_binding_ctx, csv_bytes, &put_response.data).await?;

    Ok(format!("@{BIND_STAGE_NAME}/{request_id}"))
}

async fn ensure_stage(
    stage_binding_ctx: &StageBindingContext<'_>,
    flags: &StageBindingFlags,
) -> Result<(), StageBindingError> {
    let scoped = stage_binding_ctx.use_scoped_stage;

    if !scoped && flags.stage_state.load(Ordering::Relaxed) == StageState::Created {
        return Ok(());
    }

    let query_input = QueryInput::new(if scoped {
        CREATE_SCOPED_STAGE_SQL
    } else {
        CREATE_SESSION_STAGE_SQL
    });

    let response = snowflake_query_with_client(
        stage_binding_ctx.client,
        stage_binding_ctx.query_parameters.clone(),
        stage_binding_ctx.session_token.reveal(),
        query_input,
        QueryOptions {
            retry_policy: stage_binding_ctx.retry_policy.clone(),
            ..Default::default()
        },
        stage_binding_ctx.xp_backend.as_deref(),
    )
    .await;

    match response {
        Ok(_) => {
            if !scoped {
                flags
                    .stage_state
                    .store(StageState::Created, Ordering::Relaxed);
            }
            Ok(())
        }
        Err(e) => {
            flags
                .stage_state
                .store(StageState::Disabled, Ordering::Relaxed);
            crate::telemetry::record_stage_binding_disabled();
            Err(e).context(CreateStageSnafu)
        }
    }
}

async fn issue_put_query(
    stage_binding_ctx: &StageBindingContext<'_>,
    request_id: Uuid,
) -> Result<Response, StageBindingError> {
    let put_sql = format!(
        "PUT 'file:///tmp/placeholder/0' '@{BIND_STAGE_NAME}/{request_id}' overwrite=true",
    );

    let query_input = QueryInput::new(&put_sql);

    snowflake_query_with_client(
        stage_binding_ctx.client,
        stage_binding_ctx.query_parameters.clone(),
        stage_binding_ctx.session_token.reveal(),
        query_input,
        QueryOptions {
            retry_policy: stage_binding_ctx.retry_policy.clone(),
            ..Default::default()
        },
        stage_binding_ctx.xp_backend.as_deref(),
    )
    .await
    .context(PutQuerySnafu)
}

async fn upload_blob(
    stage_binding_ctx: &StageBindingContext<'_>,
    csv_bytes: &[u8],
    data: &Data,
) -> Result<(), StageBindingError> {
    if stage_binding_ctx.xp_backend.is_some() {
        return Err(RestError::from(
            crate::xp_backend::BackendError::unsupported("upload_stream"),
        ))
        .context(PutQuerySnafu);
    }
    // This path builds `StageInfo` outside `perform_put_get_transfer`, so the
    // connection's TLS, proxy, and CRL settings are threaded here via
    // `StageTransport` — the same three the file-path path threads.
    let transport = file_manager::StageTransport {
        tls_config: stage_binding_ctx
            .query_parameters
            .client_info
            .tls_config
            .clone(),
        proxy_config: stage_binding_ctx
            .query_parameters
            .client_info
            .proxy_config
            .clone(),
        crl_worker: stage_binding_ctx.crl_worker.clone(),
    };
    let single = data
        .to_bind_stage_upload_data(
            stage_binding_ctx.use_s3_regional_url_session_param,
            &transport,
        )
        .context(MalformedPutResponseSnafu)?;

    // No `StageInfoRefresher` is needed here: CSV binding payloads are small
    // (a few KB at most) and upload in well under the storage-credential
    // expiry window (typically ≥15 minutes). A presigned-URL rotation mid-upload
    // is therefore not a realistic concern, unlike the large-file PUT/GET path
    // where files can run for minutes.
    //
    // Note: this path still uses the default TLS version window rather than the
    // connection's narrowed one (see
    // adr/tls_version_enforcement_implementation_notes.md, "Known gaps"); only
    // the proxy settings are threaded here.
    upload_in_memory_file(
        csv_bytes.to_vec(),
        single,
        stage_binding_ctx.put_get_policy,
        file_manager::TransferCtx::new(None, stage_binding_ctx.cleanup),
    )
    .await
    .context(UploadSnafu)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_stage_path_format_matches_legacy_drivers() {
        let id = Uuid::nil();
        let path = format!("@{BIND_STAGE_NAME}/{id}");
        assert_eq!(path, "@SYSTEM$BIND/00000000-0000-0000-0000-000000000000");
    }

    #[test]
    fn session_stage_sql_matches_legacy_format() {
        assert!(CREATE_SESSION_STAGE_SQL.contains("CREATE TEMPORARY STAGE"));
        assert!(CREATE_SESSION_STAGE_SQL.contains("SYSTEM$BIND"));
        assert!(CREATE_SESSION_STAGE_SQL.contains("type=csv"));
        assert!(CREATE_SESSION_STAGE_SQL.contains("field_optionally_enclosed_by='\"'"));
    }

    fn scoped_temp_params(enabled: bool) -> HashMap<String, Setting> {
        HashMap::from([(
            PYTHON_SNOWPARK_USE_SCOPED_TEMP_OBJECTS.to_string(),
            Setting::Bool(enabled),
        )])
    }

    #[test]
    fn scoped_stage_needs_both_wrapper_preset_and_session_parameter() {
        assert!(resolve_scoped_stage(
            &WrapperPresets::python(),
            &scoped_temp_params(true)
        ));
        assert!(!resolve_scoped_stage(
            &WrapperPresets::python(),
            &scoped_temp_params(false)
        ));
        assert!(!resolve_scoped_stage(
            &WrapperPresets::python(),
            &HashMap::new()
        ));
        assert!(!resolve_scoped_stage(
            &WrapperPresets::jdbc(),
            &scoped_temp_params(true)
        ));
    }

    #[test]
    fn scoped_stage_sql_matches_python_connector() {
        assert_eq!(
            CREATE_SCOPED_STAGE_SQL,
            "CREATE OR REPLACE SCOPED TEMPORARY STAGE SYSTEM$BIND \
             file_format=(type=csv field_optionally_enclosed_by='\"' encoding='UTF8' escape_unenclosed_field=NONE)"
        );
    }

    #[test]
    fn flags_bundle_clones_to_share_state() {
        let flags = StageBindingFlags {
            stage_state: Arc::new(AtomicStageState::new(StageState::Disabled)),
        };
        // State changes via one clone must be visible through any other clone
        // because the Arc is shared — that's why this struct uses
        // `Arc<AtomicStageState>` rather than a bare value.
        let cloned = flags.clone();
        assert_eq!(
            cloned.stage_state.load(Ordering::Relaxed),
            StageState::Disabled
        );
        flags
            .stage_state
            .store(StageState::Created, Ordering::Relaxed);
        assert_eq!(
            cloned.stage_state.load(Ordering::Relaxed),
            StageState::Created
        );
    }
}
