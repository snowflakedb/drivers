---
category: internal
---

- Changed the session-refresh limit from one refresh per context to one refresh per failure streak: a call that succeeds with the refreshed token ends the streak, and a later session-token expiry is refreshed again. The shared `execute_with_refresh` loop reports each success to the refresher, and a `SessionExpired` that is not refreshed is now logged as a warning. (snowflakedb/drivers#3312)
