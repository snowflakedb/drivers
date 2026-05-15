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
//! handle on the caller's thread and then pushes a small
//! [`TelemetryEvent`] onto a bounded mpsc channel via a non-blocking
//! [`try_send`](tokio::sync::mpsc::Sender::try_send). A single
//! long-lived **drainer task**, spawned at `env_allocated` time on a
//! dedicated single-worker tokio runtime
//! ([`OdbcGlobals`](crate::api::runtime::OdbcGlobals)), receives events
//! and issues the actual `telemetry_send_*` RPCs.
//!
//! Calls for handles that don't resolve to a connected session
//! (env/desc/null handles, freshly allocated Dbc still in
//! `Disconnected`) are silently dropped before the channel push.
//!
//! On overflow ([`try_send`](tokio::sync::mpsc::Sender::try_send)
//! returning [`TrySendError::Full`](tokio::sync::mpsc::error::TrySendError::Full))
//! the event is dropped (fire-and-forget) and a [`tracing::debug`] record
//! is emitted (target `odbc::telemetry`) so operators can diagnose loss.
//!
//! [`ErrorSource`]: crate::api::error::ErrorSource
//!
//! [`DatabaseDriverClient`]: sf_core::protobuf::apis::database_driver_v1::DatabaseDriverClient
//! [`DatabaseDriverClient::telemetry_send_api_usage`]: sf_core::protobuf::apis::database_driver_v1::DatabaseDriverClient::telemetry_send_api_usage
//! [`DatabaseDriverClient::telemetry_send_wrapper_error`]: sf_core::protobuf::apis::database_driver_v1::DatabaseDriverClient::telemetry_send_wrapper_error

use std::sync::Arc;

use odbc_sys as sql;
use sf_core::protobuf::apis::database_driver_v1::DatabaseDriverClient;
use sf_core::protobuf::generated::database_driver_v1::{
    ConnectionHandle as TConnectionHandle, TelemetrySendApiUsageRequest,
    TelemetrySendWrapperErrorRequest,
};
use tokio::sync::mpsc;

use crate::api::OdbcError;
use crate::api::runtime::global;
use crate::api::types::{ConnectionState, conn_from_handle, stmt_from_handle};

/// One unit of work for the telemetry drainer task.
///
/// Kept small (24–32 bytes on 64-bit) so that the bounded channel's
/// fixed-capacity ring buffer fits comfortably in cache: with 8 KiB
/// capacity (the default in [`OdbcGlobals`](crate::api::runtime::OdbcGlobals))
/// the in-flight queue tops out at ~256 KiB even when the drainer falls
/// behind during a fetch loop.
///
/// Both variants intentionally carry `&'static str` tokens (the literal
/// API entry-point name and the snake-case error category) so that no
/// per-event allocation happens on the hot path. The drainer side does
/// the `to_string()` once when constructing the protobuf request.
pub(crate) enum TelemetryEvent {
    ApiCall {
        conn_handle: TConnectionHandle,
        api_method: &'static str,
    },
    Exception {
        conn_handle: TConnectionHandle,
        exception_type: &'static str,
        error_source: &'static str,
    },
}

/// Debug-only context when the bounded telemetry channel is full.
pub(crate) fn debug_log_telemetry_dropped_queue_full(
    event: &TelemetryEvent,
    channel_capacity: usize,
) {
    match event {
        TelemetryEvent::ApiCall {
            conn_handle,
            api_method,
        } => {
            tracing::debug!(
                target: "odbc::telemetry",
                telemetry_event = "queue_full",
                telemetry_kind = "api_call",
                conn_id = conn_handle.id,
                conn_magic = conn_handle.magic,
                api_method = *api_method,
                channel_capacity = channel_capacity,
                "in-band telemetry dropped: channel full"
            );
        }
        TelemetryEvent::Exception {
            conn_handle,
            exception_type,
            error_source,
        } => {
            tracing::debug!(
                target: "odbc::telemetry",
                telemetry_event = "queue_full",
                telemetry_kind = "wrapper_error",
                conn_id = conn_handle.id,
                conn_magic = conn_handle.magic,
                exception_type = *exception_type,
                error_source = *error_source,
                channel_capacity = channel_capacity,
                "in-band telemetry dropped: channel full"
            );
        }
    }
}

/// Long-lived drainer that owns the receiver end of the telemetry
/// channel. Spawned once at `env_allocated` time on the dedicated
/// telemetry runtime; exits when the last [`Sender`](mpsc::Sender) is
/// dropped (i.e. the [`OdbcGlobals`](crate::api::runtime::OdbcGlobals)
/// is being torn down).
pub(crate) async fn drain_telemetry(
    mut rx: mpsc::Receiver<TelemetryEvent>,
    client: Arc<DatabaseDriverClient>,
) {
    while let Some(event) = rx.recv().await {
        dispatch(&client, event).await;
    }
    tracing::debug!("telemetry drainer exiting (channel closed)");
}

async fn dispatch(client: &DatabaseDriverClient, event: TelemetryEvent) {
    match event {
        TelemetryEvent::ApiCall {
            conn_handle,
            api_method,
        } => {
            let _ = client
                .telemetry_send_api_usage(TelemetrySendApiUsageRequest {
                    conn_handle: Some(conn_handle),
                    api_method: api_method.to_string(),
                })
                .await;
        }
        TelemetryEvent::Exception {
            conn_handle,
            exception_type,
            error_source,
        } => {
            let _ = client
                .telemetry_send_wrapper_error(TelemetrySendWrapperErrorRequest {
                    conn_handle: Some(conn_handle),
                    exception_type: exception_type.to_string(),
                    error_source: error_source.to_string(),
                })
                .await;
        }
    }
}

/// Record an `api_call` event for the given ODBC entry point.
///
/// Best-effort: returns immediately without blocking the caller and
/// without ever calling into tokio's task system on the hot path. If
/// the runtime is not initialised, the handle does not resolve to a
/// connected session, or the channel is full, the event is silently
/// dropped.
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
    rt.record_telemetry(TelemetryEvent::ApiCall {
        conn_handle,
        api_method,
    });
}

/// Record an `exception` event derived from an `OdbcError`.
///
/// Best-effort, with the same dropping rules as [`record_api_usage`].
/// `OdbcError` itself is **not** moved through the channel — only the
/// already-classified `(exception_type, error_source)` `&'static str`s
/// (returned by [`OdbcError::telemetry_classification`]) are, so the
/// borrow on the caller's `&OdbcError` does not outlive this function.
pub fn record_wrapper_error(handle_type: sql::HandleType, handle: sql::Handle, err: &OdbcError) {
    let Some(conn_handle) = resolve_conn_handle(handle_type, handle) else {
        return;
    };
    let (exception_type, error_source) = err.telemetry_classification();
    let Ok(rt) = global() else {
        return;
    };
    rt.record_telemetry(TelemetryEvent::Exception {
        conn_handle,
        exception_type,
        error_source: error_source.into(),
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
