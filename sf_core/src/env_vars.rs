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

/// Enable diagnostic file logging (layer-file) with no level filter.
/// When `"true"`, all log events (core and wrapper) are written to a file.
pub const SNOWFLAKE_TROUBLESHOOTING_ENABLED: &str = "SNOWFLAKE_TROUBLESHOOTING_ENABLED";

/// Directory for diagnostic log files when troubleshooting is enabled.
/// Defaults to the current working directory if unset.
pub const SNOWFLAKE_TROUBLESHOOTING_REPORT_PATH: &str = "SNOWFLAKE_TROUBLESHOOTING_REPORT_PATH";

// ---------------------------------------------------------------------------
// Azure Workload Identity Federation
//
// All of the variables below are set by the Azure runtime (App Service /
// Functions host, or the AKS Azure Workload Identity mutating webhook), not by
// the driver or the user.
// ---------------------------------------------------------------------------

/// Client ID of the user-assigned Managed Identity to request a token for.
/// Unset ⇒ the system-assigned identity is used.
pub const MANAGED_IDENTITY_CLIENT_ID: &str = "MANAGED_IDENTITY_CLIENT_ID";

/// Local Managed Identity token endpoint exposed by the Azure App Service /
/// Functions host. Paired with [`IDENTITY_HEADER`].
pub const IDENTITY_ENDPOINT: &str = "IDENTITY_ENDPOINT";

/// Shared secret required by the [`IDENTITY_ENDPOINT`] token endpoint, sent in
/// the `X-IDENTITY-HEADER` request header.
pub const IDENTITY_HEADER: &str = "IDENTITY_HEADER";

/// Legacy (pre-2019) equivalent of [`IDENTITY_ENDPOINT`], still set by older
/// Azure Functions runtimes.
pub const MSI_ENDPOINT: &str = "MSI_ENDPOINT";

/// Legacy (pre-2019) equivalent of [`IDENTITY_HEADER`].
pub const MSI_SECRET: &str = "MSI_SECRET";

/// AKS Workload Identity: client ID of the Entra ID application registration
/// that the pod's federated identity credential is bound to.
pub const AZURE_CLIENT_ID: &str = "AZURE_CLIENT_ID";

/// AKS Workload Identity: Entra ID tenant that issues the access token.
pub const AZURE_TENANT_ID: &str = "AZURE_TENANT_ID";

/// AKS Workload Identity: path to the projected Kubernetes service-account
/// token that is exchanged with Entra ID for an access token.
pub const AZURE_FEDERATED_TOKEN_FILE: &str = "AZURE_FEDERATED_TOKEN_FILE";

/// Additive escape hatch for the WORKLOAD_IDENTITY host allowlist: a
/// comma-separated list of extra hostname suffixes recognized alongside the
/// built-in `snowflakecomputing.com`/`.cn`/`.mil` suffixes. Read only from
/// the process environment, never from the DSN or connection parameters, so
/// connection configuration cannot influence the allowlist. Entries are
/// additive: they extend the recognized-host list and cannot disable it.
pub const SNOWFLAKE_WIF_ALLOWED_HOST_SUFFIXES: &str = "SNOWFLAKE_WIF_ALLOWED_HOST_SUFFIXES";
