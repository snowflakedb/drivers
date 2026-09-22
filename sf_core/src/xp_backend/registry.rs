//! Write-once host-transport slot owned by [`crate::apis::database_driver_v1::DatabaseDriverV1`].
//!
//! Lookups are `OnceLock::get` after init. The C registration entry point attaches
//! to the [`XpSlot`] installed when the first
//! [`crate::apis::database_driver_v1::DatabaseDriverV1`] is constructed, or
//! parks the adapter until that construction if the host registers first.

use std::sync::{Arc, Mutex, OnceLock};

use super::{BackendError, SnowflakeBackend};
use crate::env_vars;
#[cfg(not(test))]
use crate::utils::sync::MutexRecoverExt;

static C_TARGET: OnceLock<Arc<XpSlot>> = OnceLock::new();
static C_PENDING: Mutex<Option<Arc<dyn SnowflakeBackend>>> = Mutex::new(None);

#[cfg(test)]
std::thread_local! {
    static TEST_TARGET: std::cell::RefCell<Option<Arc<XpSlot>>> =
        const { std::cell::RefCell::new(None) };
    static TEST_PENDING: std::cell::RefCell<Option<Arc<dyn SnowflakeBackend>>> =
        const { std::cell::RefCell::new(None) };
}

pub(crate) fn install_c_registration_target(slot: Arc<XpSlot>) {
    let _ = C_TARGET.set(slot);
}

pub(crate) fn register_backend(backend: Arc<dyn SnowflakeBackend>) -> Result<(), BackendError> {
    if let Some(slot) = current_c_target() {
        return slot.register(backend);
    }
    park_pending(backend)
}

fn current_c_target() -> Option<Arc<XpSlot>> {
    #[cfg(test)]
    {
        TEST_TARGET.with(|target| target.borrow().clone())
    }
    #[cfg(not(test))]
    C_TARGET.get().cloned()
}

fn park_pending(backend: Arc<dyn SnowflakeBackend>) -> Result<(), BackendError> {
    #[cfg(test)]
    {
        TEST_PENDING.with(|pending| {
            if pending.borrow().is_some() {
                return Err(BackendError::already_registered());
            }
            *pending.borrow_mut() = Some(backend);
            Ok(())
        })
    }
    #[cfg(not(test))]
    {
        let mut pending = C_PENDING.lock_recover();
        if pending.is_some() {
            return Err(BackendError::already_registered());
        }
        *pending = Some(backend);
        Ok(())
    }
}

fn take_pending_c_backend() -> Option<Arc<dyn SnowflakeBackend>> {
    #[cfg(test)]
    {
        TEST_PENDING.with(|pending| pending.borrow_mut().take())
    }
    #[cfg(not(test))]
    C_PENDING.lock_recover().take()
}

#[cfg(test)]
pub(crate) struct TestCRegistration;

#[cfg(test)]
impl TestCRegistration {
    pub(crate) fn isolate() -> Self {
        TEST_TARGET.with(|target| *target.borrow_mut() = None);
        TEST_PENDING.with(|pending| *pending.borrow_mut() = None);
        Self
    }

    pub(crate) fn attach_to(slot: Arc<XpSlot>) -> Self {
        TEST_TARGET.with(|target| *target.borrow_mut() = Some(slot));
        Self
    }
}

#[cfg(test)]
impl Drop for TestCRegistration {
    fn drop(&mut self) {
        TEST_TARGET.with(|target| *target.borrow_mut() = None);
        TEST_PENDING.with(|pending| *pending.borrow_mut() = None);
    }
}

enum XpMode {
    Http,
    Xp(OnceLock<Arc<dyn SnowflakeBackend>>),
}

pub struct XpSlot {
    mode: XpMode,
}

impl XpSlot {
    pub fn new(running_inside_xp: bool, backend: Option<Arc<dyn SnowflakeBackend>>) -> Self {
        let mode = if running_inside_xp {
            let backend = backend.or_else(take_pending_c_backend);
            if backend.is_some() {
                tracing::info!("registered host backend; queries will route through it");
            } else {
                tracing::debug!(inside_xp = true, "resolved host-backend environment");
            }
            let slot = match backend {
                Some(backend) => OnceLock::from(backend),
                None => OnceLock::new(),
            };
            XpMode::Xp(slot)
        } else {
            if backend.is_some() {
                tracing::warn!(
                    "a host backend was registered but {} is not set, so it will not be used",
                    env_vars::SNOWFLAKE_RUNNING_INSIDE_XP
                );
            } else {
                tracing::debug!(inside_xp = false, "resolved host-backend environment");
            }
            XpMode::Http
        };
        Self { mode }
    }

    pub fn from_env(backend: Option<Arc<dyn SnowflakeBackend>>) -> Self {
        Self::new(env_running_inside_xp(), backend)
    }

    pub fn register(&self, backend: Arc<dyn SnowflakeBackend>) -> Result<(), BackendError> {
        match &self.mode {
            XpMode::Http => {
                tracing::warn!(
                    "a host backend was registered but {} is not set, so it will not be used",
                    env_vars::SNOWFLAKE_RUNNING_INSIDE_XP
                );
                Ok(())
            }
            XpMode::Xp(slot) => {
                slot.set(backend)
                    .map_err(|_| BackendError::already_registered())?;
                tracing::info!("registered host backend; queries will route through it");
                Ok(())
            }
        }
    }

    pub fn active(&self) -> Result<Option<&Arc<dyn SnowflakeBackend>>, BackendError> {
        match &self.mode {
            XpMode::Http => Ok(None),
            XpMode::Xp(slot) => slot
                .get()
                .map(Some)
                .ok_or_else(BackendError::not_registered),
        }
    }

    pub fn running_inside_xp(&self) -> bool {
        matches!(self.mode, XpMode::Xp(_))
    }
}

pub fn env_running_inside_xp() -> bool {
    std::env::var_os(env_vars::SNOWFLAKE_RUNNING_INSIDE_XP).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xp_backend::TestBackend;

    #[test]
    fn registered_backend_is_returned_inside_xp() {
        let backend: Arc<dyn SnowflakeBackend> = Arc::new(TestBackend);
        let slot = XpSlot::new(true, Some(Arc::clone(&backend)));

        let active = slot
            .active()
            .expect("registered backend should be active")
            .expect("XP mode should use the backend");

        assert!(Arc::ptr_eq(active, &backend));
    }

    #[test]
    fn missing_backend_fails_inside_xp() {
        let slot = XpSlot::new(true, None);

        let err = match slot.active() {
            Err(err) => err,
            Ok(_) => panic!("XP mode should require a backend"),
        };

        assert_eq!(err.code, crate::xp_backend::error_codes::NOT_REGISTERED);
    }

    #[test]
    fn ordinary_client_uses_http() {
        let slot = XpSlot::new(false, None);

        assert!(!slot.running_inside_xp());
        assert!(
            slot.active()
                .expect("HTTP mode should not require a backend")
                .is_none()
        );
    }

    #[test]
    fn xp_mode_is_independent_of_whether_a_backend_is_registered() {
        assert!(XpSlot::new(true, None).running_inside_xp());
        assert!(XpSlot::new(true, Some(Arc::new(TestBackend))).running_inside_xp());
    }

    #[test]
    fn injected_backend_is_ignored_outside_xp() {
        let slot = XpSlot::new(false, Some(Arc::new(TestBackend)));

        assert!(
            slot.active()
                .expect("HTTP mode should not require a backend")
                .is_none()
        );
    }

    #[test]
    fn late_registration_is_ignored_outside_xp() {
        let slot = XpSlot::new(false, None);
        slot.register(Arc::new(TestBackend))
            .expect("registration outside XP should not fail");

        assert!(
            slot.active()
                .expect("HTTP mode should not require a backend")
                .is_none()
        );
    }

    #[test]
    fn backend_cannot_be_replaced() {
        let slot = XpSlot::new(true, Some(Arc::new(TestBackend)));

        let err = slot
            .register(Arc::new(TestBackend))
            .expect_err("second registration should fail");

        assert_eq!(err.code, crate::xp_backend::error_codes::ALREADY_REGISTERED);
    }

    #[test]
    fn pending_c_backend_is_consumed_by_the_first_xp_slot_only() {
        let _isolation = TestCRegistration::isolate();

        register_backend(Arc::new(TestBackend)).expect("first park should succeed");
        let err = register_backend(Arc::new(TestBackend)).expect_err("second park should fail");
        assert_eq!(err.code, crate::xp_backend::error_codes::ALREADY_REGISTERED);

        let first = XpSlot::new(true, None);
        first
            .active()
            .expect("pending backend should attach")
            .expect("XP mode should use the pending backend");

        let second = XpSlot::new(true, None);
        let err = match second.active() {
            Err(err) => err,
            Ok(_) => panic!("second slot should not see the consumed pending backend"),
        };
        assert_eq!(err.code, crate::xp_backend::error_codes::NOT_REGISTERED);
    }

    #[test]
    fn http_slot_does_not_consume_pending_c_backend() {
        let _isolation = TestCRegistration::isolate();
        let backend: Arc<dyn SnowflakeBackend> = Arc::new(TestBackend);

        register_backend(Arc::clone(&backend)).expect("parking should succeed");
        let http = XpSlot::new(false, None);
        assert!(
            http.active()
                .expect("HTTP mode should not require a backend")
                .is_none()
        );

        let xp = XpSlot::new(true, None);
        let active = xp
            .active()
            .expect("pending backend should attach")
            .expect("XP mode should use the pending backend");
        assert!(Arc::ptr_eq(active, &backend));
    }

    #[test]
    fn late_registration_attaches_before_first_use() {
        let slot = XpSlot::new(true, None);
        let backend: Arc<dyn SnowflakeBackend> = Arc::new(TestBackend);
        slot.register(Arc::clone(&backend))
            .expect("registration should succeed");

        let active = slot
            .active()
            .expect("registered backend should be active")
            .expect("XP mode should use the backend");
        assert!(Arc::ptr_eq(active, &backend));
    }
}
