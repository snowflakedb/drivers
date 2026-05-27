---
name: end-of-task-self-review
description: Self-review checklist run before declaring a coding task done, opening a PR, or pushing the final commit. Catches commit-scope drift, loose test assertions, refactor-stale comments, error-string changes that need callouts, test fixtures that self-sabotage, dead-code helpers, and signature smells. Use after implementing a feature, completing a bug fix, finishing a refactor, or whenever about to write a commit message or open a PR. Distilled from review findings on the OAuth-core PR series.
# Canonical source. .cursor/skills/end-of-task-self-review/SKILL.md is a pointer to this file.
# Skills fire on demand so pointer files are safe there — the agent reads this file fresh
# at invocation time. alwaysApply rules cannot use pointers for the same reason; see
# .cursor/rules/*.mdc frontmatter for the detailed explanation.
---

# End-of-Task Self-Review

Run this checklist after substantial changes, before writing the final
commit message or opening a PR. Each item is something a reviewer
catches that you can catch first.

## Checklist

```
- [ ] Commit scope matches the subject (no surprise production changes)
- [ ] Every assertion is as strict as the layer allows
- [ ] Renamed/moved code: no stale comments referencing the old location
- [ ] Changed user-facing strings (errors, logs): called out in commit body
- [ ] Test fixtures don't permanently disable what other tests need
- [ ] No dead-code helpers (added but never called)
- [ ] Public test helpers' return values are actually used
- [ ] Mock path matchers are anchored or exact (no substring footguns)
- [ ] `assert!(matches!(...))` calls include a context message
- [ ] `#[ignore]`'d tests either have assertions or aren't `#[test]`
```

## How to run each check

### 1. Commit scope matches the subject

List every changed file. Ask: "is this consistent with what the subject
claims?"

- Subject says "testing" → production files in the diff are a smell.
- Subject says "refactor" → behavioral changes need a callout.
- Subject says "fix bug X" → drive-by cleanups should split out.

If the diff genuinely spans both production and tests, the subject
should reflect that. Use a two-section commit body:

```
SUBJECT-PREFIX: <honest summary that covers both>

Production changes:
- bullet per behavioral change

Tests:
- bullet per test addition / fixture / mock
```

### 2. Assertion strictness

Default to the **strictest** assertion that captures the intent. Move
down this ladder only when the layer above forces you to:

1. `assert_eq!(err.code(), 390303)` — typed, exact.
2. `assert!(matches!(err, ErrorKind::TokenExpired { .. }), "got {err:?}")` — typed variant.
3. `assert!(err.to_string().contains("390303"), "got {err}")` — string, exact substring.
4. `assert!(patterns.iter().any(|p| s.contains(p)))` — only as last resort.

If you're at level 3-4 because of an architectural boundary (FFI, proto,
RPC) flattening the error, **say so explicitly** with a `TODO(TICKET-N)`
and write the strictest pattern you can. Never use `any(...)` over an
array containing a word like `OAuth` or `Error` — that matches almost
anything and silently defangs the test.

### 3. Refactor-stale comments

After moving or renaming a function/type, grep for:

```bash
rg 'OldFnName|OldTypeName' --type rust
```

Two places that especially go stale:

- Inline `// validated by X` / `// see Y` comments.
- Module-level docs that name internal helpers.

Comments that name the *wrong* function are worse than no comment.

### 4. User-facing string changes

If the diff touches an error `Display` impl, a `tracing::error!`
message, or any user-visible string, call it out in the commit body.
SREs grep for these strings; dashboards and runbooks remember the old
wording.

```bash
git diff -U0 main..HEAD -- '**/*.rs' | rg '^\+' | rg -i 'display|tracing::|println|panic'
```

Verify the new wording against any cross-driver / cross-language
consistency requirement.

### 5. Test fixtures don't self-sabotage

If a fixture mutates global state, sets a kill switch, or installs a
no-op stub, scan every test that uses it for cases that need the *real*
behavior. Common smells:

- `Once::call_once` setting an irreversible flag in a fixture
  constructor — any test that wants the un-flagged behavior is broken
  by construction.
- A no-op default stub installed via `cfg(test)` with no override path.

If a test exists to drive the suppressed path, the fixture needs an
opt-out (`with_X_interactive(...)`), or the test belongs in a different
binary that doesn't build with the suppressed cfg.

### 6. No dead-code helpers

```bash
cargo clippy --tests -- -W dead_code 2>&1 | rg 'never used'
```

`pub fn helper(...)` in a test module that no test calls is dead weight.
Either wire it in or delete it. Write the test first, mount the fixture,
see it fire, **then** commit the helper.

### 7. Return values that callers discard

If a helper returns `&Self` "for chaining" but every caller discards it,
drop the return type. Same for `Result<...>` that callers always
`.unwrap()` at the same layer — return the inner type.

```bash
rg '\) -> &Self \{' --type rust  # find the candidates
```

### 8. Mock path matchers

Substring matchers in test infrastructure are footguns. Anchor with
`^…$` or use exact-match.

```rust
// ❌ Matches /oauth/token-request-evil too
.and(path_regex(r"/oauth/token-request"))

// ✅
.and(path_regex(r"^/oauth/token-request$"))
// or
.and(path("/oauth/token-request"))
```

### 9. `assert!(matches!(...))` without context

Failure prints "assertion failed: matches!(...)" — useless. Always pair
with a context message:

```rust
assert!(
    matches!(err, OAuthError::StateMismatch { .. }),
    "expected StateMismatch, got {err:?}"
);
```

### 10. `#[ignore]`'d tests need assertions or a different home

A `#[test]` with `#[ignore]` and no assertions is almost always wrong:

- Real test that should run → de-ignore it, or move to a feature-gated
  bundle that does run.
- Documentation artifact → move to a doc comment or `examples/`.
- Manual reproducer → fixture must support manual driving (print the
  URL, don't suppress the launcher in *this* fixture, etc.).

## Output

If any item fails, fix it before declaring done. If a fix needs to be
deferred (e.g. assertion strictness blocked by an FFI boundary that's
out of scope), add a `TODO(TICKET-N)` with a real ticket reference and
note it in the PR description so reviewers don't catch it cold.

For *design-time* discipline (test seams, parsing duplication, error
structure across layers, visibility, manual `Debug`/`Clone` impls,
documented contracts), see the
`.claude/rules/code-review-design-discipline.md` rule (auto-loaded via
`CLAUDE.md`).
