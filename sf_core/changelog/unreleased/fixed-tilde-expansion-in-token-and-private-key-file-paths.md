---
category: bugfix
---

- Fixed `token_file_path` and `private_key_file` not expanding a leading `~` to the user's home directory, so paths such as `~/.snowflake/pat.token` in `connections.toml` failed to load. Both now resolve against the home directory, like `connections_file` and PUT source paths. (snowflakedb/drivers#1373)
