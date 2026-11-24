---
category: internal
---

- Added a wrapper-supplied redirect-input provider to OAuth authorization code login, which core calls when the system browser cannot be opened and whose pasted URL it validates against the redirect URI before the state check and token exchange. (snowflakedb/drivers#3288)
