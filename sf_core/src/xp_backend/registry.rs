//! Write-once host-transport slot owned by [`crate::apis::database_driver_v1::DatabaseDriverV1`].
//!
//! Lookups are `OnceLock::get` after init. The C registration entry point
//! attaches by calling [`crate::apis::database_driver_v1::DatabaseDriverV1::register_xp_backend`]
//! on the wrapper's existing driver instance.

use std::sync::{Arc, OnceLock};

use super::{BackendError, SnowflakeBackend};
use crate::env_vars;

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

        assert!(
            slot.active()
                .expect("HTTP mode should not require a backend")
                .is_none()
        );
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
