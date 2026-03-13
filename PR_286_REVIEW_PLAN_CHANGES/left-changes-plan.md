# Left Changes Implementation Plan

This plan is the single source of truth for the remaining logout-gherkin work after PR comment changes `#10` through `#15`.

It is written as a handoff for another AI to implement the remaining Gherkin changes so the result is:

- complete rather than "good enough"
- concrete rather than vague
- architecturally correct by layer
- testable without lying about what implementations verify
- aligned with the standards in `.cursor/rules/gherkin-expert.md`
- reviewed for implementation honesty using `.cursor/rules/gherkin-implementation-reviewer.md`

This plan **supersedes**:

- `PR_286_REVIEW_PLAN_CHANGES/self-noticed/20-verify-5s-default.md`
- `PR_286_REVIEW_PLAN_CHANGES/self-noticed/21-refresh-flow-time-budget.md`

Their concerns are intentionally covered here as part of one coherent implementation strategy.

---

## Non-Negotiable Standards

The implementing AI must follow these rules while editing gherkins:

1. Do not approve or keep a scenario just because it is "close enough".
2. Do not use vague steps like "works", "succeeds appropriately", or "depends on configuration".
3. Do not keep timing assertions that rely on narrow wall-clock windows unless there is a strong, explicit reason and the mechanism being tested is proven.
4. Do not write scenarios that imply a timeout mechanism unless the scenario also makes clear **which timeout semantics** it is supposed to test.
5. Do not produce Gherkins that a later implementation could satisfy for the wrong reason.
6. Do not add coverage in the wrong layer. Mock-heavy protocol behavior belongs in Core, not Shared.
7. Every retained or added scenario must be implementable as an honest test, not a "lying test".

---

## First Principle: Clarify Timeout Semantics Before Editing

Before modifying the remaining timeout-related gherkins, the implementing AI must decide what timeout semantics each scenario is meant to specify.

At minimum, distinguish between these concepts:

1. **Request-level timeout**
   Meaning: an individual HTTP request is interrupted because the request itself exceeded a timeout.

2. **Total retry budget timeout**
   Meaning: the overall logout operation has a total elapsed-time budget across attempts / sleeps / subflows.

3. **Refresh-flow budget accounting**
   Meaning: token refresh consumes part of the same total logout budget and must not reset it.

4. **Wrapper default vs core default**
   Meaning: is the 5-second default a Core default, or a value passed into Core by wrappers like Python?

The implementing AI must **not** assume these are the same thing.

### Why this matters

The currently visible retry helper in `sf_core/src/http/retry.rs` checks elapsed budget before an attempt starts and before backoff sleep, but not while `send().await` is blocked. That means a Gherkin about "server holds connection open" may be testing a lower-level request timeout rather than the retry budget itself.

Because of that, a scenario must not claim to prove total-budget semantics if it is really only meaningful for a request-level timeout, and vice versa.

---

## Remaining Work Summary

The remaining Gherkin work should be implemented as a **small set of focused gherkins**, each with its own matrix where needed.

Do **not** collapse everything into one mega-matrix.

Each Gherkin below is described as a tuple:

`(gherkin_name, what_it_should_prove, rationale_for_way_of_testing)`

### Tuple 1

**Gherkin name**: `configured_timeout_boundary_behavior`

**What it should prove**:
- A logout operation succeeds when the response arrives before the configured timeout boundary.
- A logout operation fails when the response exceeds the configured timeout boundary.
- Different configured timeout values actually influence behavior rather than being ignored or replaced by a hidden constant.

**Rationale for way of testing**:
- This replaces weak wall-clock assertions like `Request completes within <timeout_seconds> seconds`.
- It tests the real decision boundary: before-timeout vs after-timeout.
- It should use multiple concrete timeout values so a hidden hardcoded timeout cannot satisfy all rows.
- The matrix should stay cheap; prefer short delays that still separate pass/fail cases clearly.

### Tuple 2

**Gherkin name**: `timeout_is_not_split_per_attempt`

**What it should prove**:
- The configured timeout is not naively divided across retry attempts.
- A response that fits within the total intended timeout still succeeds even when retry count is high enough that a split-budget implementation would fail early.

**Rationale for way of testing**:
- This directly targets the bug class called out in review discussion: `total_timeout / attempts`-style logic.
- It should be a dedicated scenario or very small matrix, not hidden inside a large success matrix.
- The row values should be chosen so a split-budget algorithm obviously fails while the intended total-budget algorithm succeeds.

### Tuple 3

**Gherkin name**: `retry_budget_respected_independently_of_timeout`

**What it should prove**:
- Max attempts / retry budget are honored independently from timeout behavior.
- Exhausting attempts causes the expected strict vs best-effort outcomes.
- Timeout-related edits do not weaken or accidentally re-spec retry-count behavior.

**Rationale for way of testing**:
- Timeout and retry count are separate axes.
- Keeping this explicit prevents later implementations from passing timeout scenarios while silently violating retry-count semantics.
- Existing retry-config scenarios may already cover most of this; the implementing AI should minimize change if the current gherkins already honestly specify it.

### Tuple 4

**Gherkin name**: `refresh_consumes_total_logout_budget`

**What it should prove**:
- Time spent in token refresh counts against the same total logout budget.
- Remaining budget after refresh constrains the retried logout.

**Rationale for way of testing**:
- This is the core concern behind the current self-noticed `#21` issue.
- The current refresh-budget scenario is too weak if the retried logout succeeds immediately, because a buggy implementation may still pass.
- The rewritten shape must force the remaining-budget behavior to become observable.

### Tuple 5

**Gherkin name**: `refresh_does_not_reset_attempts_or_budget`

**What it should prove**:
- Receiving `SESSION_TOKEN_EXPIRED 390112` and entering refresh flow does **not** reset elapsed budget.
- Refresh flow does **not** restart the full logout retry sequence from scratch.
- No extra logout attempts occur after budget exhaustion.

**Rationale for way of testing**:
- This is the most bug-specific, high-value timeout scenario still missing or too weak today.
- It should be separate from the generic timeout matrix because otherwise causality becomes ambiguous.
- The ideal shape is: first logout returns `390112`, refresh is delayed, retried logout is also delayed enough that only a buggy budget-reset implementation could succeed or keep retrying.

### Tuple 6

**Gherkin name**: `strategy_changes_outcome_not_timeout_math`

**What it should prove**:
- Strict vs best-effort changes the surfaced outcome (`throw` vs `log and succeed`) without changing the underlying timeout accounting expectations.
- Timeout-math scenarios do not accidentally become strategy-specific unless they really need to be.

**Rationale for way of testing**:
- This keeps strategy semantics separate from timing semantics.
- Otherwise it becomes too easy for a scenario to fail because of strategy outcome handling rather than because timeout accounting is wrong.

---

## Minimal-Change Strategy Against Current Gherkins

The implementing AI should compare the tuple set above against the existing `tests/definitions/core/session/logout.feature` and choose the **minimal** change set that achieves equivalent coverage honestly.

That means:

1. **Keep** existing scenarios that already specify the correct invariant clearly.
2. **Rewrite** scenarios that currently rely on wall-clock assertions or under-specified timeout wording.
3. **Add** only the scenarios needed to make refresh-budget-reset and timeout-splitting bugs directly observable.
4. **Do not** duplicate existing coverage just because a tuple exists in this plan.

### Likely current areas to revisit

The implementing AI should review these areas first:

- `should timeout after 5 seconds by default when server does not respond`
- `should include token refresh time in total logout timeout budget`
- `should honor provided timeout config and succeed for each <strategy_type>`
- timeout failure-path outlines for strict / best-effort

The implementing AI must explicitly decide whether each of those scenarios should be:

- kept as-is
- rewritten
- split into multiple scenarios
- replaced by a more diagnostic scenario

---

## Specific Guidance for the Former `#20` Concern

The old `#20` concern must now be addressed within this plan rather than as a separate note.

### Required decision

Before finalizing timeout wording, determine whether the "5 second default" is:

- a real Core default
- a wrapper-provided value passed to Core
- only a design-doc intent that current code does not yet clearly implement

### Editing rule

Until that is clarified, do **not** strengthen any Gherkin that asserts a Core default of 5 seconds as if it were already proven by the code.

### Acceptable outcomes

If it is a wrapper-provided value only:
- rewrite the relevant Core scenario so it talks about a **configured timeout** rather than a Core default.

If it is a real Core default:
- split config assertion from behavior assertion where useful
- still avoid narrow wall-clock timing bands

If it remains uncertain after code review:
- prefer wording that is honest about configured behavior rather than default-source claims

---

## Specific Guidance for the Former `#21` Concern

The old `#21` concern must also be handled inside this plan.

### Required invariant

At least one scenario must make this bug observable:

1. logout gets `SESSION_TOKEN_EXPIRED 390112`
2. refresh consumes significant time
3. retried logout still has only the **remaining** time budget
4. a buggy implementation that resets budget or restarts retries would fail the scenario for an observable reason

### Good shape

A strong version looks like:

- first logout attempt returns `390112`
- refresh succeeds after delay
- retried logout response is delayed long enough that it can only succeed if the budget was wrongly reset
- expected outcome makes budget exhaustion and lack of extra attempts observable

### Weak shapes to avoid

- retried logout succeeds immediately after refresh
- only checking a loose `wall-clock <= X seconds` bound
- relying on the final timeout error alone without making remaining-budget behavior observable

---

## Best-Practice Review Checklist

The implementing AI must review its final gherkin changes against the standards implied by both review rules.

### From `gherkin-expert`

The final gherkins must be:

- **complete**: all important timeout/retry/refresh concerns covered
- **specific**: concrete values, no vague timing language unless intentionally high-level
- **validated**: ready for validator/test implementation work
- **architecturally correct**: Core holds the dense protocol / timeout logic
- **backward-compatible or explicitly BCR**: if behavior differs from old drivers, the gherkin/comments must say so

### From `gherkin-implementation-reviewer`

The final gherkins must not set up future "lying tests". In particular, avoid:

- claiming config without a realistic way to set it in tests
- claiming timeout semantics without knowing which mechanism is under test
- matrices whose rows can all pass for the same hidden constant
- steps that verify internal implementation details instead of observable behavior
- outlines whose example rows cannot realistically be implemented as distinct configurations

---

## Deliverables for the Implementing AI

The implementing AI should produce all of the following:

1. **Updated Gherkin file changes**
   The minimal set of edits required in `tests/definitions/core/session/logout.feature` and any directly affected files.

2. **Coverage mapping**
   A short checklist that maps each tuple in this plan to:
   - existing scenario kept
   - rewritten scenario
   - new scenario added

3. **Rationale comments in Gherkins where useful**
   Add short comments only where they help explain why a matrix or scenario exists, especially for:
   - split-budget detection
   - refresh-budget accounting
   - wrapper-default vs core-default ambiguity

4. **Review note**
   A short summary explaining why the final shape avoids flaky wall-clock assertions while still detecting the important timeout bugs.

---

## Success Criteria

This plan is successfully implemented only if the resulting Gherkins satisfy all of these:

1. No remaining timeout scenario depends on a narrow wall-clock band like `5s..6s` to prove correctness.
2. The final scenarios make timeout bugs observable by **behavioral boundaries**, not probability.
3. Refresh-flow budget accounting is tested explicitly, not assumed to be covered by generic timeout cases.
4. The source of the 5-second timeout claim is handled honestly.
5. The scenario set is compact and layered correctly, not bloated.
6. Another AI could implement honest tests from these gherkins without inventing semantics that the Gherkins failed to specify.

---

## Final Instruction to the Implementing AI

If, after review, the remaining timeout-related Gherkins are still missing one of the tuple invariants above, mark the work as:

`INCOMPLETE - MISSING SCENARIOS`

Do not call the suite "good enough" until every important timeout / retry / refresh invariant is either:

- clearly covered by an honest existing scenario, or
- explicitly added / rewritten.
