# #11 -- Clarify mock returns 500 on ALL attempts

**File**: `tests/definitions/python/session/logout.feature` lines 95-102
**Reviewer**: boler
**Status**: fpawlowski said "added" but not in current code

## Before

```gherkin
Scenario: should use best-effort error handling strategy by default
  Given Snowflake Python client is created with default parameters
  And Server will return 500 Internal Server Error on logout
  When Connection is closed
  Then Error is logged as WARN
  And close() method does not raise exception
  And Connection cleanup succeeds
  And Error handling strategy is best-effort by default
```

## After

```gherkin
  Scenario: should use best-effort error handling strategy by default
    Given Snowflake Python client is created with default parameters
    And Server will return 500 Internal Server Error on all logout attempts
    When Connection is closed
    Then Error is logged as WARN
    And close() method does not raise exception
    And Connection cleanup succeeds
    And Error handling strategy is best-effort by default
```

## Rationale

Without "all attempts", the mock could return 500 on the first attempt, then 200 on retry, and close would succeed -- hiding the fact that best-effort is supposed to swallow the error. The scenario's intent is to verify error suppression, so the server must fail consistently.

## Human Comment

_Add human comment here._

## Comment Answer Proposition

_Add proposed reviewer reply here._
