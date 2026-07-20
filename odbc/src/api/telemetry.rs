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
/// Silently drops the event if the handle does not resolve to a
/// connected session (env/desc handles, null handles, statement
/// handles whose Dbc is still disconnected).
pub fn record_api_usage(
    handle_type: sql::HandleType,
    handle: sql::Handle,
    api_method: &'static str,
) {
    let Some(conn_handle) = resolve_conn_handle(handle_type, handle) else {
        return;
    };
    let Ok(globals) = global() else { return };
    let result = globals.block_on(async |client| {
        client
            .telemetry_send_api_usage(
                TelemetrySendApiUsageRequest {
                    conn_handle: Some(conn_handle),
                    api_method: api_method.to_string(),
                    // ODBC records bare entry-point names; argument capture is a
                    // wrapper-level concern that ODBC does not implement.
                    passed_arguments: Vec::new(),
                },
                tokio_util::sync::CancellationToken::new(),
            )
            .await
    });
    if let Err(e) = result {
        tracing::debug!(error = ?e, "telemetry_send_api_usage failed");
    }
}

/// Record an `exception` event derived from an `OdbcError`.
///
/// Same drop rules as [`record_api_usage`]. `OdbcError` itself is
/// **not** forwarded across the wire — only the already-classified
/// `(exception_type, error_source)` `&'static str`s returned by
/// [`OdbcError::telemetry_classification`].
pub fn record_wrapper_error(handle_type: sql::HandleType, handle: sql::Handle, err: &OdbcError) {
    let Some(conn_handle) = resolve_conn_handle(handle_type, handle) else {
        return;
    };
    let Ok(globals) = global() else { return };
    let (exception_type, error_source) = err.telemetry_classification();
    let error_source: &'static str = error_source.into();
    let result = globals.block_on(async |client| {
        client
            .telemetry_send_wrapper_error(
                TelemetrySendWrapperErrorRequest {
                    conn_handle: Some(conn_handle),
                    exception_type: exception_type.to_string(),
                    error_source: error_source.to_string(),
                },
                tokio_util::sync::CancellationToken::new(),
            )
            .await
    });
    if let Err(e) = result {
        tracing::debug!(error = ?e, "telemetry_send_wrapper_error failed");
    }
}

/// Resolve any ODBC handle to the protobuf [`TConnectionHandle`] of
/// its owning, currently-connected session. Returns `None` for
/// handles that do not correspond to a live session (env/desc
/// handles, null handles, statement handles whose Dbc is still
/// `Disconnected`, etc.).
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
            connected_conn_handle(&dbc)
        }
        sql::HandleType::Stmt => {
            let stmt = stmt_from_handle(handle).ok()?;
            let conn_id = stmt.conn_id;
            let dbc = global().ok()?.dbc_registry.get(conn_id).ok()?;
            connected_conn_handle(&dbc)
        }
        // Env, Desc, and any unknown handle type cannot have an associated
        // session. (Descriptor handles route through their owning statement
        // in ODBC, but the SQL* entry points pass us the descriptor handle
        // directly; we don't track ownership here — events for descriptor
        // calls drop quietly until a richer mapping is added.)
        _ => None,
    }
}

fn connected_conn_handle(dbc: &crate::api::Dbc) -> Option<TConnectionHandle> {
    match &dbc.connection.lock().state {
        ConnectionState::Connected { conn_handle, .. } => Some(*conn_handle),
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
