---
category: bugfix
---

- Fixed OAuth login to retry temporary identity provider token endpoint failures: client-credentials and authorization-code requests retry timeouts, dropped connections, and HTTP 408, 429, and 5xx, while refresh-token requests, whose tokens may rotate, retry only failed connection attempts and HTTP 429 and 503, all within the authentication timeout. (snowflakedb/drivers#2878)
- Fixed OAuth authorization-code login to keep the cached refresh token when the refresh request fails for a temporary reason, and to leave half of the remaining authentication timeout for the browser login when the refresh stalls. (snowflakedb/drivers#2878)
- Changed OAuth authorization-code login to report that the first token request may have used the code when its retry is rejected with `invalid_grant`, instead of showing only `invalid_grant`. (snowflakedb/drivers#2878)
