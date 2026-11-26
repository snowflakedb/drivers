---
category: breaking
---

- Changed the keys that a `customCredentialManager` receives to the format shared by all Snowflake drivers. Tokens stored by 3.x are not found under the new keys, so each user logs in once more after upgrading. (snowflakedb/drivers#3626)
