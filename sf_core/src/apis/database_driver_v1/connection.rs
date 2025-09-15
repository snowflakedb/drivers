use super::error::ApiError;
use super::global_state::{CONN_HANDLE_MANAGER, DB_HANDLE_MANAGER};
use super::{Handle, Setting};
use snafu::location;
use std::collections::HashMap;
use std::sync::Mutex;

pub struct Connection {
    pub settings: HashMap<String, Setting>,
    pub session_token: Option<String>,
}

impl Default for Connection {
    fn default() -> Self {
        Self::new()
    }
}

impl Connection {
    pub fn new() -> Self {
        Connection {
            settings: HashMap::new(),
            session_token: None,
        }
    }
}

pub fn connection_new() -> Handle {
    CONN_HANDLE_MANAGER.add_handle(Mutex::new(Connection::new()))
}

#[allow(clippy::result_large_err)]
pub fn connection_init(conn_handle: Handle, db_handle: Handle) -> Result<(), ApiError> {
    // Merge DB settings into connection (without overwriting existing connection keys)
    if let (Some(db_ptr), Some(conn_ptr)) = (
        DB_HANDLE_MANAGER.get_obj(db_handle),
        CONN_HANDLE_MANAGER.get_obj(conn_handle),
    ) {
        let db = db_ptr.lock().unwrap();
        let mut conn = conn_ptr.lock().unwrap();
        for (k, v) in db.settings.iter() {
            conn.settings.entry(k.clone()).or_insert_with(|| v.clone());
        }
    }

    // Build LoginParameters from merged settings and perform login IF credentials are present
    let maybe_token = {
        let conn_ptr =
            CONN_HANDLE_MANAGER
                .get_obj(conn_handle)
                .ok_or_else(|| ApiError::InvalidArgument {
                    argument: "Connection handle not found".to_string(),
                    location: location!(),
                })?;
        let conn = conn_ptr.lock().unwrap();

        // Decide whether we have enough info to attempt login now
        let has_user =
            matches!(conn.settings.get("user"), Some(Setting::String(s)) if !s.is_empty());
        let has_password =
            matches!(conn.settings.get("password"), Some(Setting::String(s)) if !s.is_empty());
        let has_token =
            matches!(conn.settings.get("token"), Some(Setting::String(s)) if !s.is_empty());
        let has_pk_file = matches!(conn.settings.get("private_key_file"), Some(Setting::String(s)) if !s.is_empty());
        let authenticator = match conn.settings.get("authenticator") {
            Some(Setting::String(s)) => s.as_str(),
            _ => "",
        };
        let creds_ready = match authenticator {
            "SNOWFLAKE_JWT" => has_user && has_pk_file,
            "PROGRAMMATIC_ACCESS_TOKEN" => has_user && has_token,
            _ => has_user && has_password,
        };
        let has_account =
            matches!(conn.settings.get("account"), Some(Setting::String(s)) if !s.is_empty());
        let has_server_hint =
            conn.settings.contains_key("server_url") || conn.settings.contains_key("host");

        if !(creds_ready && has_account && has_server_hint) {
            // If an explicit authenticator is set, validate immediately and error on missing params
            if !authenticator.is_empty() {
                let validation =
                    crate::config::rest_parameters::LoginParameters::from_settings(&conn.settings)
                        .map(|_| ())
                        .map_err(|e| ApiError::Configuration {
                            location: location!(),
                            source: e,
                        });
                validation?;
            }
            return Ok(());
        }

        // Build TLS and runtime
        let tls_cfg = crate::tls::TlsConfig::from_settings(&conn.settings);
        let _client = crate::tls::create_tls_client_with_config(tls_cfg).map_err(|_| {
            ApiError::GenericError {
                location: location!(),
            }
        })?;
        let login_parameters =
            crate::config::rest_parameters::LoginParameters::from_settings(&conn.settings)
                .map_err(|e| ApiError::Configuration {
                    location: location!(),
                    source: e,
                })?;
        let rt = tokio::runtime::Runtime::new().map_err(|e| ApiError::RuntimeCreation {
            location: location!(),
            source: e,
        })?;
        Some(
            rt.block_on(async { crate::rest::snowflake::snowflake_login(&login_parameters).await })
                .map_err(|source| ApiError::Login {
                    location: location!(),
                    source,
                })?,
        )
    };

    // Store session token if we logged in
    if let Some(token) = maybe_token
        && let Some(conn_ptr) = CONN_HANDLE_MANAGER.get_obj(conn_handle)
    {
        let mut conn = conn_ptr.lock().unwrap();
        conn.session_token = Some(token);
    }
    Ok(())
}

#[allow(clippy::result_large_err)]
pub fn connection_set_option(handle: Handle, key: String, value: Setting) -> Result<(), ApiError> {
    match CONN_HANDLE_MANAGER.get_obj(handle) {
        Some(conn_ptr) => {
            let mut conn = conn_ptr.lock().unwrap();
            conn.settings.insert(key, value);
            Ok(())
        }
        None => Err(ApiError::InvalidArgument {
            argument: "Connection handle not found".to_string(),
            location: location!(),
        }),
    }
}

#[allow(clippy::result_large_err)]
pub fn connection_release(conn_handle: Handle) -> Result<(), ApiError> {
    match CONN_HANDLE_MANAGER.delete_handle(conn_handle) {
        true => Ok(()),
        false => Err(ApiError::InvalidArgument {
            argument: "Failed to release connection handle".to_string(),
            location: location!(),
        }),
    }
}
