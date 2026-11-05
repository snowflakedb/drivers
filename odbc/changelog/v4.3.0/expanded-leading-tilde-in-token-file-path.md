---
category: feature
---

- Added support for a leading `~` in `token_file_path`, so a path such as `~/.snowflake/pat.token` is read from the home directory when loading a PAT, OAuth, or OIDC bearer token. (snowflakedb/drivers#3049)
