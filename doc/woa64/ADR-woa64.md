# ADR: Windows ARM64 CI for Universal Driver

## Problem Statement

We need Windows ARM64 CI to be first-class and reliable across the universal driver stack (Rust Core first, then Python/JDBC/ODBC workflows that depend on it). Today, the ARM64 path is unstable because:

- Build tooling on `windows-11-arm` often defaults to x64 behavior.
- Some dependencies (notably OpenSSL and FIPS-related paths) are architecture-sensitive at both build and runtime.
- Several fixes that looked correct at compile time still failed during test execution (runtime DLL architecture mismatch).

The core problem is not a single failing command; it is an end-to-end architecture consistency issue across setup, build, link, and test phases.

> **[Update 2026-03-06]** The above describes the state at the time of writing. As of PR 408 run `22769605360`, error 193 is resolved and 228 unit tests execute and pass on ARM64. The architecture consistency issue described here was real but is now largely addressed. 7 e2e PUT/GET tests still fail — these are a separate category (test assertion / external infrastructure failures, not build or loader failures) and are tracked separately.

---

## Success Criteria
p
1. Rust Core ARM64 CI job completes `cargo build` and `cargo test` on `windows-11-arm` without architecture/linker/runtime errors.
2. No `%1 is not a valid Win32 application (os error 193)` in Rust Core ARM64 test execution.
3. No ARM64/x64 linker conflicts (for example `machine type 'x64' conflicts with target machine type 'ARM64'`) in enabled feature sets for the ARM64 lane.
4. Workflow implementation uses org-allowed actions only (no startup failures from disallowed third-party actions).
5. ARM64 behavior is deterministic across reruns (no “passes only on one runner instance” behavior).
6. The status gate (`rust-core-status`) includes and enforces the ARM64 job result.

---

## How We Check Success

- **Primary CI signal**: `test_windows_arm64` job in `.github/workflows/test-rust-core.yml` is green on PR and rerun.
- **Status gate signal**: `rust-core-status` is green and explicitly depends on `test_windows_arm64`.
- **Negative log checks**: Confirm logs do not contain:
  - `os error 193`
  - `machine type 'x64' conflicts with target machine type 'ARM64'`
  - `startup_failure` from disallowed actions
- **Runtime architecture check**: Ensure the failing test binary now executes (not just links) and produces normal Rust test output.
- **Reproducibility check**: Trigger at least one rerun and verify the same result without manual environment edits.
- **Scope check for follow-up PRs**: Python/JDBC/ODBC ARM64 jobs remain green (or unchanged if not in scope), with Rust Core ARM64 as the baseline dependency.

---

## Context

Adding Windows on ARM64 (WoA64) build and test support to GitHub Actions CI workflows for the universal driver project. The project has Rust core (`sf_core`) with dependencies on OpenSSL, aws-lc-rs (TLS), cmake-based C builds, and protobuf code generation.

Target runner: `windows-11-arm` (GitHub-hosted ARM64 Windows runner).

---

## Issues Encountered & Approaches Tried

### 1. CMake installs x86_64 MSI on ARM64 runner

**Problem**: `choco install cmake` downloads `cmake-4.2.3-windows-x86_64.msi` which fails with MSI error 1603 on ARM64.

**Solution**: CMake is pre-installed on `windows-11-arm` runners. Skip choco install, use the pre-installed one.

**Lesson**: Always check what's pre-installed on the runner before installing via package manager. ARM64 runners have different package availability than x64.

---

### 2. vcpkg installs wrong architecture

**Problem**: `vcpkg install zlib:x64-windows` installs x64 zlib on ARM64 runner.

**Solution**: Use architecture-aware triplet: `vcpkg install zlib:arm64-windows`.

**Lesson**: All vcpkg triplets must be parameterized by architecture. Never hardcode `x64-windows`.

---

### 3. CMake (aws-lc-sys/aws-lc-fips-sys) links against x64 SDK libraries

**Problem**: The runner's VS environment is pre-configured for x64 (`VSCMD_ARG_TGT_ARCH=x64`, `LIB` points to `\x64\` paths). CMake with Ninja generator picks up x64 libraries when building for ARM64 target, causing linker errors:
```
library machine type 'x64' conflicts with target machine type 'ARM64'
```

**Approaches tried**:

#### 3a. Separate step writing LIB to GITHUB_ENV (using env var expansion)
```yaml
- name: Set up ARM64 SDK library paths
  run: |
    $vcTools = $env:VCToolsInstallDir
    ...
    echo "LIB=$lib" >> $env:GITHUB_ENV
```
**Result**: Failed. `$env:VCToolsInstallDir` and `$env:WindowsSdkDir` were empty in the step because they're set via GITHUB_ENV from a prior step (runner's VS setup), not as real process env vars. Produced relative paths like `lib\arm64` instead of absolute paths.

**Lesson**: GITHUB_ENV vars from step N are NOT available as `$env:` in step N itself — only in step N+1. Cannot use env var expansion to construct paths in the same step that reads GITHUB_ENV.

#### 3b. Separate step with string replacement on $env:LIB
```yaml
- name: Set up ARM64 SDK library paths
  run: |
    $newLib = $env:LIB -replace '\\x64', '\arm64'
    echo "LIB=$newLib" >> $env:GITHUB_ENV
```
**Result**: Failed. `$env:LIB` was empty in the PowerShell step (same GITHUB_ENV timing issue). Wrote `LIB=` (empty) which made things worse.

**Lesson**: Same root cause as 3a. Process env vars set via GITHUB_ENV are not available in subsequent steps' `$env:` scope in PowerShell when they conflict with system-level vars.

#### 3c. Inline LIB replacement in the cargo build step
```yaml
- name: Build and test
  shell: pwsh
  run: |
    $env:LIB = $env:LIB -replace '\\x64', '\arm64'
    cargo build ...
```
**Result**: Failed. Even though `$env:LIB` had the x64 paths in this step, the `aws-lc-fips-sys` cmake crate internally calls `vcvarsall.bat x64` in its subprocess, completely overriding our ARM64 LIB.

**Lesson**: The Rust `cmake` crate (v0.1.54) and `aws-lc-fips-sys` build.rs internally invoke vcvarsall.bat and reset the VS environment to x64. External env var manipulation cannot override what happens inside the build script subprocess.

#### 3d. ilammy/msvc-dev-cmd@v1 GitHub Action
```yaml
- uses: ilammy/msvc-dev-cmd@v1
  with:
    arch: aarch64
```
**Result**: Failed. `startup_failure` — the Snowflake GitHub org restricts third-party actions. The workflow couldn't even parse.

**Lesson**: Always verify that third-party GitHub Actions are allowed in the target org before using them. Prefer inline solutions for restricted environments.

#### 3e. Inline vcvarsall.bat arm64 call using cmd shell (CURRENT)
```yaml
- name: Build and test
  shell: cmd
  run: |
    call "C:\...\vcvarsall.bat" arm64
    cargo build --package sf_core
```
**Result**: Partially works. The vcvarsall arm64 call sets up the ARM64 environment correctly for the cmd process. The `aws-lc-fips-sys` cmake crate still internally calls vcvarsall x64, BUT the ARM64 paths get appended to LIB alongside x64 paths. The x64 paths still come first and cmake still fails for FIPS.

**Workaround**: Skip `--all-features` (drop `fips` feature). Without FIPS, `aws-lc-sys` (non-FIPS) builds correctly because it uses a different build approach that respects the ARM64 environment.

**Lesson**: `call vcvarsall.bat arm64` in `shell: cmd` is the correct approach for setting up ARM64 MSVC environment. The FIPS limitation is specific to `aws-lc-fips-sys` cmake behavior, not a general problem.

---

### 4. Protoc download doesn't support windows-aarch64

**Problem**: `proto_generator/src/protoc_installer.rs` panics with `Unsupported platform for protoc download: windows-aarch64`.

**Solution**: Map `("windows", "aarch64")` to `"win64"` — the x64 protoc binary runs on ARM64 via x64 emulation.

**Lesson**: x64 binaries work on ARM64 Windows via emulation. For tools that don't have native ARM64 builds, the x64 version is acceptable.

---

### 5. Chocolatey openssl package not available for ARM64

**Problem**: `choco install openssl -y` fails with "package not found" on ARM64 runner.

**Discovery**: OpenSSL IS pre-installed on `windows-11-arm` at `C:\Program Files\OpenSSL`, but it's the x64 build.

**Solution**: Set `OPENSSL_DIR=C:\Program Files\OpenSSL` to use pre-installed headers. The build succeeds because openssl-sys finds headers and import libraries.

**Lesson**: Pre-installed packages on ARM64 runners may be x64 versions running under emulation. Check actual binary architecture, not just presence.

---

### 6. ARM64 test binary fails with "not a valid Win32 application" (error 193)

**Problem**: `cargo build` succeeds (ARM64 binary compiled), but `cargo test` fails at runtime:
```
could not execute process `sf_core-*.exe` (never executed)
%1 is not a valid Win32 application. (os error 193)
```

**Initially assumed root cause**: The ARM64 test binary links against `sf_core.dll` (dylib crate type) which depends on x64 OpenSSL DLLs at runtime. ARM64 process cannot load x64 DLLs.

> **[Review comment — stale]** This root cause was wrong. It was disproven by Solution B (vcpkg ARM64 OpenSSL) — error 193 persisted even with verified ARM64 OpenSSL. The actual root cause was the `.def` file linker argument applied to test executables (see Resolution section below). The lesson about "compile-time success ≠ runtime success" is still valid, but the specific attribution to OpenSSL DLLs was incorrect.

**Actual root cause** (confirmed in commit `f0bd3d65`): `sf_core/build.rs` used `cargo:rustc-link-arg=/DEF:exports.def` which applies the `/DEF:` flag to **all** link targets including test executables. The `LIBRARY sf_core` directive in the `.def` file instructs the MSVC linker to produce a DLL. When applied to a test `.exe`, the resulting PE has DLL characteristics — Windows cannot execute it as a process, returning error 193. Fix: change to `cargo:rustc-cdylib-link-arg` and change crate-type from `dylib` to `cdylib`.

> **[Review note]** This was a hypothesis stated as a confirmed root cause. It has since been tested and disproved. Solution B (vcpkg ARM64 OpenSSL) was tried and error 193 persisted with correct ARM64 OpenSSL — proving OpenSSL DLLs were not the cause. The confirmed root cause is in the Resolution section below: `cargo:rustc-link-arg=/DEF:exports.def` applied the DLL linker flag to test executables, producing PEs with DLL characteristics that Windows cannot execute as processes. This is not ARM64-specific — it would affect any Windows `cargo test` for `sf_core`.

**Status**: RESOLVED — see Resolution section.

> **[Update 2026-03-06]** `cargo test` now executes on ARM64. Unit tests: 228 passed, 0 failed. Remaining failures (7) are e2e PUT/GET tests that panic in `snowflake_test_client.rs:201` — these are a different failure category (test assertion / external infrastructure) and are not caused by the architecture mismatch described here. They must be investigated separately.

**Approaches tried before resolution**:
- `OPENSSL_STATIC=1` — failed at compile time (no static libs on runner)
- `vcpkg install openssl:arm64-windows` — build succeeded, error 193 persisted (disproved OpenSSL hypothesis)
- Vendored OpenSSL — skipped (OpenSSL disproven as cause)

**Lesson**: Compile-time success does not guarantee runtime success. But more importantly: error 193 on Windows has multiple causes — architecture mismatch is only one. PE type mismatch (DLL executed as process) is another. Always check PE characteristics with `dumpbin /headers`, not just machine type.

> **[Additional lesson from this issue]** Do not conflate failure categories. Error 193 (runtime loader failure) and e2e test assertion failures are distinct problems requiring distinct fixes. Resolving one does not imply the other is resolved.

---

## Summary of Key Lessons

1. **GITHUB_ENV timing**: Vars written to GITHUB_ENV are only available in subsequent steps, not the current step or via `$env:` in PowerShell within the same workflow run context as system-level vars.

2. **Third-party actions**: Cannot assume third-party GitHub Actions are allowed. Always have inline fallbacks.

3. **cmake Rust crate**: The `cmake` crate (v0.1.54) internally calls `vcvarsall.bat x64` regardless of target architecture. This cannot be overridden externally. Only the `fips` feature triggers this code path via `aws-lc-fips-sys`.

4. **x64 emulation on ARM64 Windows**: x64 binaries (protoc, tools) work via emulation. But x64 DLLs cannot be loaded by ARM64 processes — the emulation is process-level, not DLL-level.

> **[Review comment — partially stale]** This lesson is technically correct but was over-applied during the investigation. We attributed error 193 to "ARM64 process loading x64 DLL" when the actual cause was a PE type mismatch (DLL-flagged exe). The lesson should be: x64 DLL loading by ARM64 processes IS a real constraint, but it was not the constraint that caused our specific failure.

5. **Pre-installed software architecture**: `windows-11-arm` runners have many x64 packages pre-installed (OpenSSL, VS tools defaulting to x64). Never assume pre-installed = native ARM64.

6. **`shell: cmd` vs `shell: pwsh`**: Use `shell: cmd` with `call vcvarsall.bat` when you need env vars to persist within the same process for cargo/cmake. PowerShell and GITHUB_ENV approaches don't work reliably for this use case.

---

## Current Working Configuration

> **[Review comment — stale]** This section shows the configuration as of the initial investigation, before the `.def` file fix and vcpkg OpenSSL were added. The actual working configuration on the branch includes: vcpkg ARM64 OpenSSL, `--target aarch64-pc-windows-msvc`, OpenSSL DLL copying, `cdylib` crate type, and `rustc-cdylib-link-arg`. See the Resolution section and the actual workflow file on `origin/SNOW-3045931-woa64-rust-core-fix-msvc` for the current state.

```yaml
# STALE — see Resolution section for current configuration
test_windows_arm64:
  runs-on: windows-11-arm
  steps:
    - uses: actions/checkout@v4
    - name: Cache Rust dependencies
      uses: actions/cache@v4
      ...
    - name: Configure OpenSSL
      shell: pwsh
      run: |
        echo "OPENSSL_DIR=C:\Program Files\OpenSSL" >> $env:GITHUB_ENV
    - name: Decode secrets
      run: ./scripts/decode_secrets.sh
      shell: bash
    - name: Build and test
      shell: cmd
      run: |
        call vcvarsall.bat arm64
        set PARAMETER_PATH=...
        cargo build --package sf_core        # ✅ Works
        cargo test --package sf_core         # ❌ Was error 193 — now fixed (see Resolution)
```

---

## Next Steps: Resolving Issue #6

> **[Review comment — all solutions below are now historical]** The actual fix was changing `cargo:rustc-link-arg` to `cargo:rustc-cdylib-link-arg` in `sf_core/build.rs` and changing crate-type from `dylib` to `cdylib`. Solutions D+A and B were tried and provided useful negative evidence (A: no static libs exist; B: OpenSSL is not the cause). Diagnostic placement was flawed (`cargo build` does not produce test exes). Solution E was never tried but its hypothesis (dylib-specific issue) was directionally correct. The `.def` file hypothesis was listed in "Assumptions that may be wrong" but was not prioritized — in hindsight it should have been investigated first given that `odbc/build.rs` already used the correct `rustc-cdylib-link-arg` pattern.


> **[Review note 2026-03-06]** The first bullet of the original recommendation ("execute now") recommended vcpkg + static OpenSSL. This has since been overtaken by CI results — error 193 is gone, making that recommendation obsolete. The second bullet ("guard against wrong diagnosis") was prescient. The third bullet (definition of done) is partially met: unit tests pass, but 7 e2e tests still fail and reproducibility has not been confirmed via a clean rerun.


Several conclusions in this ADR are **inferred, not verified**:

- **"Error 193 is caused by x64 OpenSSL DLLs"** — We never ran `dumpbin /headers` on the pre-installed OpenSSL DLLs or on `sf_core.dll` to confirm architecture. Error 193 can also be caused by: a corrupted binary, a missing CRT dependency, or _any_ x64 DLL on the load path — not just OpenSSL.
- **"The pre-installed OpenSSL at `C:\Program Files\OpenSSL` is x64"** — Assumed from the runner image docs and the PATH entry. The `lib\VC\arm64\MD` subdirectory might actually exist and contain valid ARM64 libs, but our `OPENSSL_LIB_DIR` setting might have been wrong for other reasons.
- **"Static linking (OPENSSL_STATIC=1) will fix the runtime DLL issue"** — If the static `.lib` files are also x64, this will fail at link time instead of runtime. We also don't know if `openssl-sys` supports static linking against the pre-installed OpenSSL layout on this runner.
- **"`sf_core` dylib is the DLL causing error 193"** — The error says the _test binary_ itself couldn't execute. It could be the test exe, or `sf_core.dll`, or any transitive DLL. We haven't isolated which.

> **[Review comment]** This "assumptions" section was the most valuable part of the investigation. The 4th bullet ("sf_core dylib") was closest to the truth — the test binary itself was the problem, but because it was being linked as a DLL, not because of a transitive dependency. The framing of "we haven't isolated which" was exactly right and should have driven us to diagnostics before OpenSSL fixes.

---

### Solutions tried (with results)

#### D+A. Diagnostics + `OPENSSL_STATIC=1` — commit `a86a83e2`

**What**: Set `OPENSSL_STATIC=1` and `OPENSSL_DIR=C:\Program Files\OpenSSL` (no `OPENSSL_LIB_DIR`). Added `dumpbin` diagnostics after `cargo build` to check artifact architectures.

**Why**: Static linking would eliminate runtime DLL dependency. Diagnostics would reveal actual binary architectures.

**Result**: **FAILED at compile time.**
```
error: could not find native static library `libssl`, perhaps an -L flag is missing?
```
The pre-installed OpenSSL has no static libraries — only import libs (`.lib` stubs) for x64 DLLs. `openssl-sys` cannot do static linking here.

**Diagnostics**: Did not execute — `cargo build` failed before reaching `dumpbin` commands.

**What we learned**: The pre-installed OpenSSL layout is: `include/` (headers) + `lib/` (x64 import libs only) + `bin/` (x64 DLLs). No static `.lib` files, no ARM64 variants.

> **[Review comment]** Solution A served its purpose as a fast probe. The compile-time failure was informative. However, the diagnostic commands were placed after cargo build which meant they never ran. A better design would have been two separate commits: one for diagnostics only, one for the static linking attempt.

---

#### B. `vcpkg install openssl:arm64-windows` — commit `4584bcc6`

**What**: Build native ARM64 OpenSSL from source via vcpkg. Set `OPENSSL_DIR=C:\vcpkg\installed\arm64-windows` and `OPENSSL_LIB_DIR=C:\vcpkg\installed\arm64-windows\lib`.

**Why**: Guaranteed ARM64 OpenSSL — correct headers, static libs, and DLLs all matching the target architecture.

**Result**: **BUILD SUCCEEDED, TESTS STILL FAIL.**
```
Finished `test` profile [unoptimized + debuginfo] target(s) in 2m 47s
Running unittests src\lib.rs (target\debug\deps\sf_core-078df06007e46933.exe)
error: test failed, to rerun pass `-p sf_core --lib`
  could not execute process ... (never executed)
  %1 is not a valid Win32 application. (os error 193)
```

vcpkg built ARM64 OpenSSL correctly. `cargo build --package sf_core` compiled and linked successfully with ARM64 OpenSSL (14min total including vcpkg build). But `cargo test` still fails with error 193 at runtime.

**Critical insight**: **OpenSSL was never the cause of error 193.** The error persists even with verified ARM64 OpenSSL. The problem is in the test binary execution itself — likely the `sf_core.dll` dylib (from `crate-type = ["dylib", "rlib"]`) or some other transitive DLL.

> **[Review comment]** This was the pivotal moment. Solution B definitively disproved the OpenSSL hypothesis. The "critical insight" is correctly stated. However, the next sentence ("likely the sf_core.dll dylib... or some other transitive DLL") still frames it as a DLL issue. The actual cause was the test exe itself being flagged as a DLL by the linker. The investigation should have pivoted to examining the test binary PE headers at this point, not to more DLL-focused approaches.

---

#### D (retry). Dumpbin diagnostics with vcpkg OpenSSL — commit `e70d761f`

**What**: Added `dumpbin /headers` and `dumpbin /dependents` commands between `cargo build` and `cargo test` to check:
- Architecture of `sf_core.dll` (ARM64 or x64?)
- Architecture of the test binary
- Runtime DLL dependencies of `sf_core.dll`

**Result**: DLL copies confirmed (4 files copied). Error 193 persists.

> **[Review comment]** The diagnostic placement was still wrong. cargo build does not produce test executables — only cargo test --no-run does. The dumpbin on sf_core.dll showed ARM64 (correct), but the test exe was never examined because it did not exist at that point in the step. This is the second time diagnostics were placed incorrectly. Lesson: understand the cargo build pipeline — cargo build produces library artifacts, cargo test --no-run produces test executables.

---

#### Exe diagnostics — commit `beca70a7`

**What**: Added `dumpbin /headers` on the exact test exe hash, and a direct execution attempt (`sf_core-078df06007e46933.exe --list`).

**Result**: **Critical contradiction found.**
- sf_core.dll → `AA64 machine (ARM64)` ✅
- Direct execution of the test exe → **exit code 1** (NOT 193) — the binary CAN run when called from cmd
- But `cargo test` → **error 193 "never executed"** — cargo can't spawn the same binary

**This rules out**: binary format issues, DLL architecture mismatches, missing DLLs, PATH issues, and cache staleness.

**New hypothesis**: The issue is how `cargo test` spawns the test process on `aarch64-pc-windows-msvc`. When cargo calls `CreateProcessW()` to run the test binary, something in the process spawning chain fails with error 193. This may be:
- A Rust toolchain bug with `dylib` crate test execution on ARM64 Windows
- cargo using an x64 process launcher for ARM64 binaries
- The `shell: cmd` `call vcvarsall.bat arm64` environment not being inherited correctly by cargo's subprocess spawning

**Status**: Out of standard CI-level workarounds. Need to investigate at the Rust toolchain/cargo level, or try Solution E (rlib-only) or Solution F (x64 fallback) to unblock. The diagnostic output will definitively tell us whether the DLL/EXE are ARM64, and which DLLs are needed at runtime.

> **[Review comment — misleading observation]** The "direct execution → exit code 1" observation was later identified as misleading. The cmd || fallback printed a prior %ERRORLEVEL%, not the exe actual exit code. The test exe could NOT actually run — it had DLL characteristics. The "critical contradiction" was not a contradiction at all; it was a diagnostic error. This led to a wrong hypothesis about cargo process spawning, wasting further iterations. Lesson: when a diagnostic result seems contradictory, question the diagnostic methodology before building theories on it.

---

### Solutions not yet tried

#### E. Skip dylib, test rlib only (isolates the failure point)

`sf_core` has `crate-type = ["dylib", "rlib"]`. `cargo test` may try to load `sf_core.dll` at runtime. The error 193 occurs when executing the test binary — possibly because the test binary loads `sf_core.dll` which in turn loads a wrong-architecture transitive DLL.

**Approach**: Test with `cargo test --package sf_core --lib` to see if the issue is specific to having the dylib. If rlib-only tests pass, the issue is dylib-specific.

**Effort**: 1 command change. **Risk**: Partial solution — downstream consumers (Python/JDBC) need the dylib. But isolates the root cause.

> **[Review comment]** Solution E was never tried but its hypothesis was directionally correct — the issue WAS dylib-specific. The actual fix (changing to cdylib + rustc-cdylib-link-arg) is essentially a refined version of "the dylib crate type is the problem." If E had been tried, it would have passed (rlib tests do not get the /DEF: flag), which would have immediately pointed to the dylib/def file as the culprit. This was the cheapest experiment that would have given the most signal.

#### C. Vendored OpenSSL (most self-contained)

Enable `openssl = { features = ["vendored"] }` in Cargo.toml. Builds from bundled source. No longer needed since Solution B proved OpenSSL is not the issue.

**Status**: **SKIPPED** — OpenSSL is not the cause.

#### F. Fallback: build x64 on ARM64 runner (last resort)

`cargo test --target x86_64-pc-windows-msvc` builds and runs x64 binaries. All x64 deps work natively.

**Effort**: Minimal. **Risk**: Not testing native ARM64. **Status**: Available as fallback.

---

### Assessment summary (updated with results)

| Solution | Commit | Result | What we learned |
|----------|--------|--------|-----------------|
| **D+A: Static + diagnostics** | `a86a83e2` | Compile-time failure: no static libs | Pre-installed OpenSSL is x64 import libs only |
| **B: vcpkg ARM64 OpenSSL** | `4584bcc6` | Build OK, test error 193 | **OpenSSL is NOT the cause of error 193** |
| **D retry: dumpbin diagnostics** | `e70d761f` | Awaiting CI | Will reveal actual binary architectures |
| **E: rlib-only test** | Not yet tried | — | Will isolate dylib vs rlib as failure point |
| **C: Vendored OpenSSL** | Skipped | — | Not needed (OpenSSL disproven) |
| **F: x64 fallback** | Not yet tried | — | Last resort |


> **[Review comment]** The D retry and Exe diagnostics rows should be updated to reflect actual results. The original table said "Awaiting CI" for D retry — this was never updated. The assessment table should always be kept current as results come in.
---

### Revised understanding

> **[Review comment — superseded]** This "revised understanding" was written mid-investigation and turned out to be wrong. The PATH hypothesis (OpenSSL DLLs not discoverable) was tested and did not resolve error 193. The actual root cause was the `/DEF:` linker flag applied to test executables. See Resolution section.

The original hypothesis ("x64 OpenSSL DLLs cause error 193") was **partially right but for the wrong reason**. The issue was never that OpenSSL was x64 — vcpkg correctly built ARM64 OpenSSL. The issue was that the ARM64 OpenSSL DLLs (`libcrypto-3-arm64.dll`, `libssl-3-arm64.dll`) were installed by vcpkg at `C:\vcpkg\installed\arm64-windows\bin\` which was **not on the PATH**. Windows couldn't find them at runtime.

Diagnostics confirmed:
- `sf_core.dll` → ARM64 (correct)
- OpenSSL dependency → `libcrypto-3-arm64.dll`, `libssl-3-arm64.dll` (correct ARM64 names)
- Missing runtime PATH → the DLLs existed but weren't discoverable

### Next action

Wait for commit `e70d761f` CI results (dumpbin diagnostics). Based on the output:
- If both DLL and EXE are ARM64 → investigate transitive DLL deps and exports.def
- If DLL is x64 → the cargo build is producing wrong architecture despite ARM64 toolchain
- If EXE is x64 → similar build configuration issue

### Useful commands to fetch CI results

```bash
# Check PR 408 status (fix-msvc branch)
gh pr checks 408 2>&1 | grep -E "ARM64|rust-core-status|test.*Windows"

# Get failed job log (update run/job IDs from checks output)
gh run view <RUN_ID> --job <JOB_ID> --log-failed 2>&1 | tail -30

# Search full log for dumpbin/diagnostic output
gh run view <RUN_ID> --job <JOB_ID> --log 2>&1 | grep -iE "machine|dumpbin|architecture|dependents|===|AA64|IMAGE_FILE"

# Check which commit a CI run used
gh api repos/snowflakedb/universal-driver/actions/runs/<RUN_ID> --jq '.head_sha'

# List recent runs for the branch
gh run list --branch SNOW-3045931-woa64-rust-core-fix-msvc --workflow "Rust Core CI" --limit 5

# Check all runs for a specific commit
gh api "repos/snowflakedb/universal-driver/actions/runs?head_sha=<COMMIT_SHA>&per_page=10" \
  --jq '.workflow_runs[] | "\(.id) \(.name) \(.status) \(.conclusion)"'

# Get the OpenSSL env var state from a run
gh run view <RUN_ID> --job <JOB_ID> --log 2>&1 | grep -E "OPENSSL_DIR|OPENSSL_LIB|OPENSSL_STATIC|LIB ="

# Check if vcpkg install succeeded
gh run view <RUN_ID> --job <JOB_ID> --log 2>&1 | grep -iE "vcpkg|openssl:arm64|installed"
```

---

## Resolution of Issue #6 — commit `f0bd3d65`

### Commentary on earlier hypotheses

The investigation above spent significant time on OpenSSL architecture, DLL PATH discovery, and cargo process spawning. In hindsight:

- **"OpenSSL DLLs cause error 193"** — Disproven by Solution B. vcpkg ARM64 OpenSSL eliminated OpenSSL as a variable entirely, but error 193 persisted.
- **"The test exe can run directly from cmd but cargo can't spawn it"** — This observation from commit `beca70a7` was misleading. The cmd `||` fallback printed a prior `%ERRORLEVEL%`, not the exe's actual exit code.
- **"cargo host toolchain might be x64-emulated"** — Disproven by diagnostics in commit `4978fcb2`.
- **"Stale cached artifacts from target/"** — Removing target from cache (commit `21e60e52`) did not fix the issue.

The diagnostic placement between `cargo build` and `cargo test` was also flawed: `cargo build` does not produce test executables.

> **[Review comment]** This commentary section is the most honest part of the ADR. It explicitly lists what was wrong and why. The pattern of "hypothesis → test → disprove → new hypothesis" is good scientific method, but the iteration speed was slow because diagnostics were repeatedly placed incorrectly and misleading observations were not questioned.

### Actual root cause

The cause was in `sf_core/build.rs`:

```rust
#[cfg(target_os = "windows")]
{
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let def_path = std::path::Path::new(&manifest_dir).join("exports.def");
    println!("cargo:rustc-link-arg=/DEF:{}", def_path.display());
}
```

Per Cargo documentation, `cargo:rustc-link-arg` passes flags to the linker for benchmarks, binaries, cdylib crates, examples, and **tests**. The `/DEF:exports.def` contains `LIBRARY sf_core`, which instructs the MSVC linker to produce a DLL. When applied to the test exe, the linker produces a PE with DLL characteristics. Windows cannot execute a DLL as a process → error 193.

This is not ARM64-specific. It would affect any Windows `cargo test` for `sf_core`. No prior Windows test job existed, so the bug was latent.

Fix: `cargo:rustc-cdylib-link-arg` (scopes to cdylib only, matches `odbc/build.rs` pattern).

> **[Review comment]** The root cause is well-documented and the fix is correct. One observation: the odbc crate already used rustc-cdylib-link-arg — a simple grep across the codebase for rustc-link-arg vs rustc-cdylib-link-arg would have revealed the inconsistency without any CI iteration. This is a case where code review would have been faster than CI-based debugging.

### Changes made

1. `sf_core/build.rs` — `cargo:rustc-link-arg` → `cargo:rustc-cdylib-link-arg`
2. `sf_core/Cargo.toml` — `crate-type` from `["dylib", "rlib"]` to `["cdylib", "rlib"]`
3. `.github/workflows/test-rust-core.yml` — Removed diagnostic steps, kept clean workflow

### CI result

Run `22769605360`, job `66046665530` (commit `f0bd3d65`):
- Build: succeeded
- Unit tests: 45 passed, 3 ignored

> **[Review note]** An earlier recording of this result said "Unit tests: 45 passed" — that was the e2e test suite result, not the full unit test suite. The correct unit test count is 228 passed across all non-e2e suites.
- E2E tests: 7 failed — all `put_get::*` tests (`syntax error ... unexpected '~'`, Windows short path names)

> **[Review comment]** The 7 e2e failures are test logic bugs (Windows path handling with ~), not architecture or CI infrastructure issues. These are a separate concern and should not block the ARM64 CI PR. They likely affect Windows x64 too — they are latent bugs exposed by running tests on Windows for the first time.

### Additional lessons

7. **`cargo:rustc-link-arg` scope**: Applies to test executables. Use `cargo:rustc-cdylib-link-arg` with `cdylib` for shared-library-only linker flags.
8. **Error 193 differential diagnosis**: Covers both architecture mismatches AND PE type mismatches (DLL executed as process).
9. **Diagnostic placement**: `cargo build` does not produce test binaries. Use `cargo test --no-run`.

### Remaining work

1. **PUT/GET E2E test failures on Windows**: 7 tests fail due to `~` in short paths. Latent bug; not ARM64-specific.
2. **FIPS feature on ARM64**: Blocked by `aws-lc-fips-sys` cmake. Requires upstream fix.
3. **Propagation to downstream PRs**: The `cdylib` and `build.rs` changes need merging into Python, JDBC, ODBC branches.
4. **Reproducibility rerun**: A second CI run should confirm deterministic behavior.

> **[Review comment]** Items 1 and 2 are correctly scoped as separate concerns. Item 3 is critical — the cdylib change affects how Python/JDBC/ODBC load sf_core, and the downstream PRs need to be tested with this change. Item 4 (reproducibility) maps to Success Criterion #5 and has not been verified yet.

---

## Addendum: Downstream Workflow Status After Rust Core Fix

### Python ARM64 (`PR 240`)

- **Status**: Not yet green. Progressed to `python_tests` phase failures.
- **Resolved**: vcpkg ARM64 OpenSSL, vcvarsall arm64, ARM64-native Python selection, wheel path glob resolution.
- **Current blocker**: Wheel install failure inside `python_tests`.

### JDBC ARM64 (`PR 402`)

- **Status**: Not yet green. Progressed to JNI binding failures.
- **Resolved**: Disallowed action, ARM64 output path, Gradle `CORE_PATH` override, exports.def added.
- **Current blocker**: `UnsatisfiedLinkError` for `nativeHandleMessage`. Working hypothesis: explicit JNI registration in `JNI_OnLoad`.

### ODBC ARM64 (`PR 403`)

- **Status**: Not yet green. Now running; failure matches Windows x64.
- **Resolved**: Workflow trigger for stacked PRs, ARM64 setup aligned.
- **Current blocker**: `SQLDriverConnect` → SQLSTATE `IM001`. Working hypothesis: missing `SQLDriverConnectW`.

### Working conclusions

1. Rust Core Windows ARM64 (non-FIPS) is the stable baseline.
2. Python, JDBC, ODBC are each at different stages. None is green yet.
3. FIPS remains excluded on Windows ARM64.

> **[Review comment]** These downstream statuses are observations, not conclusions. Each is at a different failure category (wheel install, JNI binding, ODBC driver manager) — none are architecture issues. The framing "not yet green" is accurate. The "working hypothesis" labels are appropriately hedged. One concern: the JDBC "exports.def added" note implies a .def file was added to jdbc_bridge — this should be verified to use rustc-cdylib-link-arg, not rustc-link-arg, to avoid repeating the same bug.


---

## Plan Entry 1: Remaining Downstream Issues

### Motivation

Rust Core Windows ARM64 is stable enough to use as the baseline, but the downstream lanes are still not green:

- **Python**: ARM64 wheel builds pass, but `python_tests` on Windows ARM64 still fail during the package-install / dependency-build stage.
- **JDBC**: ARM64 DLL loading/path issues are resolved, but Java still fails at JNI binding time.
- **ODBC**: the workflow now runs, but Windows x64 and Windows ARM64 both fail immediately at `SQLDriverConnect`, while Linux has a separate PUT/GET `503` failure.

The goal of this plan is to pick the highest-probability solution paths first, based on the review comments already embedded in this ADR and the latest CI observations.

### Rationale / review synthesis

The most valuable insights extracted from the embedded AI review comments are:

1. **Failure category discipline matters.**
   - The earlier Rust Core debugging wasted time when runtime, linker, loader, and test failures were treated as one class of issue.
   - We should not repeat that pattern downstream.

2. **Prefer the narrowest missing-interface hypothesis over broad architecture theories.**
   - The current downstream failures are no longer generic "Windows ARM64 environment" problems.
   - Python is in dependency build / install territory.
   - JDBC is in JNI method binding territory.
   - ODBC Windows is in ODBC driver-manager / API-surface territory.

3. **Use codebase analogies and direct API-surface comparison earlier.**
   - The Rust Core fix could have been found faster by comparing crates already using the correct Windows linker/export pattern.
   - The same lesson applies now:
     - compare implemented vs exported functions
     - compare ANSI vs wide entry points
     - compare explicit registration vs name-based lookup

4. **Do not overstate current hypotheses.**
   - The items below are ordered by probability, but they are still hypotheses to test.
   - None should be treated as confirmed until CI moves forward in the expected way.

### Way of doing / proposed change

The implementation order below is probability-first, not codebase-first.

#### Python: three most probable solution paths

1. **Make the Windows ARM64 install/test environment inherit the same ARM64 OpenSSL toolchain setup as the wheel-build job.**
   - Current evidence: `python_tests` fails while building `cryptography` from source via `maturin`, and the log still points to the Rust OpenSSL-on-Windows guidance.
   - Most likely gap: the ARM64 OpenSSL/vcvarsall setup currently applied to `build_wheel` is not equivalently applied inside the Windows ARM64 `python_tests` environment before `hatch run ... install-wheel`.
   - Expected fix shape:
     - install/configure ARM64 OpenSSL in `python_tests` as well
     - ensure `vcvarsall.bat arm64` is invoked for the dependency build step
     - make the install-wheel phase inherit those env vars explicitly

2. **Prevent `cryptography` from falling back to source build on Windows ARM64 if a compatible wheel should exist.**
   - Current evidence: the failing log shows `cryptography` being pulled as an sdist and built through `maturin`.
   - Most likely causes:
     - wheel tag mismatch
     - resolver preference causing source build
     - ARM64 wheel not selected because of interpreter/platform metadata mismatch
   - Expected fix shape:
     - inspect why `uv` selected the sdist instead of a wheel
     - prefer/install binary wheels for `cryptography` on Windows ARM64 if available
     - if needed, pin to a version with known ARM64 Windows wheels

3. **Pre-build or pre-install the problematic native dependency set for Windows ARM64 test jobs.**
   - If source builds remain unavoidable, make them deterministic instead of letting the test install phase discover missing prerequisites ad hoc.
   - Expected fix shape:
     - bootstrap the problematic native dependency chain before `install-wheel`
     - or cache/preinstall the resolved environment used by `hatch`
     - keep this as a fallback if the more precise resolver/toolchain fixes above do not work

#### JDBC: three most probable solution paths

1. **Register the native method explicitly in `JNI_OnLoad`.**
   - Current evidence: the ARM64 DLL now builds and loads from the correct target directory, but Java still throws `UnsatisfiedLinkError` for `nativeHandleMessage`.
   - That means the problem has moved from DLL discovery to JNI binding.
   - The highest-probability fix is to stop relying on JVM symbol-name lookup and register `nativeHandleMessage` explicitly during `JNI_OnLoad`.

2. **Verify and, if necessary, tighten the JNI signature/export contract.**
   - Current evidence: the JNI implementation exists in Rust and a `.def` export file was added, but that has not yet been shown sufficient.
   - Expected fix shape:
     - verify the exact Java method descriptor against the Rust registration/export
     - verify that the exported Windows symbol set matches what the JVM would otherwise look for
     - confirm `JNI_OnLoad` itself is actually running and succeeding on ARM64

3. **If explicit registration still fails, treat it as a class-loader / binding-context issue rather than a path issue.**
   - Current evidence no longer supports more path tweaking as the primary next step.
   - Expected fix shape:
     - inspect whether `FindClass` / class-loader context inside `JNI_OnLoad` is the real blocker
     - adjust the registration flow to avoid loader-context ambiguity
     - only after that revisit export naming or library-loading strategy

#### ODBC: three most probable solution paths

1. **Implement and export the Windows wide connect entry points, especially `SQLDriverConnectW` and likely `SQLConnectW`.**
   - Current evidence: Windows x64 and Windows ARM64 both fail immediately with `IM001` at `SQLDriverConnect`.
   - This strongly suggests a Windows driver-manager/API-surface mismatch rather than an ARM64 environment problem.
   - The strongest current hypothesis is that the Windows ODBC manager expects wide-char connect entry points that are not implemented/exported today.

2. **Expand `odbc/exports.def` to match the API surface actually implemented and required on Windows.**
   - Current evidence: `odbc/src/c_api.rs` implements more functions than are listed in `odbc/exports.def`.
   - On Windows the `.def` file is authoritative for exports.
   - Expected fix shape:
     - align `exports.def` with the implemented Windows-visible API
     - include any newly added wide entry points
     - verify that exported symbols match the functions the driver manager and tests need first

3. **Treat the Linux ODBC failure as a separate ODBC subproblem after the Windows connect issue is fixed.**
   - Current evidence: Linux x64 fails in a PUT/GET test with backend `503 Service Unavailable`, which does not match the Windows `IM001` failure.
   - This should not be mixed into the Windows driver-manager fix.
   - Expected fix shape:
     - isolate whether the Linux failure is flaky infrastructure, a test bug, or a real ODBC behavior issue
     - handle it after the Windows API-surface problem is resolved

### Commit SHA

- not committed yet

### Observations afterwards

- Pending.

### Conclusion status

- **partially supported**
- This plan is based on the latest observed CI failures and the strongest review insights currently present in this ADR, but the proposed solution paths are still hypotheses to validate.

### Open questions / next step

1. Validate the JDBC explicit JNI registration change in CI.
2. Implement the first ODBC Windows-wide connect entry point fix and rerun PR 403.
3. Return to Python and make the Windows ARM64 `python_tests` install environment match the successful ARM64 build environment.

---

## Plan Entry 2: Python ARM64 — DLL load failure investigation

### Iteration 2a — `cryptography` source build (OpenSSL missing)

**Motivation**: `python_tests` on `windows-11-arm` failed building `cryptography==46.0.5` from source because `OPENSSL_DIR` was unset.

**Rationale**: The `build_wheel` job has ARM64 OpenSSL setup (vcpkg + env vars), but `python_tests` did not. `cryptography` has no ARM64 Windows wheel on PyPI, so `uv` falls back to source build via `maturin`, which needs OpenSSL.

**Change**: Added `vcpkg install openssl:arm64-windows` + `OPENSSL_DIR`/`OPENSSL_LIB_DIR`/`RUSTFLAGS`/`CMAKE_GENERATOR_PLATFORM` to the `python_tests` job for `windows-11-arm`.

**Commit**: `e045cfc1`

**Observation**: `cryptography` built successfully. Failure moved to test runtime: `RuntimeError: Couldn't load core driver dependency` at `c_api.py:50`.

**Conclusion**: OpenSSL fix was correct and necessary. New blocker is native DLL loading.

---

### Iteration 2b — DLL name mismatch (`libsf_core.dll` vs `sf_core.dll`)

**Motivation**: The wheel inspection showed `snowflake/connector/_core/sf_core.dll` but `c_api.py` looked for `libsf_core.dll`. On Windows, `cdylib` crates produce `sf_core.dll` (no `lib` prefix), unlike Linux/macOS.

**Rationale**: `_CORE_LIB_NAME = "libsf_core"` was used uniformly. This works on Linux (`libsf_core.so`) and macOS (`libsf_core.dylib`), but not on Windows where the cargo output is `sf_core.dll`. No prior Windows Python tests existed to catch this. The matrix was `[ubuntu-latest, windows-11-arm]` — `windows-11-arm` was added by this PR.

**Change**: Modified `c_api.py` to use `"sf_core.dll"` on Windows.

**Commit**: `4ad2d7f9`

**Observation (from run `22890645310` / commit `d8d8a1fb`)**: Diagnostic confirmed:
- `sf_core.dll` exists in `_core/` (16 MB)
- `dumpbin /headers` shows `AA64 machine (ARM64)` — correct architecture
- DLL name fix was present in the installed package (line number shifted to 52)
- **But** the diagnostic step itself was broken (`for /r` constructed fake paths, `dumpbin` failed on them, step exited nonzero, **tests never ran**)

**Conclusion**: DLL name mismatch was real and the fix is correct. Architecture is confirmed ARM64. But we have NOT yet seen the test run with the name fix because the broken diagnostic step blocked execution.

---

### Iteration 2c — Diagnostic fix (current)

**Motivation**: The broken diagnostic step prevented us from seeing whether the DLL name fix resolves the `ctypes.CDLL` load failure or whether there's a deeper issue (missing dependency DLLs, Python 3.8+ restricted DLL search).

**Rationale**: Need two pieces of information:
1. The actual `OSError` message from `ctypes.CDLL` (added to RuntimeError in `d8d8a1fb`)
2. What runtime DLLs `sf_core.dll` depends on (`dumpbin /dependents`)

**Change**: Made diagnostic step `continue-on-error: true`, simplified to direct `dumpbin` on the known `_core/sf_core.dll` path, added `dumpbin /dependents`.

**Commit**: `f16640e8`

**Observation**: Pending — CI running.

**Open hypotheses** (ordered by probability):
1. **DLL name fix was sufficient** — `ctypes.CDLL("sf_core.dll")` will work now that the file exists with the right name. We just never got to test it because the diagnostic step broke the pipeline.
2. **Python 3.8+ DLL search restriction** — even with the right name, `ctypes.CDLL` on Windows uses `LOAD_LIBRARY_SEARCH_DEFAULT_DIRS` which doesn't include PATH. If `sf_core.dll` has runtime dependencies (e.g. OpenSSL DLLs), they won't be found unless they're next to `sf_core.dll` or registered via `os.add_dll_directory()`.
3. **`sf_core.dll` has no external DLL dependencies** — if `aws-lc-rs` (non-FIPS) and OpenSSL are statically linked, `sf_core.dll` is self-contained and hypothesis 1 should hold.

**What the next CI run will tell us**:
- `dumpbin /dependents` → shows if there are runtime DLL deps
- Improved error message → shows the exact `OSError`
- If tests run and pass → hypothesis 1 was correct all along

### Iteration 2d — winmode=0 fallback + diagnostic fixes (current)

**Motivation**: Previous iterations confirmed sf_core.dll is ARM64 and has correct dependencies (libcrypto-3-arm64.dll, libssl-3-arm64.dll both bundled in _core/), yet `ctypes.CDLL` still fails with `WinError 127` ("procedure not found"). The diagnostic step was broken (cmd `for /r` without wildcards doesn't check file existence, producing phantom paths) and Python-level diagnostics used invisible `logging.warning()` calls.

**Rationale / hypothesis**: Python 3.8+ uses restricted DLL search (`LOAD_LIBRARY_SEARCH_DEFAULT_DIRS | LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR`) by default. On ARM64 Windows this may not correctly resolve dependency DLLs despite `os.add_dll_directory()` and `LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR` both pointing to `_core/`. Falling back to `winmode=0` (traditional search, which uses PATH) would confirm or disprove this: the vcpkg ARM64 OpenSSL bin dir is on PATH in CI.

**Changes**:
1. `c_api.py` — If default CDLL load fails on Windows, print diagnostic to stderr and retry with `winmode=0`. If both fail, raise the original error.
2. `test-python.yml` — Rewrote diagnostic step in PowerShell (correctly locates sf_core.dll in _core/ directory). Added Python DLL load test that tries each DLL individually, then tests sf_core.dll with both default and winmode=0.
3. `hatch_build.py` — Changed `rglob("*")` to `iterdir()` so only the top-level sf_core shared library is copied from the release dir. Proc-macro DLLs from `deps/` are no longer bundled in the wheel.

**Commit**: `43bca78b`

**What the CI run will tell us**:
- The Python DLL load test will show which individual DLLs load and which fail
- If `winmode=0` succeeds, the issue is with restricted DLL search on ARM64 Windows
- If `winmode=0` also fails, the issue is truly a missing procedure (function not in a dependency DLL)
- The diagnostic stderr output from c_api.py will show _core/ directory contents and the exact error

**Observation**: Pending — CI running.

**Conclusion status**: still unclear

### Review from other AIs

_(empty — pending review)_

---

## Plan Entry 3: Python ARM64 — Resolution

### Iteration history (chronological)

**2a** (already documented): `cryptography` source build failed — `OPENSSL_DIR` unset in `python_tests`.
Fix: vcpkg + static OpenSSL + vcvarsall arm64 in the test job.

**2b** (already documented): `RuntimeError: Couldn't load core driver` — wrong DLL name `libsf_core.dll`.
Fix: Windows cdylib produces `sf_core.dll` (no lib prefix); updated `c_api.py`.

**2c–2d** (already documented): Diagnostic iterations; `WinError 127` persisted.

---

### 2e — Static OpenSSL makes sf_core.dll self-contained

**Observations**:
- `dumpbin /dependents sf_core.dll` showed `libcrypto-3-arm64.dll`, `libssl-3-arm64.dll` as runtime deps
- `WinError 127` = `ERROR_PROC_NOT_FOUND` — a symbol the DLL expected was absent at `LoadLibrary` time
- OpenSSL DLLs were being bundled next to `sf_core.dll` in `_core/`, but `ctypes.CDLL` still failed

**Supposition**: Dynamic OpenSSL left sf_core.dll with external deps; `arm64-windows-static-md` embeds OpenSSL at link time, making sf_core.dll self-contained and eliminating all runtime DLL deps.

**Fix**: `OPENSSL_STATIC=1` + `arm64-windows-static-md` vcpkg triplet in wheel build.
Commit: `1f8f43a2`

**Conclusion**: Static OpenSSL eliminated runtime DLL deps. DLL became self-contained.

---

### 2f — strip=true on ARM64 cdylib removes .pdata → WinError 127

**Observations**:
- After static OpenSSL, `WinError 127` still appeared
- cargo workspace `Cargo.toml` had `strip = true` in `[profile.release]`
- ARM64 Windows requires `.pdata` section (exception unwind tables) for DLL loading

**Supposition**: `llvm-strip --strip-all` on ARM64 PE removes `.pdata`; without exception tables, Windows DLL loader fails with `ERROR_PROC_NOT_FOUND` at `LoadLibrary`.

**Fix**: `--config profile.release.strip=false` in cargo args, ARM64-only.
Commit: `4d1ee077`

**Conclusion**: Confirmed. `strip=false` resolved the `WinError 127`.

---

### 2g — iterdir() vs rglob() — host-arch proc-macro DLLs bundled

**Observations**:
- `release/deps/` contained DLLs compiled for the host (proc-macro build tools)
- `rglob("*")` recursively collected all `.dll` files including `deps/`
- Bundling host-arch proc-macro DLLs into the wheel caused spurious load errors

**Fix**: `iterdir()` instead of `rglob()` — only top-level `release/` files collected.
Commit: `4d1ee077`

**Conclusion**: `release/deps/` DLLs are build-time tools, not runtime dependencies. Never bundle them.

---

### 2h — --target aarch64-pc-windows-msvc is redundant

**Observations**:
- CI log: `syncing channel updates for '1.89.0-aarch64-pc-windows-msvc'`
- `windows-11-arm` runners install native ARM64 Rust toolchain by default
- Passing `--target` to the native host triple is a no-op; but changes output path to `<target_dir>/<triple>/release/`

**Fix**: Remove `--target` and simplify `release_dir` path to `<target_dir>/release/`.
Commits: `bbc86694`, `90ba546b`

**Conclusion**: Never assume host toolchain is x64 on ARM64 runners. Verify from CI logs.

---

### Summary of root causes and fixes

| Issue | Root cause | Fix |
|-------|-----------|-----|
| Wrong DLL name | Windows cdylib omits `lib` prefix | `_CORE_LIB_STEM` → `sf_core.dll` |
| `cryptography` build fail | `OPENSSL_DIR` unset in test job | vcpkg ARM64 static OpenSSL in `python_tests` |
| WinError 127 (DLL deps) | sf_core.dll dynamically linked OpenSSL | `OPENSSL_STATIC=1` + `arm64-windows-static-md` |
| WinError 127 (strip) | `strip=true` removes `.pdata` on ARM64 cdylib | `--config profile.release.strip=false` |
| Spurious load errors | `rglob()` bundled host-arch proc-macro DLLs | `iterdir()` — top-level only |
| DLL search restriction | Python 3.8+ `LOAD_LIBRARY_SEARCH_DEFAULT_DIRS` | `os.add_dll_directory(_core/)` |
| ARM64 Python not selected | uv defaults to x86_64 on `windows-11-arm` | Explicit `cpython-X.Y-windows-aarch64-none` |
| Redundant `--target` | Stale assumption about host arch | Remove; runners are native ARM64 |

### Key lessons (additions to ADR)

10. **cdylib + strip on ARM64 Windows**: `strip = true` removes `.pdata` (exception unwind tables). Always set `strip = false` for ARM64 Windows cdylib builds.

11. **Static linking for bundled native extensions**: When packaging a Rust cdylib inside a Python wheel, static link all C dependencies. Dynamic deps require runtime DLL resolution that ctypes restricted-search cannot satisfy.

12. **`rglob()` in cargo artifact collection**: `release/deps/` contains proc-macro DLLs (build-time tools). Use `iterdir()` to collect only top-level release artifacts.

13. **ARM64 Windows runners have native Rust**: `windows-11-arm` installs `aarch64-pc-windows-msvc` as the host triple. Passing `--target` for the host triple is redundant and changes output paths.

### CI result

Python CI on `windows-11-arm`: all jobs green. Matrix: py3.11, 3.12, 3.13 on ARM64; py3.9, 3.10, 3.14 on Linux only (no ARM64 Python wheel available for those versions on PyPI as of this writing).

