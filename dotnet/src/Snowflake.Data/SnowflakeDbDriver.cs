using Snowflake.Data.Interop;
using Snowflake.Data.Proto;

namespace Snowflake.Data;

public sealed class SnowflakeDbDriver
{
    private readonly IDatabaseDriverService _driver;

    public SnowflakeDbDriver()
        : this(SfCoreTransport.Instance)
    {
    }

    internal SnowflakeDbDriver(ICoreTransport transport)
    {
        _driver = new DatabaseDriverServiceClient(transport);
    }

    /// <summary>Reports this build's linked TLS provider without opening a connection.</summary>
    public SnowflakeTlsStatus GetTlsStatus()
    {
        var response = _driver.DriverGetTlsStatus(new DriverGetTlsStatusRequest());
        return new SnowflakeTlsStatus(response.TlsProviderIsFips, response.FipsTlsBuildEnabled);
    }
}
