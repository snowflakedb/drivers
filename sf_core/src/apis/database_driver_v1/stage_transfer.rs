//! Stage uploads and downloads that take local paths or a byte stream plus a
//! stage location and PUT/GET options, instead of PUT/GET SQL.
//!
//! With an XP backend registered the transfer runs straight against the backend:
//! stage names and options are interpreted as the 4.x stored-procedure connector
//! did, and no query is submitted. Without one, the arguments are rendered into
//! a single PUT or GET statement that takes the normal GS path, with the stage
//! location and local paths passed through to GS as the caller wrote them.

use std::collections::BTreeMap;
use std::sync::Arc;

use snafu::{OptionExt, ResultExt};
use tokio::sync::Mutex;
use tracing::Instrument;

use super::connection::Connection;
use super::error::*;
use super::global_state::DatabaseDriverV1;
use super::query::{
    download_files_via_xp_backend, list_stage_files_via_xp_backend, remote_file_not_found,
    upload_files_via_xp_backend, upload_stream_via_xp_backend,
};
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

/// PUT or GET options by name, with values written as in PUT/GET SQL.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StageTransferOptions(BTreeMap<String, String>);

impl StageTransferOptions {
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
    fn resolve_for_xp_put(&self) -> Result<XpPutOptions, ApiError> {
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

    /// Checks the options an XP download honors. A non-empty `PATTERN` is not
    /// supported, and other names are ignored.
    fn check_for_xp_get(&self) -> Result<(), ApiError> {
        for (name, value) in &self.0 {
            match name.to_ascii_uppercase().as_str() {
                "PARALLEL" => validate_parallel(value)?,
                "PATTERN" if !value.trim().is_empty() => {
                    return NotImplementedSnafu {
                        feature: "GET with PATTERN in stored procedures",
                    }
                    .fail();
                }
                _ => {}
            }
        }
        Ok(())
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

/// The stage file an XP download stream reads: `source_filename` inside the
/// stage location `stage_name`, named as an XP GET names it.
pub(super) fn xp_download_stream_source(stage_name: &str, source_filename: &str) -> String {
    let stage = normalize_stage_location(stage_name);
    format!("{}/{source_filename}", stage.trim_end_matches('/'))
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

// PUT and GET take no bind parameters for their file and stage operands, so
// they and the options are written into the statement. The local path and stage
// location go in as the caller wrote them, quoting included, and option names
// and values are not checked: the 4.x connector wrote options and the local
// path the same way, and GS parses the statement.
fn sql_local_operand(local_path: &str) -> String {
    let local = local_path.trim();
    if local.starts_with('\'') || local.starts_with(FILE_URI_PREFIX) {
        local.to_string()
    } else {
        format!("{FILE_URI_PREFIX}{local}")
    }
}

fn sql_stage_operand(stage_location: &str) -> String {
    let stage = stage_location.trim();
    if stage.starts_with('\'')
        || stage.starts_with('@')
        || stage.starts_with(SNOWFLAKE_NESTED_STAGE_PREFIX)
        || stage.starts_with('/')
    {
        stage.to_string()
    } else {
        format!("@{stage}")
    }
}

fn render_put_sql(
    local_file_name: &str,
    stage_location: &str,
    options: &StageTransferOptions,
) -> String {
    format!(
        "PUT {} {} {}",
        sql_local_operand(local_file_name),
        sql_stage_operand(stage_location),
        options.render()
    )
    .trim_end()
    .to_string()
}

fn render_get_sql(
    stage_location: &str,
    target_directory: &str,
    options: &StageTransferOptions,
) -> String {
    format!(
        "GET {} {} {}",
        sql_stage_operand(stage_location),
        sql_local_operand(target_directory),
        options.render()
    )
    .trim_end()
    .to_string()
}

fn render_stream_put_sql(
    destination: &StreamDestination,
    options: &StageTransferOptions,
) -> String {
    format!(
        "PUT {FILE_URI_PREFIX}{} {} {}",
        destination.filename,
        destination.stage_dir,
        options.render()
    )
    .trim_end()
    .to_string()
}

#[derive(Debug, Clone, Copy)]
enum TransferDirection {
    Upload,
    Download,
}

impl TransferDirection {
    /// The `command` a GS PUT or GET response carries, which shapes the
    /// result set's columns.
    fn gs_command(self) -> &'static str {
        match self {
            Self::Upload => "UPLOAD",
            Self::Download => "DOWNLOAD",
        }
    }
}

/// Settings every XP transfer reads from the connection.
struct XpTransferSettings {
    retry_policy: RetryPolicy,
    put_compress_level: u32,
    put_tempdir: Option<std::path::PathBuf>,
    unsafe_file_write: bool,
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
            unsafe_file_write: conn.unsafe_file_write(),
            transport: conn.stage_transport(&self.crl_manager, &self.root_store_provider, transfer),
        }
    }

    async fn xp_backend_for(
        &self,
        conn: &Arc<Mutex<Connection>>,
    ) -> Result<Option<Arc<dyn SnowflakeBackend>>, ApiError> {
        conn.lock().await.xp_backend_arc().context(QuerySnafu)
    }

    async fn xp_transfer_result(
        &self,
        conn: &Arc<Mutex<Connection>>,
        direction: TransferDirection,
        rowset_data: RowsetData,
    ) -> Result<ResultSetInfo, ApiError> {
        let descriptor = response_to_descriptor(
            &query_response::Data::command_only(direction.gs_command()),
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
        options: StageTransferOptions,
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
                    .stage_transfer_via_statement(operation_ctx, conn_handle, sql)
                    .await;
            };

            let _session_guard = self.lock_session_if_needed(&conn).await;
            let xp_options = options.resolve_for_xp_put()?;
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
            self.xp_transfer_result(&conn, TransferDirection::Upload, rowset_data)
                .await
        }
        .instrument(crate::snowflake_op_span!(
            "connection_stage_upload",
            telemetry_id
        ))
        .await
    }

    /// Downloads the stage file, or every file under the stage prefix,
    /// `stage_location` names into the local directory `target_directory`.
    ///
    /// With an XP backend the files are the ones `list_stage_files` reports for
    /// the prefix, downloaded one at a time. No match follows the same rule as
    /// a GET through GS: an empty result for wrappers with
    /// `legacy_empty_get_on_missing`, otherwise a remote-file-not-found error.
    pub async fn connection_stage_download(
        &self,
        operation_ctx: Option<&OperationCtx>,
        conn_handle: Handle,
        stage_location: String,
        target_directory: String,
        options: StageTransferOptions,
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
                let sql = render_get_sql(&stage_location, &target_directory, &options);
                return self
                    .stage_transfer_via_statement(operation_ctx, conn_handle, sql)
                    .await;
            };

            let _session_guard = self.lock_session_if_needed(&conn).await;
            options.check_for_xp_get()?;
            let stage = normalize_stage_location(&stage_location);
            let src_locations = list_stage_files_via_xp_backend(&stage, backend.as_ref())
                .await
                .context(QueryResponseProcessSnafu)?;
            if src_locations.is_empty() {
                if !self.wrapper_presets.legacy_empty_get_on_missing {
                    return Err(remote_file_not_found()).context(QueryResponseProcessSnafu);
                }
                return self
                    .xp_transfer_result(
                        &conn,
                        TransferDirection::Download,
                        RowsetData::Download(Vec::new()),
                    )
                    .await;
            }

            let settings = self.xp_transfer_settings(&conn).await;
            let file_count = src_locations.len();
            let download_data = file_manager::DownloadData {
                src_locations,
                local_location: local_path(&target_directory),
                stage_info: StageInfo::for_xp_backend(&stage, &settings.transport),
                encryption_materials: vec![None; file_count],
                presigned_urls: vec![None; file_count],
                flavor: self.wrapper_presets.put_get_resultset_flavor.clone(),
                multipart: MultipartParams::from_server(None, Some(1)),
                unsafe_file_write: settings.unsafe_file_write,
                get_fastfail: true,
                cwd: None,
                transfer: settings.transport.transfer.clone(),
            };
            let rowset_data = download_files_via_xp_backend(
                download_data,
                &settings.retry_policy,
                backend.as_ref(),
                operation_ctx,
            )
            .await
            .context(QueryResponseProcessSnafu)?;
            self.xp_transfer_result(&conn, TransferDirection::Download, rowset_data)
                .await
        }
        .instrument(crate::snowflake_op_span!(
            "connection_stage_download",
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
        options: StageTransferOptions,
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
            options: options.resolve_for_xp_put()?,
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
        self.xp_transfer_result(&conn, TransferDirection::Upload, rowset_data)
            .await
    }

    /// Runs `sql` through a short-lived statement and returns its result set.
    async fn stage_transfer_via_statement(
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
                argument: "A stage transfer returned multiple result sets",
            }
            .fail(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apis::database_driver_v1::ErrorKind;
    use crate::apis::database_driver_v1::global_state::WrapperPresets;
    use crate::apis::database_driver_v1::xp_stage_test_support::{StageBackend, xp_stage_driver};

    fn options(pairs: &[(&str, &str)]) -> StageTransferOptions {
        StageTransferOptions::new(pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())))
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
    fn xp_download_stream_source_names_the_stage_as_an_xp_get_does() {
        for stage_name in ["'@st/dir'", "st/dir", "@st/dir/", "  '@st/dir/'  "] {
            assert_eq!(
                xp_download_stream_source(stage_name, "f.csv"),
                "@st/dir/f.csv",
                "{stage_name:?}"
            );
        }
        assert_eq!(
            xp_download_stream_source("'@st/it\\'s'", "f.csv"),
            "@st/it's/f.csv"
        );
        assert_eq!(
            xp_download_stream_source("snow://x/y", "f.csv"),
            "snow://x/y/f.csv"
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
            options(&[]).resolve_for_xp_put().unwrap(),
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
        .resolve_for_xp_put()
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
                options(&[(name, value)]).resolve_for_xp_put().is_err(),
                "{name}={value} must be rejected"
            );
        }
        assert!(options(&[("PARALLEL", "99")]).resolve_for_xp_put().is_ok());
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

    #[test]
    fn get_sql_passes_caller_quoting_through() {
        assert_eq!(
            render_get_sql(
                "mystage/prefix1",
                "/tmp/target",
                &options(&[("parallel", "10"), ("pattern", "'.*test.*[.]csv'")]),
            ),
            "GET @mystage/prefix1 file:///tmp/target parallel=10 pattern='.*test.*[.]csv'"
        );
        assert_eq!(
            render_get_sql("'@st/it\\'s'", "'file:///tmp/my dir'", &options(&[])),
            "GET '@st/it\\'s' 'file:///tmp/my dir'"
        );
    }

    #[test]
    fn xp_get_options_reject_a_pattern_and_a_malformed_parallel() {
        let pattern = options(&[("pattern", "'.*[.]csv'")])
            .check_for_xp_get()
            .expect_err("a PATTERN must be rejected");
        assert_eq!(pattern.kind(), ErrorKind::NotImplemented);
        for (name, value) in [("PARALLEL", "0"), ("PARALLEL", "100"), ("parallel", "four")] {
            let err = options(&[(name, value)])
                .check_for_xp_get()
                .expect_err("a malformed PARALLEL must be rejected");
            assert_eq!(err.kind(), ErrorKind::InvalidArgument);
        }
    }

    #[test]
    fn xp_get_options_accept_an_empty_pattern_and_ignore_unknown_names() {
        assert!(
            options(&[
                ("PATTERN", ""),
                ("parallel", "4"),
                ("NOT_A_GET_OPTION", "x")
            ])
            .check_for_xp_get()
            .is_ok()
        );
    }

    async fn xp_get_with_an_empty_listing(presets: WrapperPresets) -> Result<usize, ApiError> {
        let (driver, conn) = xp_stage_driver(presets, Arc::new(StageBackend::new("[]", b"")));
        let target = tempfile::tempdir().expect("tempdir");
        let info = driver
            .connection_stage_download(
                None,
                conn,
                "@st/none".to_string(),
                target.path().display().to_string(),
                StageTransferOptions::default(),
            )
            .await?;
        let mut fetcher = driver
            .result_set_get_async_stream(info.handle)
            .await
            .expect("GET result set stream");
        let mut rows = 0;
        while let Some(batch) = fetcher.next_batch().await.expect("GET result batch") {
            rows += batch.num_rows();
        }
        assert_eq!(
            std::fs::read_dir(target.path()).expect("read dir").count(),
            0
        );
        Ok(rows)
    }

    #[tokio::test]
    async fn xp_get_with_an_empty_listing_is_an_empty_result_for_legacy_wrappers() {
        let rows = xp_get_with_an_empty_listing(WrapperPresets::jdbc())
            .await
            .expect("a legacy wrapper gets an empty GET result");
        assert_eq!(rows, 0);
    }

    #[tokio::test]
    async fn xp_get_with_an_empty_listing_is_a_missing_remote_file_otherwise() {
        let err = xp_get_with_an_empty_listing(WrapperPresets::python())
            .await
            .expect_err("an empty listing must fail the GET");
        assert_eq!(err.kind(), ErrorKind::RemoteFileNotFound);
    }
}
