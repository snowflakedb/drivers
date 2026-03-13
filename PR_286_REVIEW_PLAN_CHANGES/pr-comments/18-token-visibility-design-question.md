# #18 -- Design question: should wrapper tests inspect tokens?

**File**: `tests/definitions/shared/session/logout.feature` lines 11-17
**Reviewer**: boler
**Status**: Needs design decision

## Current

```gherkin
Scenario Outline: should cleanup all tokens on close regardless of whether logout was sent
  # Tests that tokens are cleared regardless of logout decision
  Given Snowflake client is logged in
  And <server_session_keep_alive> is set to any value
  When Connection is closed
  Then Session token in Connection.tokens is null
  And Master token in Connection.tokens is null

  Examples:
    | server_session_keep_alive |
    | False                     |
    | True                      |
    | None                      |
```

## boler's concern

> "Why does the wrapper need to see those tokens at all? I assumed they would stay within the core and wouldn't be exposed via ConnectionInfo."

fpawlowski initially suggested tokens might be inspectable via `ConnectionGetInfoResponse` / `connection_get_info`, but boler's pushback is really about API design: wrappers may not be the right layer to observe raw token state at all.

## Repo-backed validation

The current repo does expose a `ConnectionGetInfo` RPC surface:

- `protobuf/database_driver_v1.proto` defines `ConnectionGetInfoRequest` / `ConnectionGetInfoResponse`
- generated wrapper bindings exist for `connection_get_info`

However, current proto inspection did **not** reveal an obvious token-specific `InfoCode` or a field on `ConnectionGetInfoResponse` that directly exposes session/master token state to wrappers.

So the important conclusion is:

- `ConnectionGetInfo` exists as an API surface
- but the specific shared assertion `Session token in Connection.tokens is null` is **not obviously implementable through existing wrapper-visible protocol**
- which means this is still a product/API decision, not just a missing test implementation

## Additional product/API context

This is not only a protocol-shape question. There is also compatibility pressure from past and current usage:

- some drivers exposed session token access publicly in the past
- Python, for example, exposed token access via `connection.rest.token`
- some internal libraries use drivers primarily to manage token lifecycle, e.g. SnowAPIs

So we should not assume tokens are purely core-internal unless we explicitly decide that historical wrapper-visible token access is no longer part of the intended contract.

## Options

**Option A: Move this to a core-only test**

Move the scenario out of `shared/` and into `core/`, where token cleanup can be asserted directly without exposing token internals through wrapper-facing APIs.

**Option B: Replace with a truly wrapper-observable behavior**

Do not talk about token memory/state. Instead, assert something a wrapper can actually observe after close, for example that subsequent operations requiring an active session fail immediately or that the closed connection cannot be reused.

Note: this option risks duplicating existing "connection is closed" coverage rather than specifically proving token cleanup.

**Option C: Expose only derived state, not raw tokens**

If product/API requirements truly need wrapper-visible session state, expose a derived boolean such as `has_active_session` on `ConnectionGetInfoResponse` rather than raw token/null details.

## Recommendation

This needs a team decision.

- **Option A** is the simplest and least invasive only if we explicitly decide that token visibility is no longer part of wrapper/API compatibility expectations.
- **Option C** is the cleanest wrapper-facing design if consumers only need lifecycle state rather than the raw token values.
- **Option B** is possible, but weaker: it is hard to make it specific to token cleanup without collapsing into generic closed-connection behavior that is already covered elsewhere.

Given the historical public token access and internal token-lifecycle consumers, I would **not** default blindly to Option A. First confirm whether wrapper-visible token state is still intended to be part of the contract. If yes, we likely need an explicit wrapper-visible surface and shared coverage for it.

## Human Comment


SessionToken is part of API that some drivers exposed in the past. E.g. Python driver lets users access tokens via connection.rest.token (all publicly accessible even in Python's convention).

Moreover - some of our internal libraries use drivers simply to manage the token's lifecycle - e.g. SnowAPIs.

## Comment Answer Proposition

Good point. I initially assumed we could validate this through `ConnectionGetInfo`, but I don't think we should assume tokens are purely core-internal here. Some drivers exposed session-token access publicly in the past (for example Python via `connection.rest.token`), and some internal consumers like SnowAPIs use the driver mainly for token lifecycle management.

So I think the right next step is to make the contract explicit:

- if wrapper-visible token state is no longer intended, then this should move to core-only coverage
- if that visibility is still part of the expected contract, then we should keep shared coverage but back it with an explicit wrapper-visible API surface rather than an implicit assumption
