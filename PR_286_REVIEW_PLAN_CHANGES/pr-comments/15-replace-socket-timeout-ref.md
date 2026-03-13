# #15 -- Remove socket-timeout wording from close-vs-query scenario

**File**: `tests/definitions/core/session/logout.feature` line 121
**Reviewer**: boler
**Status**: resolved in scope of `#14`

## Resolution

This specific change is now tracked as part of the broader socket-timeout cleanup in `PR_286_REVIEW_PLAN_CHANGES/pr-comments/14-remove-socket-timeout-scenario.md`.

The concrete action is:

- remove `And Socket timeout is set to 10 seconds` from `Scenario: should fail in-flight query when server response arrives after closing process started`

## Rationale

The only unique decision here is **remove, do not rename**.

Why:

1. This scenario is about **close-vs-in-flight-query behavior**, not timeout semantics.
2. Renaming to `Request timeout` would introduce a broader timeout concept that is **not clearly shown** as a first-class setting for this scenario in current core code.
3. The 3-second delayed response already serves its purpose: it keeps the query in flight long enough for `close()` to begin before the server responds.

If we later need an explicit timeout precondition here, it should be added only once the underlying timeout mechanism and terminology are nailed down elsewhere. Until then, `#14` is the actionable source of truth for this edit.

## Human Comment

_Add human comment here._

## Comment Answer Proposition

This should now be handled in scope of `#14`. I thought I had already applied this locally as part of the socket-timeout cleanup, but it looks like the AI stashed those edits before pushing. My bad. I re-applied it and kept `#14` as the actionable source of truth.
