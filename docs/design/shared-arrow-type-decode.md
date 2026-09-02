# Sharing Arrow value decode across front ends (`sf_types`)

Design note for extracting the front-end-agnostic Arrow value decode into a
shared `sf_types` crate so ODBC, the Node.js bridge, and (later) Python share
one decoder per Snowflake type instead of each carrying its own copy.

The goal, stated once: **do as much of the per-type decode work in core as
possible; keep in the wrapper only what genuinely must be per-front-end.**
DATE is the first landed example (`sf_types::SnowflakeDate`, PR #1438). TIME is
the next worked case and is the one that forces the interesting decisions,
because — unlike DATE — it needs metadata to decode and it collides with the
in-flight ODBC CHAR-fetch performance stack (#1206 → #1422).

This note captures the seam, the metadata model, the two levels of sharing,
the interaction with the performance work, and the sequencing choices. DATE and
TIME are both now implemented in `sf_types`; the sections below describe the
seam they share and record the decisions that shaped it.

## The READ / WRITE seam

A conversion splits cleanly into two halves:

- **READ** — Arrow bytes → a plain Rust value (or integer parts). Front-end
  agnostic. This is what belongs in `sf_types`.
- **WRITE** — that value → the front end's output surface. Per-wrapper.

Front-end **policy** rides along on the WRITE side and also stays per-wrapper.

| Type | READ (shared)                     | WRITE — ODBC                                  | WRITE — Node.js                          | Policy (per-wrapper)                          |
| ---- | --------------------------------- | --------------------------------------------- | ---------------------------------------- | --------------------------------------------- |
| DATE | `Date32` days → `NaiveDate`       | `SQL_DATE_STRUCT` / `YYYY-MM-DD` char / wire  | midnight `NaiveDateTime` → JS `Date`     | ODBC enforces SQL `0001..9999`; JS has no cap |
| TIME | `Int32`/`Int64` + scale → `NaiveTime` | `SQL_TIME_STRUCT` / `HH:MM:SS[.fff]` char | string via `TIME_OUTPUT_FORMAT`          | ODBC uses fixed ISO; JS applies output format |
| BOOLEAN | `BooleanArray` → `bool`        | `SQL_C_BIT` / numeric / `"1"`-or-`"0"` char   | JS boolean                               | none in ODBC decode; bind-side string/numeric → bool coercion |

The seam is why the shared crate can exist at all: the bytes-to-value step is
identical across front ends; everything downstream of the value is not.

## Metadata: two kinds, two homes

Turning an Arrow cell into a *rendered* value draws on more than the raw bytes,
but those extra inputs come from different sources and belong in different
places. The useful split is by **what the shared decoder needs**, not by where
the input originates — and, importantly, the two kinds do *not* share a source:

> **Kind 1 — needed to decode.** Without it you cannot turn the bytes into a
> value at all. This rides in the **Arrow per-column schema** Snowflake sends
> (`logicalType`, `scale`, `precision`). Example: TIME's `scale` decides where
> the decimal point sits in the raw integer. This **must** reach the shared
> decoder.

> **Kind 2 — presentation / policy.** Governs how a front end renders or bounds
> the value, and is **not** in the Arrow schema. It comes from elsewhere: a
> **session / statement parameter** returned in the query response
> (`TIME_OUTPUT_FORMAT`), or a **wrapper-intrinsic constant** the server never
> sends at all (ODBC's SQL `0001..9999` year range). This stays on the
> wrapper's **WRITE side** — the same presentation half that "front-end policy
> rides along on" in the seam above — and never reaches the shared decoder.

So the taxonomy is about the shared decoder's needs, not about intercepting or
re-homing the Arrow metadata: Kind 1 is the subset of the Arrow column metadata
the decoder consumes; Kind 2 lives in a different input channel (session params)
or is hard-coded by the wrapper, and is applied later, on the writer.

DATE needs neither — `Date32` is self-describing and no output format is applied
to it — which is exactly why it was the clean first example. TIME needs `scale`
(Kind 1, off the Arrow field) and, in Node.js, `TIME_OUTPUT_FORMAT` (Kind 2,
from the session parameters, applied on the writer).

Metadata is parsed **per column, once**, at reader selection time — e.g. ODBC's
`SnowflakeFieldType::from_field` reads `scale` off the Arrow field and builds
`SnowflakeTime { scale }`; Node.js builds `TimeMeta { scale, format }`. The hot
per-row loop never re-parses it.

### Delivering Kind-1 metadata to the shared decoder

The shared trait method is `read_arrow_type(&self, array, row_idx)` — no slot
for metadata. Options considered:

| Option | Shape | Verdict |
| ------ | ----- | ------- |
| **A. Fields on the reader struct** | `SnowflakeTime { scale }`, set once per column | **Chosen.** Parse once per column; per-row decode reads a field; trait signature stays clean. |
| B. Context arg on the method | `read_arrow_type(.., ctx)` | Rejected — forces every caller to thread and re-pass the same value every row. |
| C. Metadata-free core only | share only DATE/BOOL/TEXT | Rejected — excludes exactly the scale-bearing types (TIME/NUMBER/TIMESTAMP) that have the bug-prone decode worth sharing. |

An **enum with a variant per type carrying its params** (`Time { scale }`,
`Fixed { scale, precision }`, …) is a superset of Option A: same "carry params
once per column," plus a single exhaustive list the compiler checks. It is a
good fit for **selection** (Arrow field → which decoder + its params) but a
**trap for the per-cell decode**: a single `decode()` on the enum can only
return one type, forcing a unified `DecodedValue` union and a second per-cell
match on both sides — reintroducing the per-cell branching and killing
monomorphization. Rule: an enum may *describe/select* the column; it must not
*decode* each cell. Keep only Kind-1 params in it (`scale`, `precision`); never
`format`.

## Two levels of sharing

There are two distinct things one could pull into core, and they serve
different consumers.

| Level | What it is | Signature (conceptual) | Who wants it |
| ----- | ---------- | ---------------------- | ------------ |
| **1 — materializer** | `read_arrow_type`: bytes → chrono value | `→ NaiveDate` / `NaiveTime` | Front ends that want a checked, ergonomic Rust value (Node.js; ODBC's non-hot paths) |
| **2 — primitive** | pure numeric decode: bytes → integer parts | `civil_from_unix_days(days) → (y,m,d)`, `split_time_raw(raw, scale) → (secs, nanos)` | Front ends with a fused, materialization-free fast path (ODBC's CHAR kernel) |

Level 1 sits on top of Level 2 (or should): the materializer is "the primitive
plus validation plus a convenient chrono value." **PR #1438 shares Level 1**;
its **stacked follow-up shares Level 2 for DATE** — it lifts
`civil_from_unix_days` (the `Date32` days → `(y, m, d)` kernel, Howard Hinnant's
`civil_from_days` shifted to the Unix epoch) into `sf_types::civil` and rebuilds
`SnowflakeDate::read_arrow_type` on top of it, so the materializer and the ODBC
hot path share one implementation of the calendar math rather than two. TIME
follows the identical shape: its Level-2 primitive `split_time_raw` lives in
`sf_types::clock` and the `SnowflakeTime { scale }` materializer in
`sf_types::time` sits on top of it, so when the ODBC CHAR kernel for TIME lands
it reads the same `(secs, nanos)` parts the materializer does.

Observation worth keeping: almost no FFI boundary's *final* product is a chrono
value — each wants integer parts, an epoch scalar, or a string. So the Level-2
parts are arguably the more universal currency; Level 1 is a convenience layer
for callers that prefer safety/ergonomics over raw throughput.

## Interaction with the ODBC performance stack (#1206 → #1422)

The performance stack optimizes the **WRITE path** — specifically ODBC's bulk
`SQL_C_CHAR` fetch. It introduces a `CharKernel` framework (`conversion/batch.rs`)
that hoists per-cell fixed overhead out of the row loop and fuses read + format
+ write, and it adds fast integer formatters. Crucially, the kernels'
**fast path deliberately bypasses `read_arrow_type`** — they call the Level-2
primitive directly and never build a chrono value:

```
// DateCharKernel::write_non_null (perf stack)
let days = array.value(idx);
if !(-719_162..=2_932_896).contains(&days) {   // out of ODBC 0001..9999 range
    /* cold path → read_validate → read_arrow_type (Level 1) */
}
let (y, m, d) = int_fmt::civil_from_unix_days(days);   // hot path → Level 2
```

### The clash

The extraction and the perf stack **edit the same files and lines** (`date.rs`,
`time.rs`: imports, the `Snowflake{Date,Time}` struct, `read_arrow_type`) but at
**different seams**:

- Extraction lifts **`read_arrow_type`** (Level 1) into core.
- Perf stack factors out the **numeric primitive** (Level 2) and routes the hot
  path through it, in-crate.

This produces a guaranteed **textual** conflict (already true for the open DATE
PR #1438 vs the stack's `DateCharKernel` work) but **not** a behavioral or
performance conflict.

### Why they are compatible in substance

1. The perf fast path never calls `read_arrow_type`, so moving `read_arrow_type`
   into core cannot regress it.
2. The Level-2 primitive is a pure int→int function; under the release profile
   (`lto = "thin"`, `codegen-units = 1`) a cross-crate call to it **inlines to
   identical machine code**. Sharing it is free at runtime.

### The correct shared seam

Put the **Level-2 primitive in core**, and have *both* `read_arrow_type` and the
ODBC kernel fast path sit on it:

- `sf_types` owns `civil_from_unix_days` / `split_time_raw` **and**
  `read_arrow_type` (built on the primitive).
- ODBC's `CharKernel` calls the shared primitive (inlined; perf preserved).
- Node.js / Python call `read_arrow_type` (or the primitive) directly.

Lifting only `read_arrow_type` while leaving the primitive in ODBC would create
**three** copies of the split math (core's materializer, ODBC's primitive, JS's
decode). The primitive is the load-bearing thing to share.

This is exactly what the DATE follow-up does: `civil_from_unix_days` now lives in
`sf_types::civil` as a `pub fn`, `SnowflakeDate::read_arrow_type` materializes
*through* it (parts → `NaiveDate::from_ymd_opt`, erroring rather than panicking
on the pathological out-of-chrono-range day offsets the server never sends), and
an exhaustive test proves the kernel is byte-identical to
`epoch + Duration::days` across the whole SQL `0001..9999` range. When the perf
stack rebases, its ODBC-local `int_fmt::civil_from_unix_days` is deleted in
favour of `use sf_types::civil_from_unix_days` — the `CharKernel` hot path then
reuses the same kernel, at zero runtime cost (the call inlines under
`lto = "thin"` + `codegen-units = 1`).

### Guardrails — how sharing *would* tank performance

The refactor is free only if the shared layer avoids all of:

- a **unified per-cell value enum** (`DecodedValue`) returned from a shared
  decode — forces a per-cell match and kills monomorphization;
- **per-cell `Box<dyn>`** dispatch (per-**column** dispatch is fine);
- **forcing materialization** — the primitive must be reachable without building
  a chrono value;
- **allocation** in the shared layer (parts and stack buffers only).

Litmus test for any future shared type: *pure / monomorphized, parts-granular,
allocation-free, no per-cell dispatch or value enum.* Pass → perf survives.
Fail → that is where the cost enters, and where review should push back.

## Sequencing & mergeability

Both stacks are owned by the same author, so ordering is an internal choice, not
cross-team coordination.

| Order | Effect | Cost |
| ----- | ------ | ---- |
| **Perf stack first, extraction rebased on top** (recommended) | Extraction lands at the primitive seam the kernels already call | Small, mechanical rebase of the extraction |
| Extraction (#1438) first, perf stack rebased down | ODBC perf fully preserved (hot path bypasses the moved `read_arrow_type`); one hand-resolved `date.rs`/`time.rs` conflict | The 8-PR perf stack pays the rebase + re-verify (oracles, Jenkins) |

Either order preserves ODBC performance. The trade is only *who pays the rebase
tax* and *how much churn lands on the delicate, measured side*. Recommendation:
land the measured perf stack first, then rebase the small extraction on top and
lift the primitive into core in the same move.

## Which front ends consume which level

Current reality:

| Front end | Level 1 (`read_arrow_type`) | Level 2 (primitive) | Notes |
| --------- | --------------------------- | ------------------- | ----- |
| **ODBC** | struct / binary / timestamp targets, single-cell `SQLGetData`, cold CHAR fallback | hot bulk `SQL_C_CHAR` path (DATE kernel now `sf_types::civil_from_unix_days`) | Only front end with a fused fast path today |
| **Node.js** | yes (whole workload) | no | Bottleneck is the napi/V8 boundary, not decode |
| **Python** | **no** | **no** | `python_bridge` is a transport/logging shim over `sf_core` — no per-cell date/time decode exists yet |

Future fit, if/when they grow cell-level decode:

- **Python** is the most likely *second* Level-2 consumer. CPython's
  constructors take integers directly (`datetime.date(y, m, d)`,
  `datetime.time(h, m, s, µs)`), so `raw → parts → PyDate/PyTime` is the natural
  flow; going through chrono would be a wasteful round-trip. Caveat: Python's
  *fastest* option may be to hand Arrow batches wholesale to pyarrow/pandas and
  decode in C/NumPy — bypassing both levels entirely. Undecided.
- **Node.js** is split by type: TIME string formatting walks broken-down fields,
  so the `(secs, nanos)` parts are useful; DATE ultimately wants epoch
  milliseconds (`days * 86_400_000`), a simpler transform for which the `(y,m,d)`
  parts do not help.

### Honest framing of the payoff

Sharing pays off **most for the front ends without a fused fast path** (Node.js,
Python) — they live on Level 1 for their whole workload. For ODBC, Level-1
sharing dedups the struct/binary/single-cell/cold paths (real, but not the
hottest); the hot path only joins once Level 2 is shared too. The reason to
share Level 2 is **not** speed (it is a wash) but to collapse the decode
*algorithm* to a single source of truth and remove the last drift risk across
drivers.

## Invariants (what review should hold the line on)

1. Kind-1 metadata (`scale`, `precision`) → core, as fields on the reader.
   Kind-2 (`TIME_OUTPUT_FORMAT`, SQL year range, min-buffer checks, ASCII
   assumptions) → the wrapper's WRITE side (it is not Arrow metadata and never
   reaches the shared decoder).
2. Keep the Level-2 primitive **policy-free and boundary-agnostic** so it stays
   reusable. ODBC-isms leaking into it make it ODBC-only by accident — that is
   the failure mode, not the sharing itself.
3. An enum may select/describe a column; it must not decode each cell.
4. Apply the guardrail litmus test to every new shared type.

## Status

- DATE: Level 1 shared (`sf_types::SnowflakeDate`, PR #1438). Level-2
  primitive (`sf_types::civil_from_unix_days`) shared in the stacked follow-up
  #1468; `read_arrow_type` now sits on it. The ODBC perf stack drops its local
  copy on rebase.
- TIME: **implemented**, same shape as DATE — Level-2 `split_time_raw` in
  `sf_types::clock` (widens the raw fraction to nanoseconds so the
  `(secs, nanos)` parts feed the `NaiveTime` materializer and a parts-only
  consumer like JDBC's `LocalTime::ofNanoOfDay` alike), plus the
  `SnowflakeTime { scale }` materializer in `sf_types::time` (Level 1). `scale`
  is carried on the reader as Kind-1 metadata, parsed once per column. ODBC is
  re-pointed at the shared reader with its C-buffer/wire writers kept local.
- TIMESTAMP_TZ: Level 1 (`sf_types::SnowflakeTimestampTz`) plus Level-2 epoch
  split (`split_scaled_epoch`, `read_struct_timestamp`, `read_scaled_timestamp`).
  Biased offsets outside `0..=2880` are decode errors. ODBC WRITE/policy and
  Node `toJSON` stay in the wrappers. NTZ/LTZ Level-1 types are not extracted
  yet; they call the shared Level-2 helpers.
- BOOLEAN: Level 1 shared (`sf_types::SnowflakeBoolean`). Like DATE it is
  self-describing — a `BooleanArray` bit read with no column metadata — so there
  is no Kind-1 metadata and no Level-2 primitive (a `bool` is not a split of a
  scaled integer; the materializer is the whole decode). ODBC keeps the
  `SQL_C_BIT`/numeric/char writers and the bind-side string/numeric → bool
  coercion; it needs no `validate_value` policy.
- Open question: does Python grow a native-object decode path (Level-2 consumer)
  or hand Arrow to pyarrow wholesale (neither)? This determines how much the
  Level-2 investment pays back beyond ODBC.
