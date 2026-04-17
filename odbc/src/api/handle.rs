use std::sync::{Arc, Mutex, Weak};

use crate::api::diagnostic::DiagnosticInfo;
use crate::api::error::InvalidHandleSnafu;
use crate::api::{Connection, Environment, OdbcResult, Statement};
use odbc_sys as sql;

pub const ODBC_MAGIC_ALIVE: u32 = 0x0DBC_C0DE;
pub const ODBC_MAGIC_DEAD: u32 = 0xDEAD_BEEF;

pub enum OdbcHandle {
    Environment(Arc<Mutex<Environment>>),
    Connection(Weak<Mutex<Connection>>),
    Statement(Weak<Mutex<Statement>>),
}

#[repr(C)]
pub struct OdbcHandleWrapper {
    pub magic: u32,
    pub payload: OdbcHandle,
}

// SAFETY: This is a compile-time check to ensure that OdbcHandleWrapper is Send and Sync.
// The application can share ODBC handles accros threads so we need to ensure they are safe to use in a multi-threaded environment.
const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<OdbcHandleWrapper>();
};

/// Validate a raw `sql::Handle` and return a reference to the wrapper.
///
/// # Safety
/// The caller must ensure `handle` was originally produced by `Box::into_raw`
/// on a `Box<OdbcHandleWrapper>` and has not been freed yet.
pub fn wrapper_from_handle<'a>(handle: sql::Handle) -> OdbcResult<&'a mut OdbcHandleWrapper> {
    if handle.is_null() {
        return InvalidHandleSnafu.fail();
    }
    let wrapper = unsafe { &mut *(handle as *mut OdbcHandleWrapper) };
    if wrapper.magic == ODBC_MAGIC_DEAD {
        tracing::error!(
            "Stale handle: already freed (magic=0x{:08X})",
            wrapper.magic
        );
        return InvalidHandleSnafu.fail();
    }
    if wrapper.magic != ODBC_MAGIC_ALIVE {
        tracing::error!(
            "Invalid handle: magic number mismatch (expected 0x{ODBC_MAGIC_ALIVE:08X}, got 0x{:08X})",
            wrapper.magic
        );
        return InvalidHandleSnafu.fail();
    }
    Ok(wrapper)
}

/// Extract `Arc<Mutex<Environment>>` from an environment handle.
pub fn env_from_handle(handle: sql::Handle) -> OdbcResult<Arc<Mutex<Environment>>> {
    let wrapper = wrapper_from_handle(handle)?;
    match &wrapper.payload {
        OdbcHandle::Environment(arc) => Ok(arc.clone()),
        _ => {
            tracing::error!("Type mismatch: expected Environment handle");
            InvalidHandleSnafu.fail()
        }
    }
}

/// Extract `Arc<Mutex<Connection>>` from a connection handle by upgrading
/// the `Weak`.
///
/// Returns `InvalidHandle` if the parent `Environment` has already dropped
/// the owning `Arc<Mutex<Connection>>`.
pub fn conn_from_handle(handle: sql::Handle) -> OdbcResult<Arc<Mutex<Connection>>> {
    let wrapper = wrapper_from_handle(handle)?;
    match &wrapper.payload {
        OdbcHandle::Connection(weak) => weak.upgrade().ok_or_else(|| {
            tracing::error!(
                "Stale connection handle: parent environment has dropped this connection"
            );
            InvalidHandleSnafu.build()
        }),
        _ => {
            tracing::error!("Type mismatch: expected Connection handle");
            InvalidHandleSnafu.fail()
        }
    }
}

/// Extract `Arc<Mutex<Statement>>` from a statement handle by upgrading
/// the `Weak`.
///
/// Returns `InvalidHandle` if the parent `Connection` has already dropped
/// the owning `Arc<Mutex<Statement>>`.
pub fn stmt_from_handle(handle: sql::Handle) -> OdbcResult<Arc<Mutex<Statement>>> {
    let wrapper = wrapper_from_handle(handle)?;
    match &wrapper.payload {
        OdbcHandle::Statement(weak) => weak.upgrade().ok_or_else(|| {
            tracing::error!("Stale statement handle: parent connection has dropped this statement");
            InvalidHandleSnafu.build()
        }),
        _ => {
            tracing::error!("Type mismatch: expected Statement handle");
            InvalidHandleSnafu.fail()
        }
    }
}

/// Lock the appropriate handle mutex and pass `&mut DiagnosticInfo` to `f`.
///
/// The mutex guard is held for the duration of `f`, so callers never
/// touch raw pointers or escape a borrow from a local guard.
pub fn with_diag_info<R>(
    handle: sql::Handle,
    f: impl FnOnce(&mut DiagnosticInfo) -> R,
) -> OdbcResult<R> {
    let wrapper = wrapper_from_handle(handle)?;
    match &wrapper.payload {
        OdbcHandle::Environment(arc) => Ok(f(&mut arc.lock().unwrap().diagnostic_info)),
        OdbcHandle::Connection(weak) => {
            let arc = weak.upgrade().ok_or_else(|| InvalidHandleSnafu.build())?;
            Ok(f(&mut arc.lock().unwrap().diagnostic_info))
        }
        OdbcHandle::Statement(weak) => {
            let arc = weak.upgrade().ok_or_else(|| InvalidHandleSnafu.build())?;
            Ok(f(&mut arc.lock().unwrap().diagnostic_info))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::diagnostic::DiagnosticInfo;

    fn make_env_handle() -> sql::Handle {
        let wrapper = Box::new(OdbcHandleWrapper {
            magic: ODBC_MAGIC_ALIVE,
            payload: OdbcHandle::Environment(Arc::new(Mutex::new(Environment {
                odbc_version: 3,
                connection_pooling: sql::AttrConnectionPooling::Off,
                connection_pool_match: sql::AttrCpMatch::Strict,
                diagnostic_info: DiagnosticInfo::default(),
                child_connections: vec![],
            }))),
        });
        Box::into_raw(wrapper) as sql::Handle
    }

    #[test]
    fn extract_environment_from_valid_handle() {
        let handle = make_env_handle();
        let env_arc = env_from_handle(handle);
        assert!(env_arc.is_ok());
        assert_eq!(env_arc.unwrap().lock().unwrap().odbc_version, 3);
        unsafe { drop(Box::from_raw(handle as *mut OdbcHandleWrapper)) };
    }

    #[test]
    fn wrong_variant_returns_error() {
        let handle = make_env_handle();
        let result = conn_from_handle(handle);
        assert!(result.is_err());
        unsafe { drop(Box::from_raw(handle as *mut OdbcHandleWrapper)) };
    }

    #[test]
    fn null_handle_returns_error() {
        let result = env_from_handle(std::ptr::null_mut());
        assert!(result.is_err());
    }

    #[test]
    fn dead_magic_returns_error() {
        let handle = make_env_handle();
        let wrapper = unsafe { &mut *(handle as *mut OdbcHandleWrapper) };
        wrapper.magic = ODBC_MAGIC_DEAD;
        let result = env_from_handle(handle);
        assert!(result.is_err());
        unsafe { drop(Box::from_raw(handle as *mut OdbcHandleWrapper)) };
    }

    #[test]
    fn with_diag_info_works_for_any_variant() {
        let handle = make_env_handle();
        let result = with_diag_info(handle, |_diag| {});
        assert!(result.is_ok());
        unsafe { drop(Box::from_raw(handle as *mut OdbcHandleWrapper)) };
    }
}
