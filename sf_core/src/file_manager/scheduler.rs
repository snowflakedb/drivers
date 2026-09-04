use std::sync::Arc;

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use super::multipart::MultipartParams;

#[derive(Clone)]
pub struct TransferScheduler {
    /// Nothing closes this semaphore: the field is private to this module, the
    /// only `Arc` clones it hands out are the ones held inside a
    /// [`RequestTicket`], and no method here calls `close()`. That is what makes
    /// the acquire in [`Self::acquire_request`] infallible.
    requests: Arc<Semaphore>,
    multipart: MultipartParams,
}

impl TransferScheduler {
    pub(crate) fn for_command(multipart: MultipartParams) -> Self {
        Self {
            requests: Arc::new(Semaphore::new(multipart.command_parallel.max(1))),
            multipart,
        }
    }

    pub(super) fn multipart(&self) -> MultipartParams {
        self.multipart
    }

    /// The returned [`RequestTicket`] must stay alive for the duration of the
    /// request; dropping it returns the slot.
    pub(super) async fn acquire_request(&self) -> RequestTicket {
        let permit = Arc::clone(&self.requests).acquire_owned().await.expect(
            "the request semaphore was closed, which nothing is allowed to do: \
             this budget lives and dies with one PUT/GET command. Find the \
             close() call on it and remove it.",
        );
        RequestTicket { _permit: permit }
    }
}

pub(super) struct RequestTicket {
    _permit: OwnedSemaphorePermit,
}

#[cfg(test)]
mod tests {
    use super::super::cloud_http::{CloudSpillTarget, CloudSpilledBody, assemble_ranged_download};
    use super::super::types::TransferCtx;
    use super::*;
    use bytes::Bytes;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    fn params(parallel: i64) -> MultipartParams {
        MultipartParams::from_server(None, Some(parallel))
    }

    fn scheduler(parallel: i64) -> TransferScheduler {
        TransferScheduler::for_command(params(parallel))
    }

    #[tokio::test]
    async fn should_admit_up_to_the_command_budget() {
        let s = scheduler(3);
        let _t1 = s.acquire_request().await;
        let _t2 = s.acquire_request().await;
        let _t3 = s.acquire_request().await;
        assert_eq!(
            s.requests.available_permits(),
            0,
            "three tickets must exhaust a budget of three"
        );
    }

    #[tokio::test]
    async fn should_block_beyond_the_command_budget_until_a_ticket_drops() {
        let s = scheduler(1);
        let ticket = s.acquire_request().await;

        let waiter = tokio::spawn({
            let s = s.clone();
            async move { s.acquire_request().await }
        });
        tokio::task::yield_now().await;
        assert!(!waiter.is_finished(), "waiter must block on a full budget");

        drop(ticket);
        tokio::time::timeout(Duration::from_secs(1), waiter)
            .await
            .expect("dropping a ticket must wake the queued request")
            .expect("waiter task must not panic");
    }

    #[tokio::test]
    async fn should_return_the_slot_when_a_ticket_drops() {
        let s = scheduler(2);
        let ticket = s.acquire_request().await;
        assert_eq!(s.requests.available_permits(), 1);

        drop(ticket);
        assert_eq!(
            s.requests.available_permits(),
            2,
            "the slot must return to the budget"
        );
    }

    #[tokio::test]
    async fn should_not_truncate_a_server_parallel_above_the_part_ceiling() {
        let multipart = params(50);
        assert_eq!(
            multipart.concurrency, 16,
            "per-file part concurrency stays clamped"
        );

        let s = TransferScheduler::for_command(multipart);
        assert_eq!(
            s.requests.available_permits(),
            50,
            "the command budget must honour the server's parallel in full"
        );
    }

    #[tokio::test]
    async fn should_treat_zero_parallel_as_one() {
        let zero = MultipartParams {
            command_parallel: 0,
            ..MultipartParams::default()
        };
        let s = TransferScheduler::for_command(zero);
        assert_eq!(s.requests.available_permits(), 1);
        let _ticket = s.acquire_request().await;
    }

    #[test]
    fn should_carry_the_multipart_knobs_it_was_built_from() {
        let multipart = MultipartParams::from_server(Some(4096), Some(3));
        let s = TransferScheduler::for_command(multipart);
        assert_eq!(s.multipart().threshold.bytes(), 4096);
        assert_eq!(s.multipart().concurrency, 3);
    }

    /// The per-cloud upload leaves resolve their budget from `tx` rather than
    /// receiving one, so a batch caller's ceiling only holds if resolving again
    /// shares the caller's semaphore instead of minting a second budget.
    #[tokio::test]
    async fn should_share_the_callers_budget_when_the_ctx_carries_one() {
        let batch = scheduler(2);
        let tx = TransferCtx::default().with_scheduler(&batch);

        // A wider `multipart` than the batch was built with: the fallback must
        // not be consulted at all when the ctx already carries a budget.
        let resolved = super::super::scheduler_for(tx, params(50));
        assert_eq!(
            resolved.requests.available_permits(),
            2,
            "resolving must adopt the caller's budget, not the fallback's 50"
        );

        let _ticket = resolved.acquire_request().await;
        assert_eq!(
            batch.requests.available_permits(),
            1,
            "a ticket taken through the resolved scheduler must consume the \
             caller's slot, else each file would get a private budget"
        );
    }

    #[tokio::test]
    async fn should_mint_a_batch_of_one_when_the_ctx_carries_no_budget() {
        let resolved = super::super::scheduler_for(TransferCtx::default(), params(7));
        assert_eq!(
            resolved.requests.available_permits(),
            7,
            "a caller with no batch gets a budget sized from its own params"
        );
    }

    /// Tracks concurrent entries into a critical section: `max` ends up holding
    /// the high-water mark of simultaneous holders.
    #[derive(Default)]
    struct ConcurrencyProbe {
        in_flight: AtomicUsize,
        max: AtomicUsize,
    }

    impl ConcurrencyProbe {
        fn enter(&self) {
            let now = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
            self.max.fetch_max(now, Ordering::SeqCst);
        }

        fn leave(&self) {
            self.in_flight.fetch_sub(1, Ordering::SeqCst);
        }

        fn peak(&self) -> usize {
            self.max.load(Ordering::SeqCst)
        }
    }

    /// Runs `assemble_ranged_download` over a `total`-byte object in
    /// `chunk_size` ranges, serving each range from a synthetic payload while
    /// recording how many range fetches overlap. Returns the assembled bytes and
    /// the observed peak concurrency.
    ///
    /// The `yield_now` inside each fetch is what makes overlap possible at all:
    /// without an await point the futures would complete one at a time no matter
    /// how wide the budget is, and the test would pass vacuously.
    async fn assemble_with_probe(
        scheduler: &TransferScheduler,
        total: u64,
        chunk_size: u64,
    ) -> (Vec<u8>, usize) {
        let payload: Vec<u8> = (0..total).map(|i| (i % 251) as u8).collect();
        let probe = ConcurrencyProbe::default();
        let dir = tempfile::tempdir().expect("tempdir");
        let part_path = dir.path().join("out.bin.part");

        let body = assemble_ranged_download(
            total,
            chunk_size,
            scheduler,
            CloudSpillTarget::Part(&part_path),
            /* unsafe_file_write */ false,
            |detail: String| -> String { detail },
            |detail: String| -> String { detail },
            |range: super::super::multipart::DownloadRange| {
                let payload = &payload;
                let probe = &probe;
                async move {
                    probe.enter();
                    // Range 0 finishes last, so a real out-of-order completion is
                    // exercised, not just overlap.
                    if range.start == 0 {
                        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                    } else {
                        tokio::task::yield_now().await;
                    }
                    let bytes =
                        Bytes::copy_from_slice(&payload[range.start as usize..=range.end as usize]);
                    probe.leave();
                    Ok::<Bytes, String>(bytes)
                }
            },
        )
        .await
        .expect("ranged assembly should succeed");

        let path = match body {
            CloudSpilledBody::Part(p) => p,
            CloudSpilledBody::Temp(_) => panic!("Part target must produce a Part body"),
        };
        (
            std::fs::read(path).expect("read assembled file"),
            probe.peak(),
        )
    }

    /// For a *single* file the command budget and `buffer_unordered`'s width are
    /// both `data.parallel`, so this passes with or without the scheduler — it is
    /// a regression guard on the pre-existing per-file bound. That the budget
    /// binds when it is *narrower* than the stream's width is pinned by
    /// `should_let_the_command_budget_bind_below_the_stream_width`.
    #[tokio::test(flavor = "multi_thread")]
    async fn should_not_exceed_the_command_budget_when_fetching_ranges() {
        // 8 ranges, budget of 2: at most two ranged GETs may be in flight.
        let s = scheduler(2);
        let (assembled, peak) = assemble_with_probe(&s, 800, 100).await;

        assert_eq!(assembled.len(), 800, "every range must land in the file");
        assert!(
            peak <= 2,
            "ranged GETs exceeded the request budget: peak {peak} > 2"
        );
        assert!(
            peak > 1,
            "a budget of 2 must actually overlap two fetches, else this test \
             would pass even with the budget ignored (peak {peak})"
        );
    }

    /// The scheduler, not `buffer_unordered`'s width, is the binding limit: the
    /// stream is willing to poll 8 ranges at once but the command budget admits
    /// 2. Without the scheduler this would peak at 8.
    #[tokio::test(flavor = "multi_thread")]
    async fn should_let_the_command_budget_bind_below_the_stream_width() {
        let s = TransferScheduler::for_command(MultipartParams {
            command_parallel: 2,
            ..params(8)
        });
        let (assembled, peak) = assemble_with_probe(&s, 800, 100).await;

        assert_eq!(assembled.len(), 800);
        assert!(
            peak <= 2,
            "the command budget must bind even when the stream width is wider: \
             peak {peak} > 2"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn should_release_request_slots_after_the_download_completes() {
        // A leaked permit would silently halve the budget for the next file in
        // the batch, so assert the budget is whole again afterwards.
        let s = scheduler(3);
        let _ = assemble_with_probe(&s, 500, 100).await;

        let (_, peak) = assemble_with_probe(&s, 500, 100).await;
        assert!(
            peak > 1,
            "the second download must still reach concurrency > 1, so no slots \
             leaked from the first (peak {peak})"
        );
    }

    /// `RequestTicket` releases on drop, which Rust runs on an early `?` return
    /// too — but that guarantee is only worth as much as the test that exercises
    /// it. Fails one range, then proves the scheduler is still whole by running a
    /// second, fully successful download through the same instance.
    #[tokio::test(flavor = "multi_thread")]
    async fn should_release_request_slots_when_a_range_fetch_fails() {
        let s = scheduler(2);
        let dir = tempfile::tempdir().expect("tempdir");
        let part_path = dir.path().join("out.bin.part");

        let result = assemble_ranged_download(
            400,
            100,
            &s,
            CloudSpillTarget::Part(&part_path),
            /* unsafe_file_write */ false,
            |detail: String| -> String { detail },
            |detail: String| -> String { detail },
            |range: super::super::multipart::DownloadRange| async move {
                tokio::task::yield_now().await;
                if range.start == 0 {
                    Err("synthetic range failure".to_string())
                } else {
                    let len = (range.end - range.start + 1) as usize;
                    Ok::<Bytes, String>(Bytes::from(vec![0u8; len]))
                }
            },
        )
        .await;

        assert!(
            result.is_err(),
            "the failing range must surface as an error"
        );

        let (_, peak) = assemble_with_probe(&s, 500, 100).await;
        assert!(
            peak > 1,
            "a slot leaked on the failure path: peak {peak} after a prior fetch failed"
        );
    }
}
