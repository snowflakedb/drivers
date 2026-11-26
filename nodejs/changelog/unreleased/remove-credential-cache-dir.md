---
category: breaking
---

- Removed the `credentialCacheDir` connection option. Tokens are stored in the system keyring when one is available, such as the macOS Keychain, Windows Credential Manager or a Linux keyring. Without a keyring, they are stored in a JSON file in the `SF_TEMPORARY_CREDENTIAL_CACHE_DIR` directory, or in `$XDG_CACHE_HOME/snowflake` or `~/.cache/snowflake` when it is not set. (snowflakedb/drivers#3626)
