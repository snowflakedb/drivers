use crate::config::retry::RetryPolicy;
use crate::crl::config::CrlConfig;
use crate::crl::error::{
    CrlDistributionPointSnafu, CrlDownloadSnafu, CrlError, InvalidCrlSignatureSnafu,
    MutexPoisonedSnafu, VerificationTaskSnafu,
};
use crate::http::retry::{HttpContext, HttpError, execute_bytes_with_retry};
use chrono::{DateTime, Utc};
use fs2::FileExt;
use once_cell::sync::OnceCell;
use opentelemetry::metrics::{Counter, Histogram, Meter};
use opentelemetry::{KeyValue, global};
use reqwest::Method;
use sha2::{Digest, Sha256};
use snafu::ResultExt;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio_stream::StreamExt;
use tracing::instrument::WithSubscriber;

const TEMP_PREFIX: &str = ".crl-tmp-";
const TEMP_SUFFIX: &str = ".tmp";

/// Filename prefix for the sweeper's short-lived clock-probe temp files (see
/// [`crate::fs_lock::filesystem_now`] and `sweep_orphan_temp_files`).
/// Deliberately does NOT match the orphan temp pattern (`.crl-tmp-*.tmp`) nor a
/// 64-hex cache entry, so a probe momentarily visible to a concurrent sweeper
/// is never reaped nor read as a cache entry.
const PROBE_PREFIX: &str = ".crl-probe-";

/// Age past which an orphan temp is eligible for the lock-less age backstop.
/// Deliberately generous: it must dwarf a live write's duration, NFS
/// attribute-cache lag, and coarse mtime granularity. The sweep runs roughly
/// every 30 minutes, so a one-hour floor still reaps orphans promptly relative
/// to their (unbounded) lifetime while never threatening a live writer.
const ORPHAN_AGE_THRESHOLD: std::time::Duration = std::time::Duration::from_secs(60 * 60);

#[derive(Debug, Clone)]
pub struct CachedCrl {
    pub crl: Vec<u8>,
    pub download_time: DateTime<Utc>,
    pub url: String,
    pub expires_at: DateTime<Utc>,
    pub crl_number: Option<u128>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct OutcomeKey {
    serial: Vec<u8>,
    issuer_hash: Vec<u8>,
}

#[derive(Debug, Clone)]
struct OutcomeEntry {
    outcome: crate::tls::revocation::RevocationOutcome,
    expires_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct CrlCache {
    config: CrlConfig,
    memory_cache: Option<Arc<Mutex<HashMap<String, CachedCrl>>>>,
    outcome_cache: Option<Arc<Mutex<HashMap<OutcomeKey, OutcomeEntry>>>>,
    url_locks: Arc<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>,
    backoff: Arc<Mutex<HashMap<String, (u32, std::time::Instant)>>>,
    http_client: reqwest::Client,
    // Scheduler control channel to wake DelayQueue loop on updates
    scheduler_tx: OnceCell<tokio::sync::mpsc::Sender<SchedulerMsg>>,
    metrics: CrlMetrics,
}

#[derive(Debug)]
enum SchedulerMsg {
    Schedule(String),
}

#[derive(Debug, Clone)]
struct CrlMetrics {
    get_total: Counter<u64>,
    get_ms: Histogram<u64>,
    fetch_total: Counter<u64>,
    fetch_ms: Histogram<u64>,
    fetch_error_total: Counter<u64>,
}

/// Atomic write: write to a `tempfile::NamedTempFile` in `dir` (identifiable
/// `.crl-tmp-*` prefix and `.tmp` suffix), hold a best-effort advisory lock for
/// sweeper coordination, `sync_all`, then `persist` to `dir.join(file_name)`
/// (atomic replace: `rename(2)` on POSIX, `MoveFileExW` with
/// `MOVEFILE_REPLACE_EXISTING` on Windows). Concurrent processes/readers see
/// either the complete old file or the complete new file, never a torn one.
///
/// The caller passes `dir` and `file_name` separately (rather than a joined
/// path) so the temp file is always created in the same directory as its final
/// destination — a prerequisite for the rename to be atomic — with no
/// non-atomic fallback. We deliberately do NOT fsync the directory: it is not
/// portable (fails on Windows) and the disk cache is fully re-derivable, so a
/// rename lost to a crash just triggers a network re-fetch. On Windows,
/// `persist` cannot replace a target another process holds open (fails rather
/// than tears).
pub(crate) fn write_crl_atomic(dir: &Path, file_name: &str, data: &[u8]) -> std::io::Result<()> {
    let mut tmp = tempfile::Builder::new()
        .prefix(TEMP_PREFIX)
        .suffix(TEMP_SUFFIX)
        .tempfile_in(dir)?;
    if let Err(e) = tmp.as_file().try_lock_exclusive() {
        tracing::debug!(
            target: "sf_core::crl",
            error = %e,
            "Best-effort advisory lock on CRL temp file failed; continuing"
        );
    }
    tmp.as_file_mut().write_all(data)?;
    // `sync_all` flushes the temp's contents before the rename so a crash
    // cannot publish a rename pointing at unflushed (empty/partial) data.
    // Risk: this runs on the `spawn_blocking` write that `get()` awaits, so on
    // a hung/degraded shared filesystem the fsync can stall a revocation check
    // — the CRL fetch's network timeout does not cover it. Accepted because the
    // cache is fully re-derivable (a lost write only costs a re-fetch); revisit
    // (bounded timeout or fire-and-forget) if fsync stalls are seen in prod.
    tmp.as_file().sync_all()?;
    tmp.persist(dir.join(file_name)).map_err(|e| e.error)?;
    Ok(())
}

fn is_orphan_temp_name(name: &str) -> bool {
    name.starts_with(TEMP_PREFIX) && name.ends_with(TEMP_SUFFIX)
}

/// Remove orphaned temp files left behind when a writer crashes between
/// temp-file creation and `persist`.
///
/// Writers coordinate with this sweeper via a best-effort advisory exclusive
/// lock (`flock` / `LockFileEx`) held from immediately after the temp file is
/// opened through `rename`. The lock is a kernel/filesystem-level signal: it crosses process,
/// PID-namespace (container), and same-server host boundaries — exactly the
/// shared-cache-directory deployment — whereas a PID is only meaningful on the
/// same host *and* in the same PID namespace.
///
/// **Fail-closed on the lock:** a candidate is deleted outright only when
/// `try_lock_exclusive` succeeds (no live writer holds the lock). Contention
/// (`WouldBlock`) means a live writer is present, so the file is *never*
/// reaped. On a filesystem with working advisory locks this is authoritative.
///
/// **Age backstop for lock-less filesystems:** some backends (NFS
/// `local_lock`, 9p/virtiofs) make `try_lock` *error* rather than contend,
/// which would otherwise leak orphans forever. In that case only — a genuine
/// lock error, never mere contention — the file is reaped if it is older than
/// [`ORPHAN_AGE_THRESHOLD`]. Age is measured filesystem-relative via
/// [`crate::fs_lock::filesystem_now`] (a probe file's mtime), so the comparison
/// stays in one clock domain and is immune to client/server clock skew. If the
/// probe cannot be created the backstop is disabled for the pass (fail closed —
/// we never fall back to the local clock). The threshold dwarfs a *healthy*
/// live write (milliseconds), so the backstop only risks a writer that has
/// itself stalled for over an hour on a lock-less filesystem (e.g. a wedged
/// `fsync` or a `SIGSTOP`-ed process) — and even then the cost is a wasted
/// re-fetch, never corruption.
///
/// **Lenient cleanup is acceptable here:** the CRL disk cache is fully
/// re-derivable from the network; an orphan is a few kilobytes; deleting a
/// live temp (should it ever happen) or losing a rename to crash only costs a
/// re-fetch, never cache corruption. That makes the main downside of
/// flock-based coordination — delayed lock release on some NFS mounts — a
/// non-issue for this workload.
///
/// **Residual risk:** NFS mounted with `local_lock` (or a lock-less backend
/// that reports success without cross-client enforcement) can make a lock look
/// *free* to another client and allow deleting a live temp via the `Ok` branch
/// — the age backstop only guards the lock-*error* branch, not a false success.
/// The consequence is at worst a wasted network re-fetch, and it indicates a
/// misconfiguration.
///
/// Best-effort, bounded (`budget` candidates per pass), jittered (see the
/// background refresher), and off the hot path.
pub(crate) fn sweep_orphan_temp_files(dir: &Path, budget: usize) -> usize {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return 0,
    };

    // Filesystem-relative "now" for the age backstop, computed lazily and at
    // most once per pass — only when a candidate actually reports a lock error,
    // so working-lock filesystems (the common case) never pay for the probe.
    // The outer `None` means "not yet computed"; the inner `None` means the
    // probe failed and the backstop is disabled for the pass (fail closed).
    let mut now_fs: Option<Option<std::time::SystemTime>> = None;

    let mut removed = 0usize;
    let mut processed = 0usize;

    for entry in entries.flatten() {
        if processed >= budget {
            break;
        }
        if entry.file_type().is_ok_and(|t| t.is_dir()) {
            continue;
        }
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !is_orphan_temp_name(&name_str) {
            continue;
        }
        processed += 1;

        let path = entry.path();
        let Ok(file) = crate::fs_lock::open_read_nofollow_nonblock(&path) else {
            continue;
        };
        let lock = match file.try_lock_exclusive() {
            Ok(()) => LockOutcome::Free,
            Err(ref e) if crate::fs_lock::is_lock_contention(e) => LockOutcome::Contended,
            Err(e) => {
                tracing::debug!(
                    target: "sf_core::crl",
                    error = %e,
                    "Advisory lock unusable on CRL cache temp; using age backstop"
                );
                LockOutcome::Unsupported
            }
        };
        let (now, mtime) = match lock {
            LockOutcome::Unsupported => (
                *now_fs
                    .get_or_insert_with(|| crate::fs_lock::filesystem_now(dir, PROBE_PREFIX).ok()),
                file.metadata().and_then(|m| m.modified()).ok(),
            ),
            LockOutcome::Free | LockOutcome::Contended => (None, None),
        };
        if should_reap(lock, now, mtime, ORPHAN_AGE_THRESHOLD)
            && std::fs::remove_file(&path).is_ok()
        {
            removed += 1;
        }
    }

    tracing::debug!(
        target: "sf_core::crl",
        removed,
        dir = %dir.display(),
        "Swept orphaned CRL cache temp files"
    );
    removed
}

/// Result of trying to take the writer's advisory lock on an orphan candidate.
#[derive(Clone, Copy)]
enum LockOutcome {
    /// Lock acquired: no live writer holds it.
    Free,
    /// Lock held by another handle: a live writer is present right now.
    Contended,
    /// Advisory locking errored — not usable on this filesystem.
    Unsupported,
}

/// Pure reap decision for one orphan candidate. Kept free of I/O so the core
/// invariant is unit-testable: **contention is never overridden by age**, and
/// the age backstop applies only when locking is `Unsupported`.
///
/// For the `Unsupported` case, `true` requires a filesystem-relative `now` and
/// an `mtime` at least `threshold` in the past. Anything ambiguous — no probe
/// time, unreadable `mtime`, or a future-dated `mtime` (`duration_since`
/// errors) — is `false` (fail closed).
fn should_reap(
    lock: LockOutcome,
    now_fs: Option<std::time::SystemTime>,
    mtime: Option<std::time::SystemTime>,
    threshold: std::time::Duration,
) -> bool {
    match lock {
        LockOutcome::Free => true,
        LockOutcome::Contended => false,
        LockOutcome::Unsupported => match (now_fs, mtime) {
            (Some(now), Some(m)) => now.duration_since(m).is_ok_and(|age| age >= threshold),
            _ => false,
        },
    }
}

impl CrlMetrics {
    fn init(meter: &Meter) -> Self {
        Self {
            get_total: meter.u64_counter("crl_get_total").build(),
            get_ms: meter.u64_histogram("crl_get_ms").build(),
            fetch_total: meter.u64_counter("crl_fetch_total").build(),
            fetch_ms: meter.u64_histogram("crl_fetch_ms").build(),
            fetch_error_total: meter.u64_counter("crl_fetch_error_total").build(),
        }
    }
}

impl CrlCache {
    // Compute remaining duration until half-life. None if expired or invalid.
    fn compute_half_life_duration(
        entry: &CachedCrl,
        now: DateTime<Utc>,
    ) -> Option<std::time::Duration> {
        if now >= entry.expires_at {
            return None;
        }
        let total_ms = (entry.expires_at - entry.download_time).num_milliseconds();
        if total_ms <= 0 {
            return None;
        }
        let half_ms = total_ms / 2;
        let half_time = entry.download_time + chrono::Duration::milliseconds(half_ms);
        if now >= half_time {
            Some(std::time::Duration::from_secs(0))
        } else {
            (half_time - now).to_std().ok()
        }
    }

    #[cfg(test)]
    pub(crate) fn test_put_outcome(
        &self,
        serial: &[u8],
        issuer_der: &[u8],
        outcome: crate::tls::revocation::RevocationOutcome,
        expires_at: DateTime<Utc>,
    ) {
        if let Some(issuer_hash) = crate::tls::x509_utils::subject_der_hash(issuer_der) {
            let key = OutcomeKey {
                serial: serial.to_vec(),
                issuer_hash,
            };
            self.outcome_put(key, outcome, expires_at);
        }
    }

    // Reschedule a URL in the delay queue based on current cache state
    async fn reschedule_url(
        &self,
        dq: &mut tokio_util::time::DelayQueue<String>,
        keys: &mut HashMap<String, tokio_util::time::delay_queue::Key>,
        url: &str,
    ) {
        if let Ok(Some(entry)) = self.get_from_memory_cache(url).await
            && let Some(dur) = Self::compute_half_life_duration(&entry, Utc::now())
        {
            if let Some(old) = keys.remove(url) {
                let _ = dq.remove(&old);
            }
            let key = dq.insert(url.to_string(), dur);
            keys.insert(url.to_string(), key);
        } else {
            keys.remove(url);
        }
    }
    // Spawn a singleton scheduler using DelayQueue keyed to CRL half-life deadlines,
    // plus jittered orphan-temp sweeps when disk caching is enabled.
    fn spawn_background_refresher(this: Arc<Self>) {
        use tokio_util::time::DelayQueue;

        let disk_cache_dir = if this.config.enable_disk_caching {
            this.config.get_cache_dir()
        } else {
            None
        };
        if this.memory_cache.is_none() && disk_cache_dir.is_none() {
            return;
        }
        let memory_enabled = this.memory_cache.is_some();

        let thread_name = "crl-refresh".to_string();
        let dispatch = tracing::dispatcher::get_default(|d| d.clone());
        let _ = std::thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
                let _log_guard = tracing::dispatcher::set_default(&dispatch);
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to create CRL refresh runtime");
                rt.block_on(async move {
                    if let Some(dir) = disk_cache_dir {
                        tokio::spawn(async move {
                            const SWEEP_BUDGET: usize = 256;
                            // Sweeping is cheap (bounded, off the hot path), so
                            // run it often; jitter spreads passes across
                            // processes sharing a cache dir.
                            const THIRTY_MINUTES: u64 = 30 * 60;
                            const FIVE_MINUTES: u64 = 5 * 60;

                            tokio::time::sleep(std::time::Duration::from_secs(
                                rand::random::<u64>() % 30,
                            ))
                            .await;
                            loop {
                                let dir = dir.clone();
                                let _ = tokio::task::spawn_blocking(move || {
                                    sweep_orphan_temp_files(&dir, SWEEP_BUDGET)
                                })
                                .await;
                                let jitter = std::time::Duration::from_secs(
                                    rand::random::<u64>() % FIVE_MINUTES,
                                );
                                tokio::time::sleep(
                                    std::time::Duration::from_secs(THIRTY_MINUTES) + jitter,
                                )
                                .await;
                            }
                        });
                    }

                    if !memory_enabled {
                        loop {
                            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
                        }
                    }

                    // control channel to receive schedule updates
                    let (tx, mut rx) = tokio::sync::mpsc::channel::<SchedulerMsg>(128);
                    // publish tx to instance so put()/fetch can notify (OnceCell ensures set once)
                    let _ = this.scheduler_tx.set(tx);

                    let mut dq: DelayQueue<String> = DelayQueue::new();
                    let mut keys: HashMap<String, tokio_util::time::delay_queue::Key> =
                        HashMap::new();

                    // Seed existing entries
                    if let Some(memory) = &this.memory_cache
                        && let Ok(cache) = memory.lock()
                    {
                        for (url, entry) in cache.iter() {
                            if let Some(dur) = Self::compute_half_life_duration(entry, Utc::now()) {
                                let key = dq.insert(url.clone(), dur);
                                keys.insert(url.clone(), key);
                            }
                        }
                    }

                    loop {
                        tokio::select! {
                            // Next scheduled refresh
                            maybe_item = dq.next(), if !dq.is_empty() => {
                                if let Some(expired) = maybe_item { // a url is due
                                    let url = expired.into_inner();
                                    let me = this.clone();
                                    // refresh with per-URL lock and then reschedule based on new data
                                    let url_for_task = url.clone();
                                    let _ = tokio::spawn(async move {
                                        let lock = match me.get_url_lock(&url_for_task) { Ok(l) => l, Err(_) => return };
                                        let _guard = lock.lock().await;
                                        if let Ok(Some(entry)) = me.get_from_memory_cache(&url_for_task).await
                                            && Utc::now() < entry.expires_at
                                        {
                                            let _ = me.fetch_from_network_and_cache(&url_for_task).await;
                                        }
                                    }.with_current_subscriber()).await;
                                    // After refresh, look up updated entry and reschedule
                                    this.reschedule_url(&mut dq, &mut keys, &url).await;
                                }
                            }
                            // Updates from cache changes
                            Some(msg) = rx.recv() => {
                                match msg {
                                    SchedulerMsg::Schedule(url) => {
                                        this.reschedule_url(&mut dq, &mut keys, &url).await;
                                    }
                                }
                            }
                            else => {
                                // If nothing scheduled yet, idle briefly to avoid busy loop
                                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                            }
                        }
                    }
                });
            });
    }

    // Decide if a CRL with the given IDP scope applies to the target certificate and URL
    fn crl_applicable_for_cert(
        scope_opt: Option<crate::tls::x509_utils::IdpScope>,
        is_ca_cert: bool,
        url: &str,
    ) -> bool {
        if let Some(scope) = scope_opt {
            if let Some(uris) = &scope.dp_uris {
                // BRs: CRL partitioned by DP must include a URI that matches cert CRLDP; empty URIs (RelativeName) are non-compliant for DP matching
                if uris.is_empty() || !uris.iter().any(|u| u == url) {
                    return false;
                }
            }
            if scope.only_ca && !is_ca_cert {
                return false;
            }
            if scope.only_user && is_ca_cert {
                return false;
            }
        }
        true
    }
    fn record_revocation_outcome(
        &self,
        serial: &[u8],
        issuer_der: Option<&[u8]>,
        min_expires: Option<DateTime<Utc>>,
        outcome: &crate::tls::revocation::RevocationOutcome,
    ) {
        if let Some(issuer) = issuer_der
            && let Some(issuer_hash) = crate::tls::x509_utils::subject_der_hash(issuer)
        {
            let key = OutcomeKey {
                serial: serial.to_owned(),
                issuer_hash,
            };
            let expires_at = min_expires.unwrap_or_else(|| Utc::now() + self.config.validity_time);
            self.outcome_put(key, outcome.clone(), expires_at);
        }
    }
    fn outcome_get(&self, key: &OutcomeKey) -> Option<crate::tls::revocation::RevocationOutcome> {
        if let Some(cache) = &self.outcome_cache
            && let Ok(mut guard) = cache.lock()
        {
            if let Some(entry) = guard.get(key)
                && Utc::now() <= entry.expires_at
            {
                return Some(entry.outcome.clone());
            }
            guard.remove(key);
        }
        None
    }

    fn outcome_put(
        &self,
        key: OutcomeKey,
        outcome: crate::tls::revocation::RevocationOutcome,
        expires_at: DateTime<Utc>,
    ) {
        if let Some(cache) = &self.outcome_cache
            && let Ok(mut guard) = cache.lock()
        {
            guard.insert(
                key,
                OutcomeEntry {
                    outcome,
                    expires_at,
                },
            );
        }
    }

    pub async fn check_revocation(
        &self,
        cert_der: &[u8],
        issuer_der: Option<&[u8]>,
        issuer_candidates: Option<&[&[u8]]>,
        root_store: Option<Arc<rustls::RootCertStore>>,
    ) -> Result<crate::tls::revocation::RevocationOutcome, crate::tls::revocation::RevocationError>
    {
        use crate::tls::revocation::RevocationOutcome;
        // Check outcome cache first for a definitive answer (e.g., EE revoked), regardless of CRLDP presence
        let serial = crate::crl::certificate_parser::get_certificate_serial_number(cert_der)
            .context(crate::tls::revocation::CrlOperationSnafu)?;
        if let Some(issuer) = issuer_der
            && let Some(issuer_hash) = crate::tls::x509_utils::subject_der_hash(issuer)
        {
            let key = OutcomeKey {
                serial: serial.clone(),
                issuer_hash,
            };
            if let Some(hit) = self.outcome_get(&key) {
                return Ok(hit);
            }
        }
        let crl_urls = crate::crl::certificate_parser::extract_crl_distribution_points(cert_der)
            .context(crate::tls::revocation::DistributionPointsSnafu)?;
        if crl_urls.is_empty() {
            return Ok(RevocationOutcome::NotDetermined);
        }
        let is_ca_cert =
            crate::crl::certificate_parser::is_ca_certificate(cert_der).unwrap_or(false);
        let mut any_verified = false;
        let mut any_full_coverage = false;
        let mut min_expires: Option<DateTime<Utc>> = None;
        // Remember the last URL whose CRL failed verification alongside its error.
        // We propagate BOTH so callers receive an error that identifies which
        // distribution point failed (via CrlDistributionPoint), not just the
        // underlying cause. See CrlError::CrlDistributionPoint in error.rs.
        let mut last_verify_error: Option<(String, CrlError)> = None;
        for url in crl_urls.iter() {
            let bytes = self
                .get(url)
                .await
                .context(crate::tls::revocation::CrlOperationSnafu)?;
            let scope = crate::tls::x509_utils::extract_crl_idp_scope(&bytes)
                .ok()
                .flatten();
            if !Self::crl_applicable_for_cert(scope.clone(), is_ca_cert, url) {
                continue;
            }
            if let Ok(Some(dt)) = crate::tls::x509_utils::extract_crl_next_update(&bytes) {
                min_expires = Some(match min_expires {
                    Some(cur) => cur.min(dt),
                    None => dt,
                });
            }
            match Self::verify_and_check_crl(
                bytes,
                &serial,
                issuer_der,
                issuer_candidates,
                root_store.clone(),
            )
            .await
            {
                Ok(Some(outcome)) => {
                    self.record_revocation_outcome(&serial, issuer_der, min_expires, &outcome);
                    return Ok(outcome);
                }
                Ok(None) => {
                    any_verified = true;
                    let full_coverage = match &scope {
                        Some(scope) => !scope.has_only_some_reasons && !scope.only_attribute,
                        None => true,
                    };
                    if full_coverage {
                        any_full_coverage = true;
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        target: "sf_core::crl",
                        url = %url,
                        error = %e,
                        "CRL verification failed for distribution point"
                    );
                    last_verify_error = Some((url.clone(), e));
                }
            }
        }
        if !any_verified {
            // If CRL URLs existed but none could be verified, propagate the error
            // rather than returning NotDetermined (which could be misinterpreted as
            // "no CRL distribution points available"). Wrap with the CRLDP URL so
            // callers can identify which endpoint failed without parsing logs.
            if let Some((failed_url, err)) = last_verify_error {
                return Err(Box::new(err))
                    .context(CrlDistributionPointSnafu { url: failed_url })
                    .context(crate::tls::revocation::CrlOperationSnafu);
            }
            return Ok(RevocationOutcome::NotDetermined);
        }
        let outcome = if any_full_coverage {
            RevocationOutcome::NotRevoked
        } else {
            RevocationOutcome::NotDetermined
        };
        self.record_revocation_outcome(&serial, issuer_der, min_expires, &outcome);
        Ok(outcome)
    }

    async fn verify_and_check_crl(
        crl_bytes: Vec<u8>,
        serial: &[u8],
        issuer_der: Option<&[u8]>,
        issuer_candidates: Option<&[&[u8]]>,
        root_store: Option<Arc<rustls::RootCertStore>>,
    ) -> Result<Option<crate::tls::revocation::RevocationOutcome>, CrlError> {
        use crate::tls::revocation::RevocationOutcome;

        // The CRL signature verification (RSA/ECDSA, looped over issuer candidates
        // plus the anchor fallback) and the revoked-serial scan are CPU-bound and
        // run on the TLS-handshake path, so do them on the blocking pool — a large
        // CRL must not stall a runtime worker. Only owned inputs cross the
        // boundary: `crl_bytes` is *moved* (no copy of the potentially multi-MB
        // CRL), the root store is a cheap `Arc` clone, and the serial / issuer
        // DERs are small.
        let serial = serial.to_vec();
        let issuer_der = issuer_der.map(<[u8]>::to_vec);
        let issuer_candidates =
            issuer_candidates.map(|c| c.iter().map(|d| d.to_vec()).collect::<Vec<Vec<u8>>>());

        let join = tokio::task::spawn_blocking(
            move || -> Result<Option<RevocationOutcome>, CrlError> {
                let mut verified = crate::tls::x509_utils::verify_crl_signature(
                    &crl_bytes,
                    issuer_der.as_deref(),
                )
                .is_ok();
                if !verified && let Some(cands) = &issuer_candidates {
                    for cand in cands {
                        if crate::tls::x509_utils::verify_crl_signature(&crl_bytes, Some(cand))
                            .is_ok()
                        {
                            verified = true;
                            break;
                        }
                    }
                }
                // If still not verified, try configured root store to resolve a matching anchor and verify via its SPKI
                let mut attempted_anchor = false;
                if !verified
                    && let Some(store) = root_store.as_deref()
                    && let Some(anchor) =
                        crate::tls::x509_utils::resolve_anchor_issuer_key(&crl_bytes, store)
                {
                    attempted_anchor = true;
                    verified = crate::tls::x509_utils::verify_crl_sig_with_name_and_spki(
                        &crl_bytes,
                        anchor.subject.as_ref(),
                        anchor.subject_public_key_info.as_ref(),
                    )
                    .is_ok();
                }

                if !verified {
                    // Diagnostic only — emitted at debug level to avoid duplicating the
                    // warn emitted by the caller ("CRL verification failed for distribution
                    // point"), which includes the URL context that's more useful for ops.
                    // The failure itself is propagated via InvalidCrlSignatureSnafu and logged
                    // with full error chain by the caller.
                    tracing::debug!(
                        target: "sf_core::crl",
                        "Unable to verify CRL signature (serial={}, issuer_provided={}, anchor_attempted={})",
                        hex::encode(&serial),
                        issuer_der.is_some(),
                        attempted_anchor
                    );
                    return InvalidCrlSignatureSnafu {}.fail();
                }

                let is_revoked =
                    crate::crl::certificate_parser::check_certificate_in_crl(&serial, &crl_bytes)?;
                if is_revoked {
                    Ok(Some(RevocationOutcome::Revoked {
                        reason: None,
                        revocation_time: None,
                    }))
                } else {
                    Ok(None)
                }
            },
        )
        .await;

        match join {
            Ok(result) => result,
            // A panic in the pure-CPU verification closure is a bug; re-raise it so
            // it propagates exactly as it would have on the runtime thread before
            // this work was moved onto the blocking pool.
            Err(e) if e.is_panic() => std::panic::resume_unwind(e.into_panic()),
            Err(e) => Err(e).context(VerificationTaskSnafu),
        }
    }

    pub fn new(config: CrlConfig) -> Result<Self, CrlError> {
        let memory_cache = if config.enable_memory_caching {
            Some(Arc::new(Mutex::new(HashMap::new())))
        } else {
            None
        };
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(
                config.http_timeout.num_seconds() as u64,
            ))
            .connect_timeout(std::time::Duration::from_secs(
                config.connection_timeout.num_seconds() as u64,
            ))
            .build()
            .context(crate::crl::error::HttpClientBuildSnafu)?;

        let meter = global::meter("sf_core.crl");
        Ok(Self {
            config: config.clone(),
            memory_cache,
            outcome_cache: if config.enable_memory_caching {
                Some(Arc::new(Mutex::new(HashMap::new())))
            } else {
                None
            },
            url_locks: Arc::new(Mutex::new(HashMap::new())),
            backoff: Arc::new(Mutex::new(HashMap::new())),
            http_client,
            scheduler_tx: OnceCell::new(),
            metrics: CrlMetrics::init(&meter),
        })
    }

    pub fn global(config: CrlConfig) -> &'static Arc<CrlCache> {
        static INSTANCE: OnceCell<Arc<CrlCache>> = OnceCell::new();
        INSTANCE.get_or_init(|| {
            let cache = match CrlCache::new(config) {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!(target: "sf_core::crl", "Failed to initialize CRL cache: {e}. Falling back to default config");
                    match CrlCache::new(CrlConfig::default()) {
                        Ok(c2) => c2,
                        Err(e2) => {
                            tracing::error!(target: "sf_core::crl", "Failed to initialize fallback CRL cache: {e2}. Using minimal no-op cache.");
                            CrlCache {
                                config: CrlConfig::default(),
                                memory_cache: None,
                                outcome_cache: None,
                                url_locks: Arc::new(Mutex::new(HashMap::new())),
                                backoff: Arc::new(Mutex::new(HashMap::new())),
                                http_client: reqwest::Client::new(),
                                scheduler_tx: OnceCell::new(),
                                metrics: CrlMetrics::init(&global::meter("sf_core.crl")),
                            }
                        }
                    }
                }
            };
            let arc = Arc::new(cache);
            // Start background refresh worker once
            CrlCache::spawn_background_refresher(arc.clone());
            arc
        })
    }

    pub fn url_digest(url: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        let digest = hasher.finalize();
        hex::encode(digest)
    }

    #[cfg(test)]
    pub fn clear_caches_for_tests(&self) {
        if let Some(memory) = &self.memory_cache
            && let Ok(mut cache) = memory.lock()
        {
            cache.clear();
        }
        if let Some(outcomes) = &self.outcome_cache
            && let Ok(mut cache) = outcomes.lock()
        {
            cache.clear();
        }
        // backoff/url_locks are not critical for test isolation
    }

    pub fn get_cached(&self, url: &str) -> Result<Option<CachedCrl>, CrlError> {
        if let Some(memory) = &self.memory_cache
            && let Ok(mut cache) = memory.lock()
        {
            if let Some(entry) = cache.get(url)
                && Utc::now() <= entry.expires_at
            {
                return Ok(Some(entry.clone()));
            }
            cache.remove(url);
        }
        Ok(None)
    }

    fn should_replace_cached_crl(&self, prev: &CachedCrl, new: &CachedCrl) -> bool {
        if let (Some(prev_num), Some(new_num)) = (prev.crl_number, new.crl_number) {
            return new_num > prev_num;
        }
        // Prefer comparing thisUpdate when crlNumber is absent, as it reflects issuance time
        if let (Ok((prev_this, _)), Ok((new_this, _))) = (
            crate::tls::x509_utils::crl_times(&prev.crl),
            crate::tls::x509_utils::crl_times(&new.crl),
        ) {
            return new_this > prev_this;
        }
        true // Default to replacing if comparison is not possible
    }

    pub fn put(&self, cached_crl: CachedCrl) -> Result<(), CrlError> {
        let url_key = cached_crl.url.clone();
        if let Some(memory) = &self.memory_cache
            && let Ok(mut cache) = memory.lock()
        {
            if let Some(prev) = cache.get(&cached_crl.url)
                && !self.should_replace_cached_crl(prev, &cached_crl)
            {
                return Ok(());
            }
            cache.insert(url_key.clone(), cached_crl);
        }
        // Notify scheduler to (re)schedule this URL
        if let Some(tx) = self.scheduler_tx.get()
            && let Some(memory) = &self.memory_cache
            && let Ok(cache) = memory.lock()
            && let Some(entry) = cache.get(&url_key)
        {
            let _ = tx.try_send(SchedulerMsg::Schedule(entry.url.clone()));
        }
        Ok(())
    }

    async fn get_from_memory_cache(&self, url: &str) -> Result<Option<CachedCrl>, CrlError> {
        if let Some(memory) = &self.memory_cache
            && let Ok(mut cache) = memory.lock()
        {
            if let Some(entry) = cache.get(url)
                && Utc::now() <= entry.expires_at
            {
                return Ok(Some(entry.clone()));
            }
            cache.remove(url);
        }
        Ok(None)
    }

    async fn get_from_disk_cache(&self, url: &str) -> Result<Option<Vec<u8>>, CrlError> {
        if self.config.enable_disk_caching
            && let Some(dir) = self.config.get_cache_dir()
        {
            let file_name = Self::url_digest(url);
            let path = dir.join(file_name);
            if let Ok(bytes) = tokio::fs::read(&path).await {
                let expires_at = match crate::tls::x509_utils::extract_crl_next_update(&bytes) {
                    Ok(Some(dt)) => dt,
                    _ => Utc::now() + self.config.validity_time,
                };
                if Utc::now() <= expires_at {
                    let _ = self.put(CachedCrl {
                        crl: bytes.clone(),
                        download_time: Utc::now(),
                        url: url.to_string(),
                        expires_at,
                        crl_number: crate::tls::x509_utils::extract_crl_number(&bytes)
                            .ok()
                            .flatten(),
                    });
                    return Ok(Some(bytes));
                }
                tracing::debug!(target: "sf_core::crl", "Disk cache entry expired for {url}, refetching");
            }
        }
        Ok(None)
    }

    async fn fetch_from_network_and_cache(&self, url: &str) -> Result<Vec<u8>, CrlError> {
        let fetched = self.fetch(url).await?;
        if self.config.enable_disk_caching
            && let Some(dir) = self.config.get_cache_dir()
        {
            if let Err(e) = tokio::fs::create_dir_all(&dir).await {
                let error_type = std::any::type_name_of_val(&e);
                tracing::warn!(
                    target: "sf_core::crl",
                    dir = %dir.display(),
                    error_type,
                    "Failed to create CRL cache directory"
                );
                tracing::debug!(
                    target: "sf_core::crl",
                    dir = %dir.display(),
                    error = %e,
                    "Failed to create CRL cache directory"
                );
            }
            let file_name = Self::url_digest(url);
            let data = fetched.clone();
            let dir_for_write = dir.clone();
            let name_for_write = file_name.clone();
            match tokio::task::spawn_blocking(move || {
                write_crl_atomic(&dir_for_write, &name_for_write, &data)
            })
            .await
            {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    let error_type = std::any::type_name_of_val(&e);
                    tracing::warn!(
                        target: "sf_core::crl",
                        path = %dir.join(&file_name).display(),
                        error_type,
                        "Failed to write CRL cache to disk"
                    );
                    tracing::debug!(
                        target: "sf_core::crl",
                        path = %dir.join(&file_name).display(),
                        error = %e,
                        "Failed to write CRL cache to disk"
                    );
                }
                Err(e) => {
                    let error_type = std::any::type_name_of_val(&e);
                    tracing::warn!(
                        target: "sf_core::crl",
                        path = %dir.join(&file_name).display(),
                        error_type,
                        "CRL cache disk-write task failed"
                    );
                    tracing::debug!(
                        target: "sf_core::crl",
                        path = %dir.join(&file_name).display(),
                        error = %e,
                        "CRL cache disk-write task failed"
                    );
                }
            }
        }
        let expires_at = match crate::tls::x509_utils::extract_crl_next_update(&fetched) {
            Ok(Some(dt)) => dt,
            _ => Utc::now() + self.config.validity_time,
        };
        if let Err(e) = self.put(CachedCrl {
            crl: fetched.clone(),
            download_time: Utc::now(),
            url: url.to_string(),
            expires_at,
            crl_number: crate::tls::x509_utils::extract_crl_number(&fetched)
                .ok()
                .flatten(),
        }) {
            tracing::warn!(
                target: "sf_core::crl",
                "Failed to put CRL into memory cache for url {url}: {e}"
            );
        }
        Ok(fetched)
    }

    pub async fn get(&self, url: &str) -> Result<Vec<u8>, CrlError> {
        let start = std::time::Instant::now();
        if let Some(mem) = self.get_from_memory_cache(url).await? {
            let ms = start.elapsed().as_millis() as u64;
            self.metrics
                .get_ms
                .record(ms, &[KeyValue::new("source", "memory")]);
            self.metrics
                .get_total
                .add(1, &[KeyValue::new("source", "memory")]);
            return Ok(mem.crl);
        }
        let lock = self.get_url_lock(url)?;
        let _guard = lock.lock().await;
        if let Some(mem) = self.get_from_memory_cache(url).await? {
            return Ok(mem.crl);
        }

        if let Some(disk) = self.get_from_disk_cache(url).await? {
            let ms = start.elapsed().as_millis() as u64;
            self.metrics
                .get_ms
                .record(ms, &[KeyValue::new("source", "disk")]);
            self.metrics
                .get_total
                .add(1, &[KeyValue::new("source", "disk")]);
            return Ok(disk);
        }

        let fetched = self.fetch_from_network_and_cache(url).await?;
        let ms = start.elapsed().as_millis() as u64;
        self.metrics
            .get_ms
            .record(ms, &[KeyValue::new("source", "network")]);
        self.metrics
            .get_total
            .add(1, &[KeyValue::new("source", "network")]);
        Ok(fetched)
    }

    fn get_url_lock(&self, url: &str) -> Result<Arc<tokio::sync::Mutex<()>>, CrlError> {
        let mut locks = self.url_locks.lock().map_err(|e| {
            MutexPoisonedSnafu {
                message: format!("url_locks map poisoned: {e}"),
            }
            .build()
        })?;
        Ok(locks
            .entry(url.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone())
    }

    async fn fetch(&self, url: &str) -> Result<Vec<u8>, CrlError> {
        let start = std::time::Instant::now();
        self.maybe_sleep_backoff(url).await?;

        let ctx = HttpContext::new(Method::GET, url.to_string());

        let req_builder = || self.http_client.get(url);
        let bytes = match execute_bytes_with_retry(req_builder, &ctx, &RetryPolicy::default()).await
        {
            Ok(b) => b,
            Err(e) => {
                self.metrics.fetch_error_total.add(1, &[]);
                self.record_backoff_failure(url);
                return match e {
                    HttpError::Transport { source, .. } => Err(source).context(CrlDownloadSnafu {
                        url: url.to_string(),
                    }),
                    HttpError::DeadlineExceeded { .. }
                    | HttpError::RetryAfterExceeded { .. }
                    | HttpError::MaxAttempts { .. } => {
                        crate::crl::error::HttpTimeoutSnafu {}.fail()
                    }
                };
            }
        };
        self.record_backoff_success(url)?;
        let ms = start.elapsed().as_millis() as u64;
        self.metrics.fetch_ms.record(ms, &[]);
        self.metrics.fetch_total.add(1, &[]);
        Ok(bytes)
    }

    async fn maybe_sleep_backoff(&self, url: &str) -> Result<(), CrlError> {
        let (failures, last) = {
            let guard = self.backoff.lock().map_err(|e| {
                MutexPoisonedSnafu {
                    message: format!("backoff map poisoned: {e}"),
                }
                .build()
            })?;
            guard
                .get(url)
                .cloned()
                .unwrap_or((0, std::time::Instant::now()))
        };
        if failures == 0 {
            return Ok(());
        }
        let base_ms = 100u64;
        let cap_ms = 5_000u64;
        let exp: u32 = failures.min(5u32);
        let factor = 1u64.checked_shl(exp).unwrap_or(u64::MAX);
        let delay_ms = base_ms.saturating_mul(factor).min(cap_ms);
        let jitter = (rand::random::<u32>() % 100) as u64;
        let total_ms = delay_ms + jitter;
        let elapsed = last.elapsed();
        let needed = std::time::Duration::from_millis(total_ms);
        if elapsed < needed {
            tokio::time::sleep(needed - elapsed).await;
        }
        Ok(())
    }

    fn record_backoff_success(&self, url: &str) -> Result<(), CrlError> {
        let mut guard = self.backoff.lock().map_err(|e| {
            MutexPoisonedSnafu {
                message: format!("backoff map poisoned: {e}"),
            }
            .build()
        })?;
        guard.remove(url);
        Ok(())
    }

    fn record_backoff_failure(&self, url: &str) {
        let mut guard = self.backoff.lock().unwrap();
        let entry = guard
            .entry(url.to_string())
            .or_insert((0, std::time::Instant::now()));
        entry.0 = entry.0.saturating_add(1);
        entry.1 = std::time::Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Builder;
    use tokio::time::timeout;

    fn test_config() -> CrlConfig {
        CrlConfig {
            enable_memory_caching: true,
            enable_disk_caching: false,
            ..Default::default()
        }
    }

    #[test]
    fn is_orphan_temp_name_matches_crl_temp_pattern_only() {
        assert!(is_orphan_temp_name(&format!(
            "{TEMP_PREFIX}abc{TEMP_SUFFIX}"
        )));
        assert!(!is_orphan_temp_name(&"a".repeat(64)));
        // A clock-probe temp must never be seen as an orphan.
        assert!(!is_orphan_temp_name(&format!("{PROBE_PREFIX}abc")));
    }

    #[test]
    fn should_reap_never_overrides_a_live_writer() {
        use std::time::{Duration, SystemTime};
        let threshold = Duration::from_secs(60 * 60);
        let now = SystemTime::now();
        let ancient = Some(now - Duration::from_secs(24 * 60 * 60));

        // Lock free -> reap, authoritatively, regardless of age inputs.
        assert!(should_reap(LockOutcome::Free, None, None, threshold));
        // Contention means a live writer holds it: NEVER reap, no matter how
        // "old" the file looks. This is the core invariant.
        assert!(!should_reap(
            LockOutcome::Contended,
            Some(now),
            ancient,
            threshold
        ));
    }

    #[test]
    fn should_reap_age_backstop_is_fail_closed() {
        use std::time::{Duration, SystemTime};
        let threshold = Duration::from_secs(60 * 60);
        let now = SystemTime::now();
        let old = Some(now - Duration::from_secs(2 * 60 * 60));

        // Unsupported + demonstrably old in the FS clock domain -> reap.
        assert!(should_reap(
            LockOutcome::Unsupported,
            Some(now),
            old,
            threshold
        ));
        // Too young -> keep.
        assert!(!should_reap(
            LockOutcome::Unsupported,
            Some(now),
            Some(now - Duration::from_secs(60)),
            threshold
        ));
        // No filesystem clock (probe failed) -> fail closed.
        assert!(!should_reap(LockOutcome::Unsupported, None, old, threshold));
        // Unreadable mtime -> fail closed.
        assert!(!should_reap(
            LockOutcome::Unsupported,
            Some(now),
            None,
            threshold
        ));
        // Future-dated mtime (skew/tampering) -> fail closed.
        assert!(!should_reap(
            LockOutcome::Unsupported,
            Some(now),
            Some(now + Duration::from_secs(60 * 60)),
            threshold
        ));
    }

    #[test]
    fn crl_applicability_enforces_dp_and_type() {
        // EE cert case, DP URI must match when present
        let is_ca = false;
        let url = "http://example/crl";
        let scope_match = crate::tls::x509_utils::IdpScope {
            only_user: true,
            only_ca: false,
            only_attribute: false,
            indirect_crl: false,
            has_only_some_reasons: false,
            dp_uris: Some(vec![url.to_string()]),
        };
        let scope_mismatch = crate::tls::x509_utils::IdpScope {
            dp_uris: Some(vec!["http://other".into()]),
            ..scope_match.clone()
        };
        let scope_relname = crate::tls::x509_utils::IdpScope {
            dp_uris: Some(vec![]),
            ..scope_match.clone()
        };
        let scope_type_mismatch = crate::tls::x509_utils::IdpScope {
            only_user: false,
            only_ca: true,
            dp_uris: None,
            ..scope_match.clone()
        };

        assert!(CrlCache::crl_applicable_for_cert(
            Some(scope_match.clone()),
            is_ca,
            url
        ));
        assert!(!CrlCache::crl_applicable_for_cert(
            Some(scope_mismatch),
            is_ca,
            url
        ));
        assert!(!CrlCache::crl_applicable_for_cert(
            Some(scope_relname),
            is_ca,
            url
        ));
        assert!(!CrlCache::crl_applicable_for_cert(
            Some(scope_type_mismatch),
            is_ca,
            url
        ));

        // CA cert case: only_user should reject
        let is_ca = true;
        let scope_only_user = crate::tls::x509_utils::IdpScope {
            only_user: true,
            only_ca: false,
            dp_uris: None,
            ..scope_match.clone()
        };
        assert!(!CrlCache::crl_applicable_for_cert(
            Some(scope_only_user),
            is_ca,
            url
        ));

        // No DP (None) and no type flags => applicable
        let is_ca = false;
        let scope_no_dp = crate::tls::x509_utils::IdpScope {
            only_user: false,
            only_ca: false,
            only_attribute: false,
            indirect_crl: false,
            has_only_some_reasons: false,
            dp_uris: None,
        };
        assert!(CrlCache::crl_applicable_for_cert(
            Some(scope_no_dp),
            is_ca,
            url
        ));
    }

    #[test]
    fn put_prefers_higher_crl_number() {
        let cache = CrlCache::new(test_config()).expect("cache");
        let url = "http://example/crl".to_string();
        let future = Utc::now() + chrono::Duration::hours(1);

        let high = CachedCrl {
            crl: vec![],
            download_time: Utc::now(),
            url: url.clone(),
            expires_at: future,
            crl_number: Some(11),
        };
        let low = CachedCrl {
            crl: vec![],
            download_time: Utc::now(),
            url: url.clone(),
            expires_at: future,
            crl_number: Some(10),
        };

        cache.put(low).expect("put low");
        cache.put(high).expect("put high");
        let got = cache.get_cached(&url).expect("ok").expect("present");
        assert_eq!(got.crl_number, Some(11));
    }

    #[test]
    fn put_ignores_lower_or_equal_crl_number() {
        let cache = CrlCache::new(test_config()).expect("cache");
        let url = "http://example/crl".to_string();
        let future = Utc::now() + chrono::Duration::hours(1);

        let high = CachedCrl {
            crl: vec![],
            download_time: Utc::now(),
            url: url.clone(),
            expires_at: future,
            crl_number: Some(20),
        };
        let eq = CachedCrl {
            crl: vec![],
            download_time: Utc::now(),
            url: url.clone(),
            expires_at: future,
            crl_number: Some(20),
        };
        let low = CachedCrl {
            crl: vec![],
            download_time: Utc::now(),
            url: url.clone(),
            expires_at: future,
            crl_number: Some(19),
        };

        cache.put(high).expect("put high");
        cache.put(eq).expect("put eq");
        cache.put(low).expect("put low");
        let got = cache.get_cached(&url).expect("ok").expect("present");
        assert_eq!(got.crl_number, Some(20));
    }

    #[test]
    fn half_life_helpers_work_before_and_after_threshold() {
        let now = Utc::now();
        let entry = CachedCrl {
            crl: vec![],
            download_time: now - chrono::Duration::hours(1),
            url: "http://example/crl".to_string(),
            expires_at: now + chrono::Duration::hours(1),
            crl_number: Some(1),
        };
        // Half-life is exactly `now`
        let before =
            CrlCache::compute_half_life_duration(&entry, now - chrono::Duration::seconds(1));
        assert!(before.is_some());
        assert!(before.unwrap() > std::time::Duration::from_millis(0));
        assert!(
            CrlCache::compute_half_life_duration(&entry, now - chrono::Duration::seconds(1))
                .unwrap()
                > std::time::Duration::from_millis(0)
        );

        let at = CrlCache::compute_half_life_duration(&entry, now);
        assert_eq!(at, Some(std::time::Duration::from_secs(0)));
        assert_eq!(
            CrlCache::compute_half_life_duration(&entry, now),
            Some(std::time::Duration::from_secs(0))
        );

        let after =
            CrlCache::compute_half_life_duration(&entry, now + chrono::Duration::seconds(1));
        assert_eq!(after, Some(std::time::Duration::from_secs(0)));
        assert_eq!(
            CrlCache::compute_half_life_duration(&entry, now + chrono::Duration::seconds(1)),
            Some(std::time::Duration::from_secs(0))
        );
    }

    #[test]
    fn scheduler_is_notified_on_put() {
        let cache = CrlCache::new(test_config()).expect("cache");
        let (tx, mut rx) = tokio::sync::mpsc::channel::<SchedulerMsg>(1);
        // Set scheduler sender for test
        let _ = cache.scheduler_tx.set(tx);

        let url = "http://example/crl".to_string();
        let entry = CachedCrl {
            crl: vec![1, 2, 3],
            download_time: Utc::now(),
            url: url.clone(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
            crl_number: Some(1),
        };
        cache.put(entry).expect("put");

        // Await a notification briefly
        let rt = Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(async {
            let msg = timeout(std::time::Duration::from_millis(200), rx.recv())
                .await
                .expect("timed out");
            match msg {
                Some(SchedulerMsg::Schedule(u)) => assert_eq!(u, url),
                other => panic!("unexpected msg: {:?}", other),
            }
        });
    }
}
