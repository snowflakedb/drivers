use std::sync::Arc;

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// Tracks a megabyte-granularity memory budget for prefetched chunks.
///
/// Cheap to clone (inner state is behind [`Arc`]). Hand out [`MemoryTicket`]s
/// via [`acquire`](Self::acquire). Each ticket reserves `n` MB and
/// automatically returns them to the budget when dropped.
///
/// When a chunk is larger than the entire budget, [`acquire`](Self::acquire)
/// requests all available permits, effectively waiting for exclusive access.
/// This prevents deadlock on oversized chunks.
///
/// Backed by a [`tokio::sync::Semaphore`] (1 permit = 1 MB) which provides
/// FIFO-fair async waiting with no polling loops.
#[derive(Clone)]
pub(crate) struct MemoryBudget {
    semaphore: Arc<Semaphore>,
    limit_mb: u32,
}

/// RAII guard for a memory reservation.
///
/// Dropping the ticket returns its permits to the parent [`MemoryBudget`].
#[allow(dead_code)]
pub(crate) struct MemoryTicket(Option<OwnedSemaphorePermit>);

impl MemoryTicket {
    pub(crate) fn empty() -> Self {
        Self(None)
    }
}

impl MemoryBudget {
    pub(crate) fn new(limit_mb: u64) -> Self {
        let limit_mb = u32::try_from(limit_mb).unwrap_or(u32::MAX);
        Self {
            semaphore: Arc::new(Semaphore::new(limit_mb as usize)),
            limit_mb,
        }
    }

    /// Wait until `mb` worth of permits are available, then reserve them.
    ///
    /// If `mb` exceeds the total budget, all permits are acquired instead,
    /// which waits for exclusive access (no other tickets outstanding).
    pub(crate) async fn acquire(&self, mb: u64) -> MemoryTicket {
        if self.limit_mb == 0 {
            return MemoryTicket(None);
        }

        let permits = u32::try_from(mb).unwrap_or(u32::MAX).min(self.limit_mb);

        let permit = self
            .semaphore
            .clone()
            .acquire_many_owned(permits)
            .await
            .expect("semaphore is never closed");

        MemoryTicket(Some(permit))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn available(budget: &MemoryBudget) -> usize {
        budget.semaphore.available_permits()
    }

    #[tokio::test]
    async fn unlimited_budget() {
        let budget = MemoryBudget::new(0);
        let _ticket = budget.acquire(1_000_000).await;
    }

    #[tokio::test]
    async fn first_acquire_exceeds_limit() {
        let budget = MemoryBudget::new(100);
        let ticket = budget.acquire(200).await;
        assert_eq!(available(&budget), 0);
        drop(ticket);
        assert_eq!(available(&budget), 100);
    }

    #[tokio::test]
    async fn within_limit() {
        let budget = MemoryBudget::new(1000);
        let _t1 = budget.acquire(500).await;
        let _t2 = budget.acquire(400).await;
        assert_eq!(available(&budget), 100);
    }

    #[tokio::test]
    async fn ticket_drop_frees_capacity() {
        let budget = MemoryBudget::new(1000);
        let ticket = budget.acquire(600).await;
        assert_eq!(available(&budget), 400);
        drop(ticket);
        assert_eq!(available(&budget), 1000);
    }

    #[tokio::test]
    async fn acquire_wakes_on_ticket_drop() {
        let budget = MemoryBudget::new(100);
        let ticket = budget.acquire(90).await;

        let budget_clone = budget.clone();
        let handle = tokio::spawn(async move { budget_clone.acquire(50).await });

        tokio::task::yield_now().await;
        drop(ticket);

        let _acquired = tokio::time::timeout(std::time::Duration::from_secs(1), handle)
            .await
            .expect("acquire should complete after ticket drop")
            .unwrap();
    }

    #[tokio::test]
    async fn no_outstanding_tickets_allows_exceeding_again() {
        let budget = MemoryBudget::new(100);
        let ticket = budget.acquire(200).await;
        assert_eq!(available(&budget), 0);
        drop(ticket);

        let ticket = budget.acquire(150).await;
        assert_eq!(available(&budget), 0);
        drop(ticket);
        assert_eq!(available(&budget), 100);
    }
}
