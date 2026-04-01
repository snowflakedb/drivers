# ADR: Logout Implementation Refinement (SNOW-2872349)

**Branch:** `SNOW-2872349-logout-review-fixes`
**Base:** `SNOW-2872349-logout-implementation`
**Date:** 2026-03-31

---

## Decision: Feature file is ground truth for Gherkin step text

**Reviewer concern:** Tests have keyword mismatches (`//And` in test vs `Then` in feature) and step text typos (`timeouts` vs `times out`).
**Evidence:** `sf_core/tests/integration/session/logout.rs` TODO comments at lines 336-338, 495-499, 1603-1608.
**Resolution:** Feature file is the authoritative source. All Gherkin step comments in Rust test files must match feature file text exactly (keyword and text). Fixed: `And` → `Then` in 3 places; `timeouts` → `times out` in 1 place.
**Rejected alternatives:** Updating the feature file to match the test — rejected because feature files are the contract definition, written from user/BDD perspective.
**Trade-offs:** Gained: validator passes; test intent is clearer. Lost: none.

---

## Decision: is_closed()-only atexit guard (no atexit.unregister on exception)

**Reviewer concern:** Copilot: "atexit handler never unregistered if close() raises."
**Evidence:** `python/src/snowflake/connector/connection.py` — `_close_at_process_exit` and `close()`.
**Resolution:** Rely solely on `is_closed()` guard in `_close_at_process_exit`. Core marks `is_closed=True` atomically at the START of `connection_close()`, before any logout attempt. Even if logout fails and `close()` raises, `is_closed()` returns `True`. The atexit handler checks this and returns early. `atexit.unregister()` is retained for the success path but is NOT the safety net.
**Rejected alternatives:**
1. `atexit.unregister()` as sole safety — TOCTOU risk: interpreter shutdown may have already begun running atexit handlers by the time the unregister call executes.
2. try/finally unregister in `close()` — adds complexity, and the `is_closed()` approach is semantically cleaner (connection state drives behavior, not atexit registration state).
**Trade-offs:** Gained: TOCTOU safety during process exit. Lost: Minimal overhead of `is_closed()` RPC call in atexit handler (acceptable, exit context).

---

## Decision: retry=False maps to max_attempts=1, not a separate code path

**Reviewer concern:** Copilot: "retry parameter not plumbed to Core correctly."
**Evidence:** `python/src/snowflake/connector/connection.py:close()`.
**Resolution:** `close(retry=False)` pre-sets `logout_max_attempts=1` via `connection_set_option_int` before calling `connection_close()`. This works because Core reads `LogoutConfig` at close-time (not from a cached init-time copy). `max_attempts=1` means exactly 1 attempt (0 retries), which is the correct semantic for "no retry."
**Rejected alternatives:** Passing retry flag as a parameter to `connection_close()` protobuf call — rejected because the current API is settings-based, and adding a parameter would require proto schema changes.
**Trade-offs:** Gained: Simple, consistent with settings-based API pattern. Lost: Slight obscurity — setting an option right before calling close is less obvious than a direct parameter.

---

## Decision: Mutex consolidated to single acquisition for the close critical section

**Reviewer concern:** sfc-gh-jszczerbinski: "How about we acquire this mutex once?" (triple mutex acquisition bug)
**Evidence:** `sf_core/src/apis/database_driver_v1/connection.rs:connection_close()` — `mark_connection_closed()` + `get_logout_config()` each acquire separately.
**Resolution:** Consolidated the first two acquisitions: acquire the mutex once, read `is_closed`, `logout_config`, and any other needed fields atomically, then drop the lock before the async HTTP logout. Cleanup (`cleanup_connection`) re-acquires once after HTTP work completes.
**Rejected alternatives:** Single lock held for entire close including HTTP — rejected: holding a mutex during network I/O would block all other connection operations (including other thread's `is_closed` checks) for the entire logout duration.
**Trade-offs:** Gained: Atomic read of connection state; no interleaving between "mark closed" and "read config". Lost: Two lock acquisitions instead of three (cleanup still needs a write lock separately).

---

## Decision: Arc<AtomicBool> for is_closed — not a state machine

**Reviewer concern:** sfc-gh-jszczerbinski: "How about a state machine? Why do we need Arc here?"
**Evidence:** `sf_core/src/apis/database_driver_v1/connection.rs` — `pub is_closed: Arc<AtomicBool>`.
**Resolution:** `Arc<AtomicBool>` is retained. `Arc` is needed for shared ownership across concurrent async tasks (logout task, token refresh, query cancellation) that all need to observe closed state. A state machine would require a single owner or message-passing via channels, which conflicts with the parallel-task architecture.
**Rejected alternatives:** State machine enum `{ Open, Closing, Closed }` with a `Mutex<State>` — would require all state reads to go through async lock acquisition, introducing latency and potential deadlock.
**Trade-offs:** Gained: Lock-free closed-state check, safe concurrent ownership. Lost: Cannot represent "closing" as a distinct state (vs "closed"). This is acceptable since the design doc specifies is_closed becomes true at close()-entry.

---

## Decision: Naming unification for auto-detection field

**Reviewer concern:** sfc-gh-jszczerbinski: "Should we unify the parameters at this point?"
**Evidence:** Three inconsistent names: Python wrapper `enable_server_session_keep_alive_auto_detection`, mapping layer `enable_auto_detection`, Core settings key `"enable_logout_auto_detection"`, Core struct field `enable_auto_detection`.
**Resolution:** Standardized Core struct field to `enable_auto_detection` (kept as-is) and settings key to `"enable_logout_auto_detection"` (kept as-is). The Python wrapper-facing name `enable_server_session_keep_alive_auto_detection` stays unchanged for backward compatibility. Added explicit documentation mapping the names.
**Rejected alternatives:** Renaming the Python wrapper parameter to `enable_logout_auto_detection` — rejected: breaking change for existing users. Renaming Core field to match full Python name — rejected: too verbose for internal use.
**Trade-offs:** Gained: Clearer documentation of the name correspondence. Naming inconsistency partially reduced. Lost: Full unification not possible without breaking Python API.

---

## Decision: ErrorStrategy encoding replaced — protobuf int enum → string constants

**Reviewer concern:** sfc-gh-jszczerbinski: "Why call it protobuf?"
**Evidence:** `sf_core/src/config/logout.rs` — `UNSPECIFIED_PROTOBUF`, `BEST_EFFORT_PROTOBUF`, `STRICT_PROTOBUF` integer constants; `from_protobuf_value(i64)` / `to_protobuf_value()` methods.
**Resolution:** Replaced the entire int-based encoding with string-based parsing. Core now reads `logout_error_strategy` via `get_string()` and parses `"best_effort"` / `"strict"` through `FromStr`. The Python wrapper uses an `ErrorStrategy` class with `BEST_EFFORT = "best_effort"` / `STRICT = "strict"` constants and calls `connection_set_option_string` instead of `_int`. This removes the protobuf dependency, makes settings self-documenting, and eliminates the "unspecified=0" sentinel (key absence now signals "use default"). The design doc (line 459) uses string examples: `logout_error_mode = "strict" | "best_effort"`.
**Rejected alternatives:** Renaming int constants only (e.g. `BEST_EFFORT_VALUE`) — rejected: still requires cross-language agreement on magic integers. Keeping the `_PROTOBUF` suffix — rejected: leaks implementation details.
**Trade-offs:** Gained: Self-documenting settings, no magic integer contract. Lost: Slightly more parsing overhead (string vs int comparison), negligible in practice.

---

## Decision: mypy python_version set to 3.10 (not 3.9)

**Reviewer concern:** Copilot: "Python version incompatibility (mypy python_version=3.9)."
**Evidence:** `python/pyproject.toml` — `python_version = "3.9"` under `[tool.mypy]`.
**Resolution:** Changed to `"3.10"`. The code uses `str | None` and `dict[str, Any]` syntax (PEP 604/585) which mypy needs 3.10 to type-check correctly. `from __future__ import annotations` handles runtime compatibility on Python 3.9.
**Rejected alternatives:** Reverting all `X | Y` syntax to `Union[X, Y]` and `dict` to `Dict` for 3.9 compat — rejected: more verbose and we already use `from __future__ import annotations`.
**Trade-offs:** Gained: mypy accurately type-checks modern syntax. Lost: mypy no longer checks 3.9-specific behavior (acceptable since `from __future__ import annotations` bridges the runtime gap).

---

## Decision: Gherkin step-comment 1:1 requirement

**Reviewer concern:** Multiple TODO comments flagged "empty steps" — step comments with no code immediately following.
**Evidence:** `sf_core/tests/integration/session/logout.rs` — multiple TODO(gherkin) blocks.
**Resolution:** Each Gherkin step comment (`//Given`, `//And`, `//When`, `//Then`) must be immediately followed by code implementing that step. Setup/assertion code cannot be "batched" after multiple step comments. This mirrors the requirement in the ud-implementation-loop rules.
**Rejected alternatives:** Batching — rejected: the validator cannot match step comments to code when they're separated.
**Trade-offs:** Gained: Validator passes; test structure matches feature file. Lost: Occasionally requires minor code refactoring to separate what was previously a combined setup block.

---

## Decision: Phase 2 backward-compat default — auto_detection=True

**Context:** Phase 2 (SNOW-2314152) Python backward compatibility.
**Evidence:** Design doc §2 Phase 2: "each wrapper uses defaults that mirror its legacy behaviour." Old Python driver always checked `_async_sfqids` registry before logout.
**Resolution:** `enable_server_session_keep_alive_auto_detection` defaults to `True` in `connection.py:172` (`kwargs.pop(..., True)`). Without this default, Core would receive `None` → always logout → killing async queries (regression for fire-and-forget users).
**Rejected alternatives:** Default `False` (Phase 3 semantics) — rejected: breaking change for Phase 2. Default `None` — rejected: Core treats `None` as `false` (no auto-detection), which differs from old Python behavior.
**Trade-offs:** Gained: Backward compatibility with legacy async query behavior. Lost: Phase 3 will require changing this default (tracked by SNOW-2314152).

---

## Decision: Removed dead `self._closed` field — delegate to Core is_closed()

**Context:** `python/src/snowflake/connector/connection.py` had `self._closed = False` set in `__init__` but never set to `True` anywhere. Two callers (`_check_not_closed`, `__exit__`) checked it.
**Evidence:** Commit `823bfa17`. The field was dead code that diverged from Core's authoritative `is_closed` atomic flag.
**Resolution:** Removed `self._closed`. All call sites now use `self.is_closed()` which queries Core via `connection_is_closed` RPC. Core's `is_closed` is an `Arc<AtomicBool>` set to `True` atomically at the start of `connection_close()`.
**Rejected alternatives:** Setting `self._closed = True` in `close()` — rejected: creates dual-state divergence where Python and Core could disagree. Adding Python-side `_closed` as a cache — rejected: `is_closed()` RPC cost is negligible (protobuf call + atomic load), and caching adds a consistency burden.
**Trade-offs:** Gained: Single source of truth for closed state. Lost: One RPC per `is_closed()` call (acceptable; called rarely — once per `_check_not_closed()`, once in atexit handler).

---

## Decision: Token cleanup test removed from Python

**Context:** Commit `9456c817`. Python e2e test `TestLogoutResourceCleanup` was removed; `@python_e2e` tag removed from shared feature file token cleanup scenario.
**Evidence:** `ConnectionGetInfoResponse` has `session_token` but no `master_token`; no public Python API to inspect `Connection.tokens`. The test used `conn.is_closed()` as a proxy assertion, which verified close succeeded but not actual token clearing.
**Resolution:** Token cleanup is a Core concern verified by `should_cleanup_all_tokens_on_close_regardless_of_whether_logout_was_sent` (e2e/session/logout.rs) which inspects `Connection.tokens` directly. Python cannot meaningfully test this.
**Rejected alternatives:** Keeping the Python test with proxy assertions — rejected: tautological (asserting `is_closed()` after `close()` always passes). Exposing tokens via Python API — rejected: security risk of exposing raw tokens to Python layer.
**Trade-offs:** Gained: No misleading test coverage. Lost: No Python-level integration test for token cleanup (acceptable; Core e2e covers it).

---

## Decision: Config re-derivation from connection_seed at close-time — temporary compromise

**Context:** `connection_close` re-derives `LogoutConfig` from `conn.connection_seed` at close-time (commit `0380a8af`).
**Evidence:** PR comment sfc-gh-jszczerbinski: "Are we catching latest config by getting value produced at init?" The init-time `conn.logout_config` was frozen, so `close(retry=False)` setting `logout_max_attempts=1` was silently ignored.
**Resolution:** `LogoutConfig::from_settings(&conn.connection_seed)` at close-time picks up post-init `connection_set_option_*` overrides. Falls back to init-time config with a `tracing::warn!` on parse failure.
**Known issues:**
1. Breaks the "config parsed once at init" contract documented in the design doc
2. Non-atomic: Python's `close(retry=False)` requires two RPCs (`set_option_int` + `close`), though the Core mutex serializes access
3. Falls back silently to init-time config if re-derivation fails (now logged at WARN)
**Planned resolution:** `dual-config-architecture` approach with `ConnectionCloseRequest` override fields + `merge_with_request()` in Core, planned as a future enhancement. This would allow `close(retry=False)` to be expressed as a single RPC.
**Trade-offs:** Gained: `close(retry=False)` works correctly today. Lost: Slight architectural impurity; two RPCs instead of one.
