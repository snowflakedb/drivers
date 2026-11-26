//! Transfer admission scheduler for logical cloud request retry sequences.
//!
//! "Process-wide" means every default-constructed
//! [`crate::apis::database_driver_v1::DatabaseDriverV1`] in **one loaded
//! `sf_core` image** shares [`process_governor`]. Independently linked copies
//! of this crate each have their own `OnceLock`. An explicit
//! [`crate::apis::database_driver_v1::DriverProviders::governor`] override
//! isolates that driver (and any other roots given the same clone) from the
//! default pool — that is how tests inject a private cap.
//!
//! This module owns only transfer admission today. The scheduler grants RAII
//! permits; callers execute requests and own retry policy. Production is
//! bounded per command by `data.parallel` (the command limit in the
//! central scheduler). Process capacity is unlimited until a test or harness
//! injects [`Governor::try_with_capacity`].
//!
//! ## Permit lifetime
//!
//! The caller owns the permit and passes it into the leaf. One permit covers
//! one logical unit through its complete retry sequence: local work, send,
//! response classification, retry backoff, credential/SAS refresh of that
//! same unit, and terminal error-body reads. Do not drop and re-acquire
//! mid-retry or mid-refresh: re-acquiring caused hold-and-wait deadlocks.
//! Refresh of that unit never waits on the stage pool. Same-file HEAD and
//! the following data request share one worker permit. Multipart and ranged
//! work drop the file permit after Create/Initiate or dest setup; each part
//! or range then acquires its own. A backoff that cannot finish before the
//! deadline fails immediately instead of sleeping while still holding the
//! slot.

mod deadline;
mod transfer;

use std::num::NonZeroUsize;
use std::sync::OnceLock;

pub(crate) use deadline::Remaining;
pub use deadline::{AcquireWaitError, Deadline};
pub use transfer::{AcquireError, CommandParallelism, TransferOp};
pub(crate) use transfer::{RequestKind, RequestPermit};

/// Reserved cleanup slots beyond an explicit ordinary process cap so
/// abort/delete can make progress when every ordinary slot is held.
/// Unused while process capacity is unlimited (the default).
pub(crate) const DEFAULT_CLEANUP_RESERVE: usize = 1;

static PROCESS_GOVERNOR: OnceLock<Governor> = OnceLock::new();

/// Default [`Governor`] for this loaded `sf_core` image.
///
/// Ordinary process capacity is unlimited. Command `data.parallel` is the
/// production bound until a test or perf harness injects
/// [`Governor::try_with_capacity`] via
/// [`crate::apis::database_driver_v1::DriverProviders::governor`].
pub fn process_governor() -> Governor {
    PROCESS_GOVERNOR.get_or_init(Governor::new).clone()
}

/// Rejected [`Governor::try_with_capacity`] argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZeroCapacityError;

impl std::fmt::Display for ZeroCapacityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("governor capacity must be greater than zero")
    }
}

impl std::error::Error for ZeroCapacityError {}

/// Admission scheduler for logical cloud requests, including retries/backoff.
///
/// Awaitable from any existing tokio runtime: registration is synchronous,
/// and [`TransferOp::acquire`] only waits for a slot. Clone shares the same
/// inner scheduler.
#[derive(Clone, Debug)]
pub struct Governor {
    inner: std::sync::Arc<transfer::Inner>,
}

impl Governor {
    /// Unlimited ordinary process capacity. Each command is still bounded by
    /// `data.parallel` / [`CommandParallelism`].
    pub fn new() -> Self {
        Self {
            inner: std::sync::Arc::new(transfer::Inner::new(None, DEFAULT_CLEANUP_RESERVE)),
        }
    }

    /// Isolated scheduler for tests and [`crate::apis::database_driver_v1::DriverProviders`] injection.
    ///
    /// Sets an explicit ordinary process cap. Returns [`ZeroCapacityError`]
    /// when `max_in_flight` is 0.
    pub fn try_with_capacity(max_in_flight: usize) -> Result<Self, ZeroCapacityError> {
        let max = NonZeroUsize::new(max_in_flight).ok_or(ZeroCapacityError)?;
        Ok(Self {
            inner: std::sync::Arc::new(transfer::Inner::new(Some(max), DEFAULT_CLEANUP_RESERVE)),
        })
    }

    /// True when `self` and `other` are the same scheduler instance.
    pub fn ptr_eq(this: &Self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&this.inner, &other.inner)
    }

    /// Ordinary process cap when one was set (`None` = unlimited).
    /// Does not include reserved cleanup slots.
    pub fn capacity(&self) -> Option<usize> {
        self.inner.capacity()
    }

    /// Currently granted permits (ordinary + reserved cleanup). Test/telemetry aid.
    pub fn in_flight(&self) -> usize {
        self.inner.in_flight()
    }

    /// Granted reserved cleanup permits. Test/telemetry aid.
    #[cfg(test)]
    pub(crate) fn cleanup_reserved_in_flight(&self) -> usize {
        self.inner.cleanup_reserved_in_flight()
    }

    /// Waiters queued across all live operations. Test/telemetry aid.
    pub fn queued(&self) -> usize {
        self.inner.queued()
    }

    /// Register one PUT/GET command. Dropping the last handle clone cancels
    /// queued waiters; already-granted permits remain valid until dropped.
    pub(crate) fn register_transfer(&self, command_parallel: CommandParallelism) -> TransferOp {
        transfer::TransferOp::register(std::sync::Arc::clone(&self.inner), command_parallel)
    }

    /// Register a command from a resolved `data.parallel` integer.
    pub(crate) fn register_command(&self, parallel: usize) -> TransferOp {
        self.register_transfer(CommandParallelism::from_resolved(parallel))
    }

    #[cfg(test)]
    fn enqueue_closed_receiver(&self, op: &TransferOp) {
        let (tx, rx) = tokio::sync::oneshot::channel();
        drop(rx);
        let _ = transfer::Inner::enqueue(&self.inner, op.op_id(), RequestKind::WholeObjectPut, tx);
    }
}

impl Default for Governor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod process_tests {
    use super::*;

    #[test]
    fn process_governor_is_a_single_instance() {
        let a = process_governor();
        let b = process_governor();
        assert!(
            Governor::ptr_eq(&a, &b),
            "process_governor() must return the same instance"
        );
    }

    #[test]
    fn try_with_capacity_rejects_zero() {
        assert!(matches!(
            Governor::try_with_capacity(0),
            Err(ZeroCapacityError)
        ));
        assert_eq!(Governor::try_with_capacity(1).unwrap().capacity(), Some(1));
    }

    #[test]
    fn new_and_process_governor_have_unlimited_capacity() {
        assert_eq!(Governor::new().capacity(), None);
        assert_eq!(process_governor().capacity(), None);
    }
}
