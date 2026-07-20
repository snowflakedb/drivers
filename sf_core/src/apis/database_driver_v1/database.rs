use std::collections::HashMap;
use tokio::sync::Mutex;

use super::error::*;
use super::global_state::DatabaseDriverV1;
use super::validation::{ValidationIssue, resolve_and_apply_options};
use crate::chunks::{ChunkFormatKind, FetchChunkInput, PrefetchConfig, fetch_chunks_reader};
use crate::config::ParamStore;
use crate::config::settings::Setting;
use crate::handle_manager::Handle;
use crate::query_types::RowType;
use crate::tls;
use arrow::ffi_stream::FFI_ArrowArrayStream;
use snafu::ResultExt;

impl DatabaseDriverV1 {
    pub fn database_new(&self) -> Handle {
        self.databases.add_handle(Mutex::new(Database::new()))
    }

    pub async fn database_set_options(
        &self,
        db_handle: Handle,
        options: HashMap<String, Setting>,
    ) -> Result<Vec<ValidationIssue>, ApiError> {
        match self.databases.get_obj(db_handle) {
            Some(db_ptr) => {
                let mut db = db_ptr.lock().await;
                resolve_and_apply_options(&mut db.settings, options)
            }
            None => InvalidArgumentSnafu {
                argument: "Database handle not found".to_string(),
            }
            .fail(),
        }
    }

    pub fn database_init(&self, db_handle: Handle) -> Result<(), ApiError> {
        match self.databases.get_obj(db_handle) {
            Some(_db_ptr) => Ok(()),
            None => InvalidArgumentSnafu {
                argument: "Database handle not found".to_string(),
            }
            .fail(),
        }
    }

    pub fn database_release(&self, db_handle: Handle) -> Result<(), ApiError> {
        match self.databases.delete_handle(db_handle) {
            true => Ok(()),
            false => InvalidArgumentSnafu {
                argument: "Failed to release database handle".to_string(),
            }
            .fail(),
        }
    }

    pub async fn database_fetch_chunk(
        &self,
        conn_handle: Option<Handle>,
        chunks: Vec<FetchChunkInput>,
        chunk_format: ChunkFormatKind,
        nullable_flags: &[bool],
        row_types: Vec<RowType>,
    ) -> Result<Box<FFI_ArrowArrayStream>, ApiError> {
        let (client, prefetch_config) = match conn_handle {
            Some(conn_handle) => {
                let conn_ptr = self.connections.get_obj(conn_handle).ok_or_else(|| {
                    InvalidArgumentSnafu {
                        argument: "Connection handle not found".to_string(),
                    }
                    .build()
                })?;
                let conn = conn_ptr.lock().await;
                let client = conn
                    .http_client
                    .clone()
                    .ok_or_else(|| ConnectionNotInitializedSnafu.build())?;
                let session_params = conn.session_parameters.read().await;
                let prefetch_config = PrefetchConfig::from_session_params(&session_params);
                (client, prefetch_config)
            }
            None => {
                // TODO(SNOW-3801967): when no conn handle, we fall back to a fresh TLS client and
                //  default prefetch config, but we could restore proper configuration if it's
                //  serialized with the result chunk
                let client = tls::create_tls_client_with_config(
                    tls::TlsConfig::default(),
                    self.crl_worker.clone(),
                )
                .context(TlsClientCreationSnafu)?;
                (client, PrefetchConfig::default())
            }
        };
        let reader = fetch_chunks_reader(
            chunks,
            chunk_format,
            row_types,
            nullable_flags,
            client,
            &prefetch_config,
        )
        .await
        .context(ChunkFetchSnafu)?;

        Ok(Box::new(FFI_ArrowArrayStream::new(reader)))
    }
}

pub struct Database {
    pub(crate) settings: ParamStore,
}

impl Default for Database {
    fn default() -> Self {
        Self::new()
    }
}

impl Database {
    pub fn new() -> Self {
        Database {
            settings: ParamStore::new(),
        }
    }
}
