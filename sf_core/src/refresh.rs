//! Generic refresh-and-retry pattern for refreshable resources (auth tokens,
//! cloud-stage credentials, etc.).
//!
//! Consumers follow the same shape:
//!
//! 1. read the current resource,
//! 2. run an operation that consumes it,
//! 3. on a recoverable error, rotate the resource and retry.
//!
//! What varies per consumer — which error is recoverable, where the resource
//! is cached, how rapid-fire refreshes are *coalesced* (collapsed into one
//! upstream call when they arrive close together) — lives entirely in the
//! `Refresher` impl. The loop itself is shared.
//!
//! # Termination
//!
//! `execute_with_refresh` retries at most as many times as the refresher
//! agrees to rotate. A refresher signals "no new rotation" by returning
//! `Ok(false)` from `refresh`, and the helper then propagates the original
//! error. Callers that want a stricter cap can encode it inside `refresh`.
//!
//! # Object safety
//!
//! The trait carries `Resource` and `Err` as type parameters (rather than
//! associated types) so that `dyn Refresher<Resource, Err>` works for paths
//! that need to thread a refresher through a heterogeneous call stack (the
//! S3 file-transfer path does this).

use std::future::Future;
use std::pin::Pin;

/// A future returned by `Refresher` methods. Boxed so the trait is dyn-safe.
pub type RefreshFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Owns a refreshable resource and rotates it on demand.
///
/// `Resource` is what the operation consumes per attempt — the implementation
/// chooses between handing out a clone of a cached value or fetching freshly
/// each call. `Err` is the error type the operation returns; `should_refresh`
/// inspects it to decide whether a retry is warranted.
pub trait Refresher<Resource, Err>: Send
where
    Resource: Send,
    Err: Send,
{
    /// Read the current resource for the next attempt. Called once per
    /// iteration of `execute_with_refresh`.
    fn current(&mut self) -> RefreshFuture<'_, Result<Resource, Err>>;

    /// Whether `err` is the recoverable kind that warrants a refresh +
    /// retry. Non-recoverable errors propagate up immediately.
    fn should_refresh(&self, err: &Err) -> bool;

    /// Rotate the resource. Returns:
    ///
    /// - `Ok(true)` — rotation happened; the helper retries with the new resource.
    /// - `Ok(false)` — no new rotation (coalesced within the window or budget
    ///   exhausted); the helper propagates the original error.
    /// - `Err(e)` — the refresh itself failed; this is terminal.
    fn refresh(&mut self) -> RefreshFuture<'_, Result<bool, Err>>;
}

/// Run `operation` against `refresher`'s resource, refreshing once on
/// recoverable errors. Loops until either `operation` succeeds, the error
/// is non-recoverable, or the refresher declines to rotate again.
pub async fn execute_with_refresh<R, Resource, Err, Op, Fut, T>(
    refresher: &mut R,
    operation: Op,
) -> Result<T, Err>
where
    R: Refresher<Resource, Err> + ?Sized,
    Resource: Send,
    Err: Send,
    Op: Fn(Resource) -> Fut,
    Fut: Future<Output = Result<T, Err>>,
{
    loop {
        let resource = refresher.current().await?;
        let err = match operation(resource).await {
            Ok(value) => return Ok(value),
            Err(e) => e,
        };
        if !refresher.should_refresh(&err) {
            return Err(err);
        }
        if !refresher.refresh().await? {
            // Refresher declined to rotate: coalescing window or exhausted
            // budget. Retrying without a fresh resource would loop.
            return Err(err);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Debug, PartialEq)]
    enum TestErr {
        Recoverable,
        Fatal,
    }

    /// A minimal refresher used by the unit tests below. Holds a single
    /// integer resource that increments each time `refresh` rotates.
    struct CountingRefresher {
        value: i32,
        refresh_calls: i32,
        max_refreshes: i32,
        recoverable_errors: bool,
    }

    impl CountingRefresher {
        fn new(max_refreshes: i32) -> Self {
            Self {
                value: 0,
                refresh_calls: 0,
                max_refreshes,
                recoverable_errors: true,
            }
        }
    }

    impl Refresher<i32, TestErr> for CountingRefresher {
        fn current(&mut self) -> RefreshFuture<'_, Result<i32, TestErr>> {
            let v = self.value;
            Box::pin(async move { Ok(v) })
        }

        fn should_refresh(&self, err: &TestErr) -> bool {
            self.recoverable_errors && matches!(err, TestErr::Recoverable)
        }

        fn refresh(&mut self) -> RefreshFuture<'_, Result<bool, TestErr>> {
            Box::pin(async move {
                if self.refresh_calls >= self.max_refreshes {
                    return Ok(false);
                }
                self.refresh_calls += 1;
                self.value += 1;
                Ok(true)
            })
        }
    }

    #[tokio::test]
    async fn returns_first_success_without_refresh() {
        let mut r = CountingRefresher::new(1);
        let calls = Mutex::new(0);
        let result: Result<i32, TestErr> = execute_with_refresh(&mut r, |v| {
            *calls.lock().unwrap() += 1;
            async move { Ok(v + 100) }
        })
        .await;
        assert_eq!(result.unwrap(), 100);
        assert_eq!(*calls.lock().unwrap(), 1);
        assert_eq!(r.refresh_calls, 0);
    }

    #[tokio::test]
    async fn refreshes_once_then_succeeds() {
        let mut r = CountingRefresher::new(1);
        let calls = Mutex::new(0);
        let result: Result<i32, TestErr> = execute_with_refresh(&mut r, |v| {
            let n = {
                let mut g = calls.lock().unwrap();
                *g += 1;
                *g
            };
            async move {
                if n == 1 {
                    Err(TestErr::Recoverable)
                } else {
                    Ok(v + 100)
                }
            }
        })
        .await;
        assert_eq!(result.unwrap(), 101); // resource was rotated to 1
        assert_eq!(*calls.lock().unwrap(), 2);
        assert_eq!(r.refresh_calls, 1);
    }

    #[tokio::test]
    async fn propagates_when_refresher_declines_second_rotation() {
        let mut r = CountingRefresher::new(1); // only one refresh allowed
        let calls = Mutex::new(0);
        let result: Result<i32, TestErr> = execute_with_refresh(&mut r, |_v| {
            *calls.lock().unwrap() += 1;
            async { Err(TestErr::Recoverable) }
        })
        .await;
        assert_eq!(result.unwrap_err(), TestErr::Recoverable);
        // First op fails → refresh #1 → retry op fails → refresh attempt #2
        // returns Ok(false) → propagate.
        assert_eq!(*calls.lock().unwrap(), 2);
        assert_eq!(r.refresh_calls, 1);
    }

    #[tokio::test]
    async fn propagates_non_recoverable_error_without_refresh() {
        let mut r = CountingRefresher::new(5);
        let result: Result<i32, TestErr> =
            execute_with_refresh(&mut r, |_v| async { Err(TestErr::Fatal) }).await;
        assert_eq!(result.unwrap_err(), TestErr::Fatal);
        assert_eq!(r.refresh_calls, 0);
    }
}
