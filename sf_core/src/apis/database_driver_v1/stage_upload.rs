//! Stage uploads that take a local file or byte stream plus a stage location and
//! PUT options, instead of PUT SQL.
//!
//! With an XP backend registered the upload runs straight against the backend:
//! stage names and options are interpreted as the 4.x stored-procedure connector
//! did, and no query is submitted. Without one, the arguments are rendered into
//! a single PUT statement that takes the normal GS path, with the stage location
//! and file name passed through to GS as the caller wrote them.

use std::collections::BTreeMap;
use std::sync::Arc;

use snafu::{OptionExt, ResultExt};
use tokio::sync::Mutex;
use tracing::Instrument;

use super::connection::Connection;
use super::error::*;
use super::global_state::DatabaseDriverV1;
use super::query::{upload_files_via_xp_backend, upload_stream_via_xp_backend};
use super::result_set::{
    ExecuteQueryResult, ResultSetInfo, resolve_reader_ctx, response_to_descriptor,
};
use super::stream_transfer::UploadStreamTarget;
use crate::apis::operation_ctx::OperationCtx;
use crate::config::retry::RetryPolicy;
use crate::file_manager::{self, ByteSource, MultipartParams, SourceCompressionParam, StageInfo};
use crate::handle_manager::Handle;
use crate::rest::snowflake::query_response::{self, RowsetData};
use crate::xp_backend::SnowflakeBackend;

const SNOWFLAKE_NESTED_STAGE_PREFIX: &str = "snow://";
const FILE_URI_PREFIX: &str = "file://";
const MAX_PARALLEL: i64 = 99;

/// PUT options by name, with values written as in PUT SQL.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StageUploadOptions(BTreeMap<String, String>);

impl StageUploadOptions {
    pub fn new(options: impl IntoIterator<Item = (String, String)>) -> Self {
        Self(options.into_iter().collect())
    }

    fn render(&self) -> String {
        self.0
            .iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Reads the options an XP upload honors. Other names are ignored.
    fn resolve_for_xp(&self) -> Result<XpPutOptions, ApiError> {
        let mut resolved = XpPutOptions::default();
        for (name, value) in &self.0 {
            match name.to_ascii_uppercase().as_str() {
                "AUTO_COMPRESS" => resolved.auto_compress = parse_bool(name, value)?,
                "OVERWRITE" => resolved.overwrite = parse_bool(name, value)?,
                "SOURCE_COMPRESSION" => {
                    resolved.source_compression = SourceCompressionParam::from_name(value)
                        .with_context(|| InvalidArgumentSnafu {
                            argument: format!("Unknown SOURCE_COMPRESSION value: {value}"),
                        })?;
                }
                "PARALLEL" => validate_parallel(value)?,
                _ => {}
            }
        }
        Ok(resolved)
    }
}

fn validate_parallel(value: &str) -> Result<(), ApiError> {
    match value.trim().parse::<i64>() {
        Ok(parallel) if (1..=MAX_PARALLEL).contains(&parallel) => Ok(()),
        _ => InvalidArgumentSnafu {
            argument: format!("PARALLEL must be between 1 and {MAX_PARALLEL}: {value}"),
        }
        .fail(),
    }
}

fn parse_bool(name: &str, value: &str) -> Result<bool, ApiError> {
    match value.trim().to_ascii_uppercase().as_str() {
        "TRUE" => Ok(true),
        "FALSE" => Ok(false),
        _ => InvalidArgumentSnafu {
            argument: format!("{name} must be TRUE or FALSE: {value}"),
        }
        .fail(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct XpPutOptions {
    auto_compress: bool,
    overwrite: bool,
    source_compression: SourceCompressionParam,
}

impl Default for XpPutOptions {
    fn default() -> Self {
        Self {
            auto_compress: true,
            overwrite: false,
            source_compression: SourceCompressionParam::AutoDetect,
        }
    }
}

/// An XP chunked stage upload, resolved when the session opens and run at
/// `finish`.
#[derive(Clone)]
pub(super) struct XpStreamUpload {
    backend: Arc<dyn SnowflakeBackend>,
    destination: StreamDestination,
    options: XpPutOptions,
}

/// How a stage location is spelled into the upload's destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StageNaming {
    /// As the 4.x stored-procedure connector read it: `'`-unquoted with `\'`
    /// unescaped, and `@`-prefixed unless it starts with `@` or `snow://`.
    Xp,
    /// As the stage operand of the PUT sent to GS: one pair of `'` stripped,
    /// and `@`-prefixed unless it starts with `@`, `snow://` or `/`.
    Gs,
}

/// A stage directory and the file name written into it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct StreamDestination {
    stage_dir: String,
    filename: String,
}

/// Parses the destination of a chunked upload, which names a file. Both
/// naming rules then split at the last `/` and require a non-empty directory
/// and file name.
fn parse_stream_destination(
    stage_location: &str,
    naming: StageNaming,
) -> Result<StreamDestination, ApiError> {
    let stage = match naming {
        StageNaming::Xp => normalize_stage_location(stage_location),
        StageNaming::Gs => gs_stage_ref(stage_location),
    };
    match stage.rsplit_once('/') {
        Some((dir, file)) if !dir.is_empty() && !file.is_empty() => Ok(StreamDestination {
            stage_dir: dir.to_string(),
            filename: file.to_string(),
        }),
        _ => InvalidArgumentSnafu {
            argument: format!(
                "Upload stream destination must name a stage directory and a file: \
                 {stage_location}"
            ),
        }
        .fail(),
    }
}

fn is_single_quoted(name: &str) -> bool {
    name.len() >= 2 && name.starts_with('\'') && name.ends_with('\'')
}

/// Strips one level of single quotes and turns `\'` back into `'`.
fn unquote(name: &str) -> String {
    let name = name.trim();
    if is_single_quoted(name) {
        name[1..name.len() - 1].replace("\\'", "'")
    } else {
        name.to_string()
    }
}

/// Unquotes a stage location and adds the `@` prefix unless it is already a
/// stage name or a `snow://` URL.
fn normalize_stage_location(name: &str) -> String {
    let unquoted = unquote(name);
    if unquoted.starts_with('@') || unquoted.starts_with(SNOWFLAKE_NESTED_STAGE_PREFIX) {
        unquoted
    } else {
        format!("@{unquoted}")
    }
}

fn local_path(name: &str) -> String {
    let unquoted = unquote(name);
    unquoted
        .strip_prefix(FILE_URI_PREFIX)
        .map(str::to_string)
        .unwrap_or(unquoted)
}

fn gs_stage_ref(stage_location: &str) -> String {
    let stage = stage_location.trim();
    let stage = if is_single_quoted(stage) {
        &stage[1..stage.len() - 1]
    } else {
        stage
    };
    if stage.starts_with('@')
        || stage.starts_with(SNOWFLAKE_NESTED_STAGE_PREFIX)
        || stage.starts_with('/')
    {
        stage.to_string()
    } else {
        format!("@{stage}")
    }
}

// PUT takes no bind parameters for its file and stage operands, so they and the
// options are written into the statement. The local path and stage location go
// in as the caller wrote them, quoting included, and option names and values
// are not checked: the 4.x connector wrote options and the local path the same
// way, and GS parses the statement.
fn render_put_sql(
    local_file_name: &str,
    stage_location: &str,
    options: &StageUploadOptions,
) -> String {
    let local = local_file_name.trim();
    let local = if local.starts_with('\'') || local.starts_with(FILE_URI_PREFIX) {
        local.to_string()
    } else {
        format!("{FILE_URI_PREFIX}{local}")
    };
    let stage = stage_location.trim();
    let stage = if stage.starts_with('\'')
        || stage.starts_with('@')
        || stage.starts_with(SNOWFLAKE_NESTED_STAGE_PREFIX)
        || stage.starts_with('/')
    {
        stage.to_string()
    } else {
        format!("@{stage}")
    };
    format!("PUT {local} {stage} {}", options.render())
        .trim_end()
        .to_string()
}

fn render_stream_put_sql(destination: &StreamDestination, options: &StageUploadOptions) -> String {
    format!(
        "PUT {FILE_URI_PREFIX}{} {} {}",
        destination.filename,
        destination.stage_dir,
        options.render()
    )
    .trim_end()
    .to_string()
}

/// Settings every XP upload reads from the connection.
struct XpTransferSettings {
    retry_policy: RetryPolicy,
    put_compress_level: u32,
    put_tempdir: Option<std::path::PathBuf>,
    transport: file_manager::StageTransport,
}

impl DatabaseDriverV1 {
    async fn xp_transfer_settings(&self, conn: &Arc<Mutex<Connection>>) -> XpTransferSettings {
        let conn = conn.lock().await;
        let transfer = self.governor.register_command(1);
        XpTransferSettings {
            retry_policy: RetryPolicy::put_get(&conn.effective_settings()),
            put_compress_level: conn
                .put_compress_level(self.wrapper_presets.put_compress_level_default),
            put_tempdir: conn.put_tempdir(),
            transport: conn.stage_transport(&self.crl_manager, &self.root_store_provider, transfer),
        }
    }

    async fn xp_backend_for(
        &self,
        conn: &Arc<Mutex<Connection>>,
    ) -> Result<Option<Arc<dyn SnowflakeBackend>>, ApiError> {
        conn.lock().await.xp_backend_arc().context(QuerySnafu)
    }

    async fn xp_upload_result(
        &self,
        conn: &Arc<Mutex<Connection>>,
        rowset_data: RowsetData,
    ) -> Result<ResultSetInfo, ApiError> {
        let descriptor = response_to_descriptor(
            &query_response::Data::command_only("UPLOAD"),
            &self.wrapper_presets,
        );
        let reader_ctx = resolve_reader_ctx(conn).await?;
        let handle = self.create_result_set(descriptor.clone(), rowset_data, reader_ctx);
        Ok(ResultSetInfo { handle, descriptor })
    }

    /// Uploads the local file or glob `local_file_name` to `stage_location`.
    pub async fn connection_stage_upload(
        &self,
        operation_ctx: Option<&OperationCtx>,
        conn_handle: Handle,
        local_file_name: String,
        stage_location: String,
        options: StageUploadOptions,
    ) -> Result<ResultSetInfo, ApiError> {
        let telemetry_id = self.telemetry_id_for_conn(conn_handle).await;
        async {
            let conn = self
                .connections
                .get_obj(conn_handle)
                .context(InvalidArgumentSnafu {
                    argument: "Connection handle not found",
                })?;
            let Some(backend) = self.xp_backend_for(&conn).await? else {
                let sql = render_put_sql(&local_file_name, &stage_location, &options);
                return self
                    .stage_upload_via_statement(operation_ctx, conn_handle, sql)
                    .await;
            };

            let _session_guard = self.lock_session_if_needed(&conn).await;
            let xp_options = options.resolve_for_xp()?;
            let settings = self.xp_transfer_settings(&conn).await;
            let upload_data = file_manager::UploadData {
                src_location_pattern: local_path(&local_file_name),
                stage_info: StageInfo::for_xp_backend(
                    &normalize_stage_location(&stage_location),
                    &settings.transport,
                ),
                encryption_material: None,
                auto_compress: xp_options.auto_compress,
                source_compression: xp_options.source_compression,
                overwrite: xp_options.overwrite,
                flavor: self.wrapper_presets.put_get_resultset_flavor.clone(),
                legacy_odbc_compression_autodetect: self
                    .wrapper_presets
                    .legacy_odbc_compression_autodetect,
                skip_upload_on_content_match: false,
                multipart: MultipartParams::from_server(None, Some(1)),
                put_fastfail: true,
                put_compress_level: settings.put_compress_level,
                put_tempdir: settings.put_tempdir,
                cwd: None,
                transfer: settings.transport.transfer.clone(),
            };
            let rowset_data = upload_files_via_xp_backend(
                &upload_data,
                &settings.retry_policy,
                backend.as_ref(),
                operation_ctx,
            )
            .await
            .context(QueryResponseProcessSnafu)?;
            self.xp_upload_result(&conn, rowset_data).await
        }
        .instrument(crate::snowflake_op_span!(
            "connection_stage_upload",
            telemetry_id
        ))
        .await
    }

    /// Resolves the destination of a chunked stage upload when its session
    /// opens. Without an XP backend the upload becomes the PUT sent to GS at
    /// `finish`.
    pub(super) async fn stage_upload_stream_target(
        &self,
        conn: &Arc<Mutex<Connection>>,
        stage_location: &str,
        options: StageUploadOptions,
    ) -> Result<UploadStreamTarget, ApiError> {
        let Some(backend) = self.xp_backend_for(conn).await? else {
            let destination = parse_stream_destination(stage_location, StageNaming::Gs)?;
            return Ok(UploadStreamTarget::Sql(render_stream_put_sql(
                &destination,
                &options,
            )));
        };
        Ok(UploadStreamTarget::Xp(XpStreamUpload {
            backend,
            destination: parse_stream_destination(stage_location, StageNaming::Xp)?,
            options: options.resolve_for_xp()?,
        }))
    }

    /// Runs an XP chunked stage upload whose bytes are in `source`.
    pub(super) async fn run_xp_stream_upload(
        &self,
        conn_handle: Handle,
        upload: XpStreamUpload,
        source: ByteSource,
    ) -> Result<ResultSetInfo, ApiError> {
        let conn = self
            .connections
            .get_obj(conn_handle)
            .context(InvalidArgumentSnafu {
                argument: "Connection handle not found",
            })?;
        let _session_guard = self.lock_session_if_needed(&conn).await;
        let settings = self.xp_transfer_settings(&conn).await;
        let single = file_manager::SingleUploadData {
            source: ByteSource::Bytes(bytes::Bytes::new()),
            filename: upload.destination.filename,
            stage_info: StageInfo::for_xp_backend(
                &upload.destination.stage_dir,
                &settings.transport,
            ),
            encryption_material: None,
            auto_compress: upload.options.auto_compress,
            source_compression: upload.options.source_compression,
            overwrite: upload.options.overwrite,
            flavor: self.wrapper_presets.put_get_resultset_flavor.clone(),
            legacy_odbc_compression_autodetect: self
                .wrapper_presets
                .legacy_odbc_compression_autodetect,
            skip_upload_on_content_match: false,
            multipart: MultipartParams::from_server(None, Some(1)),
            put_compress_level: settings.put_compress_level,
            put_tempdir: settings.put_tempdir,
            transfer: settings.transport.transfer.clone(),
        };
        let rowset_data = upload_stream_via_xp_backend(
            source,
            single,
            &settings.retry_policy,
            upload.backend.as_ref(),
        )
        .await
        .context(QueryResponseProcessSnafu)?;
        self.xp_upload_result(&conn, rowset_data).await
    }

    /// Runs `sql` through a short-lived statement and returns its result set.
    async fn stage_upload_via_statement(
        &self,
        operation_ctx: Option<&OperationCtx>,
        conn_handle: Handle,
        sql: String,
    ) -> Result<ResultSetInfo, ApiError> {
        let stmt = self.statement_new(conn_handle)?;
        let result = async {
            self.statement_set_sql_query(stmt, sql).await?;
            self.statement_execute_query(operation_ctx, stmt, None, None, None, None)
                .await
        }
        .await;
        let _ = self.statement_release(stmt);
        match result? {
            ExecuteQueryResult::Single { info, .. } => Ok(info),
            ExecuteQueryResult::Multi { .. } => InvalidArgumentSnafu {
                argument: "A stage upload returned multiple result sets",
            }
            .fail(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(pairs: &[(&str, &str)]) -> StageUploadOptions {
        StageUploadOptions::new(pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())))
    }

    #[test]
    fn unquote_strips_quotes_and_unescapes_single_quotes() {
        assert_eq!(unquote("'@st/it\\'s'"), "@st/it's");
        assert_eq!(unquote("  @st  "), "@st");
        assert_eq!(unquote("'"), "'");
    }

    #[test]
    fn stage_location_gets_an_at_prefix_unless_it_has_one() {
        assert_eq!(normalize_stage_location("mystage/dir"), "@mystage/dir");
        assert_eq!(normalize_stage_location("'@mystage/dir'"), "@mystage/dir");
        assert_eq!(normalize_stage_location("snow://x/y"), "snow://x/y");
        assert_eq!(
            normalize_stage_location("@\"my stage\"/dir"),
            "@\"my stage\"/dir"
        );
    }

    #[test]
    fn local_path_drops_quotes_and_file_scheme() {
        assert_eq!(local_path("'file:///tmp/my file.csv'"), "/tmp/my file.csv");
        assert_eq!(local_path("file:///tmp/it\\'s.csv"), "/tmp/it\\'s.csv");
        assert_eq!(local_path("'file:///tmp/it\\'s.csv'"), "/tmp/it's.csv");
        assert_eq!(local_path("/tmp/x.csv"), "/tmp/x.csv");
    }

    fn destination(stage_dir: &str, filename: &str) -> StreamDestination {
        StreamDestination {
            stage_dir: stage_dir.to_string(),
            filename: filename.to_string(),
        }
    }

    #[test]
    fn stream_destination_splits_at_the_last_slash_in_both_namings() {
        for naming in [StageNaming::Xp, StageNaming::Gs] {
            assert_eq!(
                parse_stream_destination("'st/dir/name.csv'", naming).unwrap(),
                destination("@st/dir", "name.csv"),
                "{naming:?}"
            );
            for invalid in ["@st/dir/", "name.csv", "@name.csv"] {
                assert!(
                    parse_stream_destination(invalid, naming).is_err(),
                    "{naming:?} must reject {invalid:?}"
                );
            }
        }
    }

    #[test]
    fn stream_destination_naming_differs_only_where_the_two_rules_do() {
        assert_eq!(
            parse_stream_destination("/dir/name.csv", StageNaming::Gs).unwrap(),
            destination("/dir", "name.csv")
        );
        assert_eq!(
            parse_stream_destination("/dir/name.csv", StageNaming::Xp).unwrap(),
            destination("@/dir", "name.csv")
        );
        assert!(parse_stream_destination("/name.csv", StageNaming::Gs).is_err());
        assert_eq!(
            parse_stream_destination("/name.csv", StageNaming::Xp).unwrap(),
            destination("@", "name.csv")
        );
        assert_eq!(
            parse_stream_destination("'@st/it\\'s.csv'", StageNaming::Xp).unwrap(),
            destination("@st", "it's.csv")
        );
        assert_eq!(
            parse_stream_destination("'@st/it\\'s.csv'", StageNaming::Gs).unwrap(),
            destination("@st", "it\\'s.csv")
        );
    }

    #[test]
    fn xp_options_default_to_put_defaults() {
        assert_eq!(
            options(&[]).resolve_for_xp().unwrap(),
            XpPutOptions::default()
        );
    }

    #[test]
    fn xp_options_are_case_insensitive_and_ignore_unknown_names() {
        let resolved = options(&[
            ("auto_compress", "False"),
            ("OVERWRITE", "true"),
            ("source_compression", "gzip"),
            ("NOT_A_PUT_OPTION", "whatever"),
        ])
        .resolve_for_xp()
        .unwrap();
        assert_eq!(
            resolved,
            XpPutOptions {
                auto_compress: false,
                overwrite: true,
                source_compression: SourceCompressionParam::Gzip,
            }
        );
    }

    #[test]
    fn xp_options_reject_malformed_values() {
        for (name, value) in [
            ("OVERWRITE", "maybe"),
            ("AUTO_COMPRESS", "1"),
            ("SOURCE_COMPRESSION", "zip"),
            ("PARALLEL", "0"),
            ("PARALLEL", "100"),
            ("PARALLEL", "four"),
        ] {
            assert!(
                options(&[(name, value)]).resolve_for_xp().is_err(),
                "{name}={value} must be rejected"
            );
        }
        assert!(options(&[("PARALLEL", "99")]).resolve_for_xp().is_ok());
    }

    #[test]
    fn put_sql_passes_caller_quoting_through() {
        let sql = render_put_sql(
            "'file:///tmp/it\\'s.csv'",
            "mystage/dir",
            &options(&[("OVERWRITE", "TRUE"), ("AUTO_COMPRESS", "FALSE")]),
        );
        assert_eq!(
            sql,
            "PUT 'file:///tmp/it\\'s.csv' @mystage/dir AUTO_COMPRESS=FALSE OVERWRITE=TRUE"
        );
        assert_eq!(
            render_put_sql("/tmp/a.csv", "'@st'", &options(&[])),
            "PUT file:///tmp/a.csv '@st'"
        );
    }

    #[test]
    fn stream_put_sql_names_the_file_and_stage_directory() {
        assert_eq!(
            render_stream_put_sql(
                &destination("@st/dir", "name.csv"),
                &options(&[("PARALLEL", "2")])
            ),
            "PUT file://name.csv @st/dir PARALLEL=2"
        );
    }
}
