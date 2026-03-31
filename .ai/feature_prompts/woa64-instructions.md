# WoA64 CI: Orchestration Instructions

## Goal

Make Windows ARM64 CI green across all driver layers in this order:
Python (speed-up branch) → JDBC (PR #402) → ODBC (PR #403).

Rust Core ARM64 re-enable is part of the Python phase cleanup — it is not a
separate phase. The job was already passing; it was only disabled to conserve
runner capacity during Python iteration.

---

## Success criteria

1. All ARM64 CI jobs pass on their respective PRs.
2. No architecture, linker, or runtime errors (`WinError 126/127`, `os error 193`,
   `UnsatisfiedLinkError`, `IM001`).
3. Status gate jobs (`python-status`, `rust-core-status`, etc.) are green.
4. FIPS is excluded on ARM64 (upstream blocker in `aws-lc-fips-sys`). Non-FIPS only.
5. No `TEMPORARY` markers or diagnostic stderr prints remain in committed code.
6. Full test matrices restored (no ARM64-only restrictions).

---

## Key context

- `windows-11-arm` runners have x64 tools pre-installed (running under emulation).
  Rust `--target aarch64-pc-windows-msvc` must be passed explicitly or cargo
  builds for the emulated host (x64), producing mismatched binaries.
- On Windows, a Rust `cdylib` produces `sf_core.dll` (no `lib` prefix). Python's
  `ctypes.CDLL` is case-sensitive about this on Windows.
- Python 3.8+ uses restricted DLL search in `ctypes.CDLL`. PATH is not searched.
  `os.add_dll_directory()` or co-location is required for dependency DLLs.
- The `cryptography` Python package has no ARM64 Windows wheel; it must be built
  from source (needs OpenSSL headers + Rust toolchain).
- FIPS on ARM64 is blocked upstream: https://github.com/aws/aws-lc-rs/issues/1057

---

## Agent pipeline (run once per phase)

Assign one lens per reviewer. Do not change the assignment between phases:

| Reviewer | Lens |
|---|---|
| Reviewer A | **Lens A — Evidence & epistemics**: every claim backed by log/code? |
| Reviewer B | **Lens B — Failure taxonomy**: correct failure category for each error? |
| Reviewer C | **Lens C — Alternative hypotheses**: what explanations were not considered? |

### Step 1 — Write structured ADR entry

Before running reviewers, the ci-fixing-agent reads the latest CI log and writes
a new iteration entry to `doc/woa64/ADR-woa64.md` with this structure:

```markdown
### Iteration N — [title]

**Observations** (direct CI log evidence only — no interpretation):
- [exact error message or log phrase]
- [exit code, WinError number, DLL name from log]
- [what passed vs. what failed]

**Suppositions on root cause** (hypotheses derived from observations):
- [Hypothesis A]: [reasoning from observation to potential cause]
- [Hypothesis B]: [alternative explanation, why not ruled out]

**Open questions** (what evidence would distinguish between hypotheses):
- [specific diagnostic output that would confirm/refute each]
```

This entry is the shared input for all reviewers. It forces separation of
evidence from interpretation *before* the review process starts.

### Step 2 — Run 3 × first-round ADR reviewers in parallel (findings + solution proposal)

Launch 3 × `adr-reviewer-agent` simultaneously
(`.ai/feature_prompts/adr-reviewer-agent.md`).

Provide each reviewer with:
- The new ADR entry from Step 1 + full ADR history
  (`doc/woa64/ADR-windows-arm64-ci.md` or `ADR-woa64.md`)
- The latest CI failure log (if available)
- The current state of the relevant workflow file

Each reviewer applies their assigned lens AND proposes one solution with
rationale (see the "Solution proposal" template in the agent prompt).
Collect all three review outputs before proceeding to Step 3.

### Step 3 — Run 3 × second-round reviewers in parallel (criticise solutions)

Launch 3 × `adr-reviewer-agent` simultaneously, invoked with
`mode: criticise-solutions`.

Provide each reviewer with:
- All 3 solution proposals from Step 2
- The full ADR for historical context

Each reviewer criticises all 3 proposals using their assigned lens. They may
check GitHub for additional evidence (CI run logs, PR comments, issue tracker).
Collect all three criticism outputs before proceeding to Step 4.

### Step 4 — Synthesise with orchestrator

Launch 1 × `orchestrator-agent`
(`.ai/feature_prompts/orchestrator-agent.md`).

Provide:
- Step 2: all 3 review findings + 3 solution proposals
- Step 3: all 3 criticism outputs
- Full ADR for historical context
- Relevant source files (workflow YAML, c_api.py, hatch_build.py, build.rs as needed)

The orchestrator cross-references findings and criticisms, discards proposals
that received fatal criticism (evidence clearly against), and ranks the surviving
candidates by evidence strength → expected CI impact → blast radius.

### Step 5 — CI fixing loop

Launch 1 × `ci-fixing-agent`
(`.ai/feature_prompts/ci-fixing-agent.md`).

Provide:
- The orchestrator's ranked top-3 solutions
- Current PR number and target job name
- Relevant source files

The CI fixing agent works solutions in order (1 → 2 → 3), stopping at first
success. It consults `snowflake-universal-driver` for code structure questions
and `dev-env-fixes` for toolchain/environment questions.

---

## Phase schedule

### Phase 1 — Python ARM64

There are two Python PRs. Work on the **speed-up branch**; it stacks on and
supersedes PR #240 for the purpose of fixing WinError 127.

| PR | Branch | Role |
|---|---|---|
| #240 | `SNOW-3045931-add-github-actions-woa-64-workflows-for-ud-build-and-test` | Base Python WoA64 PR |
| (open) | `SNOW-3045931-speed-up-woa64-build` | **Active target** — stacks on #240, adds caching + WinError 127 fix attempt |

**Target job**: `python_tests` on `windows-11-arm`, Python 3.12
**Working branch**: `SNOW-3045931-speed-up-woa64-build`

Known state going in:
- `sf_core.dll` is ARM64 architecture (confirmed)
- `ctypes.CDLL(sf_core.dll)` fails with `WinError 127` (procedure not found)
- OpenSSL DLLs bundled next to sf_core.dll; pre-loading and winmode=0 attempted
- Speed-up branch already contains:
  - Cargo/vcpkg caching (sound — saves ~20 min per run)
  - `--config profile.release.strip=false` for ARM64 Windows (untested hypothesis)
  - **Known regression**: `hatch_build.py` reverted to `rglob("*")` instead of
    `iterdir()`. This bundles proc-macro DLLs built for x64 into the ARM64 wheel.
    Must be fixed to `iterdir()` before trusting any CI result on this branch.

**Pre-work before running the agent pipeline:**
Fix the `rglob` regression in `python/hatch_build.py` on the speed-up branch.
The line `for file in release_dir.rglob("*"):` must be `release_dir.iterdir()`
(confirmed intentional change from a prior commit on the base branch).

**Ordered solutions for the orchestrator to produce:**

Solution 1 — `strip=false` (already in branch, test first):
- The workspace release profile has `strip = true` which runs `llvm-strip` on
  the ARM64 PE after linking. On ARM64 Windows, llvm-strip can remove `.pdata`
  (exception unwind tables), which are required for DLL initialisation. Without
  `.pdata`, LoadLibrary can fail with WinError 127 on ARM64.
- Evidence: debug rlib builds work; release cdylib with strip=true fails.
  strip=false keeps all release optimisations but skips the post-link strip.
- Already implemented in `hatch_build.py` and the pre-build step in the workflow.
- After fixing the rglob regression: push and check CI.

Solution 2 — Static OpenSSL linking (if Solution 1 doesn't fix it):
- `sf_core.dll` dynamically links OpenSSL (`openssl = "0.10.73"` in Cargo.toml).
  WinError 127 may come from a missing export in the bundled OpenSSL DLLs rather
  than from strip.
- Fix: `vcpkg install openssl:arm64-windows-static-md`, set `OPENSSL_STATIC=1`
  in the build environment. sf_core.dll becomes self-contained; remove all DLL
  bundling, pre-loading, and winmode=0 fallback code.

Solution 3 — Import table diagnostic (if both above fail):
- Add `dumpbin /imports sf_core.dll` to the workflow diagnostic step.
  Compare imported DLL names against the bundled DLL filenames to find the
  exact name mismatch or missing export. Fix the specific mismatch.

**After Phase 1 goes green:**
1. Remove all `TEMPORARY` markers and diagnostic code (see clean-up checklist).
2. Re-enable `test_windows_arm64_nonfips` in `test-rust-core.yml` (change
   `if: false` to the standard condition; include in status gate).
3. Run the ADR pipeline once to validate the confirmed conclusion.

### Phase 2 — JDBC ARM64 (PR #402)

**Target job**: JDBC tests on `windows-11-arm`
**Branch**: `SNOW-3045931-woa64-jdbc`

Known blocker: `UnsatisfiedLinkError` for `nativeHandleMessage`.

Run the full ADR pipeline on the JDBC ADR. Key hypothesis to evaluate:
> `jdbc_bridge/build.rs` has no Windows exports.def or DEF-file linker arg.
> The JVM cannot find native methods by symbol name. Explicit JNI registration
> in `JNI_OnLoad` plus an exports.def (following the pattern in `sf_core/build.rs`)
> is the likely fix.

(Renumbered from Phase 3 — Rust Core re-enable folded into Phase 1 cleanup.)

### Phase 3 — ODBC ARM64 (PR #403)

**Target job**: odbc_tests on `windows-11-arm` (and also x64 Windows — same root cause)
**Branch**: `SNOW-3045931-woa64-odbc`

Known blocker: `IM001` at `SQLDriverConnect` on Windows x64 AND ARM64.

Key hypothesis to evaluate:
> `odbc/exports.def` exports `SQLDriverConnect` (ANSI) but not `SQLDriverConnectW`
> (wide-char). The Windows ODBC Driver Manager calls the wide-char variant.
> Implementing and exporting `SQLDriverConnectW` should resolve IM001.
> Fix x64 first — ARM64 inherits the same change.

---

## How to check CI progress

```bash
gh pr checks 240

# Get failed job log
gh run view <RUN_ID> --job <JOB_ID> --log-failed 2>&1 | tail -60

# Get full job log and grep for pattern
gh run view <RUN_ID> --job <JOB_ID> --log 2>&1 | grep -iE "pattern"

# Find the commit a CI run used
gh api repos/snowflakedb/universal-driver/actions/runs/<RUN_ID> --jq '.head_sha'
```

---

## PR and branch map

| Layer | PR | Branch | Status |
|---|---|---|---|
| Rust Core fix | #401 | `SNOW-3045931-woa64-rust-core-fix-msvc` | Merged |
| Windows test paths | #426 | `SNOW-3045931-woa64-fix-windows-test-paths` | Separate fix PR |
| Python base | #240 | `SNOW-3045931-add-github-actions-woa-64-workflows-for-ud-build-and-test` | Base for speed-up |
| **Python (active)** | **(open)** | `SNOW-3045931-speed-up-woa64-build` | **Active — stacks on #240** |
| JDBC | #402 | `SNOW-3045931-woa64-jdbc` | After Python |
| ODBC | #403 | `SNOW-3045931-woa64-odbc` | After JDBC |

Merge order follows the stacked PR chain above. Each layer's branch is based
on the one above it.

---

## Clean-up checklist (after each phase goes green)

- [ ] No `# TEMPORARY` in `.github/workflows/`
- [ ] No `print(..., file=sys.stderr)` in `python/src/`
- [ ] Full Python version matrix restored: `["3.9", "3.10", "3.11", "3.12", "3.13"]`
- [ ] Full OS matrix restored: `[ubuntu-latest, windows-11-arm]`
- [ ] Rust Core ARM64 job re-enabled in `test-rust-core.yml` (`if: false` → standard condition) and in status gate
- [ ] ADR updated with confirmed conclusions, reviewed by ADR pipeline

---

## Scope discipline

- Do not mix unrelated refactors into CI fixes.
- If a newly discovered issue is real but out of scope, record it in the ADR
  and defer it.
- Do not re-enable FIPS on ARM64 until the upstream blocker is resolved.
