use std::sync::Arc;

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// Counts batches handed from the prefetch pipeline to a reader. A chunk larger
/// than the limit takes every permit, then releases permits only once its
/// remaining batch count falls below the limit.
pub(crate) struct BatchBudget {
    semaphore: Arc<Semaphore>,
    limit: u32,
}

pub(crate) struct BatchTicket(Option<OwnedSemaphorePermit>);

impl BatchTicket {
    pub(crate) fn empty() -> Self {
        Self(None)
    }

    /// Release one permit when a yielded batch brings this chunk's remaining
    /// batches below the number of permits it still holds.
    pub(crate) fn batch_consumed(&mut self, remaining: usize) {
        if let Some(permit) = self.0.as_mut()
            && permit.num_permits() > remaining
        {
            drop(permit.split(1));
        }
    }
}

impl BatchBudget {
    pub(crate) fn new(limit: u32) -> Self {
        assert!(limit > 0, "result queue size must be positive");
        Self {
            semaphore: Arc::new(Semaphore::new(limit as usize)),
            limit,
        }
    }

    pub(crate) async fn acquire(&self, batches: usize) -> BatchTicket {
        let permits = batches.min(self.limit as usize) as u32;
        if permits == 0 {
            return BatchTicket::empty();
        }
        BatchTicket(Some(
            self.semaphore
                .clone()
                .acquire_many_owned(permits)
                .await
                .expect("semaphore is never closed"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn oversized_chunk_only_releases_capacity_for_unconsumed_batches() {
        let budget = BatchBudget::new(2);
        let mut ticket = budget.acquire(4).await;
        assert_eq!(budget.semaphore.available_permits(), 0);

        ticket.batch_consumed(3);
        ticket.batch_consumed(2);
        assert_eq!(budget.semaphore.available_permits(), 0);
        ticket.batch_consumed(1);
        assert_eq!(budget.semaphore.available_permits(), 1);
        ticket.batch_consumed(0);
        assert_eq!(budget.semaphore.available_permits(), 2);
    }

    #[tokio::test]
    async fn dropping_ticket_wakes_waiter() {
        let budget = BatchBudget::new(1);
        let ticket = budget.acquire(1).await;
        let waiter = tokio::spawn(async move { budget.acquire(1).await });
        tokio::task::yield_now().await;
        drop(ticket);
        tokio::time::timeout(Duration::from_secs(1), waiter)
            .await
            .expect("dropping a buffered chunk should wake the waiter")
            .expect("waiter should not panic");
    }
}
