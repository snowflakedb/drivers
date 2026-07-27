# Shared cache file cleanup: temp + atomic rename

Design note for cleaning up orphaned temporary files left in a **shared**
cache directory when writers use the temp-file + `rename` pattern.

The CRL disk cache (`sf_core/src/crl/cache.rs`) is the first concrete
application of this pattern in the universal driver.

## Problem shape

A common cache write sequence:

1. Create a uniquely named temp file in the cache directory.
2. Write payload and `fsync`.
3. `rename(temp, final_path)` — atomic on POSIX and Windows.

If the writer crashes or is killed between steps 1 and 3, the temp file
becomes a permanent orphan. With PID- and random-suffixed temp names, no
other process can infer that the file is abandoned.

The hard requirement for cleanup is narrow:

> **Never delete a temp file while a live producer still holds it open.**

Promptness of cleanup is negotiable when the cached data is fully
re-derivable (network, recompute, etc.) and orphans are small.

## Signals and isolation boundaries

| Signal | What it tells you | Crosses |
| --- | --- | --- |
| **PID liveness** | Process that created the temp is gone | Same host + same PID namespace only |
| **Advisory lock (`flock` / `LockFileEx`)** | No writer currently holds an exclusive lock on the temp fd | Process, PID namespace, and same-server host — **filesystem-dependent** |
| **Timestamp / age** | File has not been modified recently | Filesystem clock; subject to skew and NFS attribute-cache staleness |

PID liveness fails across containers (separate PID namespaces), co-located
processes on shared storage, and any environment where the sweeper cannot
observe the producer's namespace.

Advisory locks are the best default **live-writer** signal for shared cache
dirs on conventional filesystems, but behavior varies by mount type and
configuration (see substrate matrix below).

Timestamps alone cannot prove absence of a live writer (clock skew, stale
attrs, long-running writes). They are useful only as a **backstop** on
lock-less backends, combined with conservative thresholds.

## Substrate matrix

| Substrate | Advisory locks | PID liveness | Notes |
| --- | --- | --- | --- |
| Local disk / EBS | Reliable | Reliable on same host | Preferred deployment for shared cache dirs |
| NFS / EFS | Usually reliable; **`local_lock` breaks cross-client enforcement** | Host-local only | Misconfigured `local_lock` can make locks appear free to other clients |
| Container bind mounts | Same as underlying FS | Broken across containers | Separate PID namespaces |
| 9p / virtiofs / Docker Desktop file sharing | Often flaky or no-op for locks | Broken across VM boundary | Treat as lock-less unless verified |
| Sandboxes / microVMs | Depends on shared volume | **Impossible cross-guest** | Separate kernels; only shared-storage signals apply |

## Recommended default

1. **Primary: advisory-lock fail-closed sweep**
   - Writer holds an exclusive advisory lock from immediately after opening the
     temp file through `rename` (best-effort; atomicity still comes from
     `rename`). A tiny unlocked window exists between open and lock acquisition;
     a sweep hitting it can delete a not-yet-locked temp, costing only a
     re-fetch.
   - Sweeper deletes a candidate **only** if `try_lock_exclusive` succeeds.
   - Skip on contention (`WouldBlock`) and on **any** other lock error.
   - A lock-less filesystem degrades to "do not clean" rather than "delete a
     live temp".

2. **Backstop (lock-less filesystems): filesystem-relative age** *(implemented
   in the CRL cache: `filesystem_now` + `orphan_exceeds_age`)*
   - Gate it on a genuine lock **error** only — never on contention
     (`WouldBlock`), which always means a live writer. Contention must never
     fall through to the age path.
   - Get `now_probe` by creating a short-lived, randomly-named `O_EXCL` temp
     file on the same filesystem (via `tempfile`) and reading back its freshly
     stamped mtime. File creation portably stamps `mtime = now` on every
     backend. Use a random `O_EXCL` name — never a fixed path — so a co-tenant
     cannot pre-plant a symlink/FIFO/hardlink for the probe to follow, block
     on, or truncate; `O_EXCL` fails on any pre-existing entry and retries.
   - Delete temps whose mtime is older than `now_probe − threshold`. Because
     both timestamps come from the same filesystem, client/server clock skew
     and attribute-cache lag cancel.
   - Compute `now_probe` lazily — only when a candidate actually reports a lock
     *error* — so working-lock filesystems never pay for it.
   - If the probe cannot be created, disable the backstop for that pass (fail
     closed); never fall back to the local clock.
   - Use a generous threshold (CRL cache: 1 h) that dwarfs a *healthy* live
     write, the attribute-cache window, and mtime granularity. Treat a
     future-dated mtime as not-yet-old (fail closed). A writer that has itself
     stalled longer than the threshold on a lock-less filesystem can still be
     mis-reaped, costing a re-fetch — never corruption.
   - The probe's name prefix must be disjoint from both the temp pattern and
     real cache entry names so a probe momentarily visible to a concurrent
     sweeper is never reaped or read as data.

3. **Scheduling**
   - One init sweep after random jitter (e.g. 0–30 s) to avoid cold-start
     herds on shared dirs.
   - Frequent periodic sweeps are fine because a pass is cheap (CRL cache:
     ~30 min + 0–5 min jitter). Frequency is independent of the age threshold.
   - Bounded work per pass (scan budget); run off the hot path via a blocking
     pool; never overlap concurrent sweeps.

4. **Fail-closed principle**

   When uncertain, **do not delete**. A stale orphan costs disk; deleting a
   live temp costs a re-fetch or recomputation. For re-derivable caches the
   latter is acceptable only as a rare misconfiguration artifact, not as
   normal behavior.

5. **NFS `local_lock` caveat**

   With `local_lock`, locks are enforced only on the client that acquired
   them. Another client may see the file as unlocked and delete a live temp.
   Consequence for re-derivable caches: wasted re-fetch, not corruption.
   Fix the mount options; do not rely on PID or aggressive age heuristics
   instead.

## Checklist for future cleanup tasks

- [ ] Temp names are disjoint from final cache entry names.
- [ ] Writer holds an advisory exclusive lock from immediately after open
      through `rename` (best-effort; write path must not fail if lock fails).
- [ ] Sweeper is fail-closed: delete only on successful lock acquire.
- [ ] Sweep is bounded, jittered, periodic, and off the hot path.
- [ ] Document residual risk for the deployment's filesystem (NFS
      `local_lock`, virtiofs, etc.).
- [ ] Confirm cached data is re-derivable so delayed or missed cleanup is
      acceptable.
- [ ] Add lock-held / unlocked / real-entry regression tests.
- [ ] Log foreign errors at WARN with type only; full message at DEBUG.
