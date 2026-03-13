# #13 -- Fix "timeouts" typo

**File**: `tests/definitions/core/session/logout.feature` line 90
**Reviewer**: Copilot (both review rounds)

## Before

```gherkin
And The last attempt timeouts because remaining budget is less than server response time
```

## After

```gherkin
And The last attempt times out because remaining budget is less than server response time
```

## Rationale

Grammar: "timeouts" is a noun form; the verb is "times out".

## Human Comment

_Add human comment here._

## Comment Answer Proposition

_Add proposed reviewer reply here._
