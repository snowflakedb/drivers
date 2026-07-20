use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use snafu::{Location, ResultExt, Snafu};
use uuid::Uuid;

use crate::config::rest_parameters::QueryParameters;
use crate::config::retry::RetryPolicy;
use crate::file_manager;
use crate::file_manager::upload_in_memory_file;
use crate::rest::snowflake::query_response::{Data, QueryResponseError, Response};
use crate::rest::snowflake::{
    QueryExecutionMode, QueryInput, RestError, snowflake_query_with_client,
};
use crate::sensitive::SensitiveString;

pub const BIND_STAGE_NAME: &str = "SYSTEM$BIND";

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

const CREATE_STAGE_SQL: &str = "CREATE TEMPORARY STAGE IF NOT EXISTS SYSTEM$BIND \
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

impl StageBindingError {
    pub(crate) fn is_cancelled(&self) -> bool {
        match self {
            StageBindingError::CreateStage { source, .. }
            | StageBindingError::PutQuery { source, .. } => source.is_cancelled(),
            StageBindingError::Upload { source, .. } => source.is_cancelled(),
            _ => false,
        }
    }
}

pub struct StageBindingContext<'a> {
    pub client: &'a reqwest::Client,
    pub query_parameters: &'a QueryParameters,
    pub session_token: &'a SensitiveString,
    pub retry_policy: &'a RetryPolicy,
    pub put_get_policy: &'a RetryPolicy,
    pub use_s3_regional_url_session_param: bool,
}

#[derive(Clone)]
pub struct StageBindingFlags {
    pub stage_state: Arc<AtomicStageState>,
}

pub async fn upload_csv_bindings(
    ctx: &StageBindingContext<'_>,
    flags: &StageBindingFlags,
    request_id: Uuid,
    csv_bytes: &[u8],
    cancel: tokio_util::sync::CancellationToken,
) -> Result<String, StageBindingError> {
    if flags.stage_state.load(Ordering::Relaxed) == StageState::Disabled {
        return DisabledSnafu.fail();
    }

    ensure_stage(ctx, flags, cancel.clone()).await?;
    let put_response = issue_put_query(ctx, request_id, cancel.clone()).await?;
    upload_blob(ctx, csv_bytes, &put_response.data, cancel).await?;

    Ok(format!("@{BIND_STAGE_NAME}/{request_id}"))
}

async fn ensure_stage(
    ctx: &StageBindingContext<'_>,
    flags: &StageBindingFlags,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<(), StageBindingError> {
    if flags.stage_state.load(Ordering::Relaxed) == StageState::Created {
        return Ok(());
    }

    let query_input = QueryInput {
        sql: CREATE_STAGE_SQL.to_string(),
        bindings: None,
        bind_stage: None,
        describe_only: None,
        query_parameters: None,
    };

    let response = snowflake_query_with_client(
        ctx.client,
        ctx.query_parameters.clone(),
        ctx.session_token.reveal(),
        query_input,
        ctx.retry_policy,
        QueryExecutionMode::Blocking,
        cancel,
    )
    .await;

    match response {
        Ok(_) => {
            flags
                .stage_state
                .store(StageState::Created, Ordering::Relaxed);
            Ok(())
        }
        Err(e) => {
            let next_state = if e.is_cancelled() {
                StageState::Unknown
            } else {
                StageState::Disabled
            };
            flags.stage_state.store(next_state, Ordering::Relaxed);
            Err(e).context(CreateStageSnafu)
        }
    }
}

async fn issue_put_query(
    ctx: &StageBindingContext<'_>,
    request_id: Uuid,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<Response, StageBindingError> {
    let put_sql = format!(
        "PUT 'file:///tmp/placeholder/0' '@{BIND_STAGE_NAME}/{request_id}' overwrite=true",
    );

    let query_input = QueryInput {
        sql: put_sql,
        bindings: None,
        bind_stage: None,
        describe_only: None,
        query_parameters: None,
    };

    snowflake_query_with_client(
        ctx.client,
        ctx.query_parameters.clone(),
        ctx.session_token.reveal(),
        query_input,
        ctx.retry_policy,
        QueryExecutionMode::Blocking,
        cancel,
    )
    .await
    .context(PutQuerySnafu)
}

async fn upload_blob(
    ctx: &StageBindingContext<'_>,
    csv_bytes: &[u8],
    data: &Data,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<(), StageBindingError> {
    let single = data
        .to_bind_stage_upload_data(ctx.use_s3_regional_url_session_param)
        .context(MalformedPutResponseSnafu)?;

    // No `StageInfoRefresher` is needed here: CSV binding payloads are small
    // (a few KB at most) and upload in well under the storage-credential
    // expiry window (typically ≥15 minutes). A presigned-URL rotation mid-upload
    // is therefore not a realistic concern, unlike the large-file PUT/GET path
    // where files can run for minutes.
    //
    // Note: this internal path builds `StageInfo` outside
    // `perform_put_get_transfer`, so the storage client uses the default TLS
    // version window rather than the connection's narrowed one (see
    // adr/tls_version_enforcement_implementation_notes.md, "Known gaps").
    upload_in_memory_file(
        csv_bytes.to_vec(),
        single,
        ctx.put_get_policy,
        &mut None,
        cancel,
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
    fn create_stage_sql_matches_legacy_format() {
        assert!(CREATE_STAGE_SQL.contains("CREATE TEMPORARY STAGE"));
        assert!(CREATE_STAGE_SQL.contains("SYSTEM$BIND"));
        assert!(CREATE_STAGE_SQL.contains("type=csv"));
        assert!(CREATE_STAGE_SQL.contains("field_optionally_enclosed_by='\"'"));
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
