use odbc_sys as sql;
use sf_core::telemetry::ConnectionTelemetry;

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
    let Some(recorder) = resolve_recorder(handle_type, handle) else {
        return;
    };
    recorder.record_api_call(api_method);
}

/// Record an `exception` event derived from an `OdbcError`.
///
/// Same drop rules as [`record_api_usage`]. `OdbcError` itself is
/// **not** forwarded into the recorder — only the already-classified
/// `(exception_type, error_source)` `&'static str`s (returned by
/// [`OdbcError::telemetry_classification`]) are.
pub fn record_wrapper_error(handle_type: sql::HandleType, handle: sql::Handle, err: &OdbcError) {
    let Some(recorder) = resolve_recorder(handle_type, handle) else {
        return;
    };
    let (exception_type, error_source) = err.telemetry_classification();
    let error_source: &'static str = error_source.into();
    recorder.record_exception(exception_type, error_source);
}

/// Resolve any ODBC handle to a clone of the cached
/// [`ConnectionTelemetry`] for its owning, currently-connected
/// session. Returns `None` for handles that do not correspond to a
/// live session (env/desc handles, null handles, statement handles
/// whose Dbc is disconnected, etc.).
fn resolve_recorder(
    handle_type: sql::HandleType,
    handle: sql::Handle,
) -> Option<ConnectionTelemetry> {
    if handle.is_null() {
        return None;
    }
    match handle_type {
        sql::HandleType::Dbc => {
            let dbc = conn_from_handle(handle).ok()?;
            connected_recorder(&dbc)
        }
        sql::HandleType::Stmt => {
            let stmt = stmt_from_handle(handle).ok()?;
            let conn_id = stmt.conn_id;
            let dbc = global().ok()?.dbc_registry.get(conn_id).ok()?;
            connected_recorder(&dbc)
        }
        // Env, Desc, and any unknown handle type cannot have an associated
        // session. (Descriptor handles route through their owning statement
        // in ODBC, but the SQL* entry points pass us the descriptor handle
        // directly; we don't track ownership here — events for descriptor
        // calls drop quietly until a richer mapping is added.)
        _ => None,
    }
}

fn connected_recorder(dbc: &crate::api::Dbc) -> Option<ConnectionTelemetry> {
    match &dbc.connection.lock().state {
        ConnectionState::Connected { telemetry, .. } => Some(telemetry.clone()),
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
    fn resolve_recorder_returns_none_for_null_handle() {
        // No ODBC globals initialised in unit tests — and even if they were,
        // a null handle resolves to None before any registry lookup.
        assert!(resolve_recorder(sql::HandleType::Stmt, std::ptr::null_mut()).is_none());
        assert!(resolve_recorder(sql::HandleType::Dbc, std::ptr::null_mut()).is_none());
        assert!(resolve_recorder(sql::HandleType::Env, std::ptr::null_mut()).is_none());
    }

    #[test]
    fn resolve_recorder_returns_none_for_env_handle_type() {
        // Env handles never carry a session even when non-null.
        let dummy: sql::Handle = 1usize as sql::Handle;
        assert!(resolve_recorder(sql::HandleType::Env, dummy).is_none());
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
