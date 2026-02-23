# Implement file based token_cache version.

## Rules

1. Implementation is always related to a token cache JSON file.
2. File can be accessed only when strict access rights are enforced - only the owner of the file can access it and no other user or group can even read the file.
3. File should be located in safe locations in the order defined by environment variables availability. See chapter Cache file locations below for rules.
4. Key of the token should be formed same way as for the keyring based implementation already present in the repo.
5. Key should be sha265 encrypted. 
6. Token should be written as plain text.
7. Token cache file should use the exact name: `credential_cache_v2.json`.


## Structure of the cache file:

```
{
  "tokens": {
    "<sha256-hash-key>": "<token-value>",
    ...
  }
}
```

## Cache file location (in priority order):
1. `$SF_TEMPORARY_CREDENTIAL_CACHE_DIR/credential_cache_v2.json`
2. `$XDG_CACHE_HOME/snowflake/credential_cache_v2.json`
3. `$HOME/.cache/snowflake/credential_cache_v2.json`
