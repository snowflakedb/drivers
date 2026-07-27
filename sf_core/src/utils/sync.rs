//! Extension traits for `std::sync` locks that recover from poisoning instead
//! of panicking.
//!
//! A `std::sync` lock is *poisoned* when a thread panics while holding its
//! guard; every subsequent `lock()`/`read()`/`write()` then returns `Err`.
//! The protected data is still present inside the [`std::sync::PoisonError`],
//! so these helpers recover it via `into_inner()` and keep going, rather than
//! letting one thread's panic cascade into crashing every other thread that
//! shares the lock — which matters for a driver loaded into a host process
//! over FFI, where a manufactured panic is at best turned into an error at the
//! boundary and at worst undefined behavior.
//!
//! Recovery is deliberately **not silent**: each occurrence is logged at ERROR
//! with the call site, because a poisoned lock always means a panic already
//! happened somewhere and the recovered data may be inconsistent.

use std::panic::Location;
use std::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// `tracing` target for poison-recovery diagnostics.
const POISON_TARGET: &str = "sf_core::sync";

/// [`Mutex`] extension that recovers from lock poisoning.
pub trait MutexRecoverExt<T: ?Sized> {
    /// Locks the mutex, recovering the guard if the lock is poisoned.
    ///
    /// On the happy path this is exactly `mutex.lock()`. If the lock is
    /// poisoned it logs at ERROR (with the call site) and returns the inner
    /// guard via [`std::sync::PoisonError::into_inner`] instead of panicking.
    fn lock_recover(&self) -> MutexGuard<'_, T>;
}

impl<T: ?Sized> MutexRecoverExt<T> for Mutex<T> {
    #[track_caller]
    fn lock_recover(&self) -> MutexGuard<'_, T> {
        let caller = Location::caller();
        self.lock().unwrap_or_else(|poisoned| {
            tracing::error!(
                target: POISON_TARGET,
                caller = %caller,
                "recovered a poisoned Mutex (a thread previously panicked while \
                 holding it); continuing with possibly-inconsistent data",
            );
            poisoned.into_inner()
        })
    }
}

/// [`RwLock`] extension that recovers from lock poisoning.
pub trait RwLockRecoverExt<T: ?Sized> {
    /// Read-locks the `RwLock`, recovering the guard if it is poisoned.
    ///
    /// On poison, logs at ERROR (with the call site) and returns the inner
    /// guard via [`std::sync::PoisonError::into_inner`] instead of panicking.
    fn read_recover(&self) -> RwLockReadGuard<'_, T>;

    /// Write-locks the `RwLock`, recovering the guard if it is poisoned.
    ///
    /// On poison, logs at ERROR (with the call site) and returns the inner
    /// guard via [`std::sync::PoisonError::into_inner`] instead of panicking.
    fn write_recover(&self) -> RwLockWriteGuard<'_, T>;
}

impl<T: ?Sized> RwLockRecoverExt<T> for RwLock<T> {
    #[track_caller]
    fn read_recover(&self) -> RwLockReadGuard<'_, T> {
        let caller = Location::caller();
        self.read().unwrap_or_else(|poisoned| {
            tracing::error!(
                target: POISON_TARGET,
                caller = %caller,
                "recovered a poisoned RwLock on read (a thread previously panicked \
                 while holding the write guard); continuing with \
                 possibly-inconsistent data",
            );
            poisoned.into_inner()
        })
    }

    #[track_caller]
    fn write_recover(&self) -> RwLockWriteGuard<'_, T> {
        let caller = Location::caller();
        self.write().unwrap_or_else(|poisoned| {
            tracing::error!(
                target: POISON_TARGET,
                caller = %caller,
                "recovered a poisoned RwLock on write (a thread previously panicked \
                 while holding a guard); continuing with possibly-inconsistent data",
            );
            poisoned.into_inner()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// Poisons `lock` by panicking on a spawned thread while holding its guard.
    fn poison_mutex(lock: &Arc<Mutex<i32>>) {
        let clone = Arc::clone(lock);
        let _ = std::thread::spawn(move || {
            let _guard = clone.lock().expect("first lock cannot be poisoned yet");
            panic!("intentional panic to poison the mutex");
        })
        .join();
    }

    #[test]
    fn lock_recover_returns_guard_on_healthy_mutex() {
        let lock = Mutex::new(7);
        let guard = lock.lock_recover();
        assert_eq!(*guard, 7);
    }

    #[test]
    fn lock_recover_recovers_data_from_poisoned_mutex() {
        let lock = Arc::new(Mutex::new(42));
        poison_mutex(&lock);
        assert!(lock.lock().is_err(), "precondition: lock is poisoned");

        // Recovery returns the guard (not a panic) with the data intact.
        let guard = lock.lock_recover();
        assert_eq!(*guard, 42);
    }

    #[test]
    fn write_recover_recovers_data_from_poisoned_rwlock() {
        let lock = Arc::new(RwLock::new(String::from("data")));
        let clone = Arc::clone(&lock);
        let _ = std::thread::spawn(move || {
            let mut guard = clone.write().expect("first write cannot be poisoned yet");
            guard.push_str("-mutated");
            panic!("intentional panic to poison the rwlock");
        })
        .join();
        assert!(lock.read().is_err(), "precondition: lock is poisoned");

        // Both read and write recover the (mutated, possibly-inconsistent) data.
        assert_eq!(&*lock.read_recover(), "data-mutated");
        assert_eq!(&*lock.write_recover(), "data-mutated");
    }
}
