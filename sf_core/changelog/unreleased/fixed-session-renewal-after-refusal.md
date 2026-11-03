---
category: bugfix
---

- Fixed session renewal so a later request fails with the original refusal instead of contacting the token endpoint again after Snowflake has refused to renew the session. (snowflakedb/drivers#3145)
