# #10 -- Replace ConnectionClosedException with generic wording

**File**: `tests/definitions/shared/session/logout.feature` lines 38-44
**Reviewer**: boler
**Status**: fpawlowski said "fixed" but not in current code

## Before

```gherkin
Scenario: should reject queries client-side after connection is closed
  Given Snowflake client is logged in
  And Simple query SELECT 1 executes successfully
  When Connection is closed
  And Query is attempted on closed connection
  Then Query throws ConnectionClosedException
  And Error message contains "Connection is closed"
```

## After

```gherkin
Scenario: should reject queries client-side after connection is closed
  Given Snowflake client is logged in
  And Simple query SELECT 1 executes successfully
  When Connection is closed
  And Query is attempted on closed connection
  Then Query fails with a connection-closed error
```

## Rationale

`ConnectionClosedException` is a Java-specific type name. The shared feature applies to all drivers (Python raises a different exception, ODBC returns an error code). The assertion should describe the observable behavior, not a language-specific type. The separate `Error message contains` line is also removed since the error type itself conveys the meaning.

## Human Comment

_Add human comment here._

## Comment Answer Proposition

_Add proposed reviewer reply here._
