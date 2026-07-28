//! SPCS authentication probe — the in-container workload for the SPCS e2e test.
//!
//! Runs inside a Snowpark Container Services (SPCS) job service. SPCS injects:
//!
//! - `SNOWFLAKE_RUNNING_INSIDE_SPCS` — gate env var; also triggers the driver's
//!   SPCS token side-channel.
//! - `SNOWFLAKE_ACCOUNT`, `SNOWFLAKE_HOST`, `SNOWFLAKE_DATABASE`,
//!   `SNOWFLAKE_SCHEMA` — connection details; read automatically by the resolver
//!   when the gate var is set (no explicit passing required).
//! - `/snowflake/session/token` — OAuth access token used as the login
//!   credential; the probe reads this and passes it to `connect()`.
//! - `/snowflake/session/spcs_token` — service-identifier token attached to the
//!   login request as `SPCS_TOKEN` automatically by the driver (SNOW-3007075);
//!   the probe does nothing for it.
//!
//! The probe authenticates with `authenticator=OAUTH` + the OAuth token and NO
//! user (made optional for token auth by SNOW-3647715), then runs a query. It
//! exits 0 on success / non-zero on failure, so the orchestrating
//! `EXECUTE JOB SERVICE` reports `DONE` vs `FAILED` from the container exit code.

use sf_core::protobuf::apis::database_driver_v1::{
    DatabaseDriverClientBlockingExt, database_driver_client,
};
use sf_core::protobuf::generated::database_driver_v1::*;

/// Path to the OAuth access token SPCS injects for the login credential.
const OAUTH_TOKEN_PATH: &str = "/snowflake/session/token";

/// Connects using the SPCS-injected OAuth token and runs a query to prove
/// the session works.
///
/// `SNOWFLAKE_ACCOUNT`, `SNOWFLAKE_HOST`, `SNOWFLAKE_DATABASE`, and
/// `SNOWFLAKE_SCHEMA` are read automatically by the driver's resolver because
/// `SNOWFLAKE_RUNNING_INSIDE_SPCS` is set — no explicit passing required.
fn run() -> Result<String, String> {
    let token = std::fs::read_to_string(OAUTH_TOKEN_PATH)
        .map_err(|e| format!("reading OAuth token {OAUTH_TOKEN_PATH}: {e}"))?
        .trim()
        .to_string();

    let client = database_driver_client();

    let db_handle = client
        .database_new_blocking(DatabaseNewRequest {})
        .map_err(|e| format!("database_new failed: {e:?}"))?
        .db_handle
        .ok_or("database_new returned no handle")?;

    client
        .database_init_blocking(DatabaseInitRequest {
            db_handle: Some(db_handle),
        })
        .map_err(|e| format!("database_init failed: {e:?}"))?;

    let conn_handle = client
        .connection_new_blocking(ConnectionNewRequest {})
        .map_err(|e| format!("connection_new failed: {e:?}"))?
        .conn_handle
        .ok_or("connection_new returned no handle")?;

    // OAuth token auth, no user/password (user optional via SNOW-3647715).
    // account/host/database/schema come from the SPCS-injected env vars via
    // the resolver (SNOWFLAKE_RUNNING_INSIDE_SPCS is set by the platform).
    // The driver also attaches SPCS_TOKEN automatically (read_spcs_token).
    let options: Vec<(&str, String)> = vec![
        ("authenticator", "OAUTH".to_string()),
        ("token", token),
        // Identify as PythonConnector so the server enables the usual feature gates.
        ("client_app_id", "PythonConnector".to_string()),
        ("client_app_version", "5.0.0".to_string()),
    ];

    for (name, value) in options {
        client
            .connection_set_options_blocking(ConnectionSetOptionsRequest {
                conn_handle: Some(conn_handle),
                options: [(name.to_string(), ConfigSetting::from(value))]
                    .into_iter()
                    .collect(),
                ..Default::default()
            })
            .map_err(|e| format!("setting '{name}' failed: {e:?}"))?;
    }

    client
        .connection_init_blocking(ConnectionInitRequest {
            conn_handle: Some(conn_handle),
            db_handle: Some(db_handle),
            ..Default::default()
        })
        .map_err(|e| format!("connection_init (login) failed: {e:?}"))?;

    // Prove the authenticated session can execute a query.
    let stmt_handle = client
        .statement_new_blocking(StatementNewRequest {
            conn_handle: Some(conn_handle),
        })
        .map_err(|e| format!("statement_new failed: {e:?}"))?
        .stmt_handle
        .ok_or("statement_new returned no handle")?;

    client
        .statement_set_sql_query_blocking(StatementSetSqlQueryRequest {
            stmt_handle: Some(stmt_handle),
            query: "SELECT CURRENT_USER()".to_string(),
        })
        .map_err(|e| format!("set_sql_query failed: {e:?}"))?;

    client
        .statement_execute_query_blocking(StatementExecuteQueryRequest {
            stmt_handle: Some(stmt_handle),
            bindings: None,
            timeout_seconds: None,
        })
        .map_err(|e| format!("execute_query failed: {e:?}"))?;

    Ok("connected via SPCS OAuth token and executed SELECT CURRENT_USER()".to_string())
}

fn main() {
    match run() {
        Ok(message) => {
            println!("spcs_probe: SUCCESS — {message}");
            std::process::exit(0);
        }
        Err(error) => {
            eprintln!("spcs_probe: FAILURE — {error}");
            std::process::exit(1);
        }
    }
}
