/// Snowflake home directory override — if set, replaces `~/.snowflake/`.
pub const SNOWFLAKE_HOME: &str = "SNOWFLAKE_HOME";

/// Name of the connection profile to use when no explicit connection details
/// are provided. Overrides `default_connection_name` in `config.toml`.
pub const SNOWFLAKE_DEFAULT_CONNECTION_NAME: &str = "SNOWFLAKE_DEFAULT_CONNECTION_NAME";

/// Set by Snowpark Container Services (SPCS) to signal the process is running
/// inside a container. Presence (any value) enables the SPCS auth side-channel
/// and auto-reading of the connection env vars below.
pub const SNOWFLAKE_RUNNING_INSIDE_SPCS: &str = "SNOWFLAKE_RUNNING_INSIDE_SPCS";

/// Connection parameters injected by SPCS into the container environment.
/// Only read when `SNOWFLAKE_RUNNING_INSIDE_SPCS` is set.
pub const SNOWFLAKE_ACCOUNT: &str = "SNOWFLAKE_ACCOUNT";
pub const SNOWFLAKE_HOST: &str = "SNOWFLAKE_HOST";
pub const SNOWFLAKE_DATABASE: &str = "SNOWFLAKE_DATABASE";
pub const SNOWFLAKE_SCHEMA: &str = "SNOWFLAKE_SCHEMA";

/// Suppress the permissions warning when reading config files with loose
/// filesystem permissions. Useful in CI environments.
pub const SF_SKIP_WARNING_FOR_READ_PERMISSIONS_ON_CONFIG_FILE: &str =
    "SF_SKIP_WARNING_FOR_READ_PERMISSIONS_ON_CONFIG_FILE";

/// Override directory for the token credential cache file.
pub const SF_TEMPORARY_CREDENTIAL_CACHE_DIR: &str = "SF_TEMPORARY_CREDENTIAL_CACHE_DIR";

/// Override filename (not path) for the token credential cache file.
pub const SF_TEMPORARY_CREDENTIAL_CACHE_FILE_NAME: &str = "SF_TEMPORARY_CREDENTIAL_CACHE_FILE_NAME";

/// XDG base directory for user-specific cache data, used as a fallback for
/// the credential cache path when `SF_TEMPORARY_CREDENTIAL_CACHE_DIR` is unset.
pub const XDG_CACHE_HOME: &str = "XDG_CACHE_HOME";

/// User home directory, used as a final fallback for the credential cache path.
pub const HOME: &str = "HOME";

/// Disable automatic platform / cloud-provider detection.
pub const SNOWFLAKE_DISABLE_PLATFORM_DETECTION: &str = "SNOWFLAKE_DISABLE_PLATFORM_DETECTION";

/// Opt into experimental platform detection (takes effect only when the
/// stable detection is also disabled).
pub const SNOWFLAKE_EXPERIMENTAL_ENABLE_PLATFORM_DETECTION: &str =
    "SNOWFLAKE_EXPERIMENTAL_ENABLE_PLATFORM_DETECTION";

/// Test-only override for the browser opener used during external-browser auth.
/// Set to `"noop"` to suppress actual browser launches in tests.
pub const SF_TEST_BROWSER_OPENER: &str = "SF_TEST_BROWSER_OPENER";
