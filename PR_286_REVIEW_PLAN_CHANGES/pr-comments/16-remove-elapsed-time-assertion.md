# #16 -- Remove redundant elapsed time assertion

**File**: `tests/definitions/core/session/logout.feature` line 60
**Reviewer**: boler

## Before

```gherkin
Scenario: should timeout after 5 seconds by default when server does not respond
  # Tests that default timeout is applied when no override provided
  # Mock server holds connection open (10s) to verify timeout interrupts after 5s
  Given Mock HTTP server holds connection open for 10 seconds without responding
  And UD Core connection is logged in with no timeout override
  When Logout is initiated
  Then Logout request times out after approximately 5 seconds
  And Close throws timeout error
  And Total elapsed time is between 5 and 6 seconds
```

## After

```gherkin
Scenario: should timeout after 5 seconds by default when server does not respond
  # Tests that default timeout is applied when no override provided
  # Mock server holds connection open (10s) to verify timeout interrupts after 5s
  Given Mock HTTP server holds connection open for 10 seconds without responding
  And UD Core connection is logged in with no timeout override
  When Logout is initiated
  Then Logout request times out after approximately 5 seconds
  And Close throws timeout error
```

## Rationale

1. **Low-value assertion**: lines 58-59 already express the intended behavior: the operation times out and `close()` surfaces a timeout-classified failure. The extra elapsed-time window adds little confidence beyond that semantic outcome.
2. **Flaky**: the 1-second window (`5s..6s`) is too tight for CI. The retry logic uses `std::time::Instant` (monotonic and consistent across threads, but not controllable by `tokio::time::pause()`), the runtime is multi-threaded, and scheduler / reqwest / socket jitter can easily push real completion outside that band.
3. **Precedent**: existing retry integration tests in `sf_core/tests/integration/http/retry.rs` assert error types and attempt counts, not narrow wall-clock durations.



## Comment Answer Proposition

I thought I had already applied this wall-clock cleanup locally, but it looks like the AI stashed those edits before pushing. My bad. I re-applied the change and kept the follow-up notes about the remaining timeout-budget concerns separate.

## Important limitation: this scenario does not cover token-refresh budget bugs

The worry about `SESSION_TOKEN_EXPIRED` starting a refresh flow, consuming time, and then incorrectly continuing retries or resetting the budget is **real**, but line 17 is still the wrong way to catch it.

Why:

1. This scenario's setup is `Mock HTTP server holds connection open ... without responding`. In the intended model, that means logout never receives a `390112` response, so the token-refresh path should not be exercised here at all.
2. Even if a broken implementation somehow spent too much total time elsewhere, a narrow `5s..6s` wall-clock band is a noisy signal. It can fail because of scheduler / socket jitter even when budget accounting is correct, and it still does not tell us **which** sub-path consumed the time.
3. The bug you are worried about is specifically **"total timeout budget must survive across refresh + retry"**, which deserves a dedicated scenario, not an incidental timing bound in an unrelated timeout test.

## Note on line 58

Line 58 (`Logout request times out after approximately 5 seconds`) is still time-shaped, so it should be reviewed together with `../self-noticed/20-verify-5s-default.md`.

Two separate questions are in play:

- `#16`: should we keep the explicit `5s..6s` elapsed assertion? **No.**
- `#20`: should the scenario keep asserting a **default 5-second** core timeout at all? **Needs verification.**

If we want protection against refresh-budget-reset bugs, add a targeted scenario like the one described in `../self-noticed/21-refresh-flow-time-budget.md`.

See the timing analysis in `PR_286_REVIEW_PLAN.md` for the full technical breakdown.







## Human Comment

I agree that such short window may be flaky and should be removed.

However, I would argue it is not true that TimeoutError is sufficient to feel confident that our flow respects the given retries and timeouts. If we had (as it used to be in the old driver) slight modifications to the retry algorithm in logout (in comparison to the normal query flow) or we added callbacks in case of Backend code SESSION_EXPIRED, then we could end up in a situation when 

1. control is given to the refresh token flow which then restarts the loop of main retries ignoring the already used attempts and time beforehand (actual bug that we have right now in UD logout implementation based on my analysis)

2. timeout is caused by splitting total timeout for all retries (i.e. division => total time / total attempts) - thus causing timeout very early and effectively not actually checking what we expect.

etc..



But i agree that this does not decrease the flakiness issue with this description, so I would instead rely more on probability -> I actually think that we should instead cover such issues with broad matrix of tests for timeouts -> if different variations of timeouts x attempts budget x injection of token expired return codes are described in tests, we should be able to detect most of those potential issues with the algorithm.



I tried to represent such aspects in: 

[Gherkin 1.]

[Gherkin 2.]

....





# Plan for AI:

1. Create a perfect plan that would cover all such ege cases. That plan consists of multiple gherkins with broad matrices - that would assure us timeouts work as expected. For now you will just give list of tuples (gherkin_name, what_gherkins_should_prove, rationale_for_way_of_testing). Each tuple describes one gherkin. Having all gherkins we should be sure that timeouts work correctly. 
E.g. 
 Gherkin 1 : should cover that all timeouts cause failure when expected (with given combinations of big delay, similar but a bit bigger timeout, big delay, way smaller timeout, big delay, a bit smaller timeout, big delay, way smaller timeout, humongous amount of retries, big delay, a bit smaller timeout, big amount of retries, ... very few retries). 
 Gherkin 2 : should cover that all timeouts occur always when on time even if there is token refresh (token refresh should have huge delay, normal logout should cause refresh, but answer immediately (so that timeout would not be raised if budget was counted only for the logout endpoint (which would be wrong behaviour as timeout should be total)))
 etc.

 2. Refine the plan - create an agent that will check it for the best testing practices, QA and software engineering knowledge and architectural as best proffesional.

 3. Compare with the current gherkins - what is minimal set of changes that we should introduce to achieve the same coverage. 

 4. Implement that + add comments based on the tuples - what is the rationale and why achieved that way.
