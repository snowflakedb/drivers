//! Absolute deadline for one logical transfer request.
//!
//! Callers construct one [`Deadline`] at the start of a work unit (file,
//! part, range, or lifecycle request) and pass it through admission, local
//! I/O, HTTP retries, backoff, and credential refresh of that same unit.
//! File-level units start the clock in `admit_file_work` *before* the
//! acquire waits, so queue time and local preprocess share
//! `RetryPolicy.max_elapsed` with HEAD and the whole-object send.

use super::transfer::{AcquireError, RequestKind, RequestPermit, TransferOp};
use std::fmt;
use std::future::Future;
use std::time::Duration;
use tokio::time::Instant;

/// Start instant plus optional absolute expiry for one logical request.
#[derive(Clone, Copy, Debug)]
pub struct Deadline {
    start: Instant,
    until: Option<(Instant, Duration)>,
}

/// Remaining time against a [`Deadline`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Remaining {
    Unlimited,
    Left(Duration),
    Expired { elapsed: Duration, budget: Duration },
}

/// Why a deadline-bounded [`Deadline::acquire`] failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcquireWaitError {
    Acquire(AcquireError),
    DeadlineExceeded { elapsed: Duration, budget: Duration },
}

impl fmt::Display for AcquireWaitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Acquire(err) => write!(f, "{err}"),
            Self::DeadlineExceeded { elapsed, budget } => {
                write!(f, "deadline exceeded after {elapsed:?} (budget {budget:?})")
            }
        }
    }
}

impl std::error::Error for AcquireWaitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Acquire(err) => Some(err),
            Self::DeadlineExceeded { .. } => None,
        }
    }
}

impl Deadline {
    /// Starts a deadline from `RetryPolicy.max_elapsed`. `None` is unlimited.
    pub fn from_budget(budget: Option<Duration>) -> Self {
        let start = Instant::now();
        Self {
            start,
            until: budget.map(|b| (start + b, b)),
        }
    }

    pub fn elapsed(self) -> Duration {
        self.start.elapsed()
    }

    pub fn budget(self) -> Option<Duration> {
        self.until.map(|(_, budget)| budget)
    }

    pub fn remaining(self) -> Remaining {
        let Some((until, budget)) = self.until else {
            return Remaining::Unlimited;
        };
        let now = Instant::now();
        if now >= until {
            Remaining::Expired {
                elapsed: self.elapsed(),
                budget,
            }
        } else {
            Remaining::Left(until - now)
        }
    }

    pub fn check<E>(self, on_deadline: impl FnOnce(Duration, Duration) -> E) -> Result<(), E> {
        match self.remaining() {
            Remaining::Expired { elapsed, budget } => Err(on_deadline(elapsed, budget)),
            Remaining::Unlimited | Remaining::Left(_) => Ok(()),
        }
    }

    pub fn timeout_at(self) -> Option<Instant> {
        self.until.map(|(until, _)| until)
    }

    pub async fn run<T, E>(
        self,
        fut: impl Future<Output = T>,
        on_deadline: impl FnOnce(Duration, Duration) -> E,
    ) -> Result<T, E> {
        match self.until {
            None => Ok(fut.await),
            Some((until, budget)) => match tokio::time::timeout_at(until, fut).await {
                Ok(value) => Ok(value),
                Err(_) => Err(on_deadline(self.elapsed(), budget)),
            },
        }
    }

    /// Acquires one permit, bounded by this deadline.
    pub(crate) async fn acquire(
        self,
        transfer: &TransferOp,
        kind: RequestKind,
    ) -> Result<RequestPermit, AcquireWaitError> {
        self.check(|elapsed, budget| AcquireWaitError::DeadlineExceeded { elapsed, budget })?;
        match self
            .run(transfer.acquire(kind), |elapsed, budget| {
                AcquireWaitError::DeadlineExceeded { elapsed, budget }
            })
            .await
        {
            Ok(Ok(permit)) => Ok(permit),
            Ok(Err(err)) => Err(AcquireWaitError::Acquire(err)),
            Err(err) => Err(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governor::Governor;

    #[test]
    fn unlimited_deadline_never_expires() {
        let deadline = Deadline::from_budget(None);
        assert!(matches!(deadline.remaining(), Remaining::Unlimited));
        assert!(deadline.check(|_, _| "expired").is_ok());
    }

    #[tokio::test]
    async fn expired_budget_fails_check_immediately() {
        let deadline = Deadline::from_budget(Some(Duration::from_millis(1)));
        tokio::time::sleep(Duration::from_millis(5)).await;
        assert!(matches!(deadline.remaining(), Remaining::Expired { .. }));
        assert_eq!(deadline.check(|_, _| "expired"), Err("expired"));
    }

    #[tokio::test]
    async fn queue_wait_alone_exhausts_deadline() {
        let governor = Governor::try_with_capacity(1).expect("test capacity");
        let holder = governor.register_command(1);
        let blocked = governor.register_command(1);
        let _hold = holder.acquire(RequestKind::WholeObjectPut).await.unwrap();
        let deadline = Deadline::from_budget(Some(Duration::from_millis(40)));
        let err = deadline
            .acquire(&blocked, RequestKind::WholeObjectPut)
            .await
            .expect_err("queued acquire must expire");
        assert!(
            matches!(err, AcquireWaitError::DeadlineExceeded { .. }),
            "expected DeadlineExceeded, got {err:?}"
        );
    }
}
