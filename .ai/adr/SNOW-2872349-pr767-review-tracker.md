# PR #767 — Code Review Tracker

**Branch under review:** `review-improvements` (HEAD of `SNOW-2872349-close-overrides`)
**Base:** `main`
**Reviewer:** Filip Pawlowski + Claude Code (5-lens sub-agent review)
**Started:** 2026-04-06
**Plan:** `/home/fpawlowski/.claude/plans/rosy-prancing-papert.md`
— Contains the full review procedure: Phase 0 baseline, Phase 2 5-agent loop, Phase 3 final pass.
**Design doc:** `.ai/docs/UD_LOGOUT_API_DD.md`
**Prior decisions:** `.ai/adr/SNOW-2872349-logout-refinement.md`

---

## How to resume

1. Read this file top-to-bottom to understand where the review left off.
2. Find the first file with status `pending` or `in-review` in the table below.
3. Run Phase 0 baseline check (see plan) if environment may have changed.
4. Launch 5 review sub-agents in parallel for that file (see plan §Phase 2 Step 1).
5. Present Reviewer B-filtered findings to the user.
6. Update this file after the user signals satisfaction.

---

## Review status

Order: Gherkin contracts first → their implementing tests → production source.
Tests are run during review; individual failures are diagnosed (see plan §Phase 2).
User provides the gherkin-validator-expert lens personally. No code changes during review.

**Correction (2026-04-08):** Feature files and implementing tests are reviewed as a PAIR,
not sequentially. Gherkin-implementation-reviewer findings about test honesty are in-scope
during the feature file review — never deferred. See plan §Phase 2 and §Tier 3 for details.

| # | File | Status | Key finding |
|---|------|--------|-------------|
| **Tier 1 — Build & Protocol (batch)** | | | |
| 1 | `Cargo.lock` | done | tracing-test 0.2.6 added (dev) |
| 2 | `sf_core/Cargo.toml` | done | tracing-test dev dep, features=no-env-filter |
| 3 | `protobuf/database_driver_v1.proto` | done | **2 High findings** — see log |
| 4 | `python/pyproject.toml` | done | mypy→3.10, editables, dev deps |
| 5 | `python/hatch_build.py` | done | editable target added |
| **Tier 2 — Gherkin Validator Source** *(user = gherkin-validator-expert)* | | | |
| 6 | `tests/tests_format_validator/src/step_finder.rs` | done | Trivial comment removal, no issues |
| 7 | `tests/tests_format_validator/src/utils.rs` | done | Doc placement: rationale in test body should be on fn doc |
| 8 | `tests/tests_format_validator/src/validator.rs` | done | `determine_common_test_level` outer `test_file_path` reporting mismatch |
| **Tier 3 — Feature Files → Implementing Tests (paired)** | | | |
| **Pair A — Shared** | | | |
| 9 | `tests/definitions/shared/session/logout.feature` | done | **High**: cross-cutting lying-test pattern in e2e implementations |
| 10 | `sf_core/tests/e2e/session/logout.rs` + `mod.rs` | done | 4 LIE steps, 1 PARTIAL; duplicate assertion, no WireMock counting, no barrier |
| **Pair B — Core** | | | |
| 11 | `tests/definitions/core/session/logout.feature` | done | **High**: 300s Examples rows unexecuted; Python: 5s comment wrong; timeout test bypasses connection layer |
| 12 | `sf_core/tests/integration/session/logout.rs` ★ | done | 3 LIE steps, 1 PARTIAL; WARN/ERROR mismatch; duplicate assertions; //Then keyword mismatch |
| 13 | `sf_core/tests/integration/session/connection_is_closed.rs` | done | Clean — 4 unit tests, all honest |
| 14 | `sf_core/tests/integration/session/mod.rs` | done | Trivial mod additions only |
| 15 | `sf_core/tests/integration/http/retry.rs` | done | Trivial `per_request_timeout: None` field addition |
| **Pair C — Python** | | | |
| 16 | `tests/definitions/python/session/logout.feature` | done | **Critical**: `core_mock` init-only; **High**: `keep_alive=True` zero e2e coverage |
| 17 | `python/tests/e2e/session/test_logout.py` ★ | done | **Critical**: `core_mock` can't verify close-path; 3 lying Gherkin steps; dead assertion variables |
| 18 | `python/tests/integ/session/test_logout.py` | done | **High**: 7× helper duplication; wrong TODO ticket reference |
| 19 | `python/tests/unit/test_connection.py` | done | No accepted findings |
| 20 | WireMock: `logout_success.json` + `logout_503_then_success.json` + `logout_500_always.json` | done | No accepted findings |
| **Tier 4 — Rust Test Infrastructure** | | | |
| 21 | `sf_core/tests/common/mocks/retry.rs` | in-review | |
| 22 | `sf_core/tests/common/mocks/session.rs` | in-review | |
| 23 | `sf_core/tests/common/mocks/mod.rs` | in-review | |
| 24 | `sf_core/tests/common/test_server.rs` ★ | in-review | |
| 25 | `sf_core/tests/common/mod.rs` | in-review | |
| 26 | `sf_core/tests/common/snowflake_test_client.rs` | in-review | |
| **Tier 5 — Production Source (Rust Core)** | | | |
| 27 | `sf_core/src/config/logout.rs` ★ | pending | |
| 28 | `sf_core/src/config/mod.rs` | pending | |
| 29 | `sf_core/src/config/rest_parameters.rs` | pending | |
| 30 | `sf_core/src/config/retry.rs` | pending | |
| 31 | `sf_core/src/rest/snowflake/logout.rs` ★ | pending | |
| 32 | `sf_core/src/rest/snowflake/mod.rs` | pending | |
| 33 | `sf_core/src/http/retry.rs` | pending | Pre-changed by ca9cc27c: max_elapsed now hard bound |
| 34 | `sf_core/src/apis/database_driver_v1/logout.rs` ★ | pending | |
| 35 | `sf_core/src/apis/database_driver_v1/async_query_registry.rs` ★ | pending | |
| 36 | `sf_core/src/apis/database_driver_v1/connection.rs` | pending | |
| 37 | `sf_core/src/apis/database_driver_v1/statement.rs` | pending | |
| 38 | `sf_core/src/apis/database_driver_v1/error.rs` | pending | |
| 39 | `sf_core/src/apis/database_driver_v1/mod.rs` | pending | |
| 40 | `sf_core/src/protobuf/apis/database_driver_v1.rs` | pending | |
| **Tier 5 — Production Source (Python)** | | | |
| 41 | `python/src/snowflake/connector/_internal/logout_config_mapping.py` ★ | pending | |
| 42 | `python/src/snowflake/connector/_internal/snow_logging.py` | pending | |
| 43 | `python/src/snowflake/connector/connection.py` | pending | |
| **Trivial batch** | | | |
| 44–49 | `put_get_source_compression.rs`, `__init__.py` ×2, `e2e/mod.rs` | pending | |

Statuses: `pending` | `in-review` | `done` | `deferred`

★ = substantive new file (>100 lines), warrants extra scrutiny

---

## Chronological Review Log

*(Each entry is added after the user signals satisfaction on a file or batch.
Format: date · file(s) · accepted findings · decisions made · action items)*

### 2026-04-06 · Files: Cargo.lock, sf_core/Cargo.toml, protobuf/database_driver_v1.proto, python/pyproject.toml, python/hatch_build.py

**Test results:** n/a (build/protocol files — no tests to run)

**Accepted findings:**

- [Intent Compliance / Maintainability / Clean Code] **High** — `ConnectionCloseRequest.enable_logout_auto_detection` (field 3) uses a shortened name that diverges from the canonical name `enable_server_session_keep_alive_auto_detection` used by the Core struct, Python kwarg, Python option key, and the design doc (DD §2). Not a neutral alias — creates cross-layer confusion for future wrapper authors.
  - **Action:** Rename proto field to `enable_server_session_keep_alive_auto_detection`.

- [Clean Code / Maintainability] **High** — `max_retry_attempts` in proto uses 0-based retry-count semantics; Rust `max_attempts` and Python `max_attempts` use 1-based total-attempts semantics. A silent `r + 1` conversion in `merge_with_request` bridges them. A wrapper reading only the proto will produce off-by-one behaviour without any indication.
  - **Action:** Either (a) rename proto field to `max_attempts` (total attempts, align to Core/Python), or (b) add an explicit proto comment stating "0-based retries; Core converts to total_attempts = value + 1 internally" and cross-reference in `merge_with_request` doc.

- [Maintainability] **Medium** — `Cython` and `setuptools` duplicated in `[build-system].requires` and `[tool.hatch.envs.dev].dependencies` without explanation. A maintainer could remove one copy thinking it redundant, silently breaking wheel or editable installs.
  - **Action:** Add a comment at the `[tool.hatch.envs.dev]` dependencies block: "hatch dev/editable install runs hatch_build.py outside the build-system resolver — Cython and setuptools must be listed here separately."

- [Security] **Medium** — `logout_total_timeout_seconds`, `max_retry_attempts`, `logout_request_timeout_seconds` in `ConnectionCloseRequest` are `optional int32` with no stated bounds or validation contract in the Core handler. Negative or INT32_MAX values are not rejected.
  - **Action:** Add validation in the Core handler (or proto comment) documenting the accepted range (e.g., `// must be > 0; values <= 0 are rejected`) and enforce bounds in Rust before use.

**Decisions:**
- No code changes during review — all findings are action items for Filip to implement.

**Action items:**
- [x] Rename `ConnectionCloseRequest.enable_logout_auto_detection` → `enable_server_session_keep_alive_auto_detection` in proto + all callers *(verified implemented)*
- [x] Resolve `max_retry_attempts` / `max_attempts` — proto now uses `max_attempts` (1-based), no silent conversion *(verified implemented)*
- [x] Add comment explaining Cython/setuptools duplication in pyproject.toml *(verified implemented)*
- [x] Add bounds validation for int32 fields — `validate_positive_seconds`, `validate_non_negative_seconds`, `v < 1` check; 7 unit tests *(verified implemented)*

### 2026-04-06 · Files: step_finder.rs, utils.rs, validator.rs (Tier 2 — Gherkin Validator Source)

**Test results:** 37 passed, 0 failed, 0 skipped (validator unit tests); 13 new utils tests all pass

**Accepted findings:**

- [Intent Compliance / Medium] — Outer `test_file_path` in `LanguageValidation` result may silently mismatch where per-scenario steps were actually validated (`validator.rs:~1124`). When `determine_common_test_level` returns `None` (mixed levels), the level-agnostic fallback picks one file for the struct, but the per-scenario loop independently resolves files by level — if both `e2e/` and `integration/` contain a matching file, the reported path may point to the wrong one.

- [Clean Code / Medium] — Design rationale for angle-bracket stripping belongs in `normalize_for_matching` or `strings_match_normalized` doc comment, not inside `test_strings_match_normalized_with_placeholders` test body (`utils.rs:~95-111`).

- [Intent Compliance / Low] — "spells out the placeholder name" comment in test is inaccurate for the lossy normalization. Both `<max-attempts>` and `max_attempts` collapse to `maxattempts` — this is lossy collapse, not name-preservation. Doc should clarify.

- [Clean Code / Low] — Public `strings_match_normalized` doc missing Scenario Outline rationale; richer explanation only on private `normalize_for_matching` (`utils.rs:37-39`).

**Contested finding (deferred to user judgment):**

- [Intent Compliance] — `determine_common_test_level` promotes unleveled `@python` scenarios to `E2E` (default from `get_test_level_for_language`), causing false "mixed level" detection. Reviewer B said REJECT but confirmed the finding is factually correct. **User decision:** All current feature files use explicit level suffixes (`@python_e2e`, `@python_integ`), so the edge case does not fire today. Noted as a latent issue — no action required now.

**Decisions:**
- All findings are about new code introduced in this PR, not pre-existing validator logic.
- Security findings (path traversal, unwrap panics, unbounded reads) confirmed pre-existing and rejected.
- No code changes during review.

**Action items:**
- [x] Move angle-bracket rationale to `strings_match_normalized` doc — added lossy-collapse explanation *(verified implemented)*
- [x] Fix misleading "spells out the placeholder name" comment — rewritten to describe actual collapse behaviour *(verified implemented)*
- [x] Outer `test_file_path` reporting — addressed as a TODO comment in the `None` branch; no code fix needed until mixed-level features are introduced *(verified implemented)*

### 2026-04-06 · File: tests/definitions/shared/session/logout.feature (Tier 3 — Pair A contract)

**Test results:** n/a (feature file — specification only)

**Accepted findings:**

- [Gherkin Tags / High] **Tag addition is invalid when the implementing test is a lying test.** A tag (e.g. `@core_e2e`) claims "a complete, honest test exists at this level." When a tag is added alongside a test that only checks `result.is_ok()` for "token is null" steps, the tag itself overstates coverage. Tags should only be added when the test is honest. No new tag types should be introduced.

- [Gherkin Implementation / High] **CROSS-CUTTING — Lying test pattern in both Rust and Python e2e tests.** Multiple Gherkin steps claim specific verifications that the implementing tests do not actually perform:
  - `Then Session token in Connection.tokens is null` → Rust e2e checks `result.is_ok()` only (TODO: "cannot be directly verified")
  - `Then Only one logout request is sent` → Rust e2e has no request-counting infra against real server; checks `result.is_ok()` only
  - `And enable_server_session_keep_alive_auto_detection true is passed to Core` → Python e2e reads `conn.logout_config` (Python-side dataclass), not Core state
  - Same pattern for `server_session_keep_alive none is passed to Core` and all "is passed to Core" steps
  - **Rule:** Gherkin is the contract and must NOT be weakened. Tests must be fixed to match their step claims.
  - **Action:** Each lying e2e test must add real verification infrastructure (inspect Core state, count HTTP requests). The Gherkin tags and steps stay as-is — the scenarios are valid e2e contracts. The test infra needs to catch up, not the spec to be weakened.

- [Gherkin Implementation / Medium] — Concurrent close: thread count inconsistent across implementations (Rust=5, Python=3). Neither implementation uses a WireMock mock server to count requests, nor a `Barrier` to ensure threads genuinely race. Without both, the test may degenerate into sequential idempotency. Fix: WireMock with fixed delay (creates I/O concurrency window) + `Barrier` (ensures deterministic simultaneous start) + assert exactly 1 request received.

- [Security / Medium] — No scenario for token-refresh race after close. Implementation has guards (`is_closed` in `execute_query_internal` + `RefreshContext::from_arc`) but no BDD contract locks this in.

- [Clean Code / Low] — Idempotent close and concurrent close scenarios both assert "Only one logout request is sent" without inline comments distinguishing serial idempotency from thread-safety.

**Decisions:**
- Feature file (Gherkin) is sacrosanct — changing step text is absolute last resort.
- Lying-test detection is now a mandatory part of every test file review going forward.
- Cross-cutting lying-test finding applies to files #10, #17, and potentially others; will be assessed per-file.

**Action items:**
- [x] Fix Rust e2e lying tests: `connection_get_info_blocking(include_master_token)` added to `SnowflakeTestClient`; token cleanup uses it; idempotent/concurrent use WireMock + request counting *(verified implemented)*
- [x] Fix Python e2e lying tests: `core_mock.get_options_sent()` / `core_proxy.get_options_sent()` replace `conn.logout_config` reads in commit 7b37da50 *(verified implemented)*
- [x] Fix concurrent close test: WireMock with 500ms delay + `std::sync::Barrier` + assert exactly 1 request *(verified implemented in Rust e2e)*
- [x] Add `@python_e2e` to token cleanup scenario — implemented in commit d7e6b184; Python test uses `conn.rest.token` / `conn.rest.master_token` for honest assertions *(verified implemented)*
- ~~Consider adding scenario for query-in-flight + close race condition~~ — removed: sequential close→query rejection is already covered; concurrent variant needs test harness infrastructure (hold query in-flight + concurrent close) that does not exist; speculative Gherkin without implementation violates tags-only-when-implemented rule

### 2026-04-08 · File: sf_core/tests/e2e/session/logout.rs + mod.rs (Tier 3 — Pair A implementing tests)

**Test results:** 0 passed, 4 failed — `NotPresent` at `config.rs:78`. Classification: environment issue (missing `PARAMETER_PATH` for real Snowflake). Not test logic.

**Assertion honesty audit:**

| Test | Step | Grade |
|------|------|-------|
| token_cleanup | Then Session token in Connection.tokens is null | **LIE** |
| token_cleanup | And Master token in Connection.tokens is null | **LIE** (identical assertion to above) |
| idempotent | When/And close calls ×3 | HONEST |
| idempotent | Then Only one logout request is sent | **LIE** |
| idempotent | And No errors are thrown | HONEST |
| concurrent | When closed from multiple threads concurrently | HONEST |
| concurrent | Then Only one logout request is sent | **LIE** |
| concurrent | And All close calls return successfully | HONEST |
| post_close | Then the query fails with a connection-closed error | **PARTIAL** |

**Accepted findings:**

- [Gherkin Honesty / High] — Two token-step assertions both check identical `result.is_ok()` binding; second cannot fail independently. Lies confirmed on both `Then Session token is null` and `And Master token is null` steps.

- [Intent Compliance / Medium] — `"not initialized"` disjunct in error check accepts `ConnectionNotInitialized` as a valid "connection-closed error". These are distinct variants; the test would pass even if the `is_closed` guard were removed and `http_client=None` fired instead.

- [Intent Compliance / Medium] — Concurrent close has no `Barrier` (threads may serialize) and no WireMock (request count unverifiable). Test degenerates into idempotency test. Fix: WireMock with fixed delay + `Barrier` + assert exactly 1 request. *(Refines cross-cutting finding from file #9.)*

- [Maintainability / Medium] — TODOs cite parent ticket SNOW-2872349 rather than dedicated follow-up tickets; gaps will be buried when parent closes.

- [Clean Code / Medium] — Error check `contains("closed") || contains("Closed") || contains("not initialized")` — brittle; known limitation of `execute_query_no_unwrap` returning `String`.

**Decisions:**
- `@core_e2e` tag additions on lying-test scenarios are themselves invalid until tests are honest (per tags-only-when-implemented rule).
- No code changes during review.

**Action items:**
- [x] Add `connection_get_info_blocking` / token inspection API to `SnowflakeTestClient` *(verified implemented)*
- [x] Fix concurrent close test: WireMock + fixed delay + `Barrier` + assert exactly 1 request *(verified implemented)*
- [x] Resolve `"not initialized"` in error check — `execute_query_no_unwrap` now returns typed `ProtoError<DriverException>`; test matches on `ProtoError::Application(exc)` + `exc.message.contains("closed")`; disjunct gone *(verified implemented)*
- [x] Replace SNOW-2872349 TODO references with dedicated infra improvement tickets *(verified: zero SNOW-2872349 refs in sf_core/tests/e2e/session/logout.rs)*

### 2026-04-09 · Files: tests/definitions/core/session/logout.feature + sf_core/tests/integration/session/logout.rs + connection_is_closed.rs + mod.rs + http/retry.rs (Tier 3 — Pair B)

**Test results:** 20 passed, 0 failed (session::logout integration); 4 passed, 0 failed (session::connection_is_closed). 1 compiler warning: unused import `proto_utils::ProtoError` in `snowflake_test_client.rs`.

**Assertion honesty audit:**

| Test | Step | Grade |
|------|------|-------|
| `should_attempt_token_refresh_on_390112` | And Logout is retried with new session token | **LIE** — counts `logout_count >= 2`, never checks `Authorization` header on 2nd request |
| `should_ignore_session_gone_390111` | And Error is ignored | HOLLOW — behavior proven by preceding `result.is_ok()` in `//Then Close succeeds`; not a lying test |
| `should_throw_after_exhausted_retries_with_strict` | And WARN log is emitted | **LIE** — production emits `tracing::error!`; `logs_contain` is level-blind |
| `should_log_warn_and_succeed_after_exhausted_retries` | And WARN log is emitted | **PARTIAL** — level is correct (`warn!`) but `logs_contain` is level-blind |
| `should_timeout_after_5_seconds_by_default` | Given UD Core connection is logged in | **LIE** — calls `logout_session()` directly, never creates a connection |
| All other scenarios | All steps | HONEST |

**Accepted findings:**

- [Gherkin Honesty / High] — `should_attempt_token_refresh_on_390112` step "And Logout is retried with new session token" (`logout.rs:916-926`). Asserts `logout_count >= 2` only. WireMock has an `Authorization`-matching mock for the refreshed-token logout but no `.expect(1)`. BestEffort strategy would pass even if old token was reused in the retry.
  - **Action:** Add explicit `Authorization` header assertion on the second logout request, or add `.expect(1)` to the Authorization-header-specific WireMock mock.

- [Gherkin Honesty / High] — `should_throw_after_exhausted_retries_with_strict_strategy` step "And WARN log is emitted" (`logout.rs:1267`). Production code at `config/logout.rs:191` emits `tracing::error!` for strict strategy. `logs_contain` is level-blind — test passes regardless. Feature says WARN; implementation says ERROR. These are contradictory. Design decision required.
  - **Action:** Decide whether strict strategy should log at WARN or ERROR. Gherkin is sacrosanct: if the contract says WARN, change `tracing::error!` → `tracing::warn!` in strict strategy. If ERROR is correct, update Gherkin (last resort). Either way, fix assertion to be level-aware.

- [Gherkin Honesty / Medium] — `should_log_warn_and_succeed_after_exhausted_retries_with_best_effort` step "And WARN log is emitted" (`logout.rs:1361`). Current level is correct (`tracing::warn!`) but `logs_contain` is level-blind — a future change to `error!` is not caught.
  - **Action:** Use a level-aware log assertion.

- [Cross-cutting Infrastructure / Medium] — `logs_contain()` from `tracing-test` is level-blind by design: it matches on message text within span scope regardless of log level. Any test using it for level verification gives false assurance. Both log-level findings above (strict `error!` vs Gherkin WARN; best-effort level-blind after a refactor) share this root cause. Not a per-test bug — it is a test infrastructure gap.
  - **Action:** Add a level-aware assertion variant (e.g. `logs_at_level(Level::WARN, "...")`) or a custom subscriber that captures level alongside message. Apply to all exhausted-retries log assertions.

- [Gherkin Change / High] — Examples rows `| strict | 15 | 13 |` and `| best-effort | 15 | 13 |` were **deleted** from `should_honor_provided_timeout_config_and_succeed_for_each_strategy_type` in the feature file. In `main` these rows carried the comment `# Python default timeout — also tighter margin to catch hardcoded values < 13`, covering (a) Python's 15s default and (b) a deliberate canary for hardcoded values below 13s. Their removal narrows the contract. The companion comment was simultaneously changed from `# Wrappers pass their historical defaults (Python: 15s, JDBC/ODBC: 300s)` to `(Python: 5s, JDBC/ODBC: 300s)` — `Python: 5s` is factually wrong; the connector defaults to 15s.
  - **Note:** Gherkin deletion = last resort. No justification provided.
  - **Resolution (commit 96173c3b):** Rows restored; `Python: 5s` reverted to `Python: 15s`. Also: 300s rows delay updated from 10→50 and 10s row delay from 5→8. ✅ RESOLVED (Gherkin)

- [Gherkin Change / High] — Step `And Retry policy allows the default attempt number` was **deleted** from `should_honor_provided_timeout_config` in the feature file. This step verified that the default retry count (not just the timeout) is applied. Its removal means a regression in the default retry count would not be caught.
  - **Note:** Step was in `should_honor_provided_timeout_config`, not `should_timeout_after_5_seconds_by_default` (attribution error in original finding).
  - **Resolution (commit 96173c3b):** Step restored at `logout.feature:290`. ✅ RESOLVED (Gherkin)

- [Intent Compliance / High] — `should_honor_provided_timeout_config_and_succeed_for_each_strategy_type` is tagged `@core_int` but the integration test loop at `logout.rs:1114` only iterates `[(5,3), (10,5)]`. After commit `96173c3b`, the feature now has 8 rows (4 timeout values × 2 strategies). The test loop is stale on three counts: (1) `(10, 5)` delay should be `(10, 8)` per restored Gherkin, (2) `(15, 13)` rows absent, (3) `(300, 50)` rows absent. Also missing: assertion for restored `And Retry policy allows the default attempt number` step.
  - **Action:** Fix test loop to `[(5,3), (10,8), (15,13), (300,50)]` × both strategies; add `Retry policy allows the default attempt number` assertion.

- [Intent Compliance / High] — `should_timeout_after_5_seconds_by_default_when_server_does_not_respond` (`logout.rs:344-408`). Calls `logout_session()` directly with hand-built `RetryPolicy { per_request_timeout: Some(Duration::from_secs(5)), ... }`. Feature step: "Given UD Core connection is logged in with no timeout override." Never creates a connection. A regression in `connection_close_blocking`'s default retry policy would not be caught.
  - **Action:** Rewrite to use `SnowflakeTestClient::connect_integration_test()` with no timeout options set, then call `connection_close_blocking()`, verify timeout fires.

- [Clean Code / High] — `logout.rs:388-391`: two consecutive `//Then` step comments. Feature step is `Then Close throws timeout error`. Gherkin grammar requires continuation steps to use `And`. Line 388 (`//Then Logout request times out...`) should be `//And`. Test file acknowledges this in a TODO at line 340-342.
  - **Action:** Change `//Then Logout request times out after approximately 5 seconds` to `//And Logout request times out after approximately 5 seconds`.

- [Maintainability / Medium] — Duplicate `assert_eq!(logout_count, max_attempts as usize, ...)` back-to-back in both exhausted-retries tests (`logout.rs:1253-1265` and `logout.rs:1347-1359`). Second assertion checks identical condition with a different message string only.
  - **Action:** Remove the second assertion in both test functions.

- [Clean Code / Low] — `should_ignore_session_gone_390111` `//And Error is ignored` step comment (`logout.rs:622-626`) has no assertion — just `server.await.unwrap()`. The behavior is already proven by the preceding `//Then Close succeeds` / `result.is_ok()` assertion. The hollow comment is redundant and misleading. The `@core_int` tag is valid — the behavior IS covered; the comment structure is the issue.
  - **Action:** Remove the hollow `//And Error is ignored` comment, or consolidate into `//Then Close succeeds (error absorbed)`.

**Additional observations (not blocking):**
- Two integration tests have no corresponding Gherkin scenario: `should_not_send_logout_when_connection_was_never_established` and `should_cancel_individual_request_when_per_request_socket_timeout_exceeded`. No `@core_int` tag used; these are supplementary tests.
- Compiler warning: `unused import: proto_utils::ProtoError` in `snowflake_test_client.rs` — should be removed.

**Decisions:**
- No code changes during review.
- `@core_int` tags on scenarios with lying tests are themselves invalid until implementations are honest (per tags-only-when-implemented rule): token refresh, exhausted-retries (WARN/ERROR level mismatch), timeout-default. Session-gone tag is valid — behavior is proven by `result.is_ok()`; the hollow comment is a code quality issue only.
- WARN vs ERROR for strict strategy is a design question requiring user decision before the implementation can be fixed.

**Action items:**
- [x] `should_attempt_token_refresh_on_390112`: `.expect(1)` added to refreshed-token WireMock mock — proves retry used new token *(commit 59606561)*
- [x] `should_ignore_session_gone_390111`: `server.await.unwrap()` → named binding + explicit assert with failure message; step is no longer a bare unwrap. Full consolidation into `//Then Close succeeds (error absorbed)` blocked — would require Gherkin change to `tests/definitions/core/session/logout.feature:169`. *(commit 4a590696)*
- [x] Strict strategy log level: decided ERROR (aligns with `tracing::error!`); Gherkin updated `And WARN log is emitted` → `And Error log is emitted`; `logs_assert` checks `line.contains("ERROR") && line.contains("Logout failed")` *(commit 59606561)*
- [x] Best-effort log assertion: `logs_assert` checks `line.contains("WARN") && line.contains("Logout failed")` *(commit 59606561)*
- [x] Level-aware log assertion: `logs_assert` per-line matching (not `logs_contain`) *(commit 59606561)*
- [x] Restore deleted 15s/13s Examples rows + Python:5s→15s comment fix *(commit 96173c3b)*
- [x] Restore deleted `And Retry policy allows the default attempt number` step *(commit 96173c3b — step is in `should_honor_provided_timeout_config`, not `should_timeout_after_5_seconds_by_default` as originally attributed)*
- [x] `should_honor_provided_timeout_config`: test loop updated to `(5,3), (10,8), (15,13), (300,50)` × both strategies; `//And Retry policy allows the default attempt number` step comment added; step satisfied by `..Default::default()` (Given-level step, no assertion required) *(commit 59606561)*
- [x] `should_timeout_after_5_seconds_by_default`: rewritten to use `SnowflakeTestClient::connect_integration_test()` + `connection_close_blocking()`. Production fix in `http/retry.rs` makes `max_elapsed` a hard bound when `per_request_timeout` is None, enabling the test to be honest *(commit ca9cc27c)*
  - **Note:** `ca9cc27c` also changed `sf_core/src/http/retry.rs` (file #33 in review list) — this production change will be reviewed at Tier 5.
- [x] `logout.rs:388`: `//Then Logout request times out...` → `//And Logout request times out...` *(commit 59606561)*
- [x] Remove duplicate `assert_eq!` in both exhausted-retries tests *(commit 59606561)*
- [x] Remove unused import `proto_utils::ProtoError` from `snowflake_test_client.rs` *(resolved: import is actively used in 5 return types after typed-error commit 49d67c4d; confirmed removed in 59606561)*
- [x] Replace SNOW-2872349 TODO references with dedicated infra improvement tickets *(resolved: zero SNOW-2872349 refs remain in test files; replaced with SNOW-2923705)*

### 2026-04-09 · File: tests/definitions/python/session/logout.feature — Auto-cleanup section (pre-implementation review)

**Context:** Five new scenarios added under the `# Auto-cleanup Deprecation` section (lines 170–215) plus one leak-detection scenario (line 216–223). These scenarios were committed before their implementing tests exist; this review was done to catch Gherkin issues before implementation to prevent lying tests being written to match flawed steps.

**Test results:** n/a — no implementing tests exist yet for this section.

**Accepted findings:**

- [Gherkin Honesty / High] — `should_call_close_with_retry_false_from_atexit_handler` (S3): `When Process exits` is a lying test trap. A returning Python test cannot actually exit the process — any implementation will call the atexit handler function directly. When that happens the step text `When Process exits` misrepresents the action under test.
  - **Action:** Change step to `When atexit handler is invoked`.

- [Gherkin Honesty / High] — S3: `And Session is logged out if conditions allow` embeds a conditional branch in a Gherkin step. "If conditions allow" is ambiguous and makes the step non-deterministic. Gherkin steps must describe one concrete outcome.
  - **Action:** Either (a) replace with `And Session logout is attempted` (unconditional), or (b) split into a separate scenario that specifies the exact conditions under which logout is or is not sent.

- [Gherkin Honesty / High] — `should_unregister_atexit_handler_when_close_called_explicitly` (S2): `And Subsequent process exit will not trigger second close` describes a future event that cannot be directly asserted in a returning test. The preceding `Then atexit handler is unregistered` is the correct, directly testable claim. The And step restates its implication without adding assertable content.
  - **Action:** Remove the step. If narrative clarity is needed, use `And atexit handler is no longer registered for this connection` — which collapses into the preceding Then.

- [Gherkin Honesty / Medium] — `should_not_register_atexit_handler_when_auto_cleanup_explicitly_disabled` (S5): `When Process exits` has the same lying-test trap as S3. The assertion `Then No atexit handler was registered` is checkable immediately after connection init — it does not require simulating process exit.
  - **Action:** Remove `When Process exits` or replace with `When Connection is initialized`. Also remove `And No automatic close is performed` — tautological consequence of no handler being registered.

- [Clean Code / Medium] — `should_have_auto_cleanup_enabled_by_default` (S1): `When Connection configuration is checked` is a passive observation, not an action. Both Then steps (`auto_cleanup defaults to true` and `atexit handler is registered at connection init`) describe state already established in Given. The two assertions also mix concerns: config default vs behavioural side-effect of init. A future change to atexit registration mechanism (not the default value) would break both.
  - **Action:** Remove the passive When. Consider splitting into two scenarios — one for config default, one for atexit registration side-effect — or at minimum rename to make both assertions explicit in the scenario title.

- [Clean Code / Medium] — `should_emit_deprecation_warning_only_once_...` (S4): `10 Snowflake clients` is a magic number. "Only once per process" is proven by 2+. The number also implies a heavier subprocess with no documented reason.
  - **Action:** Change to `multiple Snowflake clients` in the step, or add an inline comment explaining why 10 specifically.

- [Clean Code / Low] — S3: `And No retries are attempted during atexit close` is redundant with `Then atexit handler calls close(retry=False)`. `retry=False` by definition means no retries — the And step restates the same observable fact.
  - **Action:** Remove the redundant step.

- [Clean Code / Low] — `should_emit_telemetry_and_WARN_when_connection_leaked_at_process_exit` (S6, not in auto-cleanup section): the section header is `# Auto-cleanup Deprecation`; leak telemetry is a distinct concern (observability of leaked connections, not the cleanup mechanism). A reader looking for leak detection would not find it under this section.
  - **Action:** Move S6 to a dedicated `# Leak Detection` section.

**Additional observations (not blocking):**
- S4 correctly uses subprocess framing (`Given A separate Python subprocess is spawned` / `When The subprocess exits`) — this is the right pattern for process-exit scenarios and does not carry the lying-test risk of S3/S5.
- S4 commit message note: labeled `# Phase 1 (doc for: SNOW-2314152) deprecation` among otherwise Phase 2 scenarios. Clarify whether Phase 1 behavior is being introduced here or was already present.
- S2 `And atexit handler is registered at connection init` in Given: redundant given `auto_cleanup enabled` implies registration (established by S1), but acceptable for step-level clarity.

**Decisions:**
- No code changes during review. All findings are pre-implementation corrections to be applied before tests are written.
- S4 subprocess approach is sound — no changes needed to its structure.
- S6 placement is a file organisation issue; does not block implementation.

**Action items:**
- [ ] S3: `When Process exits` → `When atexit handler is invoked`
- [ ] S3: `And Session is logged out if conditions allow` → remove or replace with concrete unconditional step
- [ ] S3: remove redundant `And No retries are attempted during atexit close`
- [ ] S2: remove `And Subsequent process exit will not trigger second close`
- [ ] S5: remove `When Process exits` (or replace); remove tautological `And No automatic close is performed`
- [ ] S1: remove passive `When Connection configuration is checked`; consider splitting into two scenarios
- [ ] S4: `10 Snowflake clients` → `multiple Snowflake clients` or add comment explaining count
- [ ] S6: move to a `# Leak Detection` section

### 2026-04-09 · Files: tests/definitions/python/session/logout.feature + python/tests/e2e/session/test_logout.py + python/tests/integ/session/test_logout.py + python/tests/unit/test_connection.py + WireMock mappings (Tier 3 — Pair C)

**Test results:**
- Integ (`hatch run dev.py3.13:integ -k test_logout`): 7 passed, 1 skipped (TestLogoutPhase5Optimization — intentionally unimplemented), 0 failed
- E2e (`hatch run dev.py3.13:e2e -k test_logout`): 21 passed, 4 skipped, 0 failed (exit code 1 from unrelated `pandas/` collection errors — pre-existing environment issue; no logout tests failed)

**Assertion honesty audit (python/tests/e2e/session/test_logout.py):**

| Test | Step | Grade |
|------|------|-------|
| `test_should_send_logout_when_auto_detection_false` | Then Auto-detection is not performed | **PARTIAL** — `assert conn.is_closed()` is a premise guard; logout request count at lines 316-318 proves logout was sent but cannot prove detection was skipped (same result if detection=True + empty registry). Subsumed by Finding 10. |
| `test_should_send_logout_when_auto_detection_false` | And Connection close metrics are recorded in telemetry | **LIE** — `_telemetry_verified = conn.is_closed()` assigned to unread variable; always passes |
| `test_should_use_best_effort_error_handling_strategy_by_default` | And Error is logged as WARN | **LIE** — `_warn_logged = True` unconditional assignment, no log capture |
| `test_should_use_best_effort_error_handling_strategy_by_default` | And close() method does not raise exception | **HOLLOW** — `_close_succeeded = conn.is_closed()` unread; behavior proven by preceding `conn.close()` not raising |
| `test_should_emit_deprecation_warning_only_once_...` | And Each auto-cleanup close is invoked with retry false | **LIE** — checks `req["request"]["method"] == "POST"`; HTTP POST proves logout was sent, not that `retry=False` was passed |
| `test_should_emit_deprecation_warning_only_once_...` | And Deprecation warning is emitted only once per process | HONEST — `result.stderr.count(warning_text) == 1` is a genuine assertion |
| All `core_mock.get_options_sent()` tests | Then [param] is passed to Core | **PARTIAL** — records init-time `connection_set_option_*` calls only; `connection_close` re-derives LogoutConfig from `connection_seed` at close-time; close-path regressions invisible |

**Accepted findings:**

- [Intent Compliance / **Critical**] — `core_mock.get_options_sent()` only records init-time `connection_set_option_*` calls. All `"is passed to Core"` `@python_e2e` tests (e.g. `test_should_pass_correct_parameters_when_server_session_keep_alive_is_none_and_auto_detection_true`, `test_should_pass_server_session_keep_alive_false_to_core_when_auto_detection_explicitly_disabled`) verify Python→Core plumbing at init, not the config consumed by `connection_close`. Close-path regressions are invisible.
  - **Action:** Switch these tests to `core_proxy` approach or add close-time inspection to verify `connection_close` receives the intended parameters.

- [Intent Compliance / **High**] — `server_session_keep_alive=True` (fire-and-forget safety net: no logout at all) has zero e2e coverage. The feature file has scenarios but none carry `@python_e2e` tags. The most important safety invariant of the feature is unexercised end-to-end.
  - **Action:** Add `@python_e2e` tagged tests that assert `len(logout_requests) == 0` when `server_session_keep_alive=True`.

- [Intent Compliance / **Medium**] — The "should emit telemetry and WARN when connection leaked at process exit" scenario has no implementing test, no tag, and no `pytest.mark.skip` / TODO. Silently absent.
  - **Action:** Add `pytest.mark.skip(reason="TODO: SNOW-XXXXXXX — telemetry not yet implemented")` as a placeholder.

- [Gherkin Honesty / **High**] — `_telemetry_verified = conn.is_closed()` (`e2e:325`): step "And Connection close metrics are recorded in telemetry" assigns `conn.is_closed()` to an unread variable. Never asserted.
  - **Action:** Replace with `pytest.mark.skip`-guarded TODO or a real telemetry assertion; remove the fake variable.

- [Gherkin Honesty / **High**] — "Each auto-cleanup close is invoked with retry false" (`e2e:824-826`): verified by `req["request"]["method"] == "POST"`. HTTP POST only proves logout was issued; says nothing about the `retry=False` argument.
  - **Action:** Use a spy/mock on `Connection.close` to assert it was called with `retry=False`, or inspect a Core-side flag.

- [Gherkin Honesty / **Medium**] — `_warn_logged = True` (`e2e:533`): step "And Error is logged as WARN" for best-effort strategy has no log capture; always passes.
  - **Action:** Replace with `pytest.mark.skip(reason="TODO: SNOW-2314153 — logging integration required")` or log capture.
  - `_close_succeeded = conn.is_closed()` (`e2e:536`): hollow variable for "And close() method does not raise exception"; behavior already proven by `conn.close()` not raising. Remove and annotate the existing `conn.close()` call with the step comment.

- [Clean Code / **Medium**] — `assert len(logout_requests) == 1` appears twice on the same variable for two distinct Gherkin steps at `e2e:600-605` ("Then Logout is not retried" and "And Only one logout request was sent to server"). Identical assertion; one step unverified.
  - **Action:** Add a `len(result.history)` or equivalent distinct assertion for "Then Logout is not retried" (retries would produce additional requests logged in the client), leaving the count assertion for "And Only one logout request was sent to server".

- [Maintainability / **High**] — `get_wiremock_requests` / `filter_logout_requests` inlined 7 times in `python/tests/integ/session/test_logout.py`; the e2e file already extracts these as module-level helpers. Filter predicate may diverge silently.
  - **Action:** Extract to a shared test helper (e.g. `tests/integ/session/helpers.py` or import from the e2e conftest).

- [Maintainability / **Medium**] — `python/tests/integ/session/test_logout.py:190-200`: `@pytest.mark.skip(reason="TODO: SNOW-2872349 - Phase 5")` references the parent PR ticket. Per Pair B decision, SNOW-2872349 TODO references in test files must be replaced with dedicated sub-task tickets (Pair B resolved this in Rust tests via SNOW-2923705).
  - **Action:** Replace `SNOW-2872349` in skip reason and `pytest.fail` body with a dedicated Phase 5 ticket (or SNOW-2923705 if appropriate).

- [Performance / **High**] — `test_should_call_close_with_retry_false_from_atexit_handler` spawns 2 sequential subprocesses each with 120s timeout. Worst-case: 240s per test. Unavoidable by design; warrants a comment so CI timeout thresholds are understood.
  - **Action:** Add a comment above the test: `# This test spawns 2 subprocesses × 120s timeout each (240s worst case). CI timeout must be >= 300s.`

- [Security / **Medium**] — `assert "Snowflake Token" in auth_header`: pytest assertion rewriting interpolates the full `auth_header` value into failure output. Even with fixture tokens, the pattern establishes a footgun.
  - **Action:** Wrap as `assert "Snowflake Token" in auth_header[:20]` or use `assertIn` with a helper that masks the token tail, or add a `# noqa: S105` with explanation.

**Rejected findings (with rationale):**

| # | Lens | Rejected because |
|---|------|-----------------|
| 1 | Clean Code / High | Line 310 `assert conn.is_closed()` is a premise guard; the auto-detection claim is substantiated by logout request count at 316-318. (Finding subsumed by Critical #10 which covers the entire `core_mock` architecture.) |
| 5 | Performance / Medium | Shared WireMock for integ tests requires per-test journal resets — not straightforwardly safe without that mechanism |
| 6 | Performance / Medium | Reviewer confirmed WireMock is scoped to the whole test body, not per-parameter; finding misread the code |
| 13 | Gherkin Honesty / High | Misattributed: `_warn_logged = True` belongs to "And Error is logged as WARN" (WARN-log step), not "And Deprecation warning is emitted only once". The latter IS genuinely asserted at lines 828-833. Dead variable covered by accepted Finding 2. |
| 16 | Security / Medium | Subprocess tests authenticate via WireMock (local mock); no real session tokens in stderr |
| 18 | Security / Low | `wiremock_url` and `private_key_path` sourced from internal test harness, not external input; not a real injection threat |
| 8 | Maintainability / Medium | **NOTE: WRONGLY REJECTED** — Reviewer B was unaware of the Pair B decision establishing that SNOW-2872349 TODO references must be replaced with dedicated tickets. This finding is escalated to ACCEPT (see action item above). |

**Decisions:**
- No code changes during review.
- `core_mock` architecture gap (Finding 10) is the highest-impact finding. The `@python_e2e` tags on all "is passed to Core" tests remain technically invalid until the tests either use `core_proxy` or add close-time inspection. However, the tag names are correct as aspirational contracts — the implementation must catch up, not the tags be removed.
- `server_session_keep_alive=True` zero coverage (Finding 11): this is the most immediately dangerous gap. A regression that accidentally sends logout when `keep_alive=True` would go undetected.
- Finding 8 re-escalated to ACCEPT after verifying the established Pair B precedent. Reviewer B rejection was based on missing context.

**Action items:**
- [x] Switch `"is passed to Core"` tests from `core_mock` to `core_proxy` *(commit aab6e673 — 2 tests switched to int_test_connection_factory + core_proxy + WireMock)*
- [x] Add `@python_e2e` test(s) asserting zero logout requests when `server_session_keep_alive=True` *(commit fea3a7d1 — parametrized True+True / True+False; verifies len==0, option passed to Core, no deprecation)*
- [x] Implement leak-detection test: `test_should_emit_telemetry_and_warn_when_connection_leaked_at_process_exit` — real FutureWarning assertion; telemetry TODO(SNOW-2912513); `@python_e2e` tag added to scenario *(commit 4a590696)*
- [x] Remove `_telemetry_verified = conn.is_closed()` fake assertion — replaced with `pass  # TODO(SNOW-2912513)` *(commit 57ec154d)*
- [x] Fix "Each auto-cleanup close is invoked with retry false": switch to `logout_503_then_success.json`; assert total==10 (no retries) + responses_503==1 (transient error exercised) *(commit 4a590696)*
- [x] Replace `_warn_logged = True` stub — replaced with FileHandler real Core log capture *(commit aab6e673)*
- [x] Remove `_close_succeeded = conn.is_closed()` hollow variable — removed; "does not raise" proven by execution flow, `pass` satisfies validator *(commit 57ec154d)*
- [x] Fix duplicate `assert len(logout_requests) == 1` — distinct assertions: count in "Then Logout is not retried", method+URL in "And Only one logout request was sent" *(commit 57ec154d)*
- [x] Extract `get_wiremock_requests` / `filter_logout_requests` to shared helper — `WiremockClient.get_requests()` / `get_logout_requests()` added *(commit aab6e673)*
- [x] Replace `SNOW-2872349` TODO references in integ — deleted TestLogoutPhase5Optimization class entirely; feature file comment updated *(commit aab6e673)*
- [x] Add 240s worst-case comment to `test_should_call_close_with_retry_false_from_atexit_handler` *(commit aab6e673)*
- [x] Fix `assert "Snowflake Token" in auth_header` — changed to `auth_header[:16] == "Snowflake Token="` *(commit 57ec154d)*

**Post-implementation findings (2026-04-13 — self-review of aab6e673):**
- [x] "No further requests after retry limit": `> 0` (wrong direction) → `== PYTHON_DEFAULT_LOGOUT_MAX_ATTEMPTS` *(commit 57ec154d)*
- Feature file S1 passive When: `When Connection configuration is checked` → `When Connection is initialized` *(commit 57ec154d)*

<!-- Entries will be appended here as review progresses -->
