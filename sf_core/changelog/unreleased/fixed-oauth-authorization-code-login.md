---
category: bugfix
---

- Fixed OAuth authorization-code login so a supplied client id is kept when no client secret is set, and the token request sends that client id without HTTP Basic authentication. When the client id is unset and Snowflake is the identity provider, the default local application client is still used. (snowflakedb/drivers#2876)
