# PR Comment Replies — WOA64 JDBC & ODBC

Copy-paste-ready replies for every open comment thread on PR #402 (JDBC WOA64)
and PR #403 (ODBC WOA64). Post these replies to GitHub; the user handles posting.

Format: `[comment-ID] File:Line | Status | Reply text`

---

## PR #402 — JDBC WOA64

---

### [2980851472] `jdbc_bridge/build.rs:15` | FIXED

> Missing `cargo:rerun-if-changed` for `exports.def`.

Fixed. `cargo:rerun-if-changed` for `exports.def` is now emitted before the
`/DEF:` linker arg in `jdbc_bridge/build.rs` (line 17). Cargo will rebuild
the crate when the export list changes, keeping the DLL export table in sync.

---

### [2980851529] `jdbc/build.gradle:199` | FIXED

> `CORE_PATH` null-only check lets empty strings through.

Fixed. The env var is now read with `?.trim() ?: null` so both `null` and
empty strings fall through to the default. The resolved path is also
normalized via `.absolutePath` before use, which prevents failures when a
relative path is passed via the environment variable.

---

### [2980851538] `.github/workflows/test-jdbc.yml:120` | FIXED

> `Get-ChildItem ... -Recurse` to find `dumpbin.exe` is slow.

Fixed. The step now uses `vswhere.exe -find` with a glob pattern to locate
`dumpbin.exe` deterministically in one shot, without a recursive directory
scan. The `-find` flag was introduced in vswhere 2.6 and is available on all
GitHub-hosted Windows runners.

---

### [2980851556] `.github/workflows/test-jdbc.yml:154` | FIXED

> `GRADLE_TEST_RETRY_COUNT` missing from Windows ARM64 test step.

Fixed. `GRADLE_TEST_RETRY_COUNT` is now set in the `env:` block of the
Windows ARM64 Gradle test step, matching the Linux/macOS steps.

---

### [2981401856] `jdbc/build.gradle:200` | FIXED

> `PARAMETER_PATH` passed as-is; relative paths would fail `System.load()`.

Fixed alongside the `CORE_PATH` change. `PARAMETER_PATH` is now normalized
to an absolute path via `file(parameterPath).absolutePath` before being
passed as a test environment variable.

---

### [2981401912] `jdbc_bridge/build.rs:18` | DONE (earlier commit)

> `.unwrap()` on `CARGO_MANIFEST_DIR` should use `.expect(...)`.

Already addressed in a previous commit — `.expect("CARGO_MANIFEST_DIR must be set by Cargo")`
is used throughout the build script. No further action needed.

---

### [2987239261] `.github/workflows/test-jdbc.yml:128` | FIXED

> `$ErrorActionPreference = 'Stop'` missing from Windows ARM64 PowerShell test step.

Fixed. `$ErrorActionPreference = 'Stop'` is now the first line of the
Windows ARM64 PowerShell step, matching the pattern used in all other
PowerShell steps in this workflow.

---

### [2987239396] `.github/workflows/test-jdbc.yml:128` | EXPLAIN

> "Either run Windows test steps or update description to say it's build+export verification only."

JDBC tests on Windows ARM64 are intentionally limited to build + DLL export
verification. Apache Arrow does not ship `arrow_cdata_jni.dll` for
Windows ARM64 through version 19.0.0 (tracked upstream at
https://github.com/apache/arrow-java). The step name has been updated to
"Build DLL + verify exports (Windows ARM64)" to make the scope explicit, and
the PR description will be updated to call this out.

---

### [2987239423] `.github/workflows/test-jdbc.yml:107` | DONE (earlier commit)

> `Join-Path` missing quotes around `${{ github.workspace }}`.

Already fixed. `Join-Path "${{ github.workspace }}" ...` is quoted in the
current version.

---

### [2987239454] `sf_core/build.rs:66` | FIXED

> `sf_core/build.rs` doesn't emit `cargo:rerun-if-changed` for `exports.def`.

Fixed. `cargo:rerun-if-changed` for `exports.def` is now emitted at
`sf_core/build.rs:68` before the `/DEF:` linker arg.

---

### [2987312464] `python/tests/e2e/adr-woa64-ci-fixes.md:18` | FIXED

> ADR file placed in `python/tests/e2e/` — wrong location.

Removed. The file was deleted in commit `14f9d238`. The relevant design
decisions are documented in `doc/woa64/ADR-woa64.md`.

---

### [2987312507] `.github/workflows/test-jdbc.yml:37` | EXPLAIN

> Linux matrix expanded from Java 8 to 8/11/17/21 without PR description mentioning it.

The expansion is intentional. It provides cross-version coverage to catch
Java-version-specific compatibility regressions in the JDBC wrapper (e.g.
reflection API changes between LTS releases). The PR description will be
updated to mention this. The added CI cost is approximately 3 extra Linux
lanes, which is acceptable for the coverage value.

---

### [2987514124] `.github/workflows/test-jdbc.yml:120` | FIXED

> DLL export verification doesn't fail when `dumpbin` is missing or exports are absent.

Fixed. The step now throws with a descriptive error if `vswhere` cannot
locate `dumpbin.exe`. It also asserts that `Java_` and `JNI_` export
prefixes are present in the DLL export table, failing the step if either is
absent. This is the only correctness gate for the ARM64 DLL since functional
JDBC tests cannot run yet (Arrow limitation above).

---

### [2987514153] `jdbc/build.gradle:201` | FIXED

> `CORE_PATH` could be relative; `System.load()` requires absolute.

Fixed. Same null-safe normalization applied as for comment [2980851529] —
`CORE_PATH` is resolved to an absolute path via `.absolutePath` before use.

---

### [2987514181] `jdbc/build.gradle:201` | FIXED

> `PARAMETER_PATH` relative path.

Fixed. Same normalization as [2981401856] — `PARAMETER_PATH` is resolved to
an absolute path.

---

### [2987514200] `python/tests/e2e/adr-woa64-ci-fixes.md:5` | FIXED

> ADR doc in wrong location.

Same as [2987312464] — file removed in commit `14f9d238`.

---

### [2987829892] `.github/workflows/test-jdbc.yml:123` | EXPLAIN

> PR description vs actual Windows behavior (tests not running).

Same explanation as [2987239396]. The step is intentionally "build + export
verify" only due to the Arrow Windows ARM64 limitation. The PR description
will be updated to reflect this.

---

### [2987829955] `.github/workflows/test-jdbc.yml:135` | FIXED

> Recursive `Get-ChildItem` to find `dumpbin` is slow.

Fixed alongside [2980851538] — `vswhere.exe -find` is used instead.

---

### [2996211860] `sf_core/build.rs:70` | FIXED

> Comment incorrectly says "Cargo passes via response file" — it's `rustc`, not Cargo.

Fixed. Comment now reads: "No quoting: rustc passes this as a single token
to the MSVC linker (using a response file internally), so the linker
receives the path verbatim."

---

### [2996211952] `jdbc_bridge/build.rs:19` | FIXED

> Same comment attribution error in `jdbc_bridge/build.rs`.

Fixed. Same correction applied — "rustc" instead of "cargo" in the comment.

---

---

## PR #403 — ODBC WOA64

---

### [2981011602] `.github/workflows/test-odbc.yml:284` | EXPLAIN

> ARM64 build missing `RUSTFLAGS: "-C linker=rust-lld"`.

`rust-lld` was intentionally removed from the Windows ODBC build in commit
`e58071fe` on `woa64-rust-core`. When `sf_core` is statically linked via
`rlib` inside a `cdylib`, `lld-link` silently produces an empty DLL export
table. The ARM64 build never had `rust-lld` for the same reason. MSVC
`link.exe` handles the symbol count correctly via the `.def` file. A comment
explaining this has been added to the ARM64 build step.

---

### [2981011667] `.github/workflows/test-odbc.yml:340` | EXPLAIN

> ARM64 ODBC tests run only 1 of ~103 tests (`e2e_query_basic_execute_query`).

Intentional limitation. The full ODBC test suite on Windows ARM64 has not
been validated yet. The smoke test confirms basic end-to-end connectivity
on the new architecture. A TODO comment with tracking ticket SNOW-3045931
has been added: "TODO SNOW-3045931: expand ARM64 test coverage once full
suite is validated on ARM64 runners."

---

### [2981011695] `.github/workflows/test-jdbc.yml:123` | FIXED (PR #402)

> Recursive `dumpbin` search (same as PR #402 [2980851538]).

Fixed in PR #402 — `vswhere.exe -find` is used. Change is visible on this
PR through the stacked branch.

---

### [2981011738] `jdbc/build.gradle:200` | FIXED (PR #402)

> `CORE_PATH` empty string handling.

Fixed in PR #402 via `?.trim() ?: null` + `.absolutePath` normalization.
Change is visible on this PR through the stacked branch.

---

### [2982376233] `.ai/feature_prompts/adr-reviewer-agent.md:5` | FIXED

> Unrelated file included in PR.

Removed. Deleted in commit `03c75cff` ("Address PR #403 review comments:
scope cleanup + /DEF quoting"). The file is no longer present on this
branch.

---

### [2982376287] `jdbc_bridge/build.rs:20` | EXPLAIN

> `/DEF:` path with spaces might break without quoting.

No quoting is needed and none should be added. `cargo:rustc-cdylib-link-arg`
passes its value as a single token to `rustc`, which uses a response file
when invoking MSVC `link.exe`. The full path (including any spaces) is
delivered to the linker as a single argument. Adding shell quotes would make
them literal characters in the path, which both `lld-link` and MSVC
`link.exe` reject. This behavior is documented in the comment at that line.

---

### [2987294784] `.github/workflows/test-jdbc.yml:139` | DONE (commit `8275403c`)

> Dumpbin step doesn't enforce exports.

Fixed in commit `8275403c`. The step now throws if `dumpbin` is missing and
asserts that required `Java_` / `JNI_` exports are present.

---

### [2987294838] `sf_core/build.rs:66` | DONE (commit `8275403c`)

> `sf_core/build.rs` missing `rerun-if-changed` for `exports.def`.

Fixed in commit `8275403c`. `cargo:rerun-if-changed` is emitted for
`exports.def` at line 68.

---

### [2987294857] `odbc/build.rs:19` | DONE (commit `8275403c`)

> `odbc/build.rs` missing `rerun-if-changed` for `exports.def`.

Fixed in commit `8275403c`.

---

### [2987294876] `.github/workflows/test-jdbc.yml:37` | EXPLAIN

> Linux matrix expansion (duplicate of PR #402 [2987312507]).

Same as PR #402 response — the expansion to Java 8/11/17/21 is intentional
for cross-version coverage. PR description will be updated.

---

### [2987693978] `.github/workflows/test-jdbc.yml:39` | EXPLAIN

> Linux matrix expansion (duplicate of PR #402 [2987312507]).

Same as above.

---

### [2987694032] `.github/workflows/test-odbc.yml:337` | FIXED

> CMake test build step has no `vcvarsall.bat arm64` and no `-A ARM64` flag.

Fixed. `CMAKE_GENERATOR_PLATFORM=ARM64` is now set as an environment
variable in the `$env:CMAKE_GENERATOR_PLATFORM = "ARM64"` line inside the
ARM64 branch of the "Build and run ODBC tests (Windows)" step (commit
`93443041`). CMake reads this variable when selecting the generator platform,
ensuring ARM64 test binaries are generated natively rather than defaulting
to x64.

---

### [2988343250] `.github/workflows/test-odbc.yml:360` | DONE (commit `9339d6df`)

> Rerun step uses `target\debug\` (x64 path) even for ARM64.

Fixed in commit `9339d6df`. The rerun step now uses
`target\aarch64-pc-windows-msvc\debug\` for ARM64 and `target\debug\` for
x64.

---

### [2996235051] `.github/workflows/test-odbc.yml:205` | FIXED

> Rust `target` dir cache key uses `${{ runner.os }}` only — x64 and ARM64 share same key.

Fixed. `${{ runner.arch }}` is now included in the cache key:
```
key: ${{ runner.os }}-${{ runner.arch }}-rust-odbc-target-${{ hashFiles('**/Cargo.lock') }}
```
This ensures the x64 and ARM64 caches are stored under separate keys and
cannot cross-contaminate each other (commit `93443041`).

---

### [2996235081] `.github/workflows/test-odbc.yml:355` | FIXED

> Same CMake ARM64 environment concern — `vcvarsall.bat arm64` from the build step
> does NOT persist to the test step; each step is a new shell.

Fixed alongside [2987694032]. `CMAKE_GENERATOR_PLATFORM=ARM64` is set as a
PowerShell env var inside the ARM64 branch of the test step, so it is active
for the duration of that step's CMake invocation without relying on
`vcvarsall.bat` persisting across step boundaries.

---
