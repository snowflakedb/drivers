//! ODBC in-band telemetry shim.
//!
//! Reports two driver-spec events to sf_core through the public protobuf
//! API on [`DatabaseDriverClient`]:
//!
//! - **`api_call`** (one per `SQL*` C entry point) — sent via
//!   [`DatabaseDriverClient::telemetry_send_api_usage`] with the literal
//!   entry-point name as `api_method`.
//! - **`exception`** (only when an entry point returned `Err`) — sent via
//!   [`DatabaseDriverClient::telemetry_send_wrapper_error`] with the
//!   `OdbcError` variant name as `exception_type` and a high-level
//!   category as `error_source`. The category is a strongly-typed
//!   [`ErrorSource`] enum (see its docs for the full bucket list); on the
//!   wire it serialises to its snake_case form via [`Display`].
//!
//! Recording is **fire-and-forget**: each helper resolves the connection
//! handle on the caller's thread and then spawns the `telemetry_send_*`
//! call directly on the shared ODBC tokio runtime via
//! [`OdbcGlobals::spawn_telemetry`](crate::api::runtime::OdbcGlobals::spawn_telemetry).
//! The spawn returns immediately to the SQL hot path; the protobuf call
//! body only records an in-memory OTel event under the per-connection
//! span (no network I/O), so it completes promptly and does not
//! meaningfully contend with foreground SQL work.
//!
//! Calls for handles that don't resolve to a connected session
//! (env/desc/null handles, freshly allocated Dbc still in
//! `Disconnected`) are silently dropped before the spawn.
//!
//! [`ErrorSource`]: crate::api::error::ErrorSource
//!
//! [`DatabaseDriverClient`]: sf_core::protobuf::apis::database_driver_v1::DatabaseDriverClient
//! [`DatabaseDriverClient::telemetry_send_api_usage`]: sf_core::protobuf::apis::database_driver_v1::DatabaseDriverClient::telemetry_send_api_usage
//! [`DatabaseDriverClient::telemetry_send_wrapper_error`]: sf_core::protobuf::apis::database_driver_v1::DatabaseDriverClient::telemetry_send_wrapper_error

use odbc_sys as sql;
use sf_core::protobuf::generated::database_driver_v1::{
    ConnectionHandle as TConnectionHandle, TelemetrySendApiUsageRequest,
    TelemetrySendWrapperErrorRequest,
};

use crate::api::OdbcError;
use crate::api::runtime::global;
use crate::api::types::{ConnectionState, conn_from_handle, stmt_from_handle};

/// Record an `api_call` event for the given ODBC entry point.
///
/// Best-effort: returns immediately after the spawn. If the runtime is
/// not initialised or the handle does not resolve to a connected session,
/// the event is silently dropped without spawning.
pub fn record_api_usage(
    handle_type: sql::HandleType,
    handle: sql::Handle,
    api_method: &'static str,
) {
    let Some(conn_handle) = resolve_conn_handle(handle_type, handle) else {
        return;
    };
    let Ok(rt) = global() else {
        return;
    };
    rt.spawn_telemetry(|client| async move {
        let _ = client
            .telemetry_send_api_usage(TelemetrySendApiUsageRequest {
                conn_handle: Some(conn_handle),
                api_method: api_method.to_string(),
            })
            .await;
    });
}

/// Record an `exception` event derived from an `OdbcError`.
///
/// Best-effort, with the same drop rules as [`record_api_usage`].
/// `OdbcError` itself is **not** moved into the spawned future — only the
/// already-classified `(exception_type, error_source)` `&'static str`s
/// (returned by [`OdbcError::telemetry_classification`]) are, so the
/// borrow on the caller's `&OdbcError` does not outlive this function.
pub fn record_wrapper_error(handle_type: sql::HandleType, handle: sql::Handle, err: &OdbcError) {
    let Some(conn_handle) = resolve_conn_handle(handle_type, handle) else {
        return;
    };
    let (exception_type, error_source) = err.telemetry_classification();
    let error_source: &'static str = error_source.into();
    let Ok(rt) = global() else {
        return;
    };
    rt.spawn_telemetry(|client| async move {
        let _ = client
            .telemetry_send_wrapper_error(TelemetrySendWrapperErrorRequest {
                conn_handle: Some(conn_handle),
                exception_type: exception_type.to_string(),
                error_source: error_source.to_string(),
            })
            .await;
    });
}

/// Resolve any ODBC handle to the protobuf [`ConnectionHandle`](TConnectionHandle)
/// of its owning, currently-connected session. Returns `None` for handles
/// that do not correspond to a live session (env/desc handles, null handles,
/// statement handles whose Dbc is disconnected, etc.).
fn resolve_conn_handle(
    handle_type: sql::HandleType,
    handle: sql::Handle,
) -> Option<TConnectionHandle> {
    if handle.is_null() {
        return None;
    }
    match handle_type {
        sql::HandleType::Dbc => {
            let dbc = conn_from_handle(handle).ok()?;
            connected_handle(&dbc)
        }
        sql::HandleType::Stmt => {
            let stmt = stmt_from_handle(handle).ok()?;
            let conn_id = stmt.conn_id;
            let dbc = global().ok()?.dbc_registry.get(conn_id).ok()?;
            connected_handle(&dbc)
        }
        // Env, Desc, and any unknown handle type cannot have an associated
        // session. (Descriptor handles route through their owning statement
        // in ODBC, but the SQL* entry points pass us the descriptor handle
        // directly; we don't track ownership here — events for descriptor
        // calls drop quietly until a richer mapping is added.)
        _ => None,
    }
}

fn connected_handle(dbc: &crate::api::Dbc) -> Option<TConnectionHandle> {
    match dbc.connection.lock().state {
        ConnectionState::Connected { conn_handle, .. } => Some(conn_handle),
        ConnectionState::Disconnected => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snafu::Location;

    fn loc() -> Location {
        Location::new("test", 0, 0)
    }

    #[test]
    fn resolve_conn_handle_returns_none_for_null_handle() {
        // No ODBC globals initialised in unit tests — and even if they were,
        // a null handle resolves to None before any registry lookup.
        assert!(resolve_conn_handle(sql::HandleType::Stmt, std::ptr::null_mut()).is_none());
        assert!(resolve_conn_handle(sql::HandleType::Dbc, std::ptr::null_mut()).is_none());
        assert!(resolve_conn_handle(sql::HandleType::Env, std::ptr::null_mut()).is_none());
    }

    #[test]
    fn resolve_conn_handle_returns_none_for_env_handle_type() {
        // Env handles never carry a session even when non-null.
        let dummy: sql::Handle = 1usize as sql::Handle;
        assert!(resolve_conn_handle(sql::HandleType::Env, dummy).is_none());
    }

    #[test]
    fn record_helpers_do_not_panic_without_globals() {
        // With ODBC globals not initialised in this unit-test process,
        // both helpers must early-return cleanly.
        record_api_usage(sql::HandleType::Stmt, std::ptr::null_mut(), "SQLExecDirect");
        record_wrapper_error(
            sql::HandleType::Dbc,
            std::ptr::null_mut(),
            &OdbcError::InvalidHandle { location: loc() },
        );
    }
}
