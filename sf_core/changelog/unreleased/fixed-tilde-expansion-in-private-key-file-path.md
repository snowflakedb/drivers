---
category: bugfix
---

- Fixed `private_key_file` not expanding a leading `~` to the user's home directory, so a path such as `~/.ssh/rsa_key.p8` in `connections.toml` failed to load. It now resolves like `token_file_path`, `connections_file` and PUT source paths, and like `private_key_file` in the legacy Python connector. (snowflakedb/drivers#1373)
