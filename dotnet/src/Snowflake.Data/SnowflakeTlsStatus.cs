namespace Snowflake.Data;

/// <summary>
/// Reports the linked TLS crypto provider and build setting, not compliance of the entire driver artifact.
/// </summary>
public readonly struct SnowflakeTlsStatus
{
    internal SnowflakeTlsStatus(bool tlsProviderIsFips, bool fipsBuildEnabled)
    {
        TlsProviderIsFips = tlsProviderIsFips;
        FipsBuildEnabled = fipsBuildEnabled;
    }

    /// <summary>Whether Rustls reports the linked TLS crypto provider as FIPS.</summary>
    public bool TlsProviderIsFips { get; }

    /// <summary>Whether sf_core was built with the fips feature.</summary>
    public bool FipsBuildEnabled { get; }
}
