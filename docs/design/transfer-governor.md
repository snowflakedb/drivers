# Transfer governor

`sf_core` admits concurrent PUT/GET work through one process-wide
scheduler: `Governor` in `sf_core/src/governor/`. The governor grants
RAII permits. Callers execute the work and own retry policy.

Production is bounded **per command** by SQL `PARALLEL` /
`data.parallel` (`op.limit` in the central scheduler). Ordinary process
capacity is unlimited until a test or harness injects
`Governor::try_with_capacity`. The process scheduler, round-robin, and
op registry are present so an injected cap can share spare slots
across commands; they do not change outcomes when capacity is
unlimited.

## Background

Snowflake SQL `PARALLEL` is a client-side PUT/GET width, not query
degree of parallelism. Legacy connectors treat it as the size of a
worker pool: PUT defaults to 4, GET to 10. Global Services validates
the option today and returns the resolved value as `data.parallel`. GS
does not execute the transfers.

A worker in that model does not start a second file or part until it
is free, so whole-file work and multipart parts share one pool of size
`N`. Universal Driver's file loops can run concurrently, and each file
can also run parts or ranges concurrently. Without shared accounting,
in-flight work becomes approximately:

```text
active files × per-file parts × overlapping commands × driver roots
```

Each of those units opens files, builds buffers, and then hits the
network. Memory and I/O grow with the product, not with `PARALLEL`.

The command limit in the central scheduler is the production fix for
files × parts multiplication inside one command. A process cap is
opt-in (tests and harnesses). `data.parallel` remains the
per-command ceiling.

## Scope

The governor covers transfer work that allocates native buffers and
does local or cloud I/O:

- PUT/GET: file preprocess, whole-object upload and download, multipart
  parts, ranged GET, same-file HEAD then data, streaming GET body drain,
  and forward-progress lifecycle requests (create/complete multipart,
  Put Block / Put Block List).

In-tree Universal Driver bridges (`python/`, `odbc/`, `jdbc/`,
`nodejs/`) construct `DatabaseDriverV1` and therefore share the default
governor for one loaded `sf_core` image. Independently linked copies of
the crate each have their own `OnceLock`. Legacy connector repositories
that load minicore for telemetry only do not share it.

The governor does not admit GS or control-plane HTTP (query, login,
logout, REST).

"Process-wide" means every default-constructed `DatabaseDriverV1` in
one loaded image shares `process_governor()`. Tests isolate a driver
by injecting `DriverProviders.governor`.

## Goals

- Bound how many transfer units may allocate and do I/O at once inside
  a command, so nested file and part fan-out cannot become a product of
  `data.parallel`.
- Keep SQL `PARALLEL` / GS `data.parallel` as each PUT/GET command's
  ceiling.
- Leave ordinary process capacity unlimited until a test or harness
  injects `Governor::try_with_capacity`. 32 is not a product default.
- Share spare process capacity, when a cap exists, across operations
  without letting one command exceed its own ceiling.
- Wait to start work rather than start it and then fail admission.

## Non-goals

- A product knob or versioned init/config ABI. `sf_core_init` accepts
  only a logging callback today. `process_governor()` latches
  `Governor::new()` on first use; rebinding after latch is not
  supported.
- A byte-accurate process-wide native-memory budget. Permits count
  work units, not bytes.
- Applying `data.parallel` to query results.
- Adaptive concurrency: `ThrottleScope`, shared cooldowns, AIMD,
  request weights, HTTP connection sharding.
- Tenant or account isolation beyond command round-robin.
- Idle/read deadlines and connection reaping for abandoned streaming
  GETs. A slow consumer can pin a permit for the life of the body.
- Socket, keepalive, or RSS limits. Granted slots are not a connection
  pool size.
- Owning request execution, credentials, typed results, or retry
  policy.

## High-level design

The useful hierarchy is accounting, not nested schedulers:

```text
process          Governor  (optional ordinary slot cap)
  operation      TransferOp
                   PUT/GET:     data.parallel
    work unit    RequestPermit
                   one file's local I/O + leaf, or one part/range
```

A permit is a lease on one concurrent unit of transfer work: local
read/compress/encrypt, the buffers that work produces, and the logical
cloud request that consumes them. Bounding that unit is how the driver
keeps memory and I/O predictable. Network in-flight is a consequence,
not the product goal.

### Scheduler

`Governor` is an admission controller. It does not take execution
closures, own request futures, or run a dedicated runtime. `acquire` is
awaitable on the caller's tokio runtime.

`Governor::new()` and `process_governor()` have unlimited ordinary
process capacity. `try_with_capacity` sets an explicit cap and rejects
zero. When a cap is set, ordinary grants also require
`in_flight < max_in_flight`.

Registration is synchronous: `register_command(data.parallel)` for
PUT/GET. The width is `resolve_command_parallel` after GS
`data.parallel` is resolved; it is stored only on the `TransferOp`
(`file_fanout()` / `op.limit`), not also on `MultipartParams`, so the
two cannot disagree. Missing, non-positive, or unrepresentable wire
values fall back to 1. That is a defensive malformed-response
fallback, not a reproduction of the SQL PUT 4 / GET 10 defaults. The
client does not duplicate GS's current `0 < n < 100` validation.

Dropping the last `TransferOp` handle fails queued waiters.
Already-granted permits remain valid until dropped.

### Command limit and fairness

Ordinary grants always require `op.in_flight < op.limit`. That check
in `pop_next_waiter` is the only command-limit enforcer: there is no
second semaphore on `acquire`. Eligibility for round-robin is that
predicate plus the process bound when a cap is set. If one command is
idle, another may use otherwise-free process capacity only up to its
own limit.

With unlimited process capacity, `process_has_room()` is always true
and `op.limit` is what binds production. Extra acquires wait in the
governor queue. Call structure keeps that queue bounded:

- files: `.buffered(file_fanout())`
- multipart producer: one pending acquire per file
- ranges: at most `concurrency` per file

A spawn-gate semaphore sized to the same limit was removed because it
duplicated `op.limit` and would have blocked the upcoming cleanup
reserve (which must bypass the command ceiling) and future
kind/priority/byte-weighted admission, all of which need the decision
in the scheduler.

`TransferOp::file_fanout()` is the command parallel, used by
`.buffered(...)` in `upload_files` / `download_files` as the maximum
in-flight files when each file is a single work unit. Per-file part
and range width (`MAX_PART_CONCURRENCY`, 16) is a memory guard on one
file's inner pipeline. It does not truncate the command limit stored
on `TransferOp`, and it does not multiply it. When a file uses inner
part concurrency, in-flight files drop so that files and parts together
stay at or under `data.parallel`.

### Acquire and hold

Leaves require a caller-owned `RequestPermit`. The caller acquires
before the work the permit is meant to bound: file open, compress,
encrypt, digest, part/range reads, and the first outbound attempt.
They do not preprocess a file or part and then wait for admission.

One permit covers one unit of transfer work through its complete retry
sequence, including backoff and credential/SAS refresh of that same
unit. A sleeping retry therefore reserves both its process and command
slot. Do not drop and re-acquire mid-retry or mid-refresh: re-acquiring
caused hold-and-wait deadlocks (a unit waiting to re-enter while
still occupying the only slot at `command_parallel=1`). Refresh of
that unit never waits on the stage pool.

Same-file HEAD and the following GET or PUT share one worker permit.
Multipart and ranged work drop the file permit after Create/Initiate
or dest setup; each part or range then acquires its own. Distinct
files still take distinct permits from the same `TransferOp`.

Reqwest: one permit per logical leaf including retries. AWS SDK
`.send()`: one permit around the whole SDK call, because SDK retries
are internal.

Successful GET permits cover the complete response body. Buffered and
ranged GETs retain the permit through collection; streaming GETs move
it into the producer task until EOF, error, consumer disconnect, or
abort.

Each logical unit has one `Deadline` from `RetryPolicy.max_elapsed`.
For whole-object work, the file-level clock starts in `admit_file_work`
before the acquire waits, so queue time and local preprocess count
against the same PUT/GET budget as HEAD and the whole-object send.
Multipart uploads use each part as a logical unit: the producer starts
a fresh deadline immediately before that part's admission, and its
queue wait, local read, HTTP retries, backoff, and credential refresh
all share that deadline. The multipart commit is another logical unit
whose deadline starts before commit admission. Ranged GET likewise
starts a fresh deadline for each range. A backoff that cannot finish
before its unit's deadline fails immediately without sleeping while
still holding the slot.

`http::retry::execute_with_retry` is not gated. That loop is shared
with GS and control-plane HTTP.

### Backpressure

Backpressure is not starting work. A file does not enter preprocess,
and a part or range future is not polled, until this command can hold
another ordinary permit (and the process can, when a cap is set). Extra
work waits in the stream (`.buffered` / `buffer_unordered`) and, once
polled, in the governor waiter queue; it is not a failed upload.

`acquire` waits until a permit is granted, the acquire future is
cancelled, the command handle is dropped (`OperationDropped`), or the
request deadline expires.

### Command handle

`StageTransport.transfer` is the compiler-enforced command handle, the
same pattern as TLS, proxy, and CRL. `TransferOp` is not copied onto
`StageInfo`: credential snapshots clone `StageInfo`, and an RAII
registration there would stay live for the snapshot lifetime. The
handle is carried on `UploadData` / `SingleUploadData` (cloned per file
in a PUT) and, for streaming GET, moved onto `DownloadStream` so it
does not drop with `StageTransport` at begin.

Integration tests that compile as an external crate use
`TransferOp::isolated_for_tests()` (`cfg(any(test, feature =
"test-utils"))`): a private governor, still admitted, not a
production-visible bypass. Tests that need a specific file fan-out
use `TransferOp::isolated_for_tests_with_parallel(n)` so the op width
matches the intended `data.parallel`.

`RequestKind` is crate-scoped and lists variants exercised by wired
leaves. It is not a public compatibility contract.

## Why this design

There is one scarce resource: concurrent transfer work in one process.
The command ceiling and the optional process cap are two predicates on
the same grant, not two types.

The permit is a worker rather than an HTTP slot because the memory
spike is local preprocess: open, compress, encrypt, digest, part
reads. Admitting at send would leave that work unbounded, which is the
failure mode this design exists to close.

Process capacity is unlimited by default because there is no product
knob that could set it, and a silent numeric cap would throttle
workloads that SQL `PARALLEL` had already authorized. Command
`data.parallel` is the bound that already exists on the wire. An
explicit cap is a test and harness injection, not a shipping default.

The command limit lives in the central scheduler because async fan-out
is not a thread pool. A legacy pool of `N` workers does not start `N`
files each doing 16 parts. Nested `.buffered` and `buffer_unordered`
still start extra `acquire` futures; `op.in_flight < op.limit` is
what keeps granted work at `data.parallel`. A second semaphore on
`acquire` would duplicate that check and hide the decision from
cleanup-reserve and weighted admission.

The governor grants permits and stops there because the work it admits
is typed, credentialed, and retried in the providers. Storing
execution closures would make the scheduler responsible for results,
body lifetimes, multipart cleanup, and retry policy it does not own.

Round-robin across operations, rather than FIFO across waiters, is
what keeps one large PUT from monopolizing an injected process cap
while a GET sits behind it. Work-conserving means an idle command does
not reserve capacity another command could use.

## Alternatives

**Per-command semaphore only.** Stops file × part multiplication inside
one PUT/GET, but cannot share an injected process cap across commands.
The process singleton exists so overlapping commands, extra
`DatabaseDriverV1` roots, and later consumers can share that cap
without a second scheduler. The command ceiling is `op.limit` on that
same scheduler, not a sidecar semaphore.

**A second scheduler type per command (`TransferScheduler`).** Process
and command limits are nested counters, not nested runtimes. A second
type would duplicate waiter state and still need a process-wide lock
to share a cap.

**A spawn-gate semaphore sized to `data.parallel`.** It kept extra
acquires out of the waiter queue, but then `op.in_flight >= op.limit`
could never fire: one invariant had two enforcers. Call structure
already bounds the queue. The gate also could not express a cleanup
reserve that bypasses the command limit.

**A literal OS-thread worker pool.** Legacy connectors size a pool of
`PARALLEL` threads. Universal Driver runs on tokio. Tasks are cheap to
spawn and expensive to let all enter preprocess. The command limit
preserves the pool's concurrency intent (at most `N` workers in
flight for a command) without pinning OS threads.

**A default process cap.** A fixed number such as 32 is a sequential
PUT/GET no-op for a single modest command, not a measured process-wide
budget. Shipping it would silently serialize work that SQL `PARALLEL`
had already authorized. The default is unlimited until a cap is
injected.

**Admit only at the HTTP send seam.** Cheap to wire on
`upload_with_retry`. It leaves compress, encrypt, and part reads
ungoverned, so RSS still tracks the fan-out product. The product goal
is memory and I/O, not outstanding sockets alone.

**Scheduler as executor (callbacks / owned futures).** One place would
then own credentials, typed cloud results, streaming body lifetimes,
and retry. Providers already own those. Admission plus RAII drop on
cancel is the smaller contract.

**Gate `http::retry::execute_with_retry`.** That helper also drives
query, login, logout, and REST. Mixing control-plane HTTP into the
transfer cap would stall session work behind PUT/GET, or the reverse.

**Optional / already-held admission at the leaf.** Leaves that take
`bool`, `Option<RequestPermit>`, or a mutex slot invite callers to
forget a permit (Azure Put Block deadlock after SAS refresh). Leaves
require a caller-owned permit as proof of admission.

## Deliberate deviations

These are intentional, not drift from an older sketch:

- The permit is held through credential/SAS/STS refresh of the same
  unit (no re-acquire). Re-acquiring caused hold-and-wait deadlocks
  at `command_parallel=1`. Refresh itself never waits on the stage
  pool.
- A backoff that cannot fit in the remaining deadline fails
  immediately instead of sleeping until the deadline while still
  holding the slot.
- HEAD and the following data request share one worker permit.
- The command limit is `op.in_flight < op.limit` in the central
  scheduler. There is no spawn-gate semaphore.

## Where the code lives

| Area | Where |
| --- | --- |
| Scheduler, `process_governor()`, `Governor` | `sf_core/src/governor/` |
| `Deadline`, `AcquireWaitError` | `sf_core/src/governor/deadline.rs` |
| `TransferOp`, `RequestPermit`, `RequestKind`, round-robin | `sf_core/src/governor/transfer.rs` |
| Test injection | `DriverProviders.governor` in `global_state.rs` |
| Command registration | `query.rs`, streaming PUT/GET, bind-stage upload |
| Command handle on the transport | `StageTransport.transfer` in `file_manager/types.rs` |
| Shared reqwest upload seam | `cloud_http::upload_with_retry` |
| File loops | `file_manager/mod.rs` `.buffered(TransferOp::file_fanout())` |
| Command parallel resolution / part cap | `file_manager/multipart.rs` |
| Provider leaves | `s3_transfer.rs`, `azure_transfer.rs`, `gcs_transfer.rs`, `cloud_http.rs` |
