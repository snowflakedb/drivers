# #17 -- Rework timeout-vs-delay scenarios after timeout path is verified

**File**: `tests/definitions/core/session/logout.feature` lines 278-356
**Reviewer**: boler
**Status**: Assess after `../self-noticed/20-verify-5s-default.md`

## boler's concern

> "What's the point of delaying the request if the delay is always shorter than the timeout?"

That concern is valid. The current rows read like trivially passing success cases, and the wall-clock style assertion (`Request completes within <timeout_seconds> seconds`) is the weakest part.

## What is definitely wrong today

### Current success path

```gherkin
Scenario Outline: should honor provided timeout config and succeed for each <strategy_type>
  Given Core logout function called with <strategy_type> strategy
  And Timeout configured to <timeout_seconds> seconds
  And Mock HTTP server delays response by <delay_seconds> seconds then returns 200
  When Logout is executed
  Then Request completes within <timeout_seconds> seconds
  And Close succeeds
```

Problems:

1. `Request completes within <timeout_seconds> seconds` is another wall-clock assertion and should not survive.
2. The examples do a poor job of proving the configured timeout is the one actually in effect.
3. The current rows do not tell us whether timeout is treated as a total budget, per-attempt budget, or an unrelated constant.

## Recommended direction

If `#20` confirms that logout really has a configurable timeout path in core, then the timeout scenarios should be rewritten to test **behavioral boundaries**, not elapsed time:

- success rows where the response arrives comfortably inside the configured timeout
- failure rows where the response exceeds the configured timeout
- multiple timeout values so a hardcoded constant is unlikely to satisfy all rows
- optionally at least one row that combines timeout + retries if we specifically want to detect an incorrect per-attempt split

## Important constraint

Do **not** overfit the markdown to a specific long-running matrix until we know the real implementation shape.

In particular, very long illustrative rows like `timeout=30s, delay=25s` may be acceptable in design prose, but they would be expensive if they ever become real E2E/integration coverage. The eventual test shape should stay cheap unless it is backed by a controlled mock that avoids real waiting.

## Suggested rewrite shape

### Success path

```gherkin
Scenario Outline: should succeed when response arrives before configured timeout for each <strategy_type>
  Given Core logout function called with <strategy_type> strategy
  And Timeout configured to <timeout_seconds> seconds
  And Mock HTTP server delays response by <delay_seconds> seconds then returns 200
  When Logout is executed
  Then Close succeeds
```

### Failure paths

```gherkin
Scenario Outline: should throw on timeout with strict strategy
  Given Core logout function called with strict strategy
  And Timeout configured to <timeout_seconds> seconds
  And Mock HTTP server delays response by <delay_seconds> seconds
  When Logout is executed
  Then Close throws timeout error

Scenario Outline: should log WARN and succeed on timeout with best-effort strategy
  Given Core logout function called with best-effort strategy
  And Timeout configured to <timeout_seconds> seconds
  And Mock HTTP server delays response by <delay_seconds> seconds
  When Logout is executed
  Then Timeout is logged as WARN
  And Close succeeds
```

## Optional stronger follow-up

If the actual implementation allows timeout + retry interaction to be configured for logout, add one dedicated scenario that proves the timeout is a **total budget** rather than a per-attempt split. That is a valuable idea, but it should be expressed as a separate explicit goal, not smuggled into every success-row example.

## Scope boundary: matrix vs refresh-flow accounting

A broader timeout/delay matrix is still worth doing because it increases confidence that:

- configured timeout values actually flow through
- success/failure boundaries are tied to the configured timeout rather than a hidden constant
- retries are not silently changing the timeout semantics

But that matrix still does **not** fully cover the specific bug class:

- logout gets `SESSION_TOKEN_EXPIRED`
- token refresh consumes part of the total budget
- retry resumes with too much budget left, or the budget is reset entirely

That is a **separate integration path** from the plain timeout/delay matrix and should be tested explicitly. See `../self-noticed/21-refresh-flow-time-budget.md`.

## Rationale

The valuable part of boler's comment is not just "delay shorter than timeout is weak". It is that these scenarios should demonstrate a real decision boundary. They should prove that the configured timeout matters, while staying independent of narrow wall-clock timing and without assuming timeout semantics that have not yet been verified in core.

## Human Comment

_Add human comment here._

## Comment Answer Proposition

I thought part of this timeout / wall-clock cleanup had already been applied locally, but it looks like the AI stashed those edits before pushing. My bad. I re-applied the update and rewrote the plan so the timeout scenarios are tracked as behavioral boundary tests rather than narrow timing checks.
