---
category: bugfix
---

- Fixed connection diagnostics so a Snowflake allowlist host that contains an underscore is probed as the hyphenated host, other allowlist hosts stay unchanged, and preserve_underscores_in_hostname still probes the underscored name. (snowflakedb/drivers#3047)
