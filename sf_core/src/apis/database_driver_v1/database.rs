use std::collections::HashMap;
use std::io;
use tokio::sync::Mutex;

use arrow::ffi_stream::FFI_ArrowArrayStream;
use arrow_ipc::reader::StreamReader;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use snafu::ResultExt;

use super::error::*;
use super::global_state::DatabaseDriverV1;
use super::validation::{ValidationIssue, resolve_and_apply_options};
use crate::chunks::{ChunkDownloadData, get_chunk_data};
use crate::config::ParamStore;
use crate::config::settings::Setting;
use crate::handle_manager::Handle;

impl DatabaseDriverV1 {
    pub fn database_new(&self) -> Handle {
        self.databases.add_handle(Mutex::new(Database::new()))
    }

    pub async fn database_set_option(
        &self,
        db_handle: Handle,
        key: String,
        value: Setting,
    ) -> Result<(), ApiError> {
        match self.databases.get_obj(db_handle) {
            Some(db_ptr) => {
                let mut db = db_ptr.lock().await;
                db.settings.insert(key, value);
                Ok(())
            }
            None => InvalidArgumentSnafu {
                argument: "Database handle not found".to_string(),
            }
            .fail(),
        }
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
        db_handle: Handle,
        input: FetchChunkInput,
    ) -> Result<Box<FFI_ArrowArrayStream>, ApiError> {
        if self.databases.get_obj(db_handle).is_none() {
            return InvalidArgumentSnafu {
                argument: "Database handle not found".to_string(),
            }
            .fail();
        }

        let bytes = match input {
            FetchChunkInput::Inline(data) => BASE64.decode(&data).context(Base64DecodingSnafu)?,
            FetchChunkInput::Remote(chunk) => {
                // TODO Configure the client properly here
                let client = reqwest::Client::new();
                get_chunk_data(&client, &chunk)
                    .await
                    .context(ChunkFetchSnafu)?
            }
        };

        let cursor = io::Cursor::new(bytes);
        let reader = StreamReader::try_new(cursor, None).context(ArrowParsingSnafu)?;
        Ok(Box::new(FFI_ArrowArrayStream::new(Box::new(reader))))
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

pub enum FetchChunkInput {
    Inline(String),
    Remote(ChunkDownloadData),
}
